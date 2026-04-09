//! 字节码结构化面板：左侧成员导航 + 右侧详情/代码
//!
//! @author sky

use super::highlight;
use super::tab::{BytecodeSelection, EditorTab};
use crate::java::class_structure::{ClassStructure, EditableAnnotation, FieldInfo, MethodInfo};
use crate::shell::{codicon, theme};
use crate::ui::widget::FlatButton;
use eframe::egui;
use egui_animation::Anim;
use std::sync::Arc;

/// 导航栏最小宽度
const MIN_NAV_WIDTH: f32 = 120.0;
/// 导航栏最大宽度
const MAX_NAV_WIDTH: f32 = 500.0;
/// 拖拽手柄宽度
const RESIZE_HANDLE_W: f32 = 6.0;
/// 导航项行高
const NAV_ROW_HEIGHT: f32 = 24.0;
/// section label 行高
const SECTION_LABEL_HEIGHT: f32 = 28.0;
/// 内容区内边距
const CONTENT_PAD: f32 = 16.0;
/// 元数据 key-value 间距
const KV_KEY_WIDTH: f32 = 100.0;
/// 代码字体大小
const CODE_FONT_SIZE: f32 = 13.0;

/// 渲染字节码结构化面板
pub fn render_bytecode_panel(ui: &mut egui::Ui, tab: &mut EditorTab) {
    if tab.class_structure.is_none() {
        ui.centered_and_justified(|ui| {
            ui.label(
                egui::RichText::new("No class data")
                    .color(theme::TEXT_MUTED)
                    .size(14.0),
            );
        });
        return;
    }
    let rect = ui.max_rect();
    let nav_w = tab.nav_width.clamp(MIN_NAV_WIDTH, rect.width() - 100.0);
    tab.nav_width = nav_w;
    let painter = ui.painter();
    // 左侧导航背景
    let nav_rect = egui::Rect::from_min_size(rect.left_top(), egui::vec2(nav_w, rect.height()));
    painter.rect_filled(nav_rect, 0.0, theme::BG_GUTTER);
    let divider_x = rect.left() + nav_w;
    // 右侧内容背景
    let content_rect =
        egui::Rect::from_min_max(egui::pos2(divider_x, rect.top()), rect.right_bottom());
    painter.rect_filled(content_rect, 0.0, theme::BG_DARKEST);
    // 拖拽 resize 手柄
    let handle_rect = egui::Rect::from_min_size(
        egui::pos2(divider_x - RESIZE_HANDLE_W / 2.0, rect.top()),
        egui::vec2(RESIZE_HANDLE_W, rect.height()),
    );
    let handle_id = ui.id().with("bc_resize");
    let handle_resp = ui.interact(handle_rect, handle_id, egui::Sense::drag());
    if handle_resp.dragged() {
        tab.nav_width = (nav_w + handle_resp.drag_delta().x)
            .clamp(MIN_NAV_WIDTH, MAX_NAV_WIDTH.min(rect.width() - 100.0));
    }
    if handle_resp.hovered() || handle_resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
    }
    // 左侧导航（immutable borrow，scope 内释放）
    let selection = tab.bc_selection;
    let mut new_selection = None;
    {
        let cs = tab.class_structure.as_ref().unwrap();
        let field_count = cs.fields.len();
        let method_count = cs.methods.len();
        let mut nav_ui = ui.new_child(egui::UiBuilder::new().max_rect(nav_rect));
        nav_ui.set_clip_rect(nav_rect);
        egui::ScrollArea::vertical()
            .id_salt(ui.id().with("bc_nav"))
            .auto_shrink(false)
            .show(&mut nav_ui, |ui| {
                ui.set_min_width(nav_w);
                new_selection = render_nav(ui, cs, selection, field_count, method_count);
            });
    }
    if let Some(sel) = new_selection {
        tab.bc_selection = sel;
    }
    // 右侧内容（可 mutable borrow）
    let selection = tab.bc_selection;
    let mut content_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
    let changed = match selection {
        BytecodeSelection::ClassInfo => {
            let cs = tab.class_structure.as_mut().unwrap();
            render_class_info_editable(&mut content_ui, cs)
        }
        BytecodeSelection::Field(idx) => {
            let cs = tab.class_structure.as_mut().unwrap();
            cs.fields
                .get_mut(idx)
                .map_or(false, |field| render_field_editable(&mut content_ui, field))
        }
        BytecodeSelection::Method(idx) => {
            let cs = tab.class_structure.as_mut().unwrap();
            cs.methods.get_mut(idx).map_or(false, |method| {
                render_method_editable(&mut content_ui, method)
            })
        }
    };
    if changed {
        tab.is_modified = true;
    }
}

// ── 左侧导航栏 ──

fn render_nav(
    ui: &mut egui::Ui,
    cs: &ClassStructure,
    selection: BytecodeSelection,
    field_count: usize,
    method_count: usize,
) -> Option<BytecodeSelection> {
    let mut new_selection = None;
    // CLASS INFO
    render_section_label(ui, "CLASS INFO");
    if render_nav_item(
        ui,
        codicon::SYMBOL_CLASS,
        &short_class_name(&cs.info.name),
        "",
        selection == BytecodeSelection::ClassInfo,
    ) {
        new_selection = Some(BytecodeSelection::ClassInfo);
    }
    // FIELDS
    if !cs.fields.is_empty() {
        ui.add_space(4.0);
        render_section_label(ui, &format!("FIELDS ({field_count})"));
        for (i, field) in cs.fields.iter().enumerate() {
            let selected = selection == BytecodeSelection::Field(i);
            let type_hint = short_descriptor(&field.descriptor);
            if render_nav_item(ui, codicon::SYMBOL_FIELD, &field.name, &type_hint, selected) {
                new_selection = Some(BytecodeSelection::Field(i));
            }
        }
    }
    // METHODS
    if !cs.methods.is_empty() {
        ui.add_space(4.0);
        render_section_label(ui, &format!("METHODS ({method_count})"));
        for (i, method) in cs.methods.iter().enumerate() {
            let selected = selection == BytecodeSelection::Method(i);
            let params = short_params(&method.descriptor);
            let label = format!("{}{params}", method.name);
            if render_nav_item(ui, codicon::SYMBOL_METHOD, &label, "", selected) {
                new_selection = Some(BytecodeSelection::Method(i));
            }
        }
    }
    new_selection
}

fn render_section_label(ui: &mut egui::Ui, text: &str) {
    let avail_w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail_w, SECTION_LABEL_HEIGHT),
        egui::Sense::hover(),
    );
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(10.0),
        theme::TEXT_MUTED,
    );
}

fn render_nav_item(
    ui: &mut egui::Ui,
    icon: &str,
    label: &str,
    suffix: &str,
    selected: bool,
) -> bool {
    let avail_w = ui.available_width();
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(avail_w, NAV_ROW_HEIGHT), egui::Sense::click());
    let painter = ui.painter();
    // 选中 / hover 背景动画
    let anim = Anim::new(ui, 0.1).with(label);
    let target_bg = if selected {
        theme::BG_HOVER
    } else if resp.hovered() {
        theme::BG_LIGHT
    } else {
        egui::Color32::TRANSPARENT
    };
    let bg = anim.color("bg", target_bg);
    if bg.a() > 0 {
        painter.rect_filled(rect, 0.0, bg);
    }
    if selected {
        painter.rect_filled(
            egui::Rect::from_min_size(rect.left_top(), egui::vec2(2.0, rect.height())),
            0.0,
            theme::VERDIGRIS,
        );
    }
    let mid_y = rect.center().y;
    let icon_x = rect.left() + 12.0;
    let text_x = icon_x + 18.0;
    painter.text(
        egui::pos2(icon_x, mid_y),
        egui::Align2::LEFT_CENTER,
        icon,
        egui::FontId::new(12.0, codicon::family()),
        if selected {
            theme::VERDIGRIS
        } else {
            theme::TEXT_MUTED
        },
    );
    let label_color = if selected {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_SECONDARY
    };
    let clip = egui::Rect::from_min_max(
        egui::pos2(text_x, rect.top()),
        egui::pos2(rect.right() - 4.0, rect.bottom()),
    );
    if suffix.is_empty() {
        let galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(12.0),
            label_color,
        );
        painter.with_clip_rect(clip).galley(
            egui::pos2(text_x, mid_y - galley.size().y / 2.0),
            galley,
            label_color,
        );
    } else {
        let suffix_galley = painter.layout_no_wrap(
            suffix.to_string(),
            egui::FontId::proportional(11.0),
            theme::TEXT_MUTED,
        );
        let suffix_w = suffix_galley.size().x;
        let label_galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(12.0),
            label_color,
        );
        let label_clip = egui::Rect::from_min_max(
            egui::pos2(text_x, rect.top()),
            egui::pos2(rect.right() - suffix_w - 12.0, rect.bottom()),
        );
        painter.with_clip_rect(label_clip).galley(
            egui::pos2(text_x, mid_y - label_galley.size().y / 2.0),
            label_galley,
            label_color,
        );
        painter.with_clip_rect(clip).galley(
            egui::pos2(
                rect.right() - 4.0 - suffix_w,
                mid_y - suffix_galley.size().y / 2.0,
            ),
            suffix_galley,
            theme::TEXT_MUTED,
        );
    }
    resp.clicked()
}

// ── 右侧：Class Info（可编辑） ──

fn render_class_info_editable(ui: &mut egui::Ui, cs: &mut ClassStructure) -> bool {
    let mut changed = false;
    egui::ScrollArea::vertical()
        .id_salt("bc_class_info")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add_space(CONTENT_PAD);
            changed |= render_editable_kv(ui, "Access", &mut cs.info.access, theme::SYN_KEYWORD);
            changed |= render_editable_kv(ui, "Name", &mut cs.info.name, theme::TEXT_PRIMARY);
            changed |=
                render_editable_kv(ui, "Extends", &mut cs.info.super_class, theme::TEXT_PRIMARY);
            if !cs.info.interfaces.is_empty() {
                render_kv(ui, "Implements", &cs.info.interfaces.join(", "));
            }
            // 注解
            ui.add_space(8.0);
            changed |= render_annotations(ui, &mut cs.info.annotations);
            // 只读属性
            if cs.info.signature.is_some()
                || cs.info.source_file.is_some()
                || cs.info.is_deprecated
                || !cs.info.version.is_empty()
            {
                ui.add_space(12.0);
                render_kv(ui, "Version", &cs.info.version);
                if let Some(sig) = &cs.info.signature {
                    render_kv(ui, "Signature", sig);
                }
                if let Some(src) = &cs.info.source_file {
                    render_kv(ui, "Source", src);
                }
                if cs.info.is_deprecated {
                    render_kv(ui, "Deprecated", "true");
                }
            }
            // 常量池
            if !cs.cp_entries.is_empty() {
                ui.add_space(16.0);
                render_constant_pool(ui, &cs.cp_entries);
            }
            ui.add_space(CONTENT_PAD);
        });
    changed
}

// ── 右侧：Field（可编辑） ──

fn render_field_editable(ui: &mut egui::Ui, field: &mut FieldInfo) -> bool {
    let mut changed = false;
    egui::ScrollArea::vertical()
        .id_salt("bc_field")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add_space(CONTENT_PAD);
            changed |= render_editable_kv(ui, "Access", &mut field.access, theme::SYN_KEYWORD);
            changed |= render_editable_kv(ui, "Name", &mut field.name, theme::TEXT_PRIMARY);
            changed |=
                render_editable_kv(ui, "Descriptor", &mut field.descriptor, theme::TEXT_PRIMARY);
            render_preview(ui, &format!("→ {}", short_descriptor(&field.descriptor)));
            if let Some(cv) = &mut field.constant_value {
                changed |= render_editable_kv(ui, "Value", cv, theme::SYN_STRING);
            }
            // 注解
            ui.add_space(8.0);
            changed |= render_annotations(ui, &mut field.annotations);
            // 只读属性（signature 等不常编辑的）
            render_readonly_attrs(
                ui,
                &field.signature,
                field.is_deprecated,
                field.is_synthetic,
            );
            ui.add_space(CONTENT_PAD);
        });
    changed
}

// ── 右侧：Method（可编辑） ──

fn render_method_editable(ui: &mut egui::Ui, method: &mut MethodInfo) -> bool {
    let mut changed = false;
    egui::ScrollArea::both()
        .id_salt("bc_method")
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.add_space(CONTENT_PAD);
            changed |= render_editable_kv(ui, "Access", &mut method.access, theme::SYN_KEYWORD);
            changed |= render_editable_kv(ui, "Name", &mut method.name, theme::TEXT_PRIMARY);
            changed |= render_editable_kv(
                ui,
                "Descriptor",
                &mut method.descriptor,
                theme::TEXT_PRIMARY,
            );
            let ret = return_type_readable(&method.descriptor);
            let params = short_params(&method.descriptor);
            render_preview(ui, &format!("→ {ret} {}{params}", method.name));
            if !method.exceptions.is_empty() {
                let throws = method
                    .exceptions
                    .iter()
                    .map(|e| short_class_name(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                render_kv(ui, "Throws", &throws);
            }
            // 注解
            ui.add_space(8.0);
            changed |= render_annotations(ui, &mut method.annotations);
            // 字节码（多行可编辑 + 语法高亮）
            if method.has_code {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(CONTENT_PAD);
                    let line_count = method.bytecode.lines().count().max(1);
                    let mut layouter = |ui: &egui::Ui,
                                        text: &dyn egui::TextBuffer,
                                        _wrap_width: f32|
                     -> Arc<egui::Galley> {
                        let s = text.as_str();
                        let spans = highlight::compute_bytecode_spans(s);
                        let job = highlight::build_layout_job(s, &spans);
                        ui.fonts_mut(|f| f.layout_job(job))
                    };
                    changed |= ui
                        .add(
                            egui::TextEdit::multiline(&mut method.bytecode)
                                .font(egui::FontId::monospace(CODE_FONT_SIZE))
                                .desired_width(f32::INFINITY)
                                .desired_rows(line_count)
                                .frame(egui::Frame::NONE)
                                .layouter(&mut layouter),
                        )
                        .changed();
                });
            } else {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(CONTENT_PAD);
                    ui.label(
                        egui::RichText::new("// no code (abstract or native)")
                            .color(theme::TEXT_MUTED)
                            .font(egui::FontId::monospace(12.0)),
                    );
                });
            }
            // 只读属性
            render_readonly_attrs(
                ui,
                &method.signature,
                method.is_deprecated,
                method.is_synthetic,
            );
            ui.add_space(CONTENT_PAD);
        });
    changed
}

// ── 注解结构化编辑 ──

/// Kotlin 编译器内部注解，编辑无意义，只读展示
fn is_kotlin_internal_annotation(type_desc: &str) -> bool {
    matches!(
        type_desc,
        "Lkotlin/Metadata;" | "Lkotlin/jvm/internal/SourceDebugExtension;"
    )
}

/// 渲染注解列表：每行一个注解（类型 + 元素），可增删改。返回是否变更
fn render_annotations(ui: &mut egui::Ui, annotations: &mut Vec<EditableAnnotation>) -> bool {
    let mut changed = false;
    let mut remove_idx = None;
    for (i, ann) in annotations.iter_mut().enumerate() {
        let readonly = is_kotlin_internal_annotation(&ann.type_desc);
        ui.horizontal(|ui| {
            ui.add_space(CONTENT_PAD);
            // @ 前缀
            ui.label(
                egui::RichText::new("@")
                    .color(theme::SYN_ANNOTATION)
                    .font(egui::FontId::monospace(12.0)),
            );
            if readonly {
                // 只读：纯 label 展示
                ui.label(
                    egui::RichText::new(&ann.type_desc)
                        .color(theme::TEXT_MUTED)
                        .font(egui::FontId::monospace(12.0)),
                );
            } else {
                // 类型描述符（可编辑）
                changed |= ui
                    .add(
                        egui::TextEdit::singleline(&mut ann.type_desc)
                            .font(egui::FontId::monospace(12.0))
                            .text_color(theme::SYN_ANNOTATION)
                            .desired_width(200.0)
                            .frame(egui::Frame::NONE),
                    )
                    .changed();
            }
            // 元素
            if !ann.elements.is_empty() {
                ui.label(
                    egui::RichText::new("(")
                        .color(theme::TEXT_MUTED)
                        .font(egui::FontId::monospace(12.0)),
                );
                let last = ann.elements.len() - 1;
                for (j, elem) in ann.elements.iter_mut().enumerate() {
                    ui.label(
                        egui::RichText::new(&format!("{} = ", elem.name))
                            .color(theme::TEXT_MUTED)
                            .font(egui::FontId::monospace(12.0)),
                    );
                    // 值
                    let color = match elem.tag {
                        b's' => theme::SYN_STRING,
                        b'I' | b'J' | b'F' | b'D' | b'B' | b'S' | b'C' => theme::SYN_NUMBER,
                        b'Z' => theme::SYN_KEYWORD,
                        _ => theme::SYN_TEXT,
                    };
                    if readonly {
                        ui.label(
                            egui::RichText::new(&elem.value)
                                .color(color)
                                .font(egui::FontId::monospace(12.0)),
                        );
                    } else {
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut elem.value)
                                    .font(egui::FontId::monospace(12.0))
                                    .text_color(color)
                                    .desired_width(120.0)
                                    .frame(egui::Frame::NONE),
                            )
                            .changed();
                    }
                    if j < last {
                        ui.label(
                            egui::RichText::new(", ")
                                .color(theme::TEXT_MUTED)
                                .font(egui::FontId::monospace(12.0)),
                        );
                    }
                }
                ui.label(
                    egui::RichText::new(")")
                        .color(theme::TEXT_MUTED)
                        .font(egui::FontId::monospace(12.0)),
                );
            }
            // 删除按钮（只读注解不显示）
            if !readonly {
                if ui
                    .add(
                        FlatButton::new(codicon::CLOSE)
                            .font_size(10.0)
                            .font_family(codicon::family())
                            .inactive_color(theme::TEXT_MUTED)
                            .min_size(egui::vec2(18.0, 18.0)),
                    )
                    .on_hover_text("Remove annotation")
                    .clicked()
                {
                    remove_idx = Some(i);
                }
            }
        });
    }
    if let Some(idx) = remove_idx {
        annotations.remove(idx);
        changed = true;
    }
    // 添加按钮
    ui.horizontal(|ui| {
        ui.add_space(CONTENT_PAD);
        if ui
            .add(
                FlatButton::new("+ annotation")
                    .font_size(11.0)
                    .inactive_color(theme::TEXT_MUTED)
                    .min_size(egui::vec2(0.0, 20.0)),
            )
            .clicked()
        {
            annotations.push(EditableAnnotation {
                type_desc: String::new(),
                elements: Vec::new(),
            });
            changed = true;
        }
    });
    changed
}

// ── 常量池 ──

/// 常量池行高
const CP_ROW_HEIGHT: f32 = 20.0;
/// 索引列宽
const CP_IDX_WIDTH: f32 = 40.0;
/// 类型列宽
const CP_TAG_WIDTH: f32 = 130.0;

/// 渲染常量池条目表
fn render_constant_pool(ui: &mut egui::Ui, entries: &[(u16, &str, String)]) {
    ui.horizontal(|ui| {
        ui.add_space(CONTENT_PAD);
        ui.label(
            egui::RichText::new(format!("CONSTANT POOL ({})", entries.len()))
                .color(theme::TEXT_MUTED)
                .font(egui::FontId::proportional(10.0)),
        );
    });
    ui.add_space(4.0);
    for &(idx, tag, ref value) in entries {
        let tag_color = cp_tag_color(tag);
        ui.horizontal(|ui| {
            ui.add_space(CONTENT_PAD);
            // index
            ui.add_sized(
                egui::vec2(CP_IDX_WIDTH, CP_ROW_HEIGHT),
                egui::Label::new(
                    egui::RichText::new(format!("#{idx}"))
                        .color(theme::TEXT_MUTED)
                        .font(egui::FontId::monospace(11.0)),
                ),
            );
            // type tag
            ui.add_sized(
                egui::vec2(CP_TAG_WIDTH, CP_ROW_HEIGHT),
                egui::Label::new(
                    egui::RichText::new(tag)
                        .color(tag_color)
                        .font(egui::FontId::monospace(11.0)),
                ),
            );
            // value
            ui.add(
                egui::Label::new(
                    egui::RichText::new(value)
                        .color(theme::TEXT_PRIMARY)
                        .font(egui::FontId::monospace(11.0)),
                )
                .truncate(),
            );
        });
    }
}

/// 根据常量类型返回颜色
fn cp_tag_color(tag: &str) -> egui::Color32 {
    match tag {
        "Utf8" => theme::TEXT_MUTED,
        "Integer" | "Float" | "Long" | "Double" => theme::SYN_NUMBER,
        "String" => theme::SYN_STRING,
        "Class" => theme::SYN_TYPE,
        "MethodRef" | "InterfaceMethodRef" => theme::SYN_KEYWORD,
        "FieldRef" => theme::VERDIGRIS,
        "NameAndType" => theme::TEXT_SECONDARY,
        _ => theme::TEXT_MUTED,
    }
}

// ── 通用 widget ──

/// 单行可编辑文本（无边框 monospace），返回 Response
fn styled_singleline(
    ui: &mut egui::Ui,
    text: &mut String,
    color: egui::Color32,
    font_size: f32,
) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(text)
            .font(egui::FontId::monospace(font_size))
            .text_color(color)
            .desired_width(f32::INFINITY)
            .frame(egui::Frame::NONE),
    )
}

/// 渲染 signature / deprecated / synthetic 只读属性块
fn render_readonly_attrs(
    ui: &mut egui::Ui,
    signature: &Option<String>,
    is_deprecated: bool,
    is_synthetic: bool,
) {
    if signature.is_none() && !is_deprecated && !is_synthetic {
        return;
    }
    ui.add_space(12.0);
    if let Some(sig) = signature {
        render_kv(ui, "Signature", sig);
    }
    if is_deprecated {
        render_kv(ui, "Deprecated", "true");
    }
    if is_synthetic {
        render_kv(ui, "Synthetic", "true");
    }
}

fn render_kv(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_space(CONTENT_PAD);
        if !key.is_empty() {
            ui.label(
                egui::RichText::new(key)
                    .color(theme::TEXT_MUTED)
                    .font(egui::FontId::proportional(12.0)),
            );
            let used = ui.min_rect().width();
            if used < KV_KEY_WIDTH {
                ui.add_space(KV_KEY_WIDTH - used);
            }
        } else {
            ui.add_space(KV_KEY_WIDTH);
        }
        ui.label(
            egui::RichText::new(value)
                .color(theme::TEXT_PRIMARY)
                .font(egui::FontId::monospace(12.0)),
        );
    });
}

/// key-value 行（value 可编辑），返回 value 是否变更
fn render_editable_kv(
    ui: &mut egui::Ui,
    key: &str,
    value: &mut String,
    color: egui::Color32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_space(CONTENT_PAD);
        ui.label(
            egui::RichText::new(key)
                .color(theme::TEXT_MUTED)
                .font(egui::FontId::proportional(12.0)),
        );
        let used = ui.min_rect().width();
        if used < KV_KEY_WIDTH {
            ui.add_space(KV_KEY_WIDTH - used);
        }
        changed = styled_singleline(ui, value, color, 13.0).changed();
    });
    changed
}

/// 可读预览行（与 KV value 列对齐）
fn render_preview(ui: &mut egui::Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(KV_KEY_WIDTH);
        ui.label(
            egui::RichText::new(text)
                .color(theme::TEXT_MUTED)
                .font(egui::FontId::proportional(11.0)),
        );
    });
}

// ── 辅助 ──

/// 从方法描述符提取可读返回类型
fn return_type_readable(descriptor: &str) -> String {
    descriptor.rfind(')').map_or_else(
        || descriptor.to_string(),
        |i| short_descriptor(&descriptor[i + 1..]),
    )
}

/// `com/example/MyClass` → `MyClass`
fn short_class_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).to_string()
}

/// `Ljava/lang/String;` → `String`, `I` → `int`
fn short_descriptor(desc: &str) -> String {
    match desc {
        "Z" => "boolean".to_string(),
        "B" => "byte".to_string(),
        "C" => "char".to_string(),
        "S" => "short".to_string(),
        "I" => "int".to_string(),
        "J" => "long".to_string(),
        "F" => "float".to_string(),
        "D" => "double".to_string(),
        "V" => "void".to_string(),
        _ if desc.starts_with('[') => format!("{}[]", short_descriptor(&desc[1..])),
        _ if desc.starts_with('L') && desc.ends_with(';') => {
            let inner = &desc[1..desc.len() - 1];
            inner.rsplit('/').next().unwrap_or(inner).to_string()
        }
        _ => desc.to_string(),
    }
}

/// `(ILjava/lang/String;)V` → `(int, String)`
fn short_params(desc: &str) -> String {
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
    match bytes[start] {
        b'Z' => ("boolean".to_string(), 1),
        b'B' => ("byte".to_string(), 1),
        b'C' => ("char".to_string(), 1),
        b'S' => ("short".to_string(), 1),
        b'I' => ("int".to_string(), 1),
        b'J' => ("long".to_string(), 1),
        b'F' => ("float".to_string(), 1),
        b'D' => ("double".to_string(), 1),
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
