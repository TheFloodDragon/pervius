//! ClassStructure → class 字节写回
//!
//! 将 UI 层编辑后的 ClassStructure 应用到原始 class 字节，
//! 生成新的 class 文件字节序列。
//!
//! @author sky

use super::assembler;
use super::bytecode;
use super::class_structure::ClassStructure;
use ristretto_classfile::attributes::{AnnotationElement, AnnotationValuePair, Attribute};
use ristretto_classfile::{
    ClassAccessFlags, ClassFile, ConstantPool, FieldAccessFlags, MethodAccessFlags,
};

/// 将编辑后的 ClassStructure 应用到原始 class 字节，返回新的 class 字节
pub fn apply_structure(raw_bytes: &[u8], cs: &ClassStructure) -> Result<Vec<u8>, String> {
    let mut cf = ClassFile::from_bytes(raw_bytes).map_err(|e| format!("parse error: {e}"))?;
    // 先反汇编一份原始字节码用于比对（跳过未修改的方法）
    let original = bytecode::disassemble(raw_bytes).ok();
    if let Some(ref orig) = original {
        log::debug!(
            "apply_structure: class={}, orig_access={}, new_access={}",
            orig.info.name,
            orig.info.access,
            cs.info.access,
        );
        for (i, (om, nm)) in orig.methods.iter().zip(cs.methods.iter()).enumerate() {
            if om.bytecode != nm.bytecode {
                log::debug!(
                    "  method[{i}] {}.{} bytecode CHANGED",
                    nm.name,
                    nm.descriptor
                );
            }
            if om.name != nm.name {
                log::debug!("  method[{i}] name CHANGED: {} -> {}", om.name, nm.name);
            }
            if om.access != nm.access {
                log::debug!(
                    "  method[{i}] access CHANGED: {} -> {}",
                    om.access,
                    nm.access
                );
            }
        }
        for (i, (of, nf)) in orig.fields.iter().zip(cs.fields.iter()).enumerate() {
            if of.name != nf.name {
                log::debug!("  field[{i}] name CHANGED: {} -> {}", of.name, nf.name);
            }
            if of.access != nf.access {
                log::debug!(
                    "  field[{i}] access CHANGED: {} -> {}",
                    of.access,
                    nf.access
                );
            }
        }
    }
    apply_class_info(&mut cf, cs);
    apply_fields(&mut cf, cs);
    // class info / fields 修改可能新增 CP 条目，用修改后的 CP 构建 lookup
    let lookup = bytecode::build_cp_lookup_from_pool(&cf.constant_pool);
    apply_methods(
        &mut cf.methods,
        &mut cf.constant_pool,
        cs,
        &lookup,
        original.as_ref(),
    )?;
    let mut buf = Vec::new();
    cf.to_bytes(&mut buf)
        .map_err(|e| format!("serialize error: {e}"))?;
    log::debug!(
        "apply_structure: {} -> {} bytes",
        raw_bytes.len(),
        buf.len()
    );
    Ok(buf)
}

// ── class info ──

fn apply_class_info(cf: &mut ClassFile, cs: &ClassStructure) {
    cf.access_flags = parse_class_flags(&cs.info.access);
    let cp = &mut cf.constant_pool;
    // 类名
    let name_idx = find_or_add_utf8(cp, &cs.info.name);
    let class_idx = find_or_add_class(cp, name_idx);
    cf.this_class = class_idx;
    // 父类
    if cs.info.super_class.is_empty() {
        cf.super_class = 0;
    } else {
        let sc_name = find_or_add_utf8(cp, &cs.info.super_class);
        cf.super_class = find_or_add_class(cp, sc_name);
    }
    // 接口
    cf.interfaces = cs
        .info
        .interfaces
        .iter()
        .map(|iface| {
            let n = find_or_add_utf8(cp, iface);
            find_or_add_class(cp, n)
        })
        .collect();
    // 注解
    apply_annotations(&mut cf.attributes, cp, &cs.info.annotations);
}

// ── fields ──

fn apply_fields(cf: &mut ClassFile, cs: &ClassStructure) {
    for (field, fi) in cf.fields.iter_mut().zip(cs.fields.iter()) {
        field.access_flags = parse_field_flags(&fi.access);
        let cp = &mut cf.constant_pool;
        field.name_index = find_or_add_utf8(cp, &fi.name);
        field.descriptor_index = find_or_add_utf8(cp, &fi.descriptor);
        apply_annotations(&mut field.attributes, cp, &fi.annotations);
    }
}

// ── methods ──

fn apply_methods(
    methods: &mut [ristretto_classfile::Method],
    cp: &mut ConstantPool,
    cs: &ClassStructure,
    lookup: &[(u16, String)],
    original: Option<&ClassStructure>,
) -> Result<(), String> {
    for (i, (method, mi)) in methods.iter_mut().zip(cs.methods.iter()).enumerate() {
        method.access_flags = parse_method_flags(&mi.access);
        method.name_index = find_or_add_utf8(cp, &mi.name);
        method.descriptor_index = find_or_add_utf8(cp, &mi.descriptor);
        // 字节码没改过的方法保持原样，不走重新汇编
        if mi.has_code && !mi.bytecode.is_empty() {
            let bytecode_changed = original
                .and_then(|o| o.methods.get(i))
                .map(|om| om.bytecode != mi.bytecode)
                .unwrap_or(false);
            if bytecode_changed {
                let mut resolve = |text: &str| -> Result<u16, String> {
                    if let Ok(idx) = bytecode::resolve_from_lookup(lookup, text) {
                        return Ok(idx);
                    }
                    create_cp_entry(cp, text)
                };
                match assembler::assemble_instructions(&mi.bytecode, &mut resolve) {
                    Ok(result) => {
                        for attr in &mut method.attributes {
                            if let Attribute::Code {
                                code,
                                exception_table,
                                attributes,
                                ..
                            } = attr
                            {
                                *code = result.instructions;
                                *exception_table = result.exception_table;
                                // StackMapTable 旧索引已失效，必须移除
                                attributes.retain(|a| {
                                    !matches!(
                                        a,
                                        Attribute::LineNumberTable { .. }
                                            | Attribute::StackMapTable { .. }
                                    )
                                });
                                // 从汇编结果重建 LineNumberTable
                                if !result.line_numbers.is_empty() {
                                    let name_idx = find_or_add_utf8(cp, "LineNumberTable");
                                    attributes.push(Attribute::LineNumberTable {
                                        name_index: name_idx,
                                        line_numbers: result.line_numbers,
                                    });
                                }
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!(
                            "Skipped bytecode reassembly for {}.{}: {e}",
                            mi.name,
                            mi.descriptor
                        );
                    }
                }
            }
        }
        apply_annotations(&mut method.attributes, cp, &mi.annotations);
    }
    Ok(())
}

// ── CP entry auto-creation ──

/// lookup 找不到时，根据文本格式自动创建对应的 CP 条目
fn create_cp_entry(cp: &mut ConstantPool, text: &str) -> Result<u16, String> {
    // String 常量: "..."
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        let s = &text[1..text.len() - 1];
        return cp.add_string(s).map_err(|e| format!("{e}"));
    }
    // MethodRef: class/Name.method(desc)ret — 含 '(' 说明是方法引用
    if let Some(paren) = text.find('(') {
        let before_paren = &text[..paren];
        let descriptor = &text[paren..];
        if let Some(dot) = before_paren.rfind('.') {
            let class_name = &before_paren[..dot];
            let method_name = &before_paren[dot + 1..];
            let class_idx = cp.add_class(class_name).map_err(|e| format!("{e}"))?;
            return cp
                .add_method_ref(class_idx, method_name, descriptor)
                .map_err(|e| format!("{e}"));
        }
    }
    // FieldRef: class/Name.field descriptor — 含 '.' 且 '.' 后有空格
    if let Some(dot) = text.rfind('.') {
        let class_name = &text[..dot];
        let after_dot = &text[dot + 1..];
        if let Some(space) = after_dot.find(' ') {
            let field_name = &after_dot[..space];
            let descriptor = after_dot[space + 1..].trim();
            let class_idx = cp.add_class(class_name).map_err(|e| format!("{e}"))?;
            return cp
                .add_field_ref(class_idx, field_name, descriptor)
                .map_err(|e| format!("{e}"));
        }
    }
    // Class 引用（含 '/'）
    if text.contains('/') {
        return cp.add_class(text).map_err(|e| format!("{e}"));
    }
    // Integer
    if let Ok(v) = text.parse::<i32>() {
        return cp.add_integer(v).map_err(|e| format!("{e}"));
    }
    // Long (后缀 L)
    if let Some(s) = text.strip_suffix('L').or_else(|| text.strip_suffix('l')) {
        if let Ok(v) = s.parse::<i64>() {
            return cp.add_long(v).map_err(|e| format!("{e}"));
        }
    }
    // Float (后缀 f)
    if let Some(s) = text.strip_suffix('f').or_else(|| text.strip_suffix('F')) {
        if let Ok(v) = s.parse::<f32>() {
            return cp.add_float(v).map_err(|e| format!("{e}"));
        }
    }
    // Double
    if let Ok(v) = text.parse::<f64>() {
        return cp.add_double(v).map_err(|e| format!("{e}"));
    }
    Err(format!("Cannot resolve CP entry: {text}"))
}

// ── annotations ──

fn apply_annotations(
    attrs: &mut Vec<Attribute>,
    cp: &mut ConstantPool,
    editable: &[super::class_structure::EditableAnnotation],
) {
    // 只更新 RuntimeVisibleAnnotations，跳过 Kotlin 内部注解
    for attr in attrs.iter_mut() {
        let anns = match attr {
            Attribute::RuntimeVisibleAnnotations { annotations, .. } => annotations,
            _ => continue,
        };
        // 按位置对齐更新（跳过 Kotlin 内部注解）
        let mut editable_iter = editable.iter();
        for ann in anns.iter_mut() {
            let type_str = cp
                .try_get_utf8(ann.type_index)
                .map(|s| s.to_string())
                .unwrap_or_default();
            // 跳过 Kotlin 内部注解（不可编辑，不消耗 editable 迭代器）
            if type_str == "Lkotlin/Metadata;"
                || type_str == "Lkotlin/jvm/internal/SourceDebugExtension;"
            {
                continue;
            }
            if let Some(ea) = editable_iter.next() {
                ann.type_index = find_or_add_utf8(cp, &ea.type_desc);
                ann.elements = ea
                    .elements
                    .iter()
                    .map(|pair| {
                        let name_index = find_or_add_utf8(cp, &pair.name);
                        let value = build_annotation_value(cp, &pair.value, pair.tag);
                        AnnotationValuePair { name_index, value }
                    })
                    .collect();
            }
        }
        break;
    }
}

/// 根据 tag 和文本值构建 AnnotationElement
fn build_annotation_value(cp: &mut ConstantPool, value: &str, tag: u8) -> AnnotationElement {
    match tag {
        b's' => {
            // 字符串常量：去引号
            let unquoted = value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(value);
            let idx = find_or_add_utf8(cp, unquoted);
            AnnotationElement::String {
                const_value_index: idx,
            }
        }
        b'I' => {
            let v: i32 = value.parse().unwrap_or(0);
            let idx = cp.add_integer(v).unwrap_or(0);
            AnnotationElement::Int {
                const_value_index: idx,
            }
        }
        b'Z' => {
            let v: i32 = if value == "true" { 1 } else { 0 };
            let idx = cp.add_integer(v).unwrap_or(0);
            AnnotationElement::Boolean {
                const_value_index: idx,
            }
        }
        b'e' => {
            // enum: "Type.NAME" 格式
            let (type_name, const_name) = value.split_once('.').unwrap_or((value, ""));
            let type_idx = find_or_add_utf8(cp, type_name);
            let name_idx = find_or_add_utf8(cp, const_name);
            AnnotationElement::Enum {
                type_name_index: type_idx,
                const_name_index: name_idx,
            }
        }
        b'c' => {
            let idx = find_or_add_utf8(cp, value);
            AnnotationElement::Class {
                class_info_index: idx,
            }
        }
        // 其他 tag（array, nested annotation 等）暂不支持编辑，原样返回 string
        _ => {
            let idx = find_or_add_utf8(cp, value);
            AnnotationElement::String {
                const_value_index: idx,
            }
        }
    }
}

// ── constant pool helpers ──

/// 查找已有的 UTF8 条目或新增
fn find_or_add_utf8(cp: &mut ConstantPool, text: &str) -> u16 {
    for idx in 1..=cp.len() {
        let idx = idx as u16;
        if let Ok(s) = cp.try_get_utf8(idx) {
            if s == text {
                return idx;
            }
        }
    }
    cp.add_utf8(text).unwrap_or(0)
}

/// 查找指向 name_index 的 Class 条目或新增
fn find_or_add_class(cp: &mut ConstantPool, name_index: u16) -> u16 {
    use ristretto_classfile::Constant;
    for idx in 1..=cp.len() {
        let idx = idx as u16;
        if let Ok(Constant::Class(ni)) = cp.try_get(idx) {
            if *ni == name_index {
                return idx;
            }
        }
    }
    cp.add(Constant::Class(name_index)).unwrap_or(0)
}

// ── access flags parsing ──

fn parse_class_flags(s: &str) -> ClassAccessFlags {
    let mut flags = ClassAccessFlags::empty();
    for word in s.split_whitespace() {
        flags |= match word {
            "public" => ClassAccessFlags::PUBLIC,
            "final" => ClassAccessFlags::FINAL,
            "super" => ClassAccessFlags::SUPER,
            "interface" => ClassAccessFlags::INTERFACE,
            "abstract" => ClassAccessFlags::ABSTRACT,
            "synthetic" => ClassAccessFlags::SYNTHETIC,
            "annotation" => ClassAccessFlags::ANNOTATION,
            "enum" => ClassAccessFlags::ENUM,
            "module" => ClassAccessFlags::MODULE,
            // as_code() 输出 "class" 但 ClassAccessFlags 没有对应 bit
            _ => ClassAccessFlags::empty(),
        };
    }
    flags
}

fn parse_field_flags(s: &str) -> FieldAccessFlags {
    let mut flags = FieldAccessFlags::empty();
    for word in s.split_whitespace() {
        flags |= match word {
            "public" => FieldAccessFlags::PUBLIC,
            "private" => FieldAccessFlags::PRIVATE,
            "protected" => FieldAccessFlags::PROTECTED,
            "static" => FieldAccessFlags::STATIC,
            "final" => FieldAccessFlags::FINAL,
            "volatile" => FieldAccessFlags::VOLATILE,
            "transient" => FieldAccessFlags::TRANSIENT,
            "synthetic" => FieldAccessFlags::SYNTHETIC,
            "enum" => FieldAccessFlags::ENUM,
            _ => FieldAccessFlags::empty(),
        };
    }
    flags
}

fn parse_method_flags(s: &str) -> MethodAccessFlags {
    let mut flags = MethodAccessFlags::empty();
    for word in s.split_whitespace() {
        flags |= match word {
            "public" => MethodAccessFlags::PUBLIC,
            "private" => MethodAccessFlags::PRIVATE,
            "protected" => MethodAccessFlags::PROTECTED,
            "static" => MethodAccessFlags::STATIC,
            "final" => MethodAccessFlags::FINAL,
            "synchronized" => MethodAccessFlags::SYNCHRONIZED,
            "bridge" => MethodAccessFlags::BRIDGE,
            "varargs" => MethodAccessFlags::VARARGS,
            "native" => MethodAccessFlags::NATIVE,
            "abstract" => MethodAccessFlags::ABSTRACT,
            "strict" | "strictfp" => MethodAccessFlags::STRICT,
            "synthetic" => MethodAccessFlags::SYNTHETIC,
            _ => MethodAccessFlags::empty(),
        };
    }
    flags
}
