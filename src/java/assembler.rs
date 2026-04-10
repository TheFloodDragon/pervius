//! JVM 字节码汇编：指令文本 → Instruction enum
//!
//! 解析 Recaf 风格大写操作码 + 内联 CP 引用的指令文本。
//! CP 引用通过外部传入的 resolve 回调解析为常量池索引。
//! 支持标签（L0: / L1:）、`.var`、`.vartype` 伪指令。
//!
//! @author sky

use indexmap::IndexMap;
use ristretto_classfile::attributes::{
    ArrayType, ExceptionTableEntry, Instruction, LineNumber, LookupSwitch, TableSwitch,
};
use std::collections::HashMap;

/// 局部变量信息（文本格式，待 CP 解析）
pub struct LocalVar {
    /// 槽索引
    pub slot: u16,
    /// 变量名
    pub name: String,
    /// 类型描述符
    pub descriptor: String,
    /// 作用域起始指令索引
    pub start_pc: u16,
    /// 作用域长度（指令数）
    pub length: u16,
}

/// 局部变量泛型类型信息（文本格式，待 CP 解析）
pub struct LocalVarType {
    /// 槽索引
    pub slot: u16,
    /// 变量名
    pub name: String,
    /// 泛型签名
    pub signature: String,
    /// 作用域起始指令索引
    pub start_pc: u16,
    /// 作用域长度（指令数）
    pub length: u16,
}

/// 汇编结果：指令序列 + 行号表 + 异常表 + 局部变量表
pub struct AssembleResult {
    /// 指令序列
    pub instructions: Vec<Instruction>,
    /// 行号映射 (instruction_index, source_line)
    pub line_numbers: Vec<LineNumber>,
    /// 异常处理表
    pub exception_table: Vec<ExceptionTableEntry>,
    /// 局部变量表（文本格式）
    pub local_variables: Vec<LocalVar>,
    /// 局部变量泛型类型表（文本格式）
    pub local_variable_types: Vec<LocalVarType>,
}

/// INVOKEINTERFACE 操作数前缀标记，区分 InterfaceMethodRef 和 MethodRef
pub const INTERFACE_METHOD_PREFIX: char = '\x01';

/// 将多行指令文本解析为 Instruction 序列
///
/// 两趟解析：
/// - Pass 1: 扫描标签定义（`L0:` 等），构建 label → instruction_index 映射
/// - Pass 2: 解析指令、伪指令，解析标签引用
pub fn assemble_instructions(
    text: &str,
    resolve_cp: &mut dyn FnMut(&str) -> Result<u16, String>,
) -> Result<AssembleResult, String> {
    // Pass 1: 收集标签定义
    let label_map = collect_labels(text);
    // Pass 2: 解析
    let mut instructions = Vec::new();
    let mut line_numbers = Vec::new();
    let mut exception_table = Vec::new();
    let mut local_variables = Vec::new();
    let mut local_variable_types = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        // 标签定义：跳过（pass 1 已处理）
        if is_label_def(trimmed) {
            continue;
        }
        // .line N
        if let Some(rest) = trimmed.strip_prefix(".line ") {
            let line_num: u16 = rest
                .trim()
                .parse()
                .map_err(|e| format!("invalid .line number '{rest}': {e}"))?;
            line_numbers.push(LineNumber {
                start_pc: instructions.len() as u16,
                line_number: line_num,
            });
            continue;
        }
        // .catch type L_from L_to L_handler
        if let Some(rest) = trimmed.strip_prefix(".catch ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 4 {
                return Err(format!(
                    "expected '.catch type from to handler', got: {trimmed}"
                ));
            }
            let catch_type = if parts[0] == "*" {
                0
            } else {
                resolve_cp(parts[0])?
            };
            let start = resolve_label_u16(parts[1], &label_map)?;
            let end = resolve_label_u16(parts[2], &label_map)?;
            let handler = resolve_label_u16(parts[3], &label_map)?;
            exception_table.push(ExceptionTableEntry {
                range_pc: start..end,
                handler_pc: handler,
                catch_type,
            });
            continue;
        }
        // .var slot name descriptor L_start L_end
        if let Some(rest) = trimmed.strip_prefix(".var ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 5 {
                return Err(format!(
                    "expected '.var slot name descriptor start end', got: {trimmed}"
                ));
            }
            let slot: u16 = parts[0].parse().map_err(|e| format!(".var slot: {e}"))?;
            let start = resolve_label_usize(parts[3], &label_map)?;
            let end = resolve_label_usize(parts[4], &label_map)?;
            local_variables.push(LocalVar {
                slot,
                name: parts[1].to_string(),
                descriptor: parts[2].to_string(),
                start_pc: start as u16,
                length: (end - start) as u16,
            });
            continue;
        }
        // .vartype slot name signature L_start L_end
        if let Some(rest) = trimmed.strip_prefix(".vartype ") {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() != 5 {
                return Err(format!(
                    "expected '.vartype slot name signature start end', got: {trimmed}"
                ));
            }
            let slot: u16 = parts[0]
                .parse()
                .map_err(|e| format!(".vartype slot: {e}"))?;
            let start = resolve_label_usize(parts[3], &label_map)?;
            let end = resolve_label_usize(parts[4], &label_map)?;
            local_variable_types.push(LocalVarType {
                slot,
                name: parts[1].to_string(),
                signature: parts[2].to_string(),
                start_pc: start as u16,
                length: (end - start) as u16,
            });
            continue;
        }
        // Switch 多行块
        let (opcode, _) = split_opcode(trimmed);
        if opcode == "TABLESWITCH" || opcode == "LOOKUPSWITCH" {
            let insn_index = instructions.len();
            let mut block = vec![trimmed.to_string()];
            for next in lines.by_ref() {
                let t = next.trim();
                block.push(t.to_string());
                if t == "}" {
                    break;
                }
            }
            let insn = if opcode == "TABLESWITCH" {
                parse_tableswitch(&block, insn_index, &label_map)?
            } else {
                parse_lookupswitch(&block, insn_index, &label_map)?
            };
            instructions.push(insn);
            continue;
        }
        // 普通指令
        let insn = parse_instruction(trimmed, resolve_cp, &label_map)?;
        instructions.push(insn);
    }
    Ok(AssembleResult {
        instructions,
        line_numbers,
        exception_table,
        local_variables,
        local_variable_types,
    })
}

// ── 标签解析 ──

/// Pass 1: 扫描所有标签定义，构建 label_name → instruction_index 映射
fn collect_labels(text: &str) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    let mut insn_count = 0;
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        // 标签定义
        if is_label_def(trimmed) {
            let label = &trimmed[..trimmed.len() - 1];
            map.insert(label.to_string(), insn_count);
            continue;
        }
        // 伪指令不占指令索引
        if trimmed.starts_with(".line ")
            || trimmed.starts_with(".catch ")
            || trimmed.starts_with(".var ")
            || trimmed.starts_with(".vartype ")
        {
            continue;
        }
        // Switch 块算 1 条指令
        let (opcode, _) = split_opcode(trimmed);
        if opcode == "TABLESWITCH" || opcode == "LOOKUPSWITCH" {
            for next in lines.by_ref() {
                if next.trim() == "}" {
                    break;
                }
            }
        }
        insn_count += 1;
    }
    map
}

/// 判断是否是标签定义行（`L0:`, `L123:` 等）
fn is_label_def(trimmed: &str) -> bool {
    trimmed.ends_with(':') && !trimmed.contains(' ') && trimmed.len() > 1
}

/// 解析标签或数字为 u16
fn resolve_label_u16(s: &str, labels: &HashMap<String, usize>) -> Result<u16, String> {
    if let Some(&idx) = labels.get(s) {
        return Ok(idx as u16);
    }
    s.parse::<u16>()
        .map_err(|e| format!("invalid label or index '{s}': {e}"))
}

/// 解析标签或数字为 usize
fn resolve_label_usize(s: &str, labels: &HashMap<String, usize>) -> Result<usize, String> {
    if let Some(&idx) = labels.get(s) {
        return Ok(idx);
    }
    s.parse::<usize>()
        .map_err(|e| format!("invalid label or index '{s}': {e}"))
}

/// 解析分支目标（标签或数字 → u16 绝对指令索引）
fn resolve_branch(s: &str, labels: &HashMap<String, usize>) -> Result<u16, String> {
    if let Some(&idx) = labels.get(s.trim()) {
        return Ok(idx as u16);
    }
    parse_u16(s)
}

/// 解析宽分支目标（标签或数字 → i32 绝对指令索引）
fn resolve_branch_wide(s: &str, labels: &HashMap<String, usize>) -> Result<i32, String> {
    if let Some(&idx) = labels.get(s.trim()) {
        return Ok(idx as i32);
    }
    parse_i32(s)
}

/// 解析 switch 目标（标签 → 相对偏移，数字 → 原样）
fn resolve_switch_target(
    s: &str,
    insn_index: usize,
    labels: &HashMap<String, usize>,
) -> Result<i32, String> {
    if let Some(&idx) = labels.get(s.trim()) {
        return Ok(idx as i32 - insn_index as i32);
    }
    parse_i32(s)
}

// ── 指令解析 ──

/// 解析单行指令文本
pub fn parse_instruction(
    line: &str,
    resolve_cp: &mut dyn FnMut(&str) -> Result<u16, String>,
    labels: &HashMap<String, usize>,
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
        // 分支目标（标签或绝对指令索引）
        "IFEQ" => Ok(Instruction::Ifeq(resolve_branch(operands, labels)?)),
        "IFNE" => Ok(Instruction::Ifne(resolve_branch(operands, labels)?)),
        "IFLT" => Ok(Instruction::Iflt(resolve_branch(operands, labels)?)),
        "IFGE" => Ok(Instruction::Ifge(resolve_branch(operands, labels)?)),
        "IFGT" => Ok(Instruction::Ifgt(resolve_branch(operands, labels)?)),
        "IFLE" => Ok(Instruction::Ifle(resolve_branch(operands, labels)?)),
        "IF_ICMPEQ" => Ok(Instruction::If_icmpeq(resolve_branch(operands, labels)?)),
        "IF_ICMPNE" => Ok(Instruction::If_icmpne(resolve_branch(operands, labels)?)),
        "IF_ICMPLT" => Ok(Instruction::If_icmplt(resolve_branch(operands, labels)?)),
        "IF_ICMPGE" => Ok(Instruction::If_icmpge(resolve_branch(operands, labels)?)),
        "IF_ICMPGT" => Ok(Instruction::If_icmpgt(resolve_branch(operands, labels)?)),
        "IF_ICMPLE" => Ok(Instruction::If_icmple(resolve_branch(operands, labels)?)),
        "IF_ACMPEQ" => Ok(Instruction::If_acmpeq(resolve_branch(operands, labels)?)),
        "IF_ACMPNE" => Ok(Instruction::If_acmpne(resolve_branch(operands, labels)?)),
        "GOTO" => Ok(Instruction::Goto(resolve_branch(operands, labels)?)),
        "JSR" => Ok(Instruction::Jsr(resolve_branch(operands, labels)?)),
        "IFNULL" => Ok(Instruction::Ifnull(resolve_branch(operands, labels)?)),
        "IFNONNULL" => Ok(Instruction::Ifnonnull(resolve_branch(operands, labels)?)),
        // 宽分支
        "GOTO_W" => Ok(Instruction::Goto_w(resolve_branch_wide(operands, labels)?)),
        "JSR_W" => Ok(Instruction::Jsr_w(resolve_branch_wide(operands, labels)?)),
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
            let mut key = String::with_capacity(operands.len() + 1);
            key.push(INTERFACE_METHOD_PREFIX);
            key.push_str(operands);
            let idx = resolve_cp(&key)?;
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

/// 零操作数指令
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

// ── switch 多行块解析 ──

fn parse_tableswitch(
    block: &[String],
    insn_index: usize,
    labels: &HashMap<String, usize>,
) -> Result<Instruction, String> {
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
            default = resolve_switch_target(rest.trim(), insn_index, labels)?;
        } else if let Some(colon_pos) = t.find(':') {
            let offset = resolve_switch_target(t[colon_pos + 1..].trim(), insn_index, labels)?;
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

fn parse_lookupswitch(
    block: &[String],
    insn_index: usize,
    labels: &HashMap<String, usize>,
) -> Result<Instruction, String> {
    let mut pairs = IndexMap::new();
    let mut default = 0i32;
    for line in &block[1..] {
        let t = line.trim();
        if t == "}" {
            break;
        }
        if let Some(rest) = t.strip_prefix("default:") {
            default = resolve_switch_target(rest.trim(), insn_index, labels)?;
        } else if let Some(colon_pos) = t.find(':') {
            let key: i32 = t[..colon_pos]
                .trim()
                .parse()
                .map_err(|e| format!("lookupswitch key: {e}"))?;
            let val = resolve_switch_target(t[colon_pos + 1..].trim(), insn_index, labels)?;
            pairs.insert(key, val);
        }
    }
    Ok(Instruction::Lookupswitch(LookupSwitch { default, pairs }))
}

// ── 辅助函数 ──

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
