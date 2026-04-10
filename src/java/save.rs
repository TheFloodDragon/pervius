//! ClassStructure → class 字节写回
//!
//! 将 UI 层编辑后的 ClassStructure 应用到原始 class 字节，
//! 生成新的 class 文件字节序列。
//!
//! @author sky

use std::path::Path;

use super::bytecode;
use super::class_structure::ClassStructure;
use super::classforge;
use super::classforge::MethodEdit;
use ristretto_classfile::attributes::{AnnotationElement, AnnotationValuePair, Attribute};
use ristretto_classfile::{
    ClassAccessFlags, ClassFile, ConstantPool, FieldAccessFlags, MethodAccessFlags,
};

/// 将编辑后的 ClassStructure 应用到原始 class 字节，返回新的 class 字节
///
/// metadata 修改（类名/字段名/access flags/注解）由 ristretto 处理，
/// 字节码修改由 classforge (ASM) 处理——常量池管理、指令编码、
/// StackMapTable 生成、max_stack/max_locals 计算全部交给 ASM。
pub fn apply_structure(
    raw_bytes: &[u8],
    cs: &ClassStructure,
    jar_path: Option<&Path>,
) -> Result<Vec<u8>, String> {
    let mut cf = ClassFile::from_bytes(raw_bytes).map_err(|e| format!("parse error: {e}"))?;
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
        }
    }
    // ristretto 处理 metadata 修改
    apply_class_info(&mut cf, cs);
    apply_fields(&mut cf, cs);
    let edits = apply_methods(
        &mut cf.methods,
        &mut cf.constant_pool,
        cs,
        original.as_ref(),
    );
    let mut buf = Vec::new();
    cf.to_bytes(&mut buf)
        .map_err(|e| format!("serialize error: {e}"))?;
    // 有字节码变动时，classforge (ASM) 接管指令编码和帧生成
    if !edits.is_empty() {
        match classforge::patch_methods(&buf, &edits, jar_path) {
            Ok(patched) => {
                log::info!(
                    "classforge: patched {} methods ({} -> {} bytes)",
                    edits.len(),
                    buf.len(),
                    patched.len()
                );
                buf = patched;
            }
            Err(e) => {
                return Err(format!("classforge failed: {e}"));
            }
        }
    }
    log::debug!(
        "apply_structure: {} -> {} bytes",
        raw_bytes.len(),
        buf.len()
    );
    Ok(buf)
}

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

fn apply_fields(cf: &mut ClassFile, cs: &ClassStructure) {
    for (field, fi) in cf.fields.iter_mut().zip(cs.fields.iter()) {
        field.access_flags = parse_field_flags(&fi.access);
        let cp = &mut cf.constant_pool;
        field.name_index = find_or_add_utf8(cp, &fi.name);
        field.descriptor_index = find_or_add_utf8(cp, &fi.descriptor);
        apply_annotations(&mut field.attributes, cp, &fi.annotations);
    }
}

/// 应用方法 metadata 修改，收集字节码编辑列表
///
/// metadata（name/desc/access/annotations）由 ristretto 处理，
/// 字节码变动只记录不处理——交给 classforge (ASM)。
fn apply_methods(
    methods: &mut [ristretto_classfile::Method],
    cp: &mut ConstantPool,
    cs: &ClassStructure,
    original: Option<&ClassStructure>,
) -> Vec<MethodEdit> {
    let mut edits = Vec::new();
    for (i, (method, mi)) in methods.iter_mut().zip(cs.methods.iter()).enumerate() {
        method.access_flags = parse_method_flags(&mi.access);
        method.name_index = find_or_add_utf8(cp, &mi.name);
        method.descriptor_index = find_or_add_utf8(cp, &mi.descriptor);
        // 字节码变动记录到 edits，由 classforge 处理
        if mi.has_code && !mi.bytecode.is_empty() {
            let bytecode_changed = original
                .and_then(|o| o.methods.get(i))
                .map(|om| om.bytecode != mi.bytecode)
                .unwrap_or(false);
            if bytecode_changed {
                edits.push(MethodEdit {
                    name: mi.name.clone(),
                    descriptor: mi.descriptor.clone(),
                    bytecode: mi.bytecode.clone(),
                });
            }
        }
        apply_annotations(&mut method.attributes, cp, &mi.annotations);
    }
    edits
}

// 已删除：create_cp_entry、INTERFACE_METHOD_PREFIX hack、resolve_from_lookup 调用、
// LVT/LVTT byte offset 转换、recompute_max_stack_locals、assembler 模块。
// 这些逻辑全部由 classforge (ASM COMPUTE_FRAMES) 接管。

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
