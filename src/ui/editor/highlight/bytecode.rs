//! JVM 字节码着色
//!
//! 行级 + 词级解析，无 tree-sitter 依赖。
//! 支持 Recaf 风格反汇编输出（大写操作码 + 内联 CP 引用）。
//!
//! @author sky

use super::{Span, TokenKind};

/// 全部 JVM 操作码（按字典序排列，用于二分查找，大写）
const OPCODES: &[&str] = &[
    "AALOAD",
    "AASTORE",
    "ACONST_NULL",
    "ALOAD",
    "ALOAD_0",
    "ALOAD_1",
    "ALOAD_2",
    "ALOAD_3",
    "ALOAD_W",
    "ANEWARRAY",
    "ARETURN",
    "ARRAYLENGTH",
    "ASTORE",
    "ASTORE_0",
    "ASTORE_1",
    "ASTORE_2",
    "ASTORE_3",
    "ASTORE_W",
    "ATHROW",
    "BALOAD",
    "BASTORE",
    "BIPUSH",
    "BREAKPOINT",
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
    "DLOAD_0",
    "DLOAD_1",
    "DLOAD_2",
    "DLOAD_3",
    "DLOAD_W",
    "DMUL",
    "DNEG",
    "DREM",
    "DRETURN",
    "DSTORE",
    "DSTORE_0",
    "DSTORE_1",
    "DSTORE_2",
    "DSTORE_3",
    "DSTORE_W",
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
    "FLOAD_0",
    "FLOAD_1",
    "FLOAD_2",
    "FLOAD_3",
    "FLOAD_W",
    "FMUL",
    "FNEG",
    "FREM",
    "FRETURN",
    "FSTORE",
    "FSTORE_0",
    "FSTORE_1",
    "FSTORE_2",
    "FSTORE_3",
    "FSTORE_W",
    "FSUB",
    "GETFIELD",
    "GETSTATIC",
    "GOTO",
    "GOTO_W",
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
    "IINC_W",
    "ILOAD",
    "ILOAD_0",
    "ILOAD_1",
    "ILOAD_2",
    "ILOAD_3",
    "ILOAD_W",
    "IMPDEP1",
    "IMPDEP2",
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
    "ISTORE_0",
    "ISTORE_1",
    "ISTORE_2",
    "ISTORE_3",
    "ISTORE_W",
    "ISUB",
    "IUSHR",
    "IXOR",
    "JSR",
    "JSR_W",
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
    "LLOAD_0",
    "LLOAD_1",
    "LLOAD_2",
    "LLOAD_3",
    "LLOAD_W",
    "LMUL",
    "LNEG",
    "LOOKUPSWITCH",
    "LOR",
    "LREM",
    "LRETURN",
    "LSHL",
    "LSHR",
    "LSTORE",
    "LSTORE_0",
    "LSTORE_1",
    "LSTORE_2",
    "LSTORE_3",
    "LSTORE_W",
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
    "RET_W",
    "RETURN",
    "SALOAD",
    "SASTORE",
    "SIPUSH",
    "SWAP",
    "TABLESWITCH",
    "WIDE",
];

/// 访问修饰符和声明关键字（按字典序排列，小写）
const KEYWORDS: &[&str] = &[
    "abstract",
    "any",
    "bridge",
    "class",
    "default",
    "define",
    "enum",
    "extends",
    "final",
    "implements",
    "interface",
    "line",
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
            tokenize_meta_line(line, offset, &mut spans);
        } else if trimmed == "Exception table:"
            || trimmed.starts_with("from ")
            || is_exception_data(trimmed)
        {
            spans.push((offset, offset + line_len, TokenKind::Muted));
        } else {
            tokenize_line(line, offset, &mut spans);
        }
        offset += line_len + 1;
    }
    spans
}

/// 判断是否为异常表数据行（纯数字 + 末尾类型名）
fn is_exception_data(trimmed: &str) -> bool {
    // 格式: "  0   8   9   java/io/IOException" 或 "  0   8   9   any"
    let mut parts = trimmed.split_ascii_whitespace();
    let first = match parts.next() {
        Some(p) => p,
        None => return false,
    };
    first.bytes().all(|b| b.is_ascii_digit()) && parts.count() >= 2
}

/// MAXSTACK / MAXLOCALS 行：关键字 Muted，= Muted，数字 Number
fn tokenize_meta_line(line: &str, base: usize, spans: &mut Vec<Span>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        let b = bytes[i];
        if b.is_ascii_whitespace() {
            let start = i;
            while i < len && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            spans.push((base + start, base + i, TokenKind::Plain));
            continue;
        }
        if b == b'=' {
            spans.push((base + i, base + i + 1, TokenKind::Muted));
            i += 1;
            continue;
        }
        if b.is_ascii_digit() {
            let start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            spans.push((base + start, base + i, TokenKind::Number));
            continue;
        }
        // MAXSTACK / MAXLOCALS 关键字
        let start = i;
        while i < len && bytes[i].is_ascii_alphabetic() {
            i += 1;
        }
        if i > start {
            spans.push((base + start, base + i, TokenKind::Muted));
        }
    }
}

fn tokenize_line(line: &str, base: usize, spans: &mut Vec<Span>) {
    let bytes = line.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    // 检测指令索引模式：前导空白 + 数字 + ':'
    i = skip_whitespace(bytes, i, len, base, spans);
    if i < len && bytes[i].is_ascii_digit() {
        let digit_start = i;
        while i < len && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i < len && bytes[i] == b':' {
            // 指令索引 → Muted
            spans.push((base + digit_start, base + i, TokenKind::Muted));
            spans.push((base + i, base + i + 1, TokenKind::Muted));
            i += 1;
        } else {
            // 不是指令索引，回退当普通 token 处理
            i = digit_start;
        }
    }
    // 主循环
    while i < len {
        let b = bytes[i];
        // 内联 // 注释
        if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            spans.push((base + i, base + len, TokenKind::Comment));
            return;
        }
        if b.is_ascii_whitespace() {
            i = skip_whitespace(bytes, i, len, base, spans);
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
        if matches!(b, b'{' | b'}' | b'(' | b')' | b';' | b'=' | b',') {
            spans.push((base + i, base + i + 1, TokenKind::Muted));
            i += 1;
            continue;
        }
        // 冒号（方法签名中的 ':' 分隔符，非指令索引 — 指令索引已在前面处理）
        if b == b':' {
            spans.push((base + i, base + i + 1, TokenKind::Muted));
            i += 1;
            continue;
        }
        // 单词（含 / . $ 等组成的标识符路径，以及 #ref、负数 -N）
        let start = i;
        while i < len && !is_token_break(bytes[i]) {
            i += 1;
        }
        if i > start {
            let word = &line[start..i];
            let kind = classify_word(word);
            spans.push((base + start, base + i, kind));
        }
    }
}

/// 跳过空白并记录 span
fn skip_whitespace(
    bytes: &[u8],
    mut i: usize,
    len: usize,
    base: usize,
    spans: &mut Vec<Span>,
) -> usize {
    let start = i;
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i > start {
        spans.push((base + start, base + i, TokenKind::Plain));
    }
    i
}

/// token 分隔符
fn is_token_break(b: u8) -> bool {
    b.is_ascii_whitespace()
        || matches!(
            b,
            b'{' | b'}' | b'(' | b')' | b';' | b':' | b'=' | b',' | b'"'
        )
}

fn classify_word(word: &str) -> TokenKind {
    // 操作码（大小写不敏感）
    let upper = word.to_ascii_uppercase();
    if OPCODES.binary_search(&upper.as_str()).is_ok() {
        return TokenKind::Keyword;
    }
    // 访问修饰符 / 声明关键字（大小写不敏感）
    let lower = word.to_ascii_lowercase();
    if KEYWORDS.binary_search(&lower.as_str()).is_ok() {
        return TokenKind::Keyword;
    }
    // 方法特殊名
    if word == "<init>" || word == "<clinit>" {
        return TokenKind::MethodDecl;
    }
    // #常量池引用
    if word.starts_with('#') && word.len() > 1 && word[1..].bytes().all(|b| b.is_ascii_digit()) {
        return TokenKind::Number;
    }
    // 数字（十进制、十六进制、负数）
    if is_number(word) {
        return TokenKind::Number;
    }
    // 类型描述符
    if is_descriptor(word) {
        return TokenKind::Type;
    }
    // 内部名称路径（java/lang/Object 等）
    if is_internal_name(word) {
        return TokenKind::Type;
    }
    // 成员引用（Owner.member 或 Owner."<init>"）
    if word.contains('.') && word.contains('/') {
        return TokenKind::Type;
    }
    // 标签（L0, L1, ...）
    if word.starts_with('L') && word.len() > 1 && word[1..].bytes().all(|b| b.is_ascii_digit()) {
        return TokenKind::Number;
    }
    TokenKind::Plain
}

fn is_number(word: &str) -> bool {
    if word.starts_with("0x") || word.starts_with("0X") {
        return word.len() > 2 && word[2..].bytes().all(|b| b.is_ascii_hexdigit());
    }
    let s = if word.starts_with('-') {
        &word[1..]
    } else {
        word
    };
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

/// JVM 类型描述符
fn is_descriptor(word: &str) -> bool {
    // 单字符基本类型
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

/// 内部名称路径（java/lang/Object）
fn is_internal_name(word: &str) -> bool {
    word.contains('/')
        && word
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'/' || b == b'$' || b == b'_')
}
