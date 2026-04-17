<div align="center">

<img src="crates/egui-shell/logo.svg" width="84" alt="Pervius logo" />

# Pervius

**Modern Java decompiler and bytecode editor.**

[Vineflower](https://github.com/Vineflower/vineflower) decompilation · [ClassForge](classforge/) bytecode rewriting · Native Rust UI

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-f74c00?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![egui](https://img.shields.io/badge/egui-0.34-1ba7f5)](https://github.com/emilk/egui)
[![Platform](https://img.shields.io/badge/Platform-Windows_·_macOS_·_Linux-8957e5)](#requirements)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)</br>
[![Decompiler](https://img.shields.io/badge/Decompiler-Vineflower_1.11.2-e76f00?logo=openjdk&logoColor=white)](https://github.com/Vineflower/vineflower)
[![Assembler](https://img.shields.io/badge/Assembler-ClassForge_1.0-b07219)](classforge/)

[Features](#features) · [Requirements](#requirements) · [Build](#build) · [Shortcuts](#shortcuts) · [中文](README_CN.md)

</div>

## Features

### Decompilation

Powered by Vineflower, with both batch JAR decompilation and on-demand single-class decompilation. Small JARs are fully decompiled upfront; large JARs decompile class-by-class on demand. Vineflower progress is parsed in real time and tracked per class. Results are cached by the JAR's SHA-256, so reopening never triggers a rebuild. Kotlin classes are auto-detected and emitted as `.kt` with original line-number mapping preserved.

<img src="screenshots/1.png" width="600" alt="Screenshot" />

### Bytecode Editing

Structured `.class` editor: the left pane navigates class info, fields, and methods; the right pane provides the matching editor. Access flags, inheritance, annotations, and descriptors are all editable, along with method instructions. On save, ClassForge (built on ASM 9.7) handles constant-pool rebuilding, StackMapTable recomputation, and `max_stack` / `max_locals`. Untouched methods are byte-copied; only modified methods trigger frame recomputation.

<img src="screenshots/3.png" width="600" alt="Screenshot" />

### Tri-View

Every `.class` can be toggled between three views with `Tab`:

- **Decompiled view** — syntax-highlighted Java / Kotlin source, read-only
- **Bytecode view** — structured editor
- **Hex view** — interactive hex inspector

Non-`.class` text files (XML, YAML, JSON, etc.) are editable directly with syntax highlighting; binary files open in the hex view.

<img src="screenshots/4.png" width="600" alt="Screenshot" />

### Code Navigation

`Ctrl+Click` (macOS `Cmd+Click`) jumps to class, method, or field definitions. Supports import resolution, same-package inference, and wildcard matching. `Ctrl+Click` on a method declaration triggers Find Usages, searching all references automatically.

### Global Search

`Double Shift` opens the search panel across every decompiled source, with regex and case-sensitivity support. Results stream back grouped by class with line-level highlighted previews; double-click to jump. The index is built in the background after decompilation completes and never blocks the UI.

<img src="screenshots/2.png" width="600" alt="Screenshot" />

### Archive Browsing

The left-hand resource tree lists JAR contents and supports `jar`, `zip`, `war`, and `ear`. Type to filter (Speed Search) with filtering computed on a background thread. Modified and decompilation states are reflected in real time. Drag-and-drop opening and a recent-files list are supported.

<img src="screenshots/5.png" width="600" alt="Screenshot" />

### Export

- **Export JAR** (`Ctrl+Shift+S`) — writes modifications back and produces a new archive
- **Export decompiled sources** (`Ctrl+Shift+E`) — exports `.java` / `.kt` to a directory, preserving the package layout

## Requirements

- `JAVA_HOME` configured

Vineflower and ClassForge are bundled and extracted to the data directory on first launch. To override, drop a JAR with the same name next to the executable (highest priority).

## Build

```bash
cargo build --release
```

ClassForge and Vineflower are embedded via `include_bytes!`, so no extra JAR copying is needed.

Build ClassForge (only required after modifying ClassForge sources):

```bash
cd classforge
./gradlew jar    # Windows: .\gradlew.bat jar
```

Copy the resulting JAR into `crates/pervius-java-bridge/libs/`, overwriting the file of the same name, then rebuild Rust.

```bash
cargo run --release
```

## Shortcuts

| Shortcut | Action |
|:---------|:-------|
| `Ctrl+O` | Open archive or single file |
| `Ctrl+S` | Save |
| `Ctrl+F` | Find |
| `Double Shift` | Global search |
| `Ctrl+Click` | Go to definition / Find Usages |
| `Tab` | Switch view |
| `Alt+1` | Toggle resource tree |
| `Ctrl+Shift+S` | Export JAR |
| `Ctrl+Shift+E` | Export decompiled sources |
| `Ctrl+,` | Settings |

All shortcuts can be customized in Settings.

## Credits

- [Vineflower](https://github.com/Vineflower/vineflower) — Java decompilation engine
- [ASM](https://asm.ow2.io/) — Java bytecode manipulation framework
- [egui](https://github.com/emilk/egui) — Rust immediate mode GUI
- [tree-sitter](https://tree-sitter.github.io/tree-sitter/) — syntax highlighting

## License

[MIT](LICENSE)
