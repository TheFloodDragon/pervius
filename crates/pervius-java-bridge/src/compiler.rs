//! ClassForge 源码编译 bridge：调用 JDK javax.tools.JavaCompiler
//!
//! @author sky

use crate::assembler;
use crate::error::BridgeError;
use crate::process;
use std::io::{Cursor, Read, Write};
use std::path::Path;

/// Kotlin 源文件输入
#[derive(Clone, Debug)]
pub struct KotlinSource {
    /// 相对文件路径，如 `com/example/Foo.kt`
    pub path: String,
    /// Kotlin 源码
    pub source: String,
}

/// 编译产物 class
#[derive(Clone, Debug)]
pub struct CompiledClass {
    /// 斜杠形式 binary name，如 `com/foo/Bar$Inner`
    pub binary_name: String,
    /// class 文件字节
    pub bytes: Vec<u8>,
}

/// 编译诊断等级
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagSeverity {
    Error,
    Warning,
    Note,
}

/// 编译诊断
#[derive(Clone, Debug)]
pub struct CompileDiagnostic {
    pub severity: DiagSeverity,
    pub line: u32,
    pub column: u32,
    pub message: String,
}

/// 编译结果
#[derive(Clone, Debug)]
pub enum CompileOutcome {
    Success(Vec<CompiledClass>),
    Errors(Vec<CompileDiagnostic>),
    JdkMissing,
}

/// 当前 Java 配置是否能找到同目录 javac
pub fn is_jdk_available() -> bool {
    let Ok(java) = process::find_java() else {
        return false;
    };
    let Some(bin) = java.parent() else {
        return false;
    };
    let javac = bin.join(if cfg!(windows) { "javac.exe" } else { "javac" });
    javac.exists()
}

/// 定位 Kotlin compiler embeddable JAR（用户可放在 exe 同目录或数据目录）
pub fn find_kotlinc() -> Result<std::path::PathBuf, BridgeError> {
    crate::find_jar("kotlinc-embeddable", |_| true, None).or_else(|_| {
        crate::find_jar("kotlin-compiler-embeddable", |_| true, None)
    })
}

/// 调用 classforge --compile 编译 Java 源码
pub fn compile_source(
    source: &str,
    binary_name: &str,
    classpath_jar: Option<&Path>,
    target: Option<u8>,
    debug: bool,
    original_class: Option<&[u8]>,
) -> Result<CompileOutcome, BridgeError> {
    let classforge = crate::find_jar(
        "classforge",
        |_| true,
        Some((crate::BUNDLED_CLASSFORGE, crate::BUNDLED_CLASSFORGE_NAME)),
    )?;
    let mut cmd = process::JavaCommand::new(&classforge)?;
    cmd.arg("--compile");
    if let Some(path) = classpath_jar {
        cmd.arg("--classpath").arg(path);
    }
    if let Some(target) = target {
        cmd.arg("--target").arg(target.to_string());
    }
    if debug {
        cmd.arg("--debug");
    }
    let mut child = cmd.spawn_with_stdin().map_err(BridgeError::SpawnFailed)?;
    if let Some(mut stdin) = child.stdin.take() {
        write_prefixed_string(&mut stdin, binary_name)?;
        write_prefixed_string_u32(&mut stdin, source)?;
    }
    let output = child.wait_with_output()?;
    if output.stdout.first() == Some(&2) {
        return Ok(CompileOutcome::JdkMissing);
    }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::Compile(stderr.into_owned()));
    }
    let outcome = parse_compile_output(&output.stdout)?;
    postprocess_compile_output(outcome, classpath_jar, original_class)
}

/// 调用 classforge --compile-kt 编译 Kotlin 源码
pub fn compile_kotlin_sources(
    sources: &[KotlinSource],
    classpath_jar: Option<&Path>,
    target: Option<u8>,
    module_name: Option<&str>,
    original_class: Option<&[u8]>,
) -> Result<CompileOutcome, BridgeError> {
    let classforge = crate::find_jar(
        "classforge",
        |_| true,
        Some((crate::BUNDLED_CLASSFORGE, crate::BUNDLED_CLASSFORGE_NAME)),
    )?;
    let kotlinc = find_kotlinc()?;
    let mut cmd = process::JavaCommand::with_classpath(
        &[&classforge, &kotlinc],
        "pervius.asm.ClassForge",
    )?;
    cmd.arg("--compile-kt");
    if let Some(path) = classpath_jar {
        cmd.arg("--classpath").arg(path);
    }
    if let Some(target) = target {
        cmd.arg("--target").arg(target.to_string());
    }
    if let Some(module_name) = module_name {
        cmd.arg("--module-name").arg(module_name);
    }
    let mut child = cmd.spawn_with_stdin().map_err(BridgeError::SpawnFailed)?;
    if let Some(mut stdin) = child.stdin.take() {
        write_kotlin_sources_protocol(&mut stdin, sources)?;
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BridgeError::Compile(stderr.into_owned()));
    }
    let outcome = parse_compile_output(&output.stdout)?;
    postprocess_compile_output(outcome, classpath_jar, original_class)
}

fn postprocess_compile_output(
    outcome: CompileOutcome,
    classpath_jar: Option<&Path>,
    original_class: Option<&[u8]>,
) -> Result<CompileOutcome, BridgeError> {
    let CompileOutcome::Success(classes) = outcome else {
        return Ok(outcome);
    };
    let target_major = original_class.and_then(assembler::class_major_version);
    let mut processed = Vec::with_capacity(classes.len());
    for mut class in classes {
        assembler::validate_class_bytes(&class.bytes, &format!("compiled class {}", class.binary_name))
            .map_err(|e| BridgeError::Compile(e.to_string()))?;
        let major = assembler::class_major_version(&class.bytes);
        if class.binary_name != "module-info" {
            // javac/kotlinc 产物不一定包含完整 StackMapTable；统一交给 ASM 重算。
            // module-info 没有普通方法体，部分 ASM 流程重算它没有收益，直接保留 javac 输出。
            class.bytes = assembler::recompute_frames(&class.bytes, classpath_jar)?;
        }
        let new_major = assembler::class_major_version(&class.bytes);
        if class.binary_name != "module-info" {
            if let (Some(expected), Some(actual)) = (target_major, new_major) {
                if actual != expected {
                    return Err(BridgeError::Compile(format!(
                        "compiled class {} has major version {}, expected {}",
                        class.binary_name, actual, expected
                    )));
                }
            }
        }
        log::debug!(
            "compiled class {}: major {:?} -> {:?}, {} bytes",
            class.binary_name,
            major,
            new_major,
            class.bytes.len()
        );
        processed.push(class);
    }
    Ok(CompileOutcome::Success(processed))
}

fn parse_compile_output(stdout: &[u8]) -> Result<CompileOutcome, BridgeError> {
    let mut cur = Cursor::new(stdout);
    let status = read_u8(&mut cur)?;
    match status {
        0 => {
            let count = read_u32(&mut cur)? as usize;
            let mut classes = Vec::with_capacity(count);
            for _ in 0..count {
                let binary_name = read_prefixed_string(&mut cur)?;
                let len = read_u32(&mut cur)? as usize;
                let mut bytes = vec![0; len];
                cur.read_exact(&mut bytes)?;
                classes.push(CompiledClass { binary_name, bytes });
            }
            Ok(CompileOutcome::Success(classes))
        }
        1 => {
            let count = read_u32(&mut cur)? as usize;
            let mut diagnostics = Vec::with_capacity(count);
            for _ in 0..count {
                let severity = match read_u8(&mut cur)? {
                    0 => DiagSeverity::Error,
                    1 => DiagSeverity::Warning,
                    _ => DiagSeverity::Note,
                };
                let line = read_u32(&mut cur)?;
                let column = read_u32(&mut cur)?;
                let message = read_prefixed_string_u32(&mut cur)?;
                diagnostics.push(CompileDiagnostic {
                    severity,
                    line,
                    column,
                    message,
                });
            }
            Ok(CompileOutcome::Errors(diagnostics))
        }
        2 => Ok(CompileOutcome::JdkMissing),
        other => Err(BridgeError::Compile(format!(
            "unexpected compiler status byte {other}"
        ))),
    }
}

fn write_kotlin_sources_protocol(
    w: &mut dyn Write,
    sources: &[KotlinSource],
) -> Result<(), BridgeError> {
    w.write_all(&(sources.len() as u32).to_be_bytes())?;
    for source in sources {
        write_prefixed_string(w, &source.path)?;
        write_prefixed_string_u32(w, &source.source)?;
    }
    Ok(())
}

fn write_prefixed_string(w: &mut dyn Write, s: &str) -> Result<(), BridgeError> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err(BridgeError::Compile("binary name is too long".to_string()));
    }
    w.write_all(&(bytes.len() as u16).to_be_bytes())?;
    w.write_all(bytes)?;
    Ok(())
}

fn write_prefixed_string_u32(w: &mut dyn Write, s: &str) -> Result<(), BridgeError> {
    let bytes = s.as_bytes();
    w.write_all(&(bytes.len() as u32).to_be_bytes())?;
    w.write_all(bytes)?;
    Ok(())
}

fn read_u8(r: &mut Cursor<&[u8]>) -> Result<u8, BridgeError> {
    let mut buf = [0; 1];
    r.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn read_u32(r: &mut Cursor<&[u8]>) -> Result<u32, BridgeError> {
    let mut buf = [0; 4];
    r.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn read_prefixed_string(r: &mut Cursor<&[u8]>) -> Result<String, BridgeError> {
    let mut len_buf = [0; 2];
    r.read_exact(&mut len_buf)?;
    let len = u16::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| BridgeError::Compile(e.to_string()))
}

fn read_prefixed_string_u32(r: &mut Cursor<&[u8]>) -> Result<String, BridgeError> {
    let len = read_u32(r)? as usize;
    let mut buf = vec![0; len];
    r.read_exact(&mut buf)?;
    String::from_utf8(buf).map_err(|e| BridgeError::Compile(e.to_string()))
}
