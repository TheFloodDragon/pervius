# egui-hex-view

基于 [egui](https://github.com/emilk/egui) 的交互式十六进制查看器 widget。

自绘 HexGrid 布局（地址列 + Hex 列 + ASCII 列），支持虚拟滚动、字节分类着色、搜索高亮，所有颜色通过 `HexTheme` 外部配置。

## 功能

- 三栏布局：`Offset` 地址列 | `00..0F` Hex 列 | `Decoded text` ASCII 列
- 字节分类着色：null / 可打印 ASCII / 控制字符 / 高位字节各自独立颜色
- 虚拟滚动：只绘制可见行，大文件无性能问题
- 交互操作：
  - 点击选中字节（hex 区或 ASCII 区），双向联动高亮
  - 拖选范围，Shift+Click 扩展选区
  - 键盘导航（方向键 / PgUp / PgDn / Home / End）
  - Ctrl+A 全选，Ctrl+C 复制（hex 或 ASCII 格式）
  - 右键菜单（Copy as Hex / Copy as ASCII / Copy Offset / Select All）
- 搜索高亮：外部传入匹配范围 `(start, end)` 和当前匹配索引
- 外部滚动定位：设置 `scroll_to_byte` 跳转到指定字节
- 固定底部 Inspector 面板：显示光标/hover 字节的多格式解读（i8/u8/i16/u16/i32/u32/i64/u64/f32/f64，大小端）

## 用法

```rust
use egui_hex_view::{HexTheme, HexViewState};

// 状态持久化在调用方
let mut state = HexViewState::default();

// 主题配色
let theme = HexTheme {
    hex_null_color: egui_hex_view::color(60, 60, 70, 128),
    hex_printable_color: Color32::from_rgb(200, 200, 210),
    // ...
};

// 每帧渲染
egui_hex_view::show(
    ui,
    &raw_bytes,        // 原始字节数据
    &mut state,        // 交互状态
    &theme,            // 配色
    &highlights,       // 搜索匹配范围 Vec<(usize, usize)>
    current_highlight, // 当前匹配索引 Option<usize>
);

// 外部触发滚动到指定字节
state.scroll_to_byte = Some(0x1A3F);
```

## 国际化

通过 `HexTheme.labels` 字段（`HexLabels`）配置所有 UI 文字标签，支持自定义翻译：

```rust
labels: HexLabels {
    empty: "(empty)".into(),
    copy_hex: "Copy as Hex".into(),
    copy_ascii: "Copy as ASCII".into(),
    copy_offset: "Copy Offset".into(),
    select_all: "Select All".into(),
    // ...
},
```
