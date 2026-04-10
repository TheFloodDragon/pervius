//! 指令格式化、标签生成、变量名解析
//!
//! @author sky

use ristretto_classfile::attributes::Instruction;
use std::collections::{BTreeSet, HashMap};

/// 收集指令中的分支目标索引（绝对指令索引）
pub(super) fn collect_branch_targets(
    insn: &Instruction,
    insn_index: usize,
    targets: &mut BTreeSet<usize>,
) {
    branch_targets!(
        insn,
        targets,
        [
            Ifeq, Ifne, Iflt, Ifge, Ifgt, Ifle, If_icmpeq, If_icmpne, If_icmplt, If_icmpge,
            If_icmpgt, If_icmple, If_acmpeq, If_acmpne, Goto, Jsr, Ifnull, Ifnonnull, Goto_w,
            Jsr_w,
        ]
    );
    match insn {
        Instruction::Tableswitch(ts) => {
            targets.insert((insn_index as i64 + ts.default as i64) as usize);
            for offset in &ts.offsets {
                targets.insert((insn_index as i64 + *offset as i64) as usize);
            }
        }
        Instruction::Lookupswitch(ls) => {
            targets.insert((insn_index as i64 + ls.default as i64) as usize);
            for (_, offset) in &ls.pairs {
                targets.insert((insn_index as i64 + *offset as i64) as usize);
            }
        }
        _ => {}
    }
}

/// 查找标签名，回退为数字
pub(super) fn label_at(labels: &HashMap<usize, String>, idx: usize) -> String {
    labels.get(&idx).cloned().unwrap_or_else(|| idx.to_string())
}

/// 将索引转为字母标签（0→A, 1→B, ..., 25→Z, 26→AA, 27→AB, ...）
pub(super) fn index_to_alpha_label(idx: usize) -> String {
    let mut s = String::new();
    let mut n = idx;
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s
}

/// 格式化单条指令为文本
pub(super) fn format_instruction(
    insn: &Instruction,
    insn_index: usize,
    resolve: &dyn Fn(u16) -> String,
    labels: &HashMap<usize, String>,
) -> String {
    // CP 引用指令（Ldc 单独处理：u8 → u16）
    if let Instruction::Ldc(idx) = insn {
        return format!("LDC {}", resolve(u16::from(*idx)));
    }
    if let Some(s) = cp_ref_format!(
        insn,
        resolve,
        [
            Ldc_w,
            Ldc2_w,
            Getstatic,
            Putstatic,
            Getfield,
            Putfield,
            Invokevirtual,
            Invokespecial,
            Invokestatic,
            Invokedynamic,
            New,
            Anewarray,
            Checkcast,
            Instanceof,
        ]
    ) {
        return s;
    }
    // CP 引用指令（含额外字段，不适合宏）
    if let Instruction::Invokeinterface(idx, _) = insn {
        return format!("INVOKEINTERFACE {}", resolve(*idx));
    }
    if let Instruction::Multianewarray(idx, dims) = insn {
        return format!("MULTIANEWARRAY {} {dims}", resolve(*idx));
    }
    // Branch 指令（绝对指令索引 → 标签）
    if let Some(s) = branch_format!(
        insn,
        labels,
        [
            Ifeq, Ifne, Iflt, Ifge, Ifgt, Ifle, If_icmpeq, If_icmpne, If_icmplt, If_icmpge,
            If_icmpgt, If_icmple, If_acmpeq, If_acmpne, Goto, Jsr, Ifnull, Ifnonnull, Goto_w,
            Jsr_w,
        ]
    ) {
        return s;
    }
    // Switch 指令（相对指令偏移 → 标签）
    match insn {
        Instruction::Tableswitch(ts) => {
            let mut s = format!("TABLESWITCH {{ // {} to {}\n", ts.low, ts.high);
            for (i, offset) in ts.offsets.iter().enumerate() {
                let target = (insn_index as i64 + *offset as i64) as usize;
                s.push_str(&format!(
                    "    {}: {}\n",
                    ts.low + i as i32,
                    label_at(labels, target)
                ));
            }
            let default_target = (insn_index as i64 + ts.default as i64) as usize;
            s.push_str(&format!(
                "    default: {}\n",
                label_at(labels, default_target)
            ));
            s.push('}');
            s
        }
        Instruction::Lookupswitch(ls) => {
            let mut s = String::from("LOOKUPSWITCH {\n");
            for (key, offset) in &ls.pairs {
                let target = (insn_index as i64 + *offset as i64) as usize;
                s.push_str(&format!("    {key}: {}\n", label_at(labels, target)));
            }
            let default_target = (insn_index as i64 + ls.default as i64) as usize;
            s.push_str(&format!(
                "    default: {}\n",
                label_at(labels, default_target)
            ));
            s.push('}');
            s
        }
        _ => uppercase_opcode(&insn.to_string()),
    }
}

/// 解析指令中的变量槽位为名称（ALOAD 0 → ALOAD this）
pub(super) fn resolve_vars_in_instruction(
    formatted: &str,
    insn_idx: usize,
    resolved_vars: &[(u16, String, usize, usize)],
    is_static: bool,
) -> String {
    // 紧凑 LOAD/STORE: ALOAD_0 → ALOAD this
    let compact_prefixes = [
        "ALOAD_", "ILOAD_", "LLOAD_", "FLOAD_", "DLOAD_", "ASTORE_", "ISTORE_", "LSTORE_",
        "FSTORE_", "DSTORE_",
    ];
    for prefix in compact_prefixes {
        if formatted.starts_with(prefix) {
            if let Ok(digit) = formatted[prefix.len()..].parse::<u16>() {
                let op = &prefix[..prefix.len() - 1];
                let vname = resolve_slot(insn_idx, digit, resolved_vars, is_static);
                return format!("{op} {vname}");
            }
        }
    }
    // 参数化 LOAD/STORE: ALOAD 0 → ALOAD this
    let var_ops = [
        "ALOAD ", "ILOAD ", "LLOAD ", "FLOAD ", "DLOAD ", "ASTORE ", "ISTORE ", "LSTORE ",
        "FSTORE ", "DSTORE ", "RET ",
    ];
    for prefix in var_ops {
        if formatted.starts_with(prefix) {
            if let Ok(slot) = formatted[prefix.len()..].trim().parse::<u16>() {
                let vname = resolve_slot(insn_idx, slot, resolved_vars, is_static);
                return format!("{prefix}{vname}");
            }
        }
    }
    // IINC: "IINC 0, 1" → "IINC name, 1"
    if formatted.starts_with("IINC ") {
        let rest = &formatted[5..];
        if let Some(comma_pos) = rest.find(',') {
            if let Ok(slot) = rest[..comma_pos].trim().parse::<u16>() {
                let vname = resolve_slot(insn_idx, slot, resolved_vars, is_static);
                return format!("IINC {vname}{}", &rest[comma_pos..]);
            }
        }
    }
    formatted.to_string()
}

/// 去除 CP 格式化文本的类型前缀，返回纯引用名称
pub(super) fn strip_cp_prefix(formatted: &str) -> String {
    let prefixes = [
        "Interface method ",
        "Method handle ",
        "Method type ",
        "Method ",
        "Field ",
        "Class ",
        "Name ",
        "Module ",
        "Package ",
    ];
    for prefix in prefixes {
        if let Some(rest) = formatted.strip_prefix(prefix) {
            return rest.to_string();
        }
    }
    // InvokeDynamic / Dynamic: #bsm_idx:name:descriptor → name + descriptor
    if let Some(rest) = formatted.strip_prefix("InvokeDynamic ") {
        return strip_bsm_ref(rest);
    }
    if let Some(rest) = formatted.strip_prefix("Dynamic ") {
        return strip_bsm_ref(rest);
    }
    // String 常量加引号
    if let Some(rest) = formatted.strip_prefix("String ") {
        return format!("\"{rest}\"");
    }
    formatted.to_string()
}

/// 解析槽位号为变量名
fn resolve_slot(
    insn_idx: usize,
    slot: u16,
    resolved_vars: &[(u16, String, usize, usize)],
    is_static: bool,
) -> String {
    if !is_static && slot == 0 {
        return "this".to_string();
    }
    for (vslot, vname, start, end) in resolved_vars {
        if *vslot == slot && insn_idx >= *start && insn_idx < *end {
            return vname.clone();
        }
    }
    slot.to_string()
}

/// 将指令文本的操作码部分转为大写
fn uppercase_opcode(line: &str) -> String {
    if let Some(idx) = line.find(' ') {
        let (opcode, rest) = line.split_at(idx);
        format!("{}{}", opcode.to_uppercase(), rest)
    } else {
        line.to_uppercase()
    }
}

/// 去除 bootstrap method 前缀并保留索引：`#0:name:desc` → `#0 name desc`
fn strip_bsm_ref(s: &str) -> String {
    if let Some(rest) = s.strip_prefix('#') {
        if let Some(colon_pos) = rest.find(':') {
            let bsm_idx = &rest[..colon_pos];
            let remainder = rest[colon_pos + 1..].replacen(':', "", 1);
            return format!("#{bsm_idx} {remainder}");
        }
    }
    s.to_string()
}
