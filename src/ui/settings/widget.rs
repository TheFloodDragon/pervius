//! 设置项 widget 原语（全自绘）
//!
//! 每个函数返回 `bool` 表示值是否被修改，调用方据此触发 auto-save。
//! 布局统一为：左侧 label + 右侧控件，固定行高，hover 整行高亮。
//! 所有控件通过 painter 手动绘制，不使用 egui 内置 Slider/ComboBox。
//!
//! @author sky

use crate::shell::theme;
use eframe::egui;

/// 设置行高度
const ROW_H: f32 = 32.0;
/// label 起始缩进
const PAD_LEFT: f32 = 16.0;
/// 控件距右边缘留白
const PAD_RIGHT: f32 = 16.0;

/// 分类标题
pub fn section_header(ui: &mut egui::Ui, title: &str) {
    let avail_w = ui.available_width();
    ui.add_space(8.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, 24.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.text(
        egui::pos2(rect.left() + PAD_LEFT, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(11.0),
        theme::TEXT_MUTED,
    );
    // 底部细线
    painter.line_segment(
        [
            egui::pos2(rect.left() + PAD_LEFT, rect.bottom()),
            egui::pos2(rect.right() - PAD_RIGHT, rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
}

/// 绘制设置行背景（hover 高亮），返回行 rect 和 response
fn row(ui: &mut egui::Ui, sense: egui::Sense) -> (egui::Rect, egui::Response) {
    let avail_w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(avail_w, ROW_H), sense);
    if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::BG_HOVER);
    }
    (rect, resp)
}

/// 绘制行左侧 label
fn paint_label(painter: &egui::Painter, rect: egui::Rect, label: &str) {
    painter.text(
        egui::pos2(rect.left() + PAD_LEFT, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        theme::TEXT_PRIMARY,
    );
}

/// 开关设置行
pub fn toggle(ui: &mut egui::Ui, label: &str, value: &mut bool) -> bool {
    let old = *value;
    let (rect, resp) = row(ui, egui::Sense::click());
    let painter = ui.painter();
    paint_label(painter, rect, label);
    // toggle 开关
    let w = 32.0;
    let h = 16.0;
    let mid_y = rect.center().y;
    let x = rect.right() - PAD_RIGHT - w;
    let track = egui::Rect::from_min_size(egui::pos2(x, mid_y - h / 2.0), egui::vec2(w, h));
    let r = h / 2.0;
    let (bg, knob_x) = if *value {
        (theme::VERDIGRIS, track.right() - r)
    } else {
        (theme::BG_LIGHT, track.left() + r)
    };
    painter.rect_filled(track, r, bg);
    painter.circle_filled(egui::pos2(knob_x, mid_y), r - 2.0, egui::Color32::WHITE);
    if resp.clicked() {
        *value = !*value;
    }
    *value != old
}

/// 自绘滑条设置行
pub fn slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
) -> bool {
    let old = *value;
    let min = *range.start();
    let max = *range.end();
    let avail_w = ui.available_width();
    // 需要 drag sense 给滑条用
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(avail_w, ROW_H), egui::Sense::click_and_drag());
    let painter = ui.painter();
    if resp.hovered() || resp.dragged() {
        painter.rect_filled(rect, 0.0, theme::BG_HOVER);
    }
    paint_label(painter, rect, label);
    let mid_y = rect.center().y;
    // 数值文字（最右）
    let value_text = if step >= 1.0 {
        format!("{}", *value as i32)
    } else {
        format!("{:.1}", *value)
    };
    let value_x = rect.right() - PAD_RIGHT;
    painter.text(
        egui::pos2(value_x, mid_y),
        egui::Align2::RIGHT_CENTER,
        &value_text,
        egui::FontId::monospace(11.0),
        theme::TEXT_SECONDARY,
    );
    // 滑条轨道
    let track_w = 100.0;
    let track_h = 4.0;
    let track_right = value_x - 32.0;
    let track_left = track_right - track_w;
    let track_rect = egui::Rect::from_min_size(
        egui::pos2(track_left, mid_y - track_h / 2.0),
        egui::vec2(track_w, track_h),
    );
    painter.rect_filled(track_rect, 2.0, theme::BG_LIGHT);
    // 填充部分
    let t = ((*value - min) / (max - min)).clamp(0.0, 1.0);
    let fill_w = track_w * t;
    if fill_w > 0.5 {
        let fill_rect =
            egui::Rect::from_min_size(track_rect.left_top(), egui::vec2(fill_w, track_h));
        painter.rect_filled(fill_rect, 2.0, theme::VERDIGRIS);
    }
    // 手柄
    let handle_x = track_left + fill_w;
    let handle_r = 6.0;
    let handle_color = if resp.dragged() {
        egui::Color32::WHITE
    } else if resp.hovered() {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    painter.circle_filled(egui::pos2(handle_x, mid_y), handle_r, handle_color);
    // 拖拽交互
    if resp.dragged() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let raw = ((pos.x - track_left) / track_w).clamp(0.0, 1.0);
            let mut v = min + raw * (max - min);
            if step > 0.0 {
                v = (v / step).round() * step;
            }
            *value = v.clamp(min, max);
        }
    }
    // 点击跳转
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if pos.x >= track_left && pos.x <= track_right {
                let raw = ((pos.x - track_left) / track_w).clamp(0.0, 1.0);
                let mut v = min + raw * (max - min);
                if step > 0.0 {
                    v = (v / step).round() * step;
                }
                *value = v.clamp(min, max);
            }
        }
    }
    (*value - old).abs() > f32::EPSILON
}

/// 自绘下拉选择设置行
pub fn dropdown<T: PartialEq + Copy + std::fmt::Display>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    options: &[T],
) -> bool {
    let old = *value;
    let (rect, resp) = row(ui, egui::Sense::click());
    let painter = ui.painter();
    paint_label(painter, rect, label);
    let mid_y = rect.center().y;
    // 当前值 + 下拉箭头
    let arrow = "\u{EAB4}"; // chevron_down
    let arrow_x = rect.right() - PAD_RIGHT;
    painter.text(
        egui::pos2(arrow_x, mid_y),
        egui::Align2::RIGHT_CENTER,
        arrow,
        egui::FontId::new(10.0, crate::shell::codicon::family()),
        theme::TEXT_MUTED,
    );
    let text = format!("{value}");
    painter.text(
        egui::pos2(arrow_x - 16.0, mid_y),
        egui::Align2::RIGHT_CENTER,
        &text,
        egui::FontId::proportional(12.0),
        theme::TEXT_SECONDARY,
    );
    // 下拉弹窗
    egui::Popup::from_toggle_button_response(&resp)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
        .show(|ui| {
            ui.style_mut().visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            for &opt in options {
                let opt_text = format!("{opt}");
                let selected = opt == *value;
                let (opt_rect, opt_resp) =
                    ui.allocate_exact_size(egui::vec2(100.0, 26.0), egui::Sense::click());
                let p = ui.painter();
                if selected {
                    p.rect_filled(opt_rect, 0.0, theme::BG_HOVER);
                } else if opt_resp.hovered() {
                    p.rect_filled(opt_rect, 0.0, theme::BG_LIGHT);
                }
                let color = if selected {
                    theme::VERDIGRIS
                } else {
                    theme::TEXT_PRIMARY
                };
                p.text(
                    egui::pos2(opt_rect.left() + 12.0, opt_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    &opt_text,
                    egui::FontId::proportional(12.0),
                    color,
                );
                if opt_resp.clicked() {
                    *value = opt;
                }
            }
        });
    *value != old
}

/// 路径选择设置行
///
/// 上行 label，下行文本输入框 + Browse 按钮，双行布局。
pub fn path_picker(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) -> bool {
    let old = value.clone();
    let avail_w = ui.available_width();
    let total_h = 56.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(avail_w, total_h), egui::Sense::hover());
    let painter = ui.painter();
    // label（上半部分）
    painter.text(
        egui::pos2(rect.left() + PAD_LEFT, rect.top() + 16.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        theme::TEXT_PRIMARY,
    );
    // Browse 按钮（右下角）
    let btn_w = 60.0;
    let btn_h = 22.0;
    let btn_x = rect.right() - PAD_RIGHT - btn_w;
    let btn_y = rect.bottom() - 8.0 - btn_h;
    let btn_rect = egui::Rect::from_min_size(egui::pos2(btn_x, btn_y), egui::vec2(btn_w, btn_h));
    let btn_id = ui.id().with(label).with("browse");
    let btn_resp = ui.interact(btn_rect, btn_id, egui::Sense::click());
    let btn_bg = if btn_resp.hovered() {
        theme::BG_LIGHT
    } else {
        theme::BG_MEDIUM
    };
    painter.rect_filled(btn_rect, 3.0, btn_bg);
    painter.rect_stroke(
        btn_rect,
        3.0,
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Outside,
    );
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        "Browse",
        egui::FontId::proportional(11.0),
        theme::TEXT_SECONDARY,
    );
    if btn_resp.clicked() {
        if let Some(path) = rfd::FileDialog::new().pick_folder() {
            *value = path.to_string_lossy().into_owned();
        }
    }
    // 文本输入框（左下角）
    let input_left = rect.left() + PAD_LEFT;
    let input_right = btn_x - 8.0;
    let input_w = (input_right - input_left).max(60.0);
    let input_rect =
        egui::Rect::from_min_size(egui::pos2(input_left, btn_y), egui::vec2(input_w, btn_h));
    // 输入框底部细线
    painter.line_segment(
        [
            egui::pos2(input_rect.left(), input_rect.bottom()),
            egui::pos2(input_rect.right(), input_rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme::BORDER),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(input_rect));
    child.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .frame(egui::Frame::NONE)
            .text_color(theme::TEXT_PRIMARY)
            .font(egui::FontId::proportional(11.0))
            .desired_width(input_w - 4.0),
    );
    *value != old
}
