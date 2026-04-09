//! JVM 字节码汇编：指令文本 → Instruction enum
//!
//! 解析 Recaf 风格大写操作码 + 内联 CP 引用的指令文本。
//! CP 引用通过外部传入的 resolve 回调解析为常量池索引。
//!
//! @author sky

use indexmap::IndexMap;
use ristretto_classfile::attributes::{ArrayType, Instruction, LookupSwitch, TableSwitch};

/// 将多行指令文本解析为 Instruction 序列
///
/// 自动跳过空行和 `//` 注释行（方法签名标记）。
/// 处理 TABLESWITCH / LOOKUPSWITCH 的多行块。
pub fn assemble_instructions(
    text: &str,
    resolve_cp: &mut dyn FnMut(&str) -> Result<u16, String>,
) -> Result<Vec<Instruction>, String> {
    let mut instructions = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        let (opcode, _) = split_opcode(trimmed);
        if opcode == "TABLESWITCH" || opcode == "LOOKUPSWITCH" {
            let mut block = vec![trimmed.to_string()];
            for next in lines.by_ref() {
                let t = next.trim();
                block.push(t.to_string());
                if t == "}" {
                    break;
                }
            }
            let insn = if opcode == "TABLESWITCH" {
                parse_tableswitch(&block)?
            } else {
                parse_lookupswitch(&block)?
            };
            instructions.push(insn);
            continue;
        }
        let insn = parse_instruction(trimmed, resolve_cp)?;
        instructions.push(insn);
    }
    Ok(instructions)
}

/// 解析单行指令文本
pub fn parse_instruction(
    line: &str,
    resolve_cp: &mut dyn FnMut(&str) -> Result<u16, String>,
) -> Result<Instruction, String> {
    let (opcode, operands) = split_opcode(line);
    if let Some(insn) = try_zero_operand(&opcode) {
        return Ok(insn);
    }
    match opcode.as_str() {
        // 立即数
        "BIPUSH" => Ok(Instruction::Bipush(parse_i8(operands)?)),
        "SIPUSH" => Ok(Instruction::Sipush(parse_i16(operands)?)),
        // u8 局部变量索引
        "ILOAD" => Ok(Instruction::Iload(parse_u8(operands)?)),
        "LLOAD" => Ok(Instruction::Lload(parse_u8(operands)?)),
        "FLOAD" => Ok(Instruction::Fload(parse_u8(operands)?)),
        "DLOAD" => Ok(Instruction::Dload(parse_u8(operands)?)),
        "ALOAD" => Ok(Instruction::Aload(parse_u8(operands)?)),
        "ISTORE" => Ok(Instruction::Istore(parse_u8(operands)?)),
        "LSTORE" => Ok(Instruction::Lstore(parse_u8(operands)?)),
        "FSTORE" => Ok(Instruction::Fstore(parse_u8(operands)?)),
        "DSTORE" => Ok(Instruction::Dstore(parse_u8(operands)?)),
        "ASTORE" => Ok(Instruction::Astore(parse_u8(operands)?)),
        "RET" => Ok(Instruction::Ret(parse_u8(operands)?)),
        // u16 分支目标（绝对指令索引）
        "IFEQ" => Ok(Instruction::Ifeq(parse_u16(operands)?)),
        "IFNE" => Ok(Instruction::Ifne(parse_u16(operands)?)),
        "IFLT" => Ok(Instruction::Iflt(parse_u16(operands)?)),
        "IFGE" => Ok(Instruction::Ifge(parse_u16(operands)?)),
        "IFGT" => Ok(Instruction::Ifgt(parse_u16(operands)?)),
        "IFLE" => Ok(Instruction::Ifle(parse_u16(operands)?)),
        "IF_ICMPEQ" => Ok(Instruction::If_icmpeq(parse_u16(operands)?)),
        "IF_ICMPNE" => Ok(Instruction::If_icmpne(parse_u16(operands)?)),
        "IF_ICMPLT" => Ok(Instruction::If_icmplt(parse_u16(operands)?)),
        "IF_ICMPGE" => Ok(Instruction::If_icmpge(parse_u16(operands)?)),
        "IF_ICMPGT" => Ok(Instruction::If_icmpgt(parse_u16(operands)?)),
        "IF_ICMPLE" => Ok(Instruction::If_icmple(parse_u16(operands)?)),
        "IF_ACMPEQ" => Ok(Instruction::If_acmpeq(parse_u16(operands)?)),
        "IF_ACMPNE" => Ok(Instruction::If_acmpne(parse_u16(operands)?)),
        "GOTO" => Ok(Instruction::Goto(parse_u16(operands)?)),
        "JSR" => Ok(Instruction::Jsr(parse_u16(operands)?)),
        "IFNULL" => Ok(Instruction::Ifnull(parse_u16(operands)?)),
        "IFNONNULL" => Ok(Instruction::Ifnonnull(parse_u16(operands)?)),
        // i32 宽分支
        "GOTO_W" => Ok(Instruction::Goto_w(parse_i32(operands)?)),
        "JSR_W" => Ok(Instruction::Jsr_w(parse_i32(operands)?)),
        // wide 局部变量索引
        "ILOAD_W" => Ok(Instruction::Iload_w(parse_u16(operands)?)),
        "LLOAD_W" => Ok(Instruction::Lload_w(parse_u16(operands)?)),
        "FLOAD_W" => Ok(Instruction::Fload_w(parse_u16(operands)?)),
        "DLOAD_W" => Ok(Instruction::Dload_w(parse_u16(operands)?)),
        "ALOAD_W" => Ok(Instruction::Aload_w(parse_u16(operands)?)),
        "ISTORE_W" => Ok(Instruction::Istore_w(parse_u16(operands)?)),
        "LSTORE_W" => Ok(Instruction::Lstore_w(parse_u16(operands)?)),
        "FSTORE_W" => Ok(Instruction::Fstore_w(parse_u16(operands)?)),
        "DSTORE_W" => Ok(Instruction::Dstore_w(parse_u16(operands)?)),
        "ASTORE_W" => Ok(Instruction::Astore_w(parse_u16(operands)?)),
        "RET_W" => Ok(Instruction::Ret_w(parse_u16(operands)?)),
        // 双操作数
        "IINC" => {
            let (a, b) = split_comma(operands)?;
            Ok(Instruction::Iinc(parse_u8(a)?, parse_i8(b)?))
        }
        "IINC_W" => {
            let (a, b) = split_comma(operands)?;
            Ok(Instruction::Iinc_w(parse_u16(a)?, parse_i16(b)?))
        }
        // 数组类型
        "NEWARRAY" => Ok(Instruction::Newarray(parse_array_type(operands)?)),
        // CP 引用（单操作数）
        "LDC" => {
            let idx = resolve_cp(operands)?;
            if idx > 255 {
                return Err(format!("LDC index {idx} > 255, use LDC_W"));
            }
            Ok(Instruction::Ldc(idx as u8))
        }
        "LDC_W" => Ok(Instruction::Ldc_w(resolve_cp(operands)?)),
        "LDC2_W" => Ok(Instruction::Ldc2_w(resolve_cp(operands)?)),
        "GETSTATIC" => Ok(Instruction::Getstatic(resolve_cp(operands)?)),
        "PUTSTATIC" => Ok(Instruction::Putstatic(resolve_cp(operands)?)),
        "GETFIELD" => Ok(Instruction::Getfield(resolve_cp(operands)?)),
        "PUTFIELD" => Ok(Instruction::Putfield(resolve_cp(operands)?)),
        "INVOKEVIRTUAL" => Ok(Instruction::Invokevirtual(resolve_cp(operands)?)),
        "INVOKESPECIAL" => Ok(Instruction::Invokespecial(resolve_cp(operands)?)),
        "INVOKESTATIC" => Ok(Instruction::Invokestatic(resolve_cp(operands)?)),
        "INVOKEDYNAMIC" => Ok(Instruction::Invokedynamic(resolve_cp(operands)?)),
        "NEW" => Ok(Instruction::New(resolve_cp(operands)?)),
        "ANEWARRAY" => Ok(Instruction::Anewarray(resolve_cp(operands)?)),
        "CHECKCAST" => Ok(Instruction::Checkcast(resolve_cp(operands)?)),
        "INSTANCEOF" => Ok(Instruction::Instanceof(resolve_cp(operands)?)),
        // CP 引用 + 额外操作数
        "INVOKEINTERFACE" => {
            let idx = resolve_cp(operands)?;
            let count = compute_interface_count(operands);
            Ok(Instruction::Invokeinterface(idx, count))
        }
        "MULTIANEWARRAY" => {
            let (type_ref, dims) = split_last_space(operands)?;
            let idx = resolve_cp(type_ref)?;
            Ok(Instruction::Multianewarray(idx, parse_u8(dims)?))
        }
        _ => Err(format!("unknown opcode: {opcode}")),
    }
}

// ---------------------------------------------------------------------------
// 零操作数指令
// ---------------------------------------------------------------------------

fn try_zero_operand(opcode: &str) -> Option<Instruction> {
    Some(match opcode {
        // 常量
        "NOP" => Instruction::Nop,
        "ACONST_NULL" => Instruction::Aconst_null,
        "ICONST_M1" => Instruction::Iconst_m1,
        "ICONST_0" => Instruction::Iconst_0,
        "ICONST_1" => Instruction::Iconst_1,
        "ICONST_2" => Instruction::Iconst_2,
        "ICONST_3" => Instruction::Iconst_3,
        "ICONST_4" => Instruction::Iconst_4,
        "ICONST_5" => Instruction::Iconst_5,
        "LCONST_0" => Instruction::Lconst_0,
        "LCONST_1" => Instruction::Lconst_1,
        "FCONST_0" => Instruction::Fconst_0,
        "FCONST_1" => Instruction::Fconst_1,
        "FCONST_2" => Instruction::Fconst_2,
        "DCONST_0" => Instruction::Dconst_0,
        "DCONST_1" => Instruction::Dconst_1,
        // 隐式索引加载
        "ILOAD_0" => Instruction::Iload_0,
        "ILOAD_1" => Instruction::Iload_1,
        "ILOAD_2" => Instruction::Iload_2,
        "ILOAD_3" => Instruction::Iload_3,
        "LLOAD_0" => Instruction::Lload_0,
        "LLOAD_1" => Instruction::Lload_1,
        "LLOAD_2" => Instruction::Lload_2,
        "LLOAD_3" => Instruction::Lload_3,
        "FLOAD_0" => Instruction::Fload_0,
        "FLOAD_1" => Instruction::Fload_1,
        "FLOAD_2" => Instruction::Fload_2,
        "FLOAD_3" => Instruction::Fload_3,
        "DLOAD_0" => Instruction::Dload_0,
        "DLOAD_1" => Instruction::Dload_1,
        "DLOAD_2" => Instruction::Dload_2,
        "DLOAD_3" => Instruction::Dload_3,
        "ALOAD_0" => Instruction::Aload_0,
        "ALOAD_1" => Instruction::Aload_1,
        "ALOAD_2" => Instruction::Aload_2,
        "ALOAD_3" => Instruction::Aload_3,
        // 数组加载
        "IALOAD" => Instruction::Iaload,
        "LALOAD" => Instruction::Laload,
        "FALOAD" => Instruction::Faload,
        "DALOAD" => Instruction::Daload,
        "AALOAD" => Instruction::Aaload,
        "BALOAD" => Instruction::Baload,
        "CALOAD" => Instruction::Caload,
        "SALOAD" => Instruction::Saload,
        // 隐式索引存储
        "ISTORE_0" => Instruction::Istore_0,
        "ISTORE_1" => Instruction::Istore_1,
        "ISTORE_2" => Instruction::Istore_2,
        "ISTORE_3" => Instruction::Istore_3,
        "LSTORE_0" => Instruction::Lstore_0,
        "LSTORE_1" => Instruction::Lstore_1,
        "LSTORE_2" => Instruction::Lstore_2,
        "LSTORE_3" => Instruction::Lstore_3,
        "FSTORE_0" => Instruction::Fstore_0,
        "FSTORE_1" => Instruction::Fstore_1,
        "FSTORE_2" => Instruction::Fstore_2,
        "FSTORE_3" => Instruction::Fstore_3,
        "DSTORE_0" => Instruction::Dstore_0,
        "DSTORE_1" => Instruction::Dstore_1,
        "DSTORE_2" => Instruction::Dstore_2,
        "DSTORE_3" => Instruction::Dstore_3,
        "ASTORE_0" => Instruction::Astore_0,
        "ASTORE_1" => Instruction::Astore_1,
        "ASTORE_2" => Instruction::Astore_2,
        "ASTORE_3" => Instruction::Astore_3,
        // 数组存储
        "IASTORE" => Instruction::Iastore,
        "LASTORE" => Instruction::Lastore,
        "FASTORE" => Instruction::Fastore,
        "DASTORE" => Instruction::Dastore,
        "AASTORE" => Instruction::Aastore,
        "BASTORE" => Instruction::Bastore,
        "CASTORE" => Instruction::Castore,
        "SASTORE" => Instruction::Sastore,
        // 栈操作
        "POP" => Instruction::Pop,
        "POP2" => Instruction::Pop2,
        "DUP" => Instruction::Dup,
        "DUP_X1" => Instruction::Dup_x1,
        "DUP_X2" => Instruction::Dup_x2,
        "DUP2" => Instruction::Dup2,
        "DUP2_X1" => Instruction::Dup2_x1,
        "DUP2_X2" => Instruction::Dup2_x2,
        "SWAP" => Instruction::Swap,
        // 算术
        "IADD" => Instruction::Iadd,
        "LADD" => Instruction::Ladd,
        "FADD" => Instruction::Fadd,
        "DADD" => Instruction::Dadd,
        "ISUB" => Instruction::Isub,
        "LSUB" => Instruction::Lsub,
        "FSUB" => Instruction::Fsub,
        "DSUB" => Instruction::Dsub,
        "IMUL" => Instruction::Imul,
        "LMUL" => Instruction::Lmul,
        "FMUL" => Instruction::Fmul,
        "DMUL" => Instruction::Dmul,
        "IDIV" => Instruction::Idiv,
        "LDIV" => Instruction::Ldiv,
        "FDIV" => Instruction::Fdiv,
        "DDIV" => Instruction::Ddiv,
        "IREM" => Instruction::Irem,
        "LREM" => Instruction::Lrem,
        "FREM" => Instruction::Frem,
        "DREM" => Instruction::Drem,
        "INEG" => Instruction::Ineg,
        "LNEG" => Instruction::Lneg,
        "FNEG" => Instruction::Fneg,
        "DNEG" => Instruction::Dneg,
        // 位运算 / 移位
        "ISHL" => Instruction::Ishl,
        "LSHL" => Instruction::Lshl,
        "ISHR" => Instruction::Ishr,
        "LSHR" => Instruction::Lshr,
        "IUSHR" => Instruction::Iushr,
        "LUSHR" => Instruction::Lushr,
        "IAND" => Instruction::Iand,
        "LAND" => Instruction::Land,
        "IOR" => Instruction::Ior,
        "LOR" => Instruction::Lor,
        "IXOR" => Instruction::Ixor,
        "LXOR" => Instruction::Lxor,
        // 类型转换
        "I2L" => Instruction::I2l,
        "I2F" => Instruction::I2f,
        "I2D" => Instruction::I2d,
        "L2I" => Instruction::L2i,
        "L2F" => Instruction::L2f,
        "L2D" => Instruction::L2d,
        "F2I" => Instruction::F2i,
        "F2L" => Instruction::F2l,
        "F2D" => Instruction::F2d,
        "D2I" => Instruction::D2i,
        "D2L" => Instruction::D2l,
        "D2F" => Instruction::D2f,
        "I2B" => Instruction::I2b,
        "I2C" => Instruction::I2c,
        "I2S" => Instruction::I2s,
        // 比较
        "LCMP" => Instruction::Lcmp,
        "FCMPL" => Instruction::Fcmpl,
        "FCMPG" => Instruction::Fcmpg,
        "DCMPL" => Instruction::Dcmpl,
        "DCMPG" => Instruction::Dcmpg,
        // 返回
        "IRETURN" => Instruction::Ireturn,
        "LRETURN" => Instruction::Lreturn,
        "FRETURN" => Instruction::Freturn,
        "DRETURN" => Instruction::Dreturn,
        "ARETURN" => Instruction::Areturn,
        "RETURN" => Instruction::Return,
        // 杂项
        "ARRAYLENGTH" => Instruction::Arraylength,
        "ATHROW" => Instruction::Athrow,
        "MONITORENTER" => Instruction::Monitorenter,
        "MONITOREXIT" => Instruction::Monitorexit,
        "WIDE" => Instruction::Wide,
        "BREAKPOINT" => Instruction::Breakpoint,
        "IMPDEP1" => Instruction::Impdep1,
        "IMPDEP2" => Instruction::Impdep2,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// switch 多行块解析
// ---------------------------------------------------------------------------

fn parse_tableswitch(block: &[String]) -> Result<Instruction, String> {
    let first = &block[0];
    let comment_start = first.find("//").ok_or("tableswitch: missing // comment")?;
    let comment = first[comment_start + 2..].trim();
    let parts: Vec<&str> = comment.split(" to ").collect();
    if parts.len() != 2 {
        return Err("tableswitch: expected 'low to high' in comment".into());
    }
    let low: i32 = parts[0]
        .trim()
        .parse()
        .map_err(|e| format!("tableswitch low: {e}"))?;
    let high: i32 = parts[1]
        .trim()
        .parse()
        .map_err(|e| format!("tableswitch high: {e}"))?;
    let mut offsets = Vec::new();
    let mut default = 0i32;
    for line in &block[1..] {
        let t = line.trim();
        if t == "}" {
            break;
        }
        if let Some(rest) = t.strip_prefix("default:") {
            default = rest
                .trim()
                .parse()
                .map_err(|e| format!("tableswitch default: {e}"))?;
        } else if let Some(colon_pos) = t.find(':') {
            let offset: i32 = t[colon_pos + 1..]
                .trim()
                .parse()
                .map_err(|e| format!("tableswitch offset: {e}"))?;
            offsets.push(offset);
        }
    }
    Ok(Instruction::Tableswitch(TableSwitch {
        default,
        low,
        high,
        offsets,
    }))
}

fn parse_lookupswitch(block: &[String]) -> Result<Instruction, String> {
    let mut pairs = IndexMap::new();
    let mut default = 0i32;
    for line in &block[1..] {
        let t = line.trim();
        if t == "}" {
            break;
        }
        if let Some(rest) = t.strip_prefix("default:") {
            default = rest
                .trim()
                .parse()
                .map_err(|e| format!("lookupswitch default: {e}"))?;
        } else if let Some(colon_pos) = t.find(':') {
            let key: i32 = t[..colon_pos]
                .trim()
                .parse()
                .map_err(|e| format!("lookupswitch key: {e}"))?;
            let val: i32 = t[colon_pos + 1..]
                .trim()
                .parse()
                .map_err(|e| format!("lookupswitch offset: {e}"))?;
            pairs.insert(key, val);
        }
    }
    Ok(Instruction::Lookupswitch(LookupSwitch { default, pairs }))
}

// ---------------------------------------------------------------------------
// 辅助：操作数解析
// ---------------------------------------------------------------------------

fn split_opcode(line: &str) -> (String, &str) {
    match line.find(' ') {
        Some(idx) => (line[..idx].to_uppercase(), line[idx + 1..].trim()),
        None => (line.to_uppercase(), ""),
    }
}

fn split_comma(s: &str) -> Result<(&str, &str), String> {
    let (a, b) = s
        .split_once(',')
        .ok_or_else(|| format!("expected 'a, b': {s}"))?;
    Ok((a.trim(), b.trim()))
}

fn split_last_space(s: &str) -> Result<(&str, &str), String> {
    let idx = s
        .rfind(' ')
        .ok_or_else(|| format!("expected 'ref N': {s}"))?;
    Ok((s[..idx].trim(), s[idx + 1..].trim()))
}

fn parse_u8(s: &str) -> Result<u8, String> {
    s.trim()
        .parse()
        .map_err(|e| format!("invalid u8 '{s}': {e}"))
}

fn parse_i8(s: &str) -> Result<i8, String> {
    s.trim()
        .parse()
        .map_err(|e| format!("invalid i8 '{s}': {e}"))
}

fn parse_u16(s: &str) -> Result<u16, String> {
    s.trim()
        .parse()
        .map_err(|e| format!("invalid u16 '{s}': {e}"))
}

fn parse_i16(s: &str) -> Result<i16, String> {
    s.trim()
        .parse()
        .map_err(|e| format!("invalid i16 '{s}': {e}"))
}

fn parse_i32(s: &str) -> Result<i32, String> {
    s.trim()
        .parse()
        .map_err(|e| format!("invalid i32 '{s}': {e}"))
}

fn parse_array_type(s: &str) -> Result<ArrayType, String> {
    match s.trim() {
        "boolean" => Ok(ArrayType::Boolean),
        "char" => Ok(ArrayType::Char),
        "float" => Ok(ArrayType::Float),
        "double" => Ok(ArrayType::Double),
        "byte" => Ok(ArrayType::Byte),
        "short" => Ok(ArrayType::Short),
        "int" => Ok(ArrayType::Int),
        "long" => Ok(ArrayType::Long),
        other => Err(format!("unknown array type: {other}")),
    }
}

/// 从方法描述符计算 invokeinterface 的 count 参数
///
/// count = 参数槽数 + 1（receiver），long/double 占 2 槽
fn compute_interface_count(method_ref: &str) -> u8 {
    let desc_start = match method_ref.find('(') {
        Some(idx) => idx,
        None => return 1,
    };
    let bytes = method_ref.as_bytes();
    let mut count: u8 = 1;
    let mut i = desc_start + 1;
    while i < bytes.len() && bytes[i] != b')' {
        match bytes[i] {
            b'J' | b'D' => {
                count = count.saturating_add(2);
                i += 1;
            }
            b'L' => {
                count = count.saturating_add(1);
                while i < bytes.len() && bytes[i] != b';' {
                    i += 1;
                }
                i += 1;
            }
            b'[' => {
                while i < bytes.len() && bytes[i] == b'[' {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'L' {
                    while i < bytes.len() && bytes[i] != b';' {
                        i += 1;
                    }
                    i += 1;
                } else {
                    i += 1;
                }
                count = count.saturating_add(1);
            }
            _ => {
                count = count.saturating_add(1);
                i += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::java::bytecode;
    use ristretto_classfile::attributes::Attribute;
    use ristretto_classfile::{ClassFile, ConstantPool};
    use std::io::Read;

    /// 从指令中提取 CP 索引（如有）
    fn cp_index(insn: &Instruction) -> Option<u16> {
        match insn {
            Instruction::Ldc(i) => Some(u16::from(*i)),
            Instruction::Ldc_w(i)
            | Instruction::Ldc2_w(i)
            | Instruction::Getstatic(i)
            | Instruction::Putstatic(i)
            | Instruction::Getfield(i)
            | Instruction::Putfield(i)
            | Instruction::Invokevirtual(i)
            | Instruction::Invokespecial(i)
            | Instruction::Invokestatic(i)
            | Instruction::Invokedynamic(i)
            | Instruction::New(i)
            | Instruction::Anewarray(i)
            | Instruction::Checkcast(i)
            | Instruction::Instanceof(i) => Some(*i),
            Instruction::Invokeinterface(i, _) | Instruction::Multianewarray(i, _) => Some(*i),
            _ => None,
        }
    }

    /// 判断两条指令的差异是否仅为 CP 索引不同但指向等价常量
    fn is_cp_alias(orig: &Instruction, asm: &Instruction, cp: &ConstantPool) -> bool {
        let (Some(oi), Some(ai)) = (cp_index(orig), cp_index(asm)) else {
            return false;
        };
        if oi == ai {
            return false;
        }
        // opcode 必须一致（variant discriminant 相同）
        if std::mem::discriminant(orig) != std::mem::discriminant(asm) {
            return false;
        }
        let ok_o = cp.try_get_formatted_string(oi);
        let ok_a = cp.try_get_formatted_string(ai);
        matches!((ok_o, ok_a), (Ok(a), Ok(b)) if a == b)
    }

    /// 遍历 JAR 中所有 class 文件，对每个方法执行：
    /// disassemble → assemble → 逐条比较
    ///
    /// CP 索引指向等价常量的差异视为 alias，不算 failure。
    #[test]
    fn roundtrip_frontier_jar() {
        let path = r"C:\Users\sky\Desktop\Server\dist\Frontier-Velocity-1.0.0.jar";
        let file = std::fs::File::open(path).expect("failed to open JAR");
        let mut archive = zip::ZipArchive::new(file).expect("failed to read ZIP");
        let mut tested_methods = 0usize;
        let mut tested_insns = 0usize;
        let mut cp_aliases = 0usize;
        let mut asm_errors = 0usize;
        let mut failures = Vec::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            if !entry.name().ends_with(".class") {
                continue;
            }
            let class_name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            let cf = match ClassFile::from_bytes(&bytes) {
                Ok(cf) => cf,
                Err(e) => {
                    failures.push(format!("[PARSE] {class_name}: {e}"));
                    continue;
                }
            };
            let structure = match bytecode::disassemble(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("[DISASM] {class_name}: {e}"));
                    continue;
                }
            };
            let lookup = match bytecode::build_cp_lookup(&bytes) {
                Ok(t) => t,
                Err(e) => {
                    failures.push(format!("[LOOKUP] {class_name}: {e}"));
                    continue;
                }
            };
            let cp = &cf.constant_pool;
            for (method, info) in cf.methods.iter().zip(structure.methods.iter()) {
                if !info.has_code || info.bytecode.is_empty() {
                    continue;
                }
                let original_code = match method.attributes.iter().find_map(|a| {
                    if let Attribute::Code { code, .. } = a {
                        Some(code)
                    } else {
                        None
                    }
                }) {
                    Some(code) => code,
                    None => continue,
                };
                let method_id = format!("{class_name}::{}.{}", info.name, info.descriptor);
                tested_methods += 1;
                tested_insns += original_code.len();
                let mut resolve = |text: &str| -> Result<u16, String> {
                    bytecode::resolve_from_lookup(&lookup, text)
                };
                let assembled = match assemble_instructions(&info.bytecode, &mut resolve) {
                    Ok(v) => v,
                    Err(e) => {
                        asm_errors += 1;
                        if asm_errors <= 10 {
                            eprintln!("[ASM] {method_id}: {e}");
                        }
                        continue;
                    }
                };
                if original_code.len() != assembled.len() {
                    failures.push(format!(
                        "[COUNT] {method_id}: expected {} insns, got {}",
                        original_code.len(),
                        assembled.len()
                    ));
                    continue;
                }
                for (j, (orig, asm)) in original_code.iter().zip(assembled.iter()).enumerate() {
                    if orig == asm {
                        continue;
                    }
                    if is_cp_alias(orig, asm, cp) {
                        cp_aliases += 1;
                        continue;
                    }
                    failures.push(format!(
                        "[INSN] {method_id}[{j}]:\n  expected: {orig}\n  actual:   {asm}"
                    ));
                }
            }
        }
        println!("\n=== Roundtrip Test Results ===");
        println!("Methods: {tested_methods}");
        println!("Instructions: {tested_insns}");
        println!("CP alias (equivalent, different slot): {cp_aliases}");
        println!("Assemble errors (multi-line strings etc.): {asm_errors}");
        println!("Semantic failures: {}", failures.len());
        for f in &failures {
            println!("  {f}");
        }
        assert!(
            failures.is_empty(),
            "{} semantic roundtrip failures detected",
            failures.len()
        );
    }
}
