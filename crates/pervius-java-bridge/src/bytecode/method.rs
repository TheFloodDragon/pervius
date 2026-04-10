//! 方法级别元数据提取与字节码反汇编
//!
//! @author sky

use super::common::extract_common_attrs;
use super::descriptor::{escape_java_string, strip_cp_prefix};
use super::format::{
    collect_branch_targets, format_instruction, index_to_alpha_label, label_at,
    resolve_vars_in_instruction,
};
use super::{resolve_class, resolve_utf8};
use crate::class_structure::MethodInfo;
use ristretto_classfile::attributes::{Attribute, Instruction};
use ristretto_classfile::{Constant, ConstantPool, MethodAccessFlags};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write;
use std::io::Cursor;

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

/// 从 Method 提取方法信息（访问标记、签名、异常表、反汇编字节码等）
pub(super) fn extract_method(
    method: &ristretto_classfile::Method,
    cp: &ConstantPool,
) -> MethodInfo {
    let access = crate::save::format_method_flags(method.access_flags);
    let is_static = method.access_flags.contains(MethodAccessFlags::STATIC);
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
                    let formatted = match insn {
                        Instruction::Ldc(idx) => format_ldc(cp, u16::from(*idx), "LDC"),
                        Instruction::Ldc_w(idx) => format_ldc(cp, *idx, "LDC_W"),
                        Instruction::Ldc2_w(idx) => format_ldc(cp, *idx, "LDC2_W"),
                        _ => format_instruction(insn, i, &resolve, &labels),
                    };
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

/// LDC 系列指令格式化（需要常量池类型信息来添加后缀）
///
/// Float → `3.14f`，Long → `42L`，String → 带转义引号。
/// Integer / Double 不加后缀（它们是各自宽度的默认类型）。
fn format_ldc(cp: &ConstantPool, idx: u16, opcode: &str) -> String {
    match cp.try_get(idx) {
        Ok(Constant::Integer(v)) => format!("{opcode} {v}"),
        Ok(Constant::Float(v)) => format!("{opcode} {v}f"),
        Ok(Constant::Long(v)) => format!("{opcode} {v}L"),
        Ok(Constant::Double(v)) => format!("{opcode} {v}"),
        Ok(Constant::String(utf8_idx)) => {
            let s = cp
                .try_get_utf8(*utf8_idx)
                .map(|s| s.to_string())
                .unwrap_or_default();
            format!("{opcode} \"{}\"", escape_java_string(&s))
        }
        Ok(Constant::Class(name_idx)) => {
            let name = cp
                .try_get_utf8(*name_idx)
                .map(|s| s.to_string())
                .unwrap_or_else(|_| format!("#{name_idx}"));
            format!("{opcode} {name}")
        }
        _ => {
            let resolved = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_else(|_| format!("#{idx}"));
            format!("{opcode} {resolved}")
        }
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
