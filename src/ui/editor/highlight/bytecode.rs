//! JVM 字节码（ASM textifier 格式）着色
//!
//! 行级 + 词级解析，无 tree-sitter 依赖。
//!
//! @author sky

use super::{Span, TokenKind};

/// JVM 操作码（全大写指令）
fn is_opcode(word: &str) -> bool {
    matches!(
        word,
        "NOP"
            | "ACONST_NULL"
            | "ICONST_M1"
            | "ICONST_0"
            | "ICONST_1"
            | "ICONST_2"
            | "ICONST_3"
            | "ICONST_4"
            | "ICONST_5"
            | "LCONST_0"
            | "LCONST_1"
            | "FCONST_0"
            | "FCONST_1"
            | "FCONST_2"
            | "DCONST_0"
            | "DCONST_1"
            | "BIPUSH"
            | "SIPUSH"
            | "LDC"
            | "LDC_W"
            | "LDC2_W"
            | "ILOAD"
            | "LLOAD"
            | "FLOAD"
            | "DLOAD"
            | "ALOAD"
            | "IALOAD"
            | "LALOAD"
            | "FALOAD"
            | "DALOAD"
            | "AALOAD"
            | "BALOAD"
            | "CALOAD"
            | "SALOAD"
            | "ISTORE"
            | "LSTORE"
            | "FSTORE"
            | "DSTORE"
            | "ASTORE"
            | "IASTORE"
            | "LASTORE"
            | "FASTORE"
            | "DASTORE"
            | "AASTORE"
            | "BASTORE"
            | "CASTORE"
            | "SASTORE"
            | "POP"
            | "POP2"
            | "DUP"
            | "DUP_X1"
            | "DUP_X2"
            | "DUP2"
            | "DUP2_X1"
            | "DUP2_X2"
            | "SWAP"
            | "IADD"
            | "LADD"
            | "FADD"
            | "DADD"
            | "ISUB"
            | "LSUB"
            | "FSUB"
            | "DSUB"
            | "IMUL"
            | "LMUL"
            | "FMUL"
            | "DMUL"
            | "IDIV"
            | "LDIV"
            | "FDIV"
            | "DDIV"
            | "IREM"
            | "LREM"
            | "FREM"
            | "DREM"
            | "INEG"
            | "LNEG"
            | "FNEG"
            | "DNEG"
            | "ISHL"
            | "LSHL"
            | "ISHR"
            | "LSHR"
            | "IUSHR"
            | "LUSHR"
            | "IAND"
            | "LAND"
            | "IOR"
            | "LOR"
            | "IXOR"
            | "LXOR"
            | "IINC"
            | "I2L"
            | "I2F"
            | "I2D"
            | "L2I"
            | "L2F"
            | "L2D"
            | "F2I"
            | "F2L"
            | "F2D"
            | "D2I"
            | "D2L"
            | "D2F"
            | "I2B"
            | "I2C"
            | "I2S"
            | "LCMP"
            | "FCMPL"
            | "FCMPG"
            | "DCMPL"
            | "DCMPG"
            | "IFEQ"
            | "IFNE"
            | "IFLT"
            | "IFGE"
            | "IFGT"
            | "IFLE"
            | "IF_ICMPEQ"
            | "IF_ICMPNE"
            | "IF_ICMPLT"
            | "IF_ICMPGE"
            | "IF_ICMPGT"
            | "IF_ICMPLE"
            | "IF_ACMPEQ"
            | "IF_ACMPNE"
            | "GOTO"
            | "JSR"
            | "RET"
            | "TABLESWITCH"
            | "LOOKUPSWITCH"
            | "IRETURN"
            | "LRETURN"
            | "FRETURN"
            | "DRETURN"
            | "ARETURN"
            | "RETURN"
            | "GETSTATIC"
            | "PUTSTATIC"
            | "GETFIELD"
            | "PUTFIELD"
            | "INVOKEVIRTUAL"
            | "INVOKESPECIAL"
            | "INVOKESTATIC"
            | "INVOKEINTERFACE"
            | "INVOKEDYNAMIC"
            | "NEW"
            | "NEWARRAY"
            | "ANEWARRAY"
            | "ARRAYLENGTH"
            | "ATHROW"
            | "CHECKCAST"
            | "INSTANCEOF"
            | "MONITORENTER"
            | "MONITOREXIT"
            | "MULTIANEWARRAY"
            | "IFNULL"
            | "IFNONNULL"
    )
}

/// 访问修饰符和声明关键字
fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "public"
            | "private"
            | "protected"
            | "static"
            | "final"
            | "abstract"
            | "native"
            | "synchronized"
            | "transient"
            | "volatile"
            | "strictfp"
            | "synthetic"
            | "bridge"
            | "varargs"
            | "class"
            | "interface"
            | "enum"
            | "extends"
            | "implements"
    )
}

pub fn collect_spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for line in source.split('\n') {
        let line_len = line.len();
        let trimmed = line.trim();
        if trimmed.starts_with("//") {
            spans.push((offset, offset + line_len, TokenKind::Comment));
        } else if trimmed.starts_with("MAXSTACK") || trimmed.starts_with("MAXLOCALS") {
            spans.push((offset, offset + line_len, TokenKind::Muted));
        } else {
            tokenize_line(line, offset, &mut spans);
        }
        offset += line_len + 1;
    }
    spans
}

fn tokenize_line(line: &str, base: usize, spans: &mut Vec<Span>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let b = bytes[i];
        // 跳过空白
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            spans.push((base + start, base + i, TokenKind::Plain));
            continue;
        }
        // 字符串 "..."
        if b == b'"' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            spans.push((base + start, base + i, TokenKind::String));
            continue;
        }
        // 标点 / 分隔符
        if matches!(b, b'{' | b'}' | b'(' | b')' | b';' | b':' | b'=' | b',') {
            spans.push((base + i, base + i + 1, TokenKind::Muted));
            i += 1;
            continue;
        }
        // 单词
        let start = i;
        while i < len
            && !bytes[i].is_ascii_whitespace()
            && !matches!(
                bytes[i],
                b'{' | b'}' | b'(' | b')' | b';' | b':' | b'=' | b',' | b'"'
            )
        {
            i += 1;
        }
        if i > start {
            let word = &line[start..i];
            let kind = classify_word(word);
            spans.push((base + start, base + i, kind));
        }
    }
}

fn classify_word(word: &str) -> TokenKind {
    if is_opcode(word) {
        return TokenKind::Keyword;
    }
    if is_keyword(word) {
        return TokenKind::Keyword;
    }
    // 数字（十进制、十六进制）
    if word.starts_with("0x")
        || word.starts_with("0X")
        || word.bytes().next().is_some_and(|b| b.is_ascii_digit())
            && word.bytes().all(|b| b.is_ascii_digit() || b == b'.')
    {
        return TokenKind::Number;
    }
    // 类型描述符：L...;  或单字符 V Z B C S I J F D
    if is_descriptor(word) {
        return TokenKind::Type;
    }
    // 标签（L0, L1, ...）
    if word.starts_with('L') && word[1..].bytes().all(|b| b.is_ascii_digit()) && word.len() > 1 {
        return TokenKind::Number;
    }
    TokenKind::Plain
}

/// 判断是否为 JVM 类型描述符
fn is_descriptor(word: &str) -> bool {
    // 单字符描述符
    if word.len() == 1
        && matches!(
            word.as_bytes()[0],
            b'V' | b'Z' | b'B' | b'C' | b'S' | b'I' | b'J' | b'F' | b'D'
        )
    {
        return true;
    }
    // L...;  对象类型描述符
    if word.starts_with('L') && word.ends_with(';') && word.len() > 2 {
        return true;
    }
    // [开头的数组描述符
    if word.starts_with('[') {
        return true;
    }
    false
}
