//! JVM class 文件反汇编：raw bytes → 结构化 class 数据
//!
//! 基于 ristretto_classfile 解析，输出 ClassStructure。
//! 操作码大写，CP 引用内联解析为可读名称（Recaf 风格）。
//!
//! @author sky

use super::class_structure::{
    AnnotationPair, ClassInfo, ClassStructure, EditableAnnotation, FieldInfo, MethodInfo,
};
use ristretto_classfile::attributes::{Annotation, AnnotationElement, Attribute, Instruction};
use ristretto_classfile::{ClassFile, ConstantPool};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::io::Cursor;

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
    let name = cp
        .try_get_class(cf.this_class)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{}", cf.this_class));
    let super_class = if cf.super_class == 0 {
        String::new()
    } else {
        cp.try_get_class(cf.super_class)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| format!("#{}", cf.super_class))
    };
    let interfaces: Vec<String> = cf
        .interfaces
        .iter()
        .map(|&idx| {
            cp.try_get_class(idx)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{idx}"))
        })
        .collect();
    let mut signature = None;
    let mut source_file = None;
    let mut annotations = Vec::new();
    let mut is_deprecated = false;
    for attr in &cf.attributes {
        match attr {
            Attribute::Signature {
                signature_index, ..
            } => {
                signature = cp
                    .try_get_utf8(*signature_index)
                    .ok()
                    .map(|s| s.to_string());
            }
            Attribute::SourceFile {
                source_file_index, ..
            } => {
                source_file = cp
                    .try_get_utf8(*source_file_index)
                    .ok()
                    .map(|s| s.to_string());
            }
            Attribute::Deprecated { .. } => {
                is_deprecated = true;
            }
            Attribute::RuntimeVisibleAnnotations {
                annotations: anns, ..
            }
            | Attribute::RuntimeInvisibleAnnotations {
                annotations: anns, ..
            } => {
                for ann in anns {
                    annotations.push(to_editable_annotation(ann, cp));
                }
            }
            _ => {}
        }
    }
    ClassInfo {
        version,
        access,
        name,
        super_class,
        interfaces,
        signature,
        source_file,
        annotations,
        is_deprecated,
        modified: false,
        saved: false,
    }
}

fn extract_field(field: &ristretto_classfile::Field, cp: &ConstantPool) -> FieldInfo {
    let access = field.access_flags.as_code().to_string();
    let name = cp
        .try_get_utf8(field.name_index)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{}", field.name_index));
    let descriptor = cp
        .try_get_utf8(field.descriptor_index)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{}", field.descriptor_index));
    let mut constant_value = None;
    let mut signature = None;
    let mut annotations = Vec::new();
    let mut is_deprecated = false;
    let mut is_synthetic = false;
    for attr in &field.attributes {
        match attr {
            Attribute::ConstantValue {
                constant_value_index,
                ..
            } => {
                constant_value = cp.try_get_formatted_string(*constant_value_index).ok();
            }
            Attribute::Signature {
                signature_index, ..
            } => {
                signature = cp
                    .try_get_utf8(*signature_index)
                    .ok()
                    .map(|s| s.to_string());
            }
            Attribute::Deprecated { .. } => {
                is_deprecated = true;
            }
            Attribute::Synthetic { .. } => {
                is_synthetic = true;
            }
            Attribute::RuntimeVisibleAnnotations {
                annotations: anns, ..
            }
            | Attribute::RuntimeInvisibleAnnotations {
                annotations: anns, ..
            } => {
                for ann in anns {
                    annotations.push(to_editable_annotation(ann, cp));
                }
            }
            _ => {}
        }
    }
    FieldInfo {
        access,
        name,
        descriptor,
        constant_value,
        signature,
        annotations,
        is_deprecated,
        is_synthetic,
        modified: false,
        saved: false,
    }
}

fn extract_method(method: &ristretto_classfile::Method, cp: &ConstantPool) -> MethodInfo {
    let access = method.access_flags.as_code().to_string();
    let is_static = access.contains("static");
    let name = cp
        .try_get_utf8(method.name_index)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{}", method.name_index));
    let descriptor = cp
        .try_get_utf8(method.descriptor_index)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{}", method.descriptor_index));
    let mut exceptions = Vec::new();
    let mut signature = None;
    let mut annotations = Vec::new();
    let mut is_deprecated = false;
    let mut is_synthetic = false;
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
                for &(_, _, _, start, end) in &local_vars {
                    targets.insert(start);
                    targets.insert(end);
                }
                for &(_, _, _, start, end) in &local_var_types {
                    targets.insert(start);
                    targets.insert(end);
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
                        cp.try_get_class(entry.catch_type)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|_| format!("#{}", entry.catch_type))
                    };
                    let from = label_at(&labels, entry.range_pc.start as usize);
                    let to = label_at(&labels, entry.range_pc.end as usize);
                    let handler = label_at(&labels, entry.handler_pc as usize);
                    writeln!(bytecode, ".catch {catch_type} {from} {to} {handler}").unwrap();
                }
                // 输出 .var（使用标签）
                for &(slot, name_idx, desc_idx, start, end) in &local_vars {
                    let name = cp
                        .try_get_utf8(name_idx)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| format!("#{name_idx}"));
                    let desc = cp
                        .try_get_utf8(desc_idx)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| format!("#{desc_idx}"));
                    let start_l = label_at(&labels, start);
                    let end_l = label_at(&labels, end);
                    writeln!(bytecode, ".var {slot} {name} {desc} {start_l} {end_l}").unwrap();
                }
                // 输出 .vartype（使用标签）
                for &(slot, name_idx, sig_idx, start, end) in &local_var_types {
                    let name = cp
                        .try_get_utf8(name_idx)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| format!("#{name_idx}"));
                    let sig = cp
                        .try_get_utf8(sig_idx)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|_| format!("#{sig_idx}"));
                    let start_l = label_at(&labels, start);
                    let end_l = label_at(&labels, end);
                    writeln!(bytecode, ".vartype {slot} {name} {sig} {start_l} {end_l}").unwrap();
                }
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
                    exceptions.push(
                        cp.try_get_class(idx)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|_| format!("#{idx}")),
                    );
                }
            }
            Attribute::Signature {
                signature_index, ..
            } => {
                signature = cp
                    .try_get_utf8(*signature_index)
                    .ok()
                    .map(|s| s.to_string());
            }
            Attribute::Deprecated { .. } => {
                is_deprecated = true;
            }
            Attribute::Synthetic { .. } => {
                is_synthetic = true;
            }
            Attribute::RuntimeVisibleAnnotations {
                annotations: anns, ..
            }
            | Attribute::RuntimeInvisibleAnnotations {
                annotations: anns, ..
            } => {
                for ann in anns {
                    annotations.push(to_editable_annotation(ann, cp));
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
        signature,
        annotations,
        is_deprecated,
        is_synthetic,
        bytecode,
        has_code,
        modified: false,
        saved: false,
    }
}

/// 收集指令中的分支目标索引（绝对指令索引）
fn collect_branch_targets(insn: &Instruction, insn_index: usize, targets: &mut BTreeSet<usize>) {
    match insn {
        Instruction::Ifeq(t)
        | Instruction::Ifne(t)
        | Instruction::Iflt(t)
        | Instruction::Ifge(t)
        | Instruction::Ifgt(t)
        | Instruction::Ifle(t)
        | Instruction::If_icmpeq(t)
        | Instruction::If_icmpne(t)
        | Instruction::If_icmplt(t)
        | Instruction::If_icmpge(t)
        | Instruction::If_icmpgt(t)
        | Instruction::If_icmple(t)
        | Instruction::If_acmpeq(t)
        | Instruction::If_acmpne(t)
        | Instruction::Goto(t)
        | Instruction::Jsr(t)
        | Instruction::Ifnull(t)
        | Instruction::Ifnonnull(t) => {
            targets.insert(*t as usize);
        }
        Instruction::Goto_w(t) | Instruction::Jsr_w(t) => {
            targets.insert(*t as usize);
        }
        Instruction::Tableswitch(ts) => {
            targets.insert((insn_index as i64 + ts.default as i64) as usize);
            for offset in &ts.offsets {
                targets.insert((insn_index as i64 + *offset as i64) as usize);
            }
        }
        Instruction::Lookupswitch(ls) => {
            targets.insert((insn_index as i64 + ls.default as i64) as usize);
            for (_, offset) in &ls.pairs {
                targets.insert((insn_index as i64 + *offset as i64) as usize);
            }
        }
        _ => {}
    }
}

/// 查找标签名，回退为数字
fn label_at(labels: &HashMap<usize, String>, idx: usize) -> String {
    labels.get(&idx).cloned().unwrap_or_else(|| idx.to_string())
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

fn format_instruction(
    insn: &Instruction,
    insn_index: usize,
    resolve: &dyn Fn(u16) -> String,
    labels: &HashMap<usize, String>,
) -> String {
    match insn {
        // CP 引用指令
        Instruction::Ldc(idx) => format!("LDC {}", resolve(u16::from(*idx))),
        Instruction::Ldc_w(idx) => format!("LDC_W {}", resolve(*idx)),
        Instruction::Ldc2_w(idx) => format!("LDC2_W {}", resolve(*idx)),
        Instruction::Getstatic(idx) => format!("GETSTATIC {}", resolve(*idx)),
        Instruction::Putstatic(idx) => format!("PUTSTATIC {}", resolve(*idx)),
        Instruction::Getfield(idx) => format!("GETFIELD {}", resolve(*idx)),
        Instruction::Putfield(idx) => format!("PUTFIELD {}", resolve(*idx)),
        Instruction::Invokevirtual(idx) => format!("INVOKEVIRTUAL {}", resolve(*idx)),
        Instruction::Invokespecial(idx) => format!("INVOKESPECIAL {}", resolve(*idx)),
        Instruction::Invokestatic(idx) => format!("INVOKESTATIC {}", resolve(*idx)),
        Instruction::Invokedynamic(idx) => format!("INVOKEDYNAMIC {}", resolve(*idx)),
        Instruction::Invokeinterface(idx, _) => format!("INVOKEINTERFACE {}", resolve(*idx)),
        Instruction::New(idx) => format!("NEW {}", resolve(*idx)),
        Instruction::Anewarray(idx) => format!("ANEWARRAY {}", resolve(*idx)),
        Instruction::Checkcast(idx) => format!("CHECKCAST {}", resolve(*idx)),
        Instruction::Instanceof(idx) => format!("INSTANCEOF {}", resolve(*idx)),
        Instruction::Multianewarray(idx, dims) => {
            format!("MULTIANEWARRAY {} {dims}", resolve(*idx))
        }
        // Branch 指令（绝对指令索引 → 标签）
        Instruction::Ifeq(t) => format!("IFEQ {}", label_at(labels, *t as usize)),
        Instruction::Ifne(t) => format!("IFNE {}", label_at(labels, *t as usize)),
        Instruction::Iflt(t) => format!("IFLT {}", label_at(labels, *t as usize)),
        Instruction::Ifge(t) => format!("IFGE {}", label_at(labels, *t as usize)),
        Instruction::Ifgt(t) => format!("IFGT {}", label_at(labels, *t as usize)),
        Instruction::Ifle(t) => format!("IFLE {}", label_at(labels, *t as usize)),
        Instruction::If_icmpeq(t) => format!("IF_ICMPEQ {}", label_at(labels, *t as usize)),
        Instruction::If_icmpne(t) => format!("IF_ICMPNE {}", label_at(labels, *t as usize)),
        Instruction::If_icmplt(t) => format!("IF_ICMPLT {}", label_at(labels, *t as usize)),
        Instruction::If_icmpge(t) => format!("IF_ICMPGE {}", label_at(labels, *t as usize)),
        Instruction::If_icmpgt(t) => format!("IF_ICMPGT {}", label_at(labels, *t as usize)),
        Instruction::If_icmple(t) => format!("IF_ICMPLE {}", label_at(labels, *t as usize)),
        Instruction::If_acmpeq(t) => format!("IF_ACMPEQ {}", label_at(labels, *t as usize)),
        Instruction::If_acmpne(t) => format!("IF_ACMPNE {}", label_at(labels, *t as usize)),
        Instruction::Goto(t) => format!("GOTO {}", label_at(labels, *t as usize)),
        Instruction::Jsr(t) => format!("JSR {}", label_at(labels, *t as usize)),
        Instruction::Ifnull(t) => format!("IFNULL {}", label_at(labels, *t as usize)),
        Instruction::Ifnonnull(t) => format!("IFNONNULL {}", label_at(labels, *t as usize)),
        Instruction::Goto_w(t) => format!("GOTO_W {}", label_at(labels, *t as usize)),
        Instruction::Jsr_w(t) => format!("JSR_W {}", label_at(labels, *t as usize)),
        // Switch 指令（相对指令偏移 → 标签）
        Instruction::Tableswitch(ts) => {
            let mut s = format!("TABLESWITCH {{ // {} to {}\n", ts.low, ts.high);
            for (i, offset) in ts.offsets.iter().enumerate() {
                let target = (insn_index as i64 + *offset as i64) as usize;
                s.push_str(&format!(
                    "    {}: {}\n",
                    ts.low + i as i32,
                    label_at(labels, target)
                ));
            }
            let default_target = (insn_index as i64 + ts.default as i64) as usize;
            s.push_str(&format!(
                "    default: {}\n",
                label_at(labels, default_target)
            ));
            s.push('}');
            s
        }
        Instruction::Lookupswitch(ls) => {
            let mut s = String::from("LOOKUPSWITCH {\n");
            for (key, offset) in &ls.pairs {
                let target = (insn_index as i64 + *offset as i64) as usize;
                s.push_str(&format!("    {key}: {}\n", label_at(labels, target)));
            }
            let default_target = (insn_index as i64 + ls.default as i64) as usize;
            s.push_str(&format!(
                "    default: {}\n",
                label_at(labels, default_target)
            ));
            s.push('}');
            s
        }
        _ => uppercase_opcode(&insn.to_string()),
    }
}

fn to_editable_annotation(ann: &Annotation, cp: &ConstantPool) -> EditableAnnotation {
    let type_desc = cp
        .try_get_utf8(ann.type_index)
        .map(|s| s.to_string())
        .unwrap_or_else(|_| format!("#{}", ann.type_index));
    let elements = ann
        .elements
        .iter()
        .map(|pair| {
            let name = cp
                .try_get_utf8(pair.name_index)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{}", pair.name_index));
            let (value, tag) = element_to_value(&pair.value, cp);
            AnnotationPair { name, value, tag }
        })
        .collect();
    EditableAnnotation {
        type_desc,
        elements,
    }
}

/// 将 AnnotationElement 转为 (格式化值, tag)
fn element_to_value(elem: &AnnotationElement, cp: &ConstantPool) -> (String, u8) {
    match elem {
        AnnotationElement::Boolean { const_value_index } => {
            let v = match cp.try_get_formatted_string(*const_value_index).as_deref() {
                Ok("0") => "false".to_string(),
                Ok("1") => "true".to_string(),
                Ok(v) => v.to_string(),
                Err(_) => format!("#{const_value_index}"),
            };
            (v, b'Z')
        }
        AnnotationElement::Byte { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'B',
        ),
        AnnotationElement::Char { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'C',
        ),
        AnnotationElement::Short { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'S',
        ),
        AnnotationElement::Int { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'I',
        ),
        AnnotationElement::Long { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'J',
        ),
        AnnotationElement::Float { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'F',
        ),
        AnnotationElement::Double { const_value_index } => (
            cp.try_get_formatted_string(*const_value_index)
                .unwrap_or_else(|_| format!("#{const_value_index}")),
            b'D',
        ),
        AnnotationElement::String { const_value_index } => {
            let s = cp
                .try_get_utf8(*const_value_index)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{const_value_index}"));
            (s, b's')
        }
        AnnotationElement::Enum {
            type_name_index,
            const_name_index,
        } => {
            let type_name = cp
                .try_get_utf8(*type_name_index)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{type_name_index}"));
            let const_name = cp
                .try_get_utf8(*const_name_index)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{const_name_index}"));
            (format!("{type_name}.{const_name}"), b'e')
        }
        AnnotationElement::Class { class_info_index } => {
            let s = cp
                .try_get_utf8(*class_info_index)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{class_info_index}"));
            (s, b'c')
        }
        AnnotationElement::Annotation { annotation } => {
            let inner = to_editable_annotation(annotation, cp);
            let desc = format_editable_annotation(&inner);
            (desc, b'@')
        }
        AnnotationElement::Array { values } => {
            let items: Vec<String> = values
                .iter()
                .map(|v| {
                    let (s, _) = element_to_value(v, cp);
                    s
                })
                .collect();
            (format!("{{{}}}", items.join(", ")), b'[')
        }
    }
}

/// EditableAnnotation → 显示字符串（嵌套注解值用）
fn format_editable_annotation(ann: &EditableAnnotation) -> String {
    let clean = ann
        .type_desc
        .strip_prefix('L')
        .and_then(|s| s.strip_suffix(';'))
        .unwrap_or(&ann.type_desc);
    if ann.elements.is_empty() {
        format!("@{clean}")
    } else {
        let pairs: Vec<String> = ann
            .elements
            .iter()
            .map(|p| format!("{} = {}", p.name, p.value))
            .collect();
        format!("@{clean}({})", pairs.join(", "))
    }
}

/// 去除 CP 格式化文本的类型前缀，返回纯引用名称
fn strip_cp_prefix(formatted: &str) -> String {
    let prefixes = [
        "Interface method ",
        "Method handle ",
        "Method type ",
        "Method ",
        "Field ",
        "Class ",
        "Name ",
        "Module ",
        "Package ",
    ];
    for prefix in prefixes {
        if let Some(rest) = formatted.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    // InvokeDynamic / Dynamic: #bsm_idx:name:descriptor → name + descriptor
    if let Some(rest) = formatted.strip_prefix("InvokeDynamic ") {
        return strip_bsm_ref(rest);
    }
    if let Some(rest) = formatted.strip_prefix("Dynamic ") {
        return strip_bsm_ref(rest);
    }
    // String 常量加引号
    if let Some(rest) = formatted.strip_prefix("String ") {
        return format!("\"{rest}\"");
    }
    formatted.to_string()
}

/// 去除 bootstrap method 前缀并保留索引：`#0:name:desc` → `#0 name desc`
fn strip_bsm_ref(s: &str) -> String {
    if let Some(rest) = s.strip_prefix('#') {
        if let Some(colon_pos) = rest.find(':') {
            let bsm_idx = &rest[..colon_pos];
            let remainder = rest[colon_pos + 1..].replacen(':', "", 1);
            return format!("#{bsm_idx} {remainder}");
        }
    }
    s.to_string()
}

/// 将指令文本的操作码部分转为大写
fn uppercase_opcode(line: &str) -> String {
    if let Some(idx) = line.find(' ') {
        let (opcode, rest) = line.split_at(idx);
        format!("{}{}", opcode.to_uppercase(), rest)
    } else {
        line.to_uppercase()
    }
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

/// 将索引转为字母标签（0→A, 1→B, ..., 25→Z, 26→AA, 27→AB, ...）
fn index_to_alpha_label(idx: usize) -> String {
    let mut s = String::new();
    let mut n = idx;
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s
}

/// 解析指令中的变量槽位为名称（ALOAD 0 → ALOAD this）
fn resolve_vars_in_instruction(
    formatted: &str,
    insn_idx: usize,
    resolved_vars: &[(u16, String, usize, usize)],
    is_static: bool,
) -> String {
    // 紧凑 LOAD/STORE: ALOAD_0 → ALOAD this
    let compact_prefixes = [
        "ALOAD_", "ILOAD_", "LLOAD_", "FLOAD_", "DLOAD_", "ASTORE_", "ISTORE_", "LSTORE_",
        "FSTORE_", "DSTORE_",
    ];
    for prefix in compact_prefixes {
        if formatted.starts_with(prefix) {
            if let Ok(digit) = formatted[prefix.len()..].parse::<u16>() {
                let op = &prefix[..prefix.len() - 1];
                let vname = resolve_slot(insn_idx, digit, resolved_vars, is_static);
                return format!("{op} {vname}");
            }
        }
    }
    // 参数化 LOAD/STORE: ALOAD 0 → ALOAD this
    let var_ops = [
        "ALOAD ", "ILOAD ", "LLOAD ", "FLOAD ", "DLOAD ", "ASTORE ", "ISTORE ", "LSTORE ",
        "FSTORE ", "DSTORE ", "RET ",
    ];
    for prefix in var_ops {
        if formatted.starts_with(prefix) {
            if let Ok(slot) = formatted[prefix.len()..].trim().parse::<u16>() {
                let vname = resolve_slot(insn_idx, slot, resolved_vars, is_static);
                return format!("{prefix}{vname}");
            }
        }
    }
    // IINC: "IINC 0, 1" → "IINC name, 1"
    if formatted.starts_with("IINC ") {
        let rest = &formatted[5..];
        if let Some(comma_pos) = rest.find(',') {
            if let Ok(slot) = rest[..comma_pos].trim().parse::<u16>() {
                let vname = resolve_slot(insn_idx, slot, resolved_vars, is_static);
                return format!("IINC {vname}{}", &rest[comma_pos..]);
            }
        }
    }
    formatted.to_string()
}

/// 解析槽位号为变量名
fn resolve_slot(
    insn_idx: usize,
    slot: u16,
    resolved_vars: &[(u16, String, usize, usize)],
    is_static: bool,
) -> String {
    if !is_static && slot == 0 {
        return "this".to_string();
    }
    for (vslot, vname, start, end) in resolved_vars {
        if *vslot == slot && insn_idx >= *start && insn_idx < *end {
            return vname.clone();
        }
    }
    slot.to_string()
}
