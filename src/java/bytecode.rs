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
use ristretto_classfile::{ClassFile, Constant, ConstantPool};
use std::collections::HashMap;
use std::fmt::Write;

/// 将 .class 原始字节反汇编为结构化 class 数据
pub fn disassemble(bytes: &[u8]) -> Result<ClassStructure, String> {
    let cf = ClassFile::from_bytes(bytes).map_err(|e| format!("parse error: {e}"))?;
    let cp = &cf.constant_pool;
    let info = extract_class_info(&cf, cp, bytes);
    let fields = cf.fields.iter().map(|f| extract_field(f, cp)).collect();
    let methods = cf.methods.iter().map(|m| extract_method(m, cp)).collect();
    let cp_entries = extract_cp_entries(cp);
    Ok(ClassStructure {
        info,
        fields,
        methods,
        cp_entries,
    })
}

/// 从常量池提取所有条目用于 UI 展示
fn extract_cp_entries(cp: &ConstantPool) -> Vec<(u16, &'static str, String)> {
    let mut entries = Vec::new();
    for idx in 1..=cp.len() {
        let idx = idx as u16;
        let constant = match cp.try_get(idx) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (tag, value) = format_constant(constant, cp, idx);
        entries.push((idx, tag, value));
    }
    entries
}

/// 格式化单个常量池条目为 (类型标签, 可读值)
fn format_constant(c: &Constant, cp: &ConstantPool, idx: u16) -> (&'static str, String) {
    match c {
        Constant::Utf8(s) => ("Utf8", s.to_string()),
        Constant::Integer(v) => ("Integer", v.to_string()),
        Constant::Float(v) => ("Float", format!("{v}f")),
        Constant::Long(v) => ("Long", format!("{v}L")),
        Constant::Double(v) => ("Double", v.to_string()),
        Constant::Class(_) => {
            let name = cp
                .try_get_class(idx)
                .map(|s| s.to_string())
                .unwrap_or_default();
            ("Class", name)
        }
        Constant::String(_) => {
            let val = cp
                .try_get_formatted_string(idx)
                .unwrap_or_default()
                .strip_prefix("String ")
                .unwrap_or("")
                .to_string();
            ("String", format!("\"{val}\""))
        }
        Constant::FieldRef { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("FieldRef", val)
        }
        Constant::MethodRef { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("MethodRef", val)
        }
        Constant::InterfaceMethodRef { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("InterfaceMethodRef", val)
        }
        Constant::NameAndType { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("NameAndType", val)
        }
        Constant::MethodHandle { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("MethodHandle", val)
        }
        Constant::MethodType(_) => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("MethodType", val)
        }
        Constant::Dynamic { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("Dynamic", val)
        }
        Constant::InvokeDynamic { .. } => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("InvokeDynamic", val)
        }
        Constant::Module(_) => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("Module", val)
        }
        Constant::Package(_) => {
            let val = cp
                .try_get_formatted_string(idx)
                .map(|s| strip_cp_prefix(&s))
                .unwrap_or_default();
            ("Package", val)
        }
    }
}

// ── class info ──

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
    }
}

// ── field ──

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
    }
}

// ── method ──

fn extract_method(method: &ristretto_classfile::Method, cp: &ConstantPool) -> MethodInfo {
    let access = method.access_flags.as_code().to_string();
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
                // 从 LineNumberTable 构建 instruction_index → line_number 映射
                let mut line_map: HashMap<usize, u16> = HashMap::new();
                for code_attr in code_attrs {
                    if let Attribute::LineNumberTable { line_numbers, .. } = code_attr {
                        for ln in line_numbers {
                            line_map.insert(ln.start_pc as usize, ln.line_number);
                        }
                    }
                }
                // 输出 .catch 指令
                for entry in exception_table {
                    let catch_type = if entry.catch_type == 0 {
                        "*".to_string()
                    } else {
                        cp.try_get_class(entry.catch_type)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|_| format!("#{}", entry.catch_type))
                    };
                    writeln!(
                        bytecode,
                        ".catch {} {} {} {}",
                        catch_type, entry.range_pc.start, entry.range_pc.end, entry.handler_pc,
                    )
                    .unwrap();
                }
                // 输出指令，在对应位置插入 .line，逻辑行之间空一行
                let mut has_prev_insn = false;
                for (i, insn) in code.iter().enumerate() {
                    if let Some(&line) = line_map.get(&i) {
                        if has_prev_insn {
                            bytecode.push('\n');
                        }
                        if !bytecode.is_empty() && !bytecode.ends_with('\n') {
                            bytecode.push('\n');
                        }
                        writeln!(bytecode, ".line {line}").unwrap();
                    }
                    if !bytecode.is_empty() && !bytecode.ends_with('\n') {
                        bytecode.push('\n');
                    }
                    write!(bytecode, "{}", format_instruction(insn, &resolve)).unwrap();
                    has_prev_insn = true;
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
    }
}

// ── instruction formatting (Recaf style) ──

fn format_instruction(insn: &Instruction, resolve: &dyn Fn(u16) -> String) -> String {
    match insn {
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
        Instruction::Tableswitch(ts) => {
            let mut s = format!("TABLESWITCH {{ // {} to {}\n", ts.low, ts.high);
            for (i, offset) in ts.offsets.iter().enumerate() {
                s.push_str(&format!("    {}: {offset}\n", ts.low + i as i32));
            }
            s.push_str(&format!("    default: {}\n", ts.default));
            s.push('}');
            s
        }
        Instruction::Lookupswitch(ls) => {
            let mut s = String::from("LOOKUPSWITCH {\n");
            for (key, offset) in &ls.pairs {
                s.push_str(&format!("    {key}: {offset}\n"));
            }
            s.push_str(&format!("    default: {}\n", ls.default));
            s.push('}');
            s
        }
        _ => uppercase_opcode(&insn.to_string()),
    }
}

// ── annotation → EditableAnnotation ──

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

// ── CP 辅助 ──

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

/// 从 .class 字节构建反向 CP 查找表（stripped text → CP index）
///
/// 只收录可被指令直接引用的 CP 条目类型（Class, FieldRef, MethodRef 等），
/// 排除 Utf8 / NameAndType 等叶子条目，避免文本重复导致错误匹配。
pub fn build_cp_lookup(bytes: &[u8]) -> Result<Vec<(u16, String)>, String> {
    let cf = ClassFile::from_bytes(bytes).map_err(|e| format!("parse error: {e}"))?;
    Ok(build_cp_lookup_from_pool(&cf.constant_pool))
}

/// 从已有 ConstantPool 构建反向 CP 查找表
pub fn build_cp_lookup_from_pool(cp: &ConstantPool) -> Vec<(u16, String)> {
    let mut table = Vec::new();
    for idx in 1..=cp.len() {
        let idx = idx as u16;
        let is_operand_type = matches!(
            cp.try_get(idx),
            Ok(Constant::Integer(_)
                | Constant::Float(_)
                | Constant::Long(_)
                | Constant::Double(_)
                | Constant::Class(_)
                | Constant::String(_)
                | Constant::FieldRef { .. }
                | Constant::MethodRef { .. }
                | Constant::InterfaceMethodRef { .. }
                | Constant::MethodHandle { .. }
                | Constant::MethodType(_)
                | Constant::Dynamic { .. }
                | Constant::InvokeDynamic { .. })
        );
        if !is_operand_type {
            continue;
        }
        if let Ok(formatted) = cp.try_get_formatted_string(idx) {
            table.push((idx, strip_cp_prefix(&formatted)));
        }
    }
    table
}

/// 在查找表中搜索匹配的 CP 索引
pub fn resolve_from_lookup(table: &[(u16, String)], text: &str) -> Result<u16, String> {
    for (idx, stripped) in table {
        if stripped == text {
            return Ok(*idx);
        }
    }
    Err(format!("CP entry not found: {text}"))
}
