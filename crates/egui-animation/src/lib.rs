//! egui 动画工具库
//!
//! 提供颜色插值、多维度动画等 egui 原生 `animate_value_with_time` 之上的扩展。
//!
//! @author sky

use egui::{Color32, Context, Id, Vec2};

/// 两个颜色之间的线性插值。
///
/// `t = 0.0` 返回 `a`，`t = 1.0` 返回 `b`。
pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

/// 对 `Color32` 做平滑过渡动画。
///
/// 工作方式类似 [`Context::animate_value_with_time`]，但直接接受目标颜色，
/// 内部将 RGBA 四通道拆分为独立的 f32 动画。
pub fn animate_color(ctx: &Context, id: Id, target: Color32, duration: f32) -> Color32 {
    let r = ctx.animate_value_with_time(id.with(0u8), target.r() as f32, duration);
    let g = ctx.animate_value_with_time(id.with(1u8), target.g() as f32, duration);
    let b = ctx.animate_value_with_time(id.with(2u8), target.b() as f32, duration);
    let a = ctx.animate_value_with_time(id.with(3u8), target.a() as f32, duration);
    Color32::from_rgba_premultiplied(r as u8, g as u8, b as u8, a as u8)
}

/// 对 `Vec2` 做平滑过渡动画。
///
/// 将 x、y 分量拆分为两个独立的 f32 动画。
pub fn animate_vec2(ctx: &Context, id: Id, target: Vec2, duration: f32) -> Vec2 {
    Vec2::new(
        ctx.animate_value_with_time(id.with("x"), target.x, duration),
        ctx.animate_value_with_time(id.with("y"), target.y, duration),
    )
}
