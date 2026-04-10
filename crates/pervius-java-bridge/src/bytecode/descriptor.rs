//! JVM 描述符解析工具：类型描述符 → 可读名称
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
