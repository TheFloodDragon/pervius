//! 内部工具宏
//!
//! @author sky

/// Access flags 双向转换：文本 ↔ bitflags
///
/// 单函数模式：只生成 parse 函数。
/// 双函数模式：同时生成 parse 和 format 函数，映射表只写一次。
/// format 遇到同一 flag 的多个 keyword（如 strict / strictfp）时取第一个。
macro_rules! parse_flags {
    ($parse_fn:ident, $format_fn:ident, $flag_type:ty, { $($keyword:literal => $flag:expr),* $(,)? }) => {
        fn $parse_fn(s: &str) -> $flag_type {
            let mut flags = <$flag_type>::empty();
            for word in s.split_whitespace() {
                flags |= match word {
                    $($keyword => $flag,)*
                    _ => <$flag_type>::empty(),
                };
            }
            flags
        }
        pub(crate) fn $format_fn(flags: $flag_type) -> String {
            let mut parts: Vec<&str> = Vec::new();
            let mut seen = <$flag_type>::empty();
            $(
                if flags.contains($flag) && !seen.contains($flag) {
                    parts.push($keyword);
                    seen |= $flag;
                }
            )*
            parts.join(" ")
        }
    };
    ($fn_name:ident, $flag_type:ty, { $($keyword:literal => $flag:expr),* $(,)? }) => {
        fn $fn_name(s: &str) -> $flag_type {
            let mut flags = <$flag_type>::empty();
            for word in s.split_whitespace() {
                flags |= match word {
                    $($keyword => $flag,)*
                    _ => <$flag_type>::empty(),
                };
            }
            flags
        }
    };
}

/// 收集分支指令的跳转目标（生成完整 match 块）
macro_rules! branch_targets {
    ($insn:expr, $targets:ident, [$($variant:ident),* $(,)?]) => {
        match $insn {
            $(Instruction::$variant(t) => { $targets.insert(*t as usize); })*
            _ => {}
        }
    };
}

/// 格式化分支指令为 "OPCODE label"（生成完整 match → Option）
macro_rules! branch_format {
    ($insn:expr, $labels:expr, [$($variant:ident),* $(,)?]) => {
        match $insn {
            $(Instruction::$variant(t) => Some(format!(
                "{} {}",
                stringify!($variant).to_uppercase(),
                label_at($labels, *t as usize)
            )),)*
            _ => None,
        }
    };
}

/// 格式化 CP 引用指令为 "OPCODE resolved"（生成完整 match → Option）
macro_rules! cp_ref_format {
    ($insn:expr, $resolve:expr, [$($variant:ident),* $(,)?]) => {
        match $insn {
            $(Instruction::$variant(idx) => Some(format!(
                "{} {}",
                stringify!($variant).to_uppercase(),
                $resolve(*idx)
            )),)*
            _ => None,
        }
    };
}

/// 转换注解原始数值类型（生成完整 match → Option）
macro_rules! ann_primitive {
    ($elem:expr, $cp:expr, $resolve_fn:expr, [$($variant:ident => $tag:literal),* $(,)?]) => {
        match $elem {
            $(AnnotationElement::$variant { const_value_index } => {
                Some(($resolve_fn($cp, *const_value_index), $tag))
            })*
            _ => None,
        }
    };
}
