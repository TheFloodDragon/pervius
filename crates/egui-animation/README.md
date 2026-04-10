# egui-animation

基于 [egui](https://github.com/emilk/egui) 的作用域动画工具。通过 `Anim` 一次性绑定 `Context`、基础 `Id` 和 `duration`，后续动画调用零样板。

## 用法

```rust
let anim = Anim::new(ui, 0.15);

// f32 值过渡
let offset = anim.f32("offset", target_offset);

// 布尔状态 → 0.0..=1.0 过渡值
let t = anim.bool("expanded", is_expanded);

// 颜色过渡（RGBA 四通道独立插值）
let color = anim.color("text", target_color);

// Vec2 过渡（x/y 分量独立插值）
let pos = anim.vec2("pos", target_pos);

// 选中/悬停背景色动画（自动处理 click 帧闪烁）
let bg = anim.select_bg(selected, hovered, clicked, sel_color, hover_color);
```

循环中用 `.with(salt)` 区分不同元素：

```rust
for (i, item) in items.iter().enumerate() {
    let anim = Anim::new(ui, 0.15).with(i);
    let bg = anim.color("bg", target);
}
```

另提供独立函数 `lerp_color(a, b, t)` 用于一次性颜色插值。
