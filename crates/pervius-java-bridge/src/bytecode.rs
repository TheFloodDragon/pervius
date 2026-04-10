//! JVM class 文件反汇编：raw bytes → 结构化 class 数据
//!
//! 基于 ristretto_classfile 解析，输出 ClassStructure。
//! 操作码大写，CP 引用内联解析为可读名称（Recaf 风格）。
//!
//! @author sky

mod annotation;
mod class;
mod common;
pub mod descriptor;
mod field;
mod format;
mod method;

use crate::class_structure::ClassStructure;
use ristretto_classfile::ConstantPool;

use class::extract_class_info;
use field::extract_field;
use method::extract_method;

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

/// 将 .class 原始字节反汇编为结构化 class 数据
pub fn disassemble(bytes: &[u8]) -> Result<ClassStructure, String> {
    let cf = ristretto_classfile::ClassFile::from_bytes(bytes)
        .map_err(|e| format!("parse error: {e}"))?;
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
