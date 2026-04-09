//! JVM 字节码（ASM textifier 格式）着色
//!
//! 行级 + 词级解析，无 tree-sitter 依赖。
//!
//! @author sky

use super::{Span, TokenKind};

/// 全部 JVM 操作码（按字典序排列，用于二分查找）
const OPCODES: &[&str] = &[
    "AALOAD",
    "AASTORE",
    "ACONST_NULL",
    "ALOAD",
    "ANEWARRAY",
    "ARETURN",
    "ARRAYLENGTH",
    "ASTORE",
    "ATHROW",
    "BALOAD",
    "BASTORE",
    "BIPUSH",
    "CALOAD",
    "CASTORE",
    "CHECKCAST",
    "D2F",
    "D2I",
    "D2L",
    "DADD",
    "DALOAD",
    "DASTORE",
    "DCMPG",
    "DCMPL",
    "DCONST_0",
    "DCONST_1",
    "DDIV",
    "DLOAD",
    "DMUL",
    "DNEG",
    "DREM",
    "DRETURN",
    "DSTORE",
    "DSUB",
    "DUP",
    "DUP2",
    "DUP2_X1",
    "DUP2_X2",
    "DUP_X1",
    "DUP_X2",
    "F2D",
    "F2I",
    "F2L",
    "FADD",
    "FALOAD",
    "FASTORE",
    "FCMPG",
    "FCMPL",
    "FCONST_0",
    "FCONST_1",
    "FCONST_2",
    "FDIV",
    "FLOAD",
    "FMUL",
    "FNEG",
    "FREM",
    "FRETURN",
    "FSTORE",
    "FSUB",
    "GETFIELD",
    "GETSTATIC",
    "GOTO",
    "I2B",
    "I2C",
    "I2D",
    "I2F",
    "I2L",
    "I2S",
    "IADD",
    "IALOAD",
    "IAND",
    "IASTORE",
    "ICONST_0",
    "ICONST_1",
    "ICONST_2",
    "ICONST_3",
    "ICONST_4",
    "ICONST_5",
    "ICONST_M1",
    "IDIV",
    "IFEQ",
    "IFGE",
    "IFGT",
    "IFLE",
    "IFLT",
    "IFNE",
    "IFNONNULL",
    "IFNULL",
    "IF_ACMPEQ",
    "IF_ACMPNE",
    "IF_ICMPEQ",
    "IF_ICMPGE",
    "IF_ICMPGT",
    "IF_ICMPLE",
    "IF_ICMPLT",
    "IF_ICMPNE",
    "IINC",
    "ILOAD",
    "IMUL",
    "INEG",
    "INSTANCEOF",
    "INVOKEDYNAMIC",
    "INVOKEINTERFACE",
    "INVOKESPECIAL",
    "INVOKESTATIC",
    "INVOKEVIRTUAL",
    "IOR",
    "IREM",
    "IRETURN",
    "ISHL",
    "ISHR",
    "ISTORE",
    "ISUB",
    "IUSHR",
    "IXOR",
    "JSR",
    "L2D",
    "L2F",
    "L2I",
    "LADD",
    "LALOAD",
    "LAND",
    "LASTORE",
    "LCMP",
    "LCONST_0",
    "LCONST_1",
    "LDC",
    "LDC2_W",
    "LDC_W",
    "LDIV",
    "LLOAD",
    "LMUL",
    "LNEG",
    "LOOKUPSWITCH",
    "LOR",
    "LREM",
    "LRETURN",
    "LSHL",
    "LSHR",
    "LSTORE",
    "LSUB",
    "LUSHR",
    "LXOR",
    "MONITORENTER",
    "MONITOREXIT",
    "MULTIANEWARRAY",
    "NEW",
    "NEWARRAY",
    "NOP",
    "POP",
    "POP2",
    "PUTFIELD",
    "PUTSTATIC",
    "RET",
    "RETURN",
    "SALOAD",
    "SASTORE",
    "SIPUSH",
    "SWAP",
    "TABLESWITCH",
];

/// 访问修饰符和声明关键字
const KEYWORDS: &[&str] = &[
    "abstract",
    "bridge",
    "class",
    "enum",
    "extends",
    "final",
    "implements",
    "interface",
    "native",
    "private",
    "protected",
    "public",
    "static",
    "strictfp",
    "synchronized",
    "synthetic",
    "transient",
    "varargs",
    "volatile",
];

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
    if OPCODES.binary_search(&word).is_ok() {
        return TokenKind::Keyword;
    }
    if KEYWORDS.binary_search(&word).is_ok() {
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
