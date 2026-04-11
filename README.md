<div align="center">

<img src="crates/egui-shell/logo.svg" width="84" alt="Pervius logo" />

# Pervius

**现代化的 Java 反编译与字节码编辑工具。**

[Vineflower](https://github.com/Vineflower/vineflower) 反编译 · [ClassForge](classforge/) 字节码重写 · Rust 原生界面

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-f74c00?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![egui](https://img.shields.io/badge/egui-0.34-1ba7f5)](https://github.com/emilk/egui)
[![Platform](https://img.shields.io/badge/Platform-Windows_·_macOS_·_Linux-8957e5)](#运行要求)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[![Decompiler](https://img.shields.io/badge/Decompiler-Vineflower_1.11.1-e76f00?logo=openjdk&logoColor=white)](https://github.com/Vineflower/vineflower)
[![Assembler](https://img.shields.io/badge/Assembler-ClassForge_1.0-b07219)](classforge/)

[特性](#特性) · [运行要求](#运行要求) · [构建](#构建) · [快捷键](#快捷键) · [项目结构](#项目结构)

</div>

## 特性

### 反编译

基于 Vineflower，支持 JAR 批量反编译和单文件按需反编译。小体积 JAR 自动全量反编译，大 JAR 按需逐类反编译。Vineflower 输出实时解析进度，逐类跟踪。结果按 JAR 的 SHA-256 缓存，重复打开不重编译。Kotlin 类自动检测并输出 `.kt`，保留原始行号映射。

<img src="screenshots/1.png" width="600" alt="截图" />

### 字节码编辑

结构化 `.class` 编辑面板：左侧导航类信息、字段、方法，右侧对应编辑区。可修改访问标志、继承关系、注解、描述符，方法指令也可直接编辑。保存时 ClassForge（基于 ASM 9.7）自动处理常量池重建、StackMapTable 重算和 max_stack/max_locals。未修改方法直接字节拷贝，仅对改动方法触发帧重算。

<img src="screenshots/3.png" width="600" alt="截图" />

### 三视图

每个 `.class` 可通过 `Tab` 在三种视图间切换：

- **反编译视图** — 语法高亮的 Java/Kotlin 源码，只读
- **字节码视图** — 结构化编辑面板
- **Hex 视图** — 交互式十六进制查看器

非 `.class` 的文本文件（XML、YAML、JSON 等）直接可编辑，带语法高亮；二进制文件以 Hex 视图打开。

<img src="screenshots/4.png" width="600" alt="截图" />

### 全局搜索

`Double Shift` 打开搜索面板，覆盖所有反编译源码，支持正则和大小写敏感。结果流式返回，按类分组，行级高亮预览，双击跳转。反编译完成后后台自动建索引，不阻塞 UI。

<img src="screenshots/2.png" width="600" alt="截图" />

### 归档浏览

左侧资源树展示 JAR 内容，支持 `jar` `zip` `war` `ear`。键入即过滤（Speed Search），过滤计算在后台线程完成。修改状态实时标记，反编译状态实时可见。支持拖拽打开和最近文件列表。

### 导出

- **导出 JAR**（`Ctrl+Shift+S`）— 修改写回 JAR，生成新归档
- **导出反编译源码**（`Ctrl+Shift+E`）— 导出 `.java`/`.kt` 到指定目录，保留包结构

## 运行要求

- 已配置 `JAVA_HOME`
- 可执行文件同目录放置 `vineflower-*.jar`
- 可执行文件同目录放置 `classforge-*.jar`

缺 `vineflower` 无法反编译；缺 `classforge` 无法写回字节码修改。

## 构建

```bash
cargo build --release
```

构建 ClassForge：

```bash
cd classforge
./gradlew jar    # Windows: .\gradlew.bat jar
```

将 `classforge/build/libs/classforge-1.0.jar` 和 `vineflower-*.jar` 复制到可执行文件同目录，然后运行：

```bash
cargo run --release
```

> `cargo run` 从 `target/debug/` 查找 JAR，`cargo run --release` 从 `target/release/` 查找。

## 快捷键

| 快捷键 | 操作 |
|:-------|:-----|
| `Ctrl+O` | 打开归档或单文件 |
| `Ctrl+S` | 保存 |
| `Ctrl+F` | 查找 |
| `Double Shift` | 全局搜索 |
| `Tab` | 切换视图 |
| `Alt+1` | 切换资源树 |
| `Ctrl+Shift+S` | 导出 JAR |
| `Ctrl+Shift+E` | 导出反编译源码 |
| `Ctrl+,` | 设置 |

所有快捷键可在设置中自定义。

## 致谢

- [Vineflower](https://github.com/Vineflower/vineflower) — Java 反编译引擎
- [ASM](https://asm.ow2.io/) — Java 字节码操作框架
- [egui](https://github.com/emilk/egui) — Rust immediate mode GUI
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/) — 语法高亮

## 许可证

[MIT](LICENSE)