//! 类级别元数据提取
//!
//! @author sky

use super::common::extract_common_attrs;
use super::descriptor::parse_class_version;
use super::resolve_class;
use crate::class_structure::ClassInfo;
use ristretto_classfile::attributes::Attribute;
use ristretto_classfile::{ClassFile, ConstantPool};

/// 从 ClassFile 提取类级别信息（版本、访问标记、名称、父类、接口、注解等）
pub(super) fn extract_class_info(cf: &ClassFile, cp: &ConstantPool, bytes: &[u8]) -> ClassInfo {
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
