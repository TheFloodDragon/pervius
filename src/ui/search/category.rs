//! 反编译器搜索分类
//!
//! 对应字节码语义维度：字符串常量、数值常量、类引用、成员引用、成员声明、指令。
//!
//! @author sky

use rust_i18n::t;

/// 搜索分类（字节码语义维度）
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SearchCategory {
    Strings,
    Values,
    ClassReferences,
    MemberReferences,
    MemberDeclarations,
    Instructions,
}

impl SearchCategory {
    pub const ALL: &[Self] = &[
        Self::Strings,
        Self::Values,
        Self::ClassReferences,
        Self::MemberReferences,
        Self::MemberDeclarations,
        Self::Instructions,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Strings => t!("search.strings").to_string(),
            Self::Values => t!("search.values").to_string(),
            Self::ClassReferences => t!("search.class_refs").to_string(),
            Self::MemberReferences => t!("search.member_refs").to_string(),
            Self::MemberDeclarations => t!("search.member_decls").to_string(),
            Self::Instructions => t!("search.instructions").to_string(),
        }
    }
}
