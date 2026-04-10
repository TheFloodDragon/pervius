//! 语言定义宏
//!
//! @author sky

/// 从单一声明生成 Language 枚举、扩展名映射和分发逻辑
///
/// - `ts` 段：tree-sitter 语言，需提供解析器和至少一个扩展名
/// - `custom` 段：自定义 tokenizer 语言，扩展名可选（无扩展名则不参与 `from_extension` 推断）
macro_rules! define_languages {
    (
        ts {
            $(
                $(#[$ts_attr:meta])*
                $ts_variant:ident($ts_mod:ident) = $ts_parser:expr => [$($ts_ext:literal),+ $(,)?]
            ),* $(,)?
        }
        custom {
            $(
                $(#[$c_attr:meta])*
                $c_variant:ident($c_mod:ident) $(=> [$($c_ext:literal),+ $(,)?])?
            ),* $(,)?
        }
    ) => {
        /// 支持高亮的语言（限 JAR 内可能出现的类型）
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum Language {
            $($(#[$ts_attr])* $ts_variant,)*
            $($(#[$c_attr])* $c_variant,)*
            Plain,
        }

        impl Language {
            /// 从文件扩展名推断
            pub fn from_extension(ext: &str) -> Self {
                match ext.to_ascii_lowercase().as_str() {
                    $($($ts_ext)|+ => Self::$ts_variant,)*
                    $($($($c_ext)|+ => Self::$c_variant,)?)*
                    _ => Self::Plain,
                }
            }
        }

        /// 收集所有着色 span（字节偏移）
        fn collect_spans(source: &str, lang: Language) -> Vec<Span> {
            match lang {
                $(Language::$c_variant => $c_mod::collect_spans(source),)*
                Language::Plain => vec![(0, source.len(), TokenKind::Plain)],
                _ => collect_treesitter_spans(source, lang),
            }
        }

        /// tree-sitter 语言 → (解析器, 分类函数)
        fn resolve_treesitter(lang: Language) -> Option<(tree_sitter::Language, ColorFn)> {
            match lang {
                $(Language::$ts_variant => Some(($ts_parser.into(), $ts_mod::classify as ColorFn)),)*
                _ => None,
            }
        }
    };
}
