//! class 文件结构化数据
//!
//! 由 bytecode.rs 解析生成，纯数据无 UI 依赖。
//!
//! @author sky

use std::collections::HashSet;

/// 解析后的 class 结构化数据
pub struct ClassStructure {
    /// class 级别元数据
    pub info: ClassInfo,
    /// 字段列表
    pub fields: Vec<FieldInfo>,
    /// 方法列表
    pub methods: Vec<MethodInfo>,
}

impl ClassStructure {
    /// 收集已修改或已保存的成员
    pub fn collect_saved_members(&self) -> HashSet<SavedMember> {
        let mut set = HashSet::new();
        if self.info.modified || self.info.saved {
            set.insert(SavedMember::ClassInfo);
        }
        for f in &self.fields {
            if f.modified || f.saved {
                set.insert(SavedMember::Field(f.name.clone(), f.descriptor.clone()));
            }
        }
        for m in &self.methods {
            if m.modified || m.saved {
                set.insert(SavedMember::Method(m.name.clone(), m.descriptor.clone()));
            }
        }
        set
    }

    /// 恢复 saved 标记（重建 class structure 后调用）
    pub fn restore_saved_flags(&mut self, saved: &HashSet<SavedMember>) {
        if saved.contains(&SavedMember::ClassInfo) {
            self.info.saved = true;
        }
        for f in &mut self.fields {
            if saved.contains(&SavedMember::Field(f.name.clone(), f.descriptor.clone())) {
                f.saved = true;
            }
        }
        for m in &mut self.methods {
            if saved.contains(&SavedMember::Method(m.name.clone(), m.descriptor.clone())) {
                m.saved = true;
            }
        }
    }
}

/// class 级别元数据
pub struct ClassInfo {
    /// 版本信息，如 "Java 21 (class 65.0)"
    pub version: String,
    /// 访问修饰符，如 "public final class"
    pub access: String,
    /// 内部名称，如 "com/example/MyClass"
    pub name: String,
    /// 父类内部名称
    pub super_class: String,
    /// 实现的接口列表
    pub interfaces: Vec<String>,
    /// 泛型签名
    pub signature: Option<String>,
    /// 源文件名
    pub source_file: Option<String>,
    /// 注解列表（结构化，可编辑）
    pub annotations: Vec<EditableAnnotation>,
    /// 是否标记 Deprecated
    pub is_deprecated: bool,
    /// 是否被编辑过（未保存）
    pub modified: bool,
    /// 是否已保存到 JAR（与原始文件不同）
    pub saved: bool,
}

/// field 信息
pub struct FieldInfo {
    /// 访问修饰符，如 "public static final"
    pub access: String,
    /// 字段名
    pub name: String,
    /// 字段类型描述符，如 "I"、"Ljava/lang/String;"
    pub descriptor: String,
    /// 编译期常量值
    pub constant_value: Option<String>,
    /// 泛型签名
    pub signature: Option<String>,
    /// 注解列表（结构化，可编辑）
    pub annotations: Vec<EditableAnnotation>,
    /// 是否标记 Deprecated
    pub is_deprecated: bool,
    /// 是否编译器生成
    pub is_synthetic: bool,
    /// 是否被编辑过（未保存）
    pub modified: bool,
    /// 是否已保存到 JAR（与原始文件不同）
    pub saved: bool,
}

/// method 信息
pub struct MethodInfo {
    /// 访问修饰符，如 "public static"
    pub access: String,
    /// 方法名
    pub name: String,
    /// 方法描述符，如 "(I)V"
    pub descriptor: String,
    /// throws 声明的异常类列表
    pub exceptions: Vec<String>,
    /// 泛型签名
    pub signature: Option<String>,
    /// 注解列表（结构化，可编辑）
    pub annotations: Vec<EditableAnnotation>,
    /// 是否标记 Deprecated
    pub is_deprecated: bool,
    /// 是否编译器生成
    pub is_synthetic: bool,
    /// 可编辑字节码指令文本，逐行
    pub bytecode: String,
    /// 是否有 Code attribute（abstract/native 没有）
    pub has_code: bool,
    /// 是否被编辑过（未保存）
    pub modified: bool,
    /// 是否已保存到 JAR（与原始文件不同）
    pub saved: bool,
}

/// 持久化的已保存成员标识（跨 tab 关闭/重开保留）
#[derive(Clone, Hash, Eq, PartialEq)]
pub enum SavedMember {
    ClassInfo,
    /// (name, descriptor)
    Field(String, String),
    /// (name, descriptor)
    Method(String, String),
}

/// 可编辑的注解
pub struct EditableAnnotation {
    /// 类型描述符，如 "Ljava/lang/Override;"
    pub type_desc: String,
    /// 元素列表（name = value 对）
    pub elements: Vec<AnnotationPair>,
}

/// 注解元素
pub struct AnnotationPair {
    /// 元素名，如 "value"
    pub name: String,
    /// 元素值（格式化可编辑字符串）
    pub value: String,
    /// JVM 元素 tag，写回时确定类型
    ///
    /// - `b'B'` byte, `b'C'` char, `b'D'` double, `b'F'` float
    /// - `b'I'` int, `b'J'` long, `b'S'` short, `b'Z'` boolean
    /// - `b's'` string, `b'e'` enum, `b'c'` class
    /// - `b'@'` nested annotation, `b'['` array
    pub tag: u8,
}
