//! 反编译器搜索分类
//!
//! 对应字节码语义维度：字符串常量、数值常量、类引用、成员引用、成员声明、指令。
//!
//! @author sky

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

    pub const fn label(self) -> &'static str {
        match self {
            Self::Strings => "Strings",
            Self::Values => "Values",
            Self::ClassReferences => "Class references",
            Self::MemberReferences => "Member references",
            Self::MemberDeclarations => "Member declarations",
            Self::Instructions => "Instructions",
        }
    }
}
