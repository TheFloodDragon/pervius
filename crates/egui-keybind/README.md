# egui-keybind

基于 [egui](https://github.com/emilk/egui) 的声明式快捷键绑定库。定义一次，检测、回调、显示共用。

## 用法

### 定义快捷键

所有构造函数均为 `const`，可直接用于常量定义：

```rust
use egui_keybind::KeyBind;
use egui::Key;

const TOGGLE_EXPLORER: KeyBind = KeyBind::alt(Key::Num1);
const OPEN_JAR: KeyBind = KeyBind::ctrl(Key::O);
const FIND: KeyBind = KeyBind::ctrl(Key::F);
const FIND_IN_FILES: KeyBind = KeyBind::ctrl_shift(Key::F);
const SAVE: KeyBind = KeyBind::ctrl(Key::S);
const CLOSE_TAB: KeyBind = KeyBind::ctrl(Key::W);
const CLOSE_ALL_TABS: KeyBind = KeyBind::ctrl_alt(Key::W);
const OPEN_SETTINGS: KeyBind = KeyBind::ctrl(Key::Comma);
const CYCLE_VIEW: KeyBind = KeyBind::key(Key::Tab);
```

生成人类可读标签或从字符串解析：

```rust
assert_eq!(OPEN_JAR.label(), "Ctrl+O");
assert_eq!(FIND_IN_FILES.label(), "Ctrl+Shift+F");

let kb = KeyBind::parse("Ctrl+Shift+E").unwrap();
```

### 快捷键映射

通过 `KeyMap` 将快捷键绑定到 `fn(&mut T)` 回调，每帧自动检测分发。修饰键多的绑定自动优先匹配（如 `Ctrl+Shift+F` 优先于 `Ctrl+F`），同一帧内多次调用不会重复消费事件：

```rust
use egui_keybind::{KeyBind, KeyMap};

struct Layout {
    explorer_visible: bool,
    keys: KeyMap<Self>,
}

// 构建（builder 风格链式注册）
let keys = KeyMap::new()
    .bind(TOGGLE_EXPLORER, |l: &mut Layout| l.explorer_visible = !l.explorer_visible)
    .bind(OPEN_JAR, |l: &mut Layout| l.request_open_jar_dialog())
    .bind(CLOSE_TAB, |l: &mut Layout| l.editor.close_active_tab())
    .bind(FIND, |l: &mut Layout| l.editor.open_find())
    .bind(SAVE, |l: &mut Layout| l.save_active_tab())
    .bind_double_shift(|l: &mut Layout| l.search.open());

// 每帧分发（mem::take 避免自引用借用）
let mut keys = std::mem::take(&mut self.keys);
keys.dispatch(ctx, self);
self.keys = keys;
```

### Double Shift

内置 Double Shift 手势检测（两次纯 Shift 释放间隔 < 400ms 且中间无其他按键触发），适合 IDE 风格的全局搜索。

### Serde 持久化

启用 `serde` feature 后，`KeyBind` 以人类可读字符串序列化，可直接存入配置文件：

```toml
[keymap]
toggle_explorer = "Alt+1"
open_jar = "Ctrl+O"
find = "Ctrl+F"
find_in_files = "Ctrl+Shift+F"
close_tab = "Ctrl+W"
```

## Features

- `serde` -- 启用 `KeyBind` 的序列化/反序列化
