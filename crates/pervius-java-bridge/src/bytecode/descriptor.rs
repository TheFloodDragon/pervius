//! JVM 描述符解析与文本工具：类型描述符 → 可读名称、字符串转义、常量池格式化
//!
//! @author sky

/// JVM 原始类型描述符 → 可读名称
fn primitive_name(ch: u8) -> Option<&'static str> {
    match ch {
        b'Z' => Some("boolean"),
        b'B' => Some("byte"),
        b'C' => Some("char"),
        b'S' => Some("short"),
        b'I' => Some("int"),
        b'J' => Some("long"),
        b'F' => Some("float"),
        b'D' => Some("double"),
        b'V' => Some("void"),
        _ => None,
    }
}

/// `Ljava/lang/String;` → `String`, `I` → `int`
pub fn short_descriptor(desc: &str) -> String {
    if desc.len() == 1 {
        if let Some(name) = primitive_name(desc.as_bytes()[0]) {
            return name.to_string();
        }
    }
    if desc.starts_with('[') {
        return format!("{}[]", short_descriptor(&desc[1..]));
    }
    if desc.starts_with('L') && desc.ends_with(';') {
        let inner = &desc[1..desc.len() - 1];
        return inner.rsplit('/').next().unwrap_or(inner).to_string();
    }
    desc.to_string()
}

/// `com/example/MyClass` → `MyClass`
pub fn short_class_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).to_string()
}

/// 从方法描述符提取可读返回类型
pub fn return_type_readable(descriptor: &str) -> String {
    descriptor.rfind(')').map_or_else(
        || descriptor.to_string(),
        |i| short_descriptor(&descriptor[i + 1..]),
    )
}

/// `(ILjava/lang/String;)V` → `(int, String)`
pub fn short_params(desc: &str) -> String {
    let Some(start) = desc.find('(') else {
        return String::new();
    };
    let Some(end) = desc.find(')') else {
        return String::new();
    };
    let params_str = &desc[start + 1..end];
    if params_str.is_empty() {
        return "()".to_string();
    }
    let mut params = Vec::new();
    let mut i = 0;
    let bytes = params_str.as_bytes();
    while i < bytes.len() {
        let (param, advance) = parse_one_type(params_str, i);
        params.push(param);
        i += advance;
    }
    format!("({})", params.join(", "))
}

fn parse_one_type(s: &str, start: usize) -> (String, usize) {
    let bytes = s.as_bytes();
    if start >= bytes.len() {
        return (String::new(), 1);
    }
    if let Some(name) = primitive_name(bytes[start]) {
        return (name.to_string(), 1);
    }
    match bytes[start] {
        b'[' => {
            let (inner, advance) = parse_one_type(s, start + 1);
            (format!("{inner}[]"), 1 + advance)
        }
        b'L' => {
            let semi = s[start..].find(';').unwrap_or(s.len() - start);
            let full = &s[start + 1..start + semi];
            let short = full.rsplit('/').next().unwrap_or(full);
            (short.to_string(), semi + 1)
        }
        _ => (String::new(), 1),
    }
}

/// 从 .class 字节解析版本信息
pub fn parse_class_version(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 8 || bytes[0..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    let minor = u16::from_be_bytes([bytes[4], bytes[5]]);
    let major = u16::from_be_bytes([bytes[6], bytes[7]]);
    let java_ver = if major >= 49 {
        format!("{}", major - 44)
    } else {
        format!("1.{}", major - 44)
    };
    Some(format!("Java {java_ver} (class {major}.{minor})"))
}

/// 转义 Java 字符串中的特殊字符（用于 LDC 和注解值 round-trip）
pub fn escape_java_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

/// 去除 CP 格式化文本的类型前缀，返回纯引用名称
pub fn strip_cp_prefix(formatted: &str) -> String {
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
    // String 常量加引号（转义特殊字符）
    if let Some(rest) = formatted.strip_prefix("String ") {
        return format!("\"{}\"", escape_java_string(rest));
    }
    formatted.to_string()
}

/// 将指令文本的操作码部分转为大写
pub fn uppercase_opcode(line: &str) -> String {
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
