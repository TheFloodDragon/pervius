//! 内部工具宏
//!
//! @author sky

/// 从空格分隔的修饰符文本解析 access flags
macro_rules! parse_flags {
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
