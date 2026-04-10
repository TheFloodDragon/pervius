//! JVM class 文件反汇编：raw bytes → 结构化 class 数据
//!
//! 基于 ristretto_classfile 解析，输出 ClassStructure。
//! 操作码大写，CP 引用内联解析为可读名称（Recaf 风格）。
//!
//! @author sky

mod annotation;
mod format;

use crate::class_structure::{
    ClassInfo, ClassStructure, EditableAnnotation, FieldInfo, MethodInfo,
};
use ristretto_classfile::attributes::{Attribute, Instruction};
use ristretto_classfile::{ClassFile, ConstantPool};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::io::Cursor;

use annotation::to_editable_annotation;
use format::{
    collect_branch_targets, format_instruction, index_to_alpha_label, label_at,
    resolve_vars_in_instruction, strip_cp_prefix,
};

/// 从常量池解析 UTF-8 条目，失败回退为 `#index`
fn resolve_utf8(cp: &ConstantPool, idx: u16) -> String {
    cp.try_get_utf8(idx)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{idx}"))
}

/// 从常量池解析 Class 条目，失败回退为 `#index`
fn resolve_class(cp: &ConstantPool, idx: u16) -> String {
    cp.try_get_class(idx)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{idx}"))
}

/// 从常量池解析格式化字符串，失败回退为 `#index`
fn resolve_const(cp: &ConstantPool, idx: u16) -> String {
    cp.try_get_formatted_string(idx)
        .unwrap_or_else(|_| format!("#{idx}"))
}

/// 从属性列表提取公共元数据（signature / deprecated / synthetic / annotations）
struct CommonAttrs {
    /// 泛型签名
    signature: Option<String>,
    /// 注解列表
    annotations: Vec<EditableAnnotation>,
    /// 是否标记 Deprecated
    is_deprecated: bool,
    /// 是否编译器生成
    is_synthetic: bool,
}

fn extract_common_attrs(attrs: &[Attribute], cp: &ConstantPool) -> CommonAttrs {
    let mut result = CommonAttrs {
        signature: None,
        annotations: Vec::new(),
        is_deprecated: false,
        is_synthetic: false,
    };
    for attr in attrs {
        match attr {
            Attribute::Signature {
                signature_index, ..
            } => {
                result.signature = cp
                    .try_get_utf8(*signature_index)
                    .ok()
                    .map(|s| s.to_string());
            }
            Attribute::Deprecated { .. } => result.is_deprecated = true,
            Attribute::Synthetic { .. } => result.is_synthetic = true,
            Attribute::RuntimeVisibleAnnotations {
                annotations: anns, ..
            }
            | Attribute::RuntimeInvisibleAnnotations {
                annotations: anns, ..
            } => {
                for ann in anns {
                    let type_desc = resolve_utf8(cp, ann.type_index);
                    if crate::KOTLIN_INTERNAL_ANNOTATIONS.contains(&type_desc.as_str()) {
                        continue;
                    }
                    result.annotations.push(to_editable_annotation(ann, cp));
                }
            }
            _ => {}
        }
    }
    result
}

/// 输出 .var 或 .vartype 指令
fn write_local_vars(
    bytecode: &mut String,
    vars: &[(u16, u16, u16, usize, usize)],
    cp: &ConstantPool,
    labels: &HashMap<usize, String>,
    directive: &str,
) {
    for &(slot, name_idx, desc_or_sig_idx, start, end) in vars {
        let name = resolve_utf8(cp, name_idx);
        let desc = resolve_utf8(cp, desc_or_sig_idx);
        let start_l = label_at(labels, start);
        let end_l = label_at(labels, end);
        writeln!(
            bytecode,
            ".{directive} {slot} {name} {desc} {start_l} {end_l}"
        )
        .unwrap();
    }
}

/// 将 .class 原始字节反汇编为结构化 class 数据
pub fn disassemble(bytes: &[u8]) -> Result<ClassStructure, String> {
    let cf = ClassFile::from_bytes(bytes).map_err(|e| format!("parse error: {e}"))?;
    let cp = &cf.constant_pool;
    let info = extract_class_info(&cf, cp, bytes);
    let fields = cf.fields.iter().map(|f| extract_field(f, cp)).collect();
    let methods = cf.methods.iter().map(|m| extract_method(m, cp)).collect();
    Ok(ClassStructure {
        info,
        fields,
        methods,
    })
}

fn extract_class_info(cf: &ClassFile, cp: &ConstantPool, bytes: &[u8]) -> ClassInfo {
    let version = parse_class_version(bytes).unwrap_or_default();
    let access = cf.access_flags.as_code().to_string();
    let name = resolve_class(cp, cf.this_class);
    let super_class = if cf.super_class == 0 {
        String::new()
    } else {
        resolve_class(cp, cf.super_class)
    };
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .map(|&idx| resolve_class(cp, idx))
        .collect();
    let common = extract_common_attrs(&cf.attributes, cp);
    let mut source_file = None;
    for attr in &cf.attributes {
        if let Attribute::SourceFile {
            source_file_index, ..
        } = attr
        {
            source_file = cp
                .try_get_utf8(*source_file_index)
                .ok()
                .map(|s| s.to_string());
        }
    }
    ClassInfo {
        version,
        access,
        name,
        super_class,
        interfaces,
        signature: common.signature,
        source_file,
        annotations: common.annotations,
        is_deprecated: common.is_deprecated,
        modified: false,
        saved: false,
    }
}

fn extract_field(field: &ristretto_classfile::Field, cp: &ConstantPool) -> FieldInfo {
    let access = field.access_flags.as_code().to_string();
    let name = resolve_utf8(cp, field.name_index);
    let descriptor = resolve_utf8(cp, field.descriptor_index);
    let common = extract_common_attrs(&field.attributes, cp);
    let mut constant_value = None;
    for attr in &field.attributes {
        if let Attribute::ConstantValue {
            constant_value_index,
            ..
        } = attr
        {
            constant_value = cp.try_get_formatted_string(*constant_value_index).ok();
        }
    }
    FieldInfo {
        access,
        name,
        descriptor,
        constant_value,
        signature: common.signature,
        annotations: common.annotations,
        is_deprecated: common.is_deprecated,
        is_synthetic: common.is_synthetic,
        modified: false,
        saved: false,
    }
}

fn extract_method(method: &ristretto_classfile::Method, cp: &ConstantPool) -> MethodInfo {
    let access = method.access_flags.as_code().to_string();
    let is_static = access.contains("static");
    let name = resolve_utf8(cp, method.name_index);
    let descriptor = resolve_utf8(cp, method.descriptor_index);
    let common = extract_common_attrs(&method.attributes, cp);
    let mut exceptions = Vec::new();
    let mut bytecode = String::new();
    let mut has_code = false;
    let resolve = |idx: u16| -> String {
        // FieldRef: ristretto 的 try_get_formatted_string 丢弃描述符，需手动拼接 "owner.name descriptor"
        if let Ok((class_idx, nat_idx)) = cp.try_get_field_ref(idx) {
            if let (Ok(class_name), Ok((name_idx, desc_idx))) = (
                cp.try_get_class(*class_idx),
                cp.try_get_name_and_type(*nat_idx),
            ) {
                let fname = cp
                    .try_get_utf8(*name_idx)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let fdesc = cp
                    .try_get_utf8(*desc_idx)
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                return format!("{class_name}.{fname} {fdesc}");
            }
        }
        cp.try_get_formatted_string(idx)
            .map(|s| strip_cp_prefix(&s))
            .unwrap_or_else(|_| format!("#{idx}"))
    };
    for attr in &method.attributes {
        match attr {
            Attribute::Code {
                code,
                exception_table,
                attributes: code_attrs,
                ..
            } => {
                has_code = true;
                // 构建 byte_offset → instruction_index 映射（LVT/LVTT 需要）
                let byte_to_insn = build_byte_to_insn_map(code);
                // 从 code 子属性提取元数据
                let mut line_map: HashMap<usize, u16> = HashMap::new();
                // LVT/LVTT 存储为 (slot, name_idx, desc/sig_idx, start_insn_idx, end_insn_idx)
                let mut local_vars: Vec<(u16, u16, u16, usize, usize)> = Vec::new();
                let mut local_var_types: Vec<(u16, u16, u16, usize, usize)> = Vec::new();
                for code_attr in code_attrs {
                    match code_attr {
                        Attribute::LineNumberTable { line_numbers, .. } => {
                            for ln in line_numbers {
                                line_map.insert(ln.start_pc as usize, ln.line_number);
                            }
                        }
                        Attribute::LocalVariableTable { variables, .. } => {
                            for v in variables {
                                // ristretto 不转换 LVT 偏移，start_pc/length 仍是字节偏移
                                let start_insn =
                                    lookup_byte_offset(&byte_to_insn, v.start_pc as usize);
                                let end_insn = lookup_byte_offset(
                                    &byte_to_insn,
                                    (v.start_pc + v.length) as usize,
                                );
                                local_vars.push((
                                    v.index,
                                    v.name_index,
                                    v.descriptor_index,
                                    start_insn,
                                    end_insn,
                                ));
                            }
                        }
                        Attribute::LocalVariableTypeTable { variable_types, .. } => {
                            for v in variable_types {
                                let start_insn =
                                    lookup_byte_offset(&byte_to_insn, v.start_pc as usize);
                                let end_insn = lookup_byte_offset(
                                    &byte_to_insn,
                                    (v.start_pc + v.length) as usize,
                                );
                                local_var_types.push((
                                    v.index,
                                    v.name_index,
                                    v.signature_index,
                                    start_insn,
                                    end_insn,
                                ));
                            }
                        }
                        _ => {}
                    }
                }
                // 收集所有需要标签的指令索引
                let mut targets = BTreeSet::new();
                for (i, insn) in code.iter().enumerate() {
                    collect_branch_targets(insn, i, &mut targets);
                }
                for entry in exception_table {
                    targets.insert(entry.range_pc.start as usize);
                    targets.insert(entry.range_pc.end as usize);
                    targets.insert(entry.handler_pc as usize);
                }
                for vars in [&local_vars, &local_var_types] {
                    for &(_, _, _, start, end) in vars {
                        targets.insert(start);
                        targets.insert(end);
                    }
                }
                // 行号位置也需要标签（LINE 指令引用）
                for &idx in line_map.keys() {
                    targets.insert(idx);
                }
                // 分配标签 A, B, C, ..., Z, AA, AB, ...
                let labels: HashMap<usize, String> = targets
                    .iter()
                    .enumerate()
                    .map(|(i, &idx)| (idx, index_to_alpha_label(i)))
                    .collect();
                // 变量名解析表（用于 LOAD/STORE 指令的变量名替换）
                let resolved_vars: Vec<(u16, String, usize, usize)> = local_vars
                    .iter()
                    .map(|&(slot, name_idx, _, start, end)| {
                        let vname = cp
                            .try_get_utf8(name_idx)
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        (slot, vname, start, end)
                    })
                    .collect();
                // 输出 .catch（使用标签）
                for entry in exception_table {
                    let catch_type = if entry.catch_type == 0 {
                        "*".to_string()
                    } else {
                        resolve_class(cp, entry.catch_type)
                    };
                    let from = label_at(&labels, entry.range_pc.start as usize);
                    let to = label_at(&labels, entry.range_pc.end as usize);
                    let handler = label_at(&labels, entry.handler_pc as usize);
                    writeln!(bytecode, ".catch {catch_type} {from} {to} {handler}").unwrap();
                }
                // 输出 .var 和 .vartype（使用标签）
                write_local_vars(&mut bytecode, &local_vars, cp, &labels, "var");
                write_local_vars(&mut bytecode, &local_var_types, cp, &labels, "vartype");
                // 输出指令（标签 + 行号 + 指令）
                let has_header = !bytecode.is_empty();
                for (i, insn) in code.iter().enumerate() {
                    let has_label = labels.contains_key(&i);
                    let has_line = line_map.contains_key(&i);
                    if (has_label || has_line) && (has_header || i > 0) {
                        bytecode.push('\n');
                    }
                    if let Some(label) = labels.get(&i) {
                        writeln!(bytecode, "{label}:").unwrap();
                    }
                    if let Some(&line) = line_map.get(&i) {
                        let label_ref = labels.get(&i).cloned().unwrap_or_else(|| i.to_string());
                        writeln!(bytecode, "LINE {label_ref} {line}").unwrap();
                    }
                    let formatted = format_instruction(insn, i, &resolve, &labels);
                    let with_vars =
                        resolve_vars_in_instruction(&formatted, i, &resolved_vars, is_static);
                    writeln!(bytecode, "{}", with_vars).unwrap();
                }
                // 尾标签（作用域结束点可能超出指令范围）
                if let Some(label) = labels.get(&code.len()) {
                    bytecode.push('\n');
                    write!(bytecode, "{label}:").unwrap();
                }
                // 去除尾部换行
                while bytecode.ends_with('\n') {
                    bytecode.pop();
                }
            }
            Attribute::Exceptions {
                exception_indexes, ..
            } => {
                for &idx in exception_indexes {
                    exceptions.push(resolve_class(cp, idx));
                }
            }
            _ => {}
        }
    }
    MethodInfo {
        access,
        name,
        descriptor,
        exceptions,
        signature: common.signature,
        annotations: common.annotations,
        is_deprecated: common.is_deprecated,
        is_synthetic: common.is_synthetic,
        bytecode,
        has_code,
        modified: false,
        saved: false,
    }
}

/// 从指令列表构建 byte_offset → instruction_index 映射
///
/// ristretto 不转换 LVT/LVTT 的偏移量（仅转换 LineNumberTable/StackMapTable/ExceptionTable），
/// 所以 LVT 中的 start_pc 和 length 仍是字节偏移。需要手动转换为指令索引。
fn build_byte_to_insn_map(code: &[Instruction]) -> BTreeMap<usize, usize> {
    let mut map = BTreeMap::new();
    let mut cursor = Cursor::new(Vec::new());
    for (insn_idx, insn) in code.iter().enumerate() {
        let pos = cursor.position() as usize;
        map.insert(pos, insn_idx);
        let _ = insn.to_bytes(&mut cursor);
    }
    // 尾部哨兵：指向指令列表末尾（用于 start_pc + length 的结束位置）
    map.insert(cursor.position() as usize, code.len());
    map
}

/// 在 byte→insn 映射中查找最近的指令索引（<= byte_offset）
fn lookup_byte_offset(map: &BTreeMap<usize, usize>, byte_offset: usize) -> usize {
    map.get(&byte_offset)
        .copied()
        .or_else(|| map.range(..=byte_offset).next_back().map(|(_, &v)| v))
        .unwrap_or(0)
}

/// 从 .class 字节解析版本信息
fn parse_class_version(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 || bytes[0..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let minor = u16::from_be_bytes([bytes[4], bytes[5]]);
    let major = u16::from_be_bytes([bytes[6], bytes[7]]);
    let java_ver = if major >= 49 {
        format!("{}", major - 44)
    } else {
        format!("1.{}", major - 44)
    };
    Some(format!("Java {java_ver} (class {major}.{minor})"))
}
