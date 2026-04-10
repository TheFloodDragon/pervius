//! Tab 生命周期：打开、创建、保存、编码检测
//!
//! @author sky

use super::Layout;
use crate::ui::editor::view_toggle::ActiveView;
use crate::ui::editor::EditorTab;
use egui_editor::highlight::Language;
use pervius_java_bridge::class_structure::SavedMember;
use pervius_java_bridge::decompiler::{self, CachedSource};
use rust_i18n::t;
use std::collections::HashSet;

impl Layout {
    /// 处理 explorer 中点击的文件
    pub(super) fn handle_pending_open(&mut self) {
        let Some(entry_path) = self.file_panel.pending_open.take() else {
            return;
        };
        // 已打开的 tab 直接聚焦
        if self.editor.focus_tab(&entry_path) {
            return;
        }
        let Some(jar) = &self.jar else { return };
        let Some(raw) = jar.get(&entry_path) else {
            return;
        };
        let bytes = raw.to_vec();
        let hash = jar.hash.as_str();
        let is_modified = jar.is_modified(&entry_path);
        // 已修改条目优先从 JAR 内存缓存读取反编译结果
        let mem_cached = if is_modified {
            jar.get_decompiled(&entry_path).cloned()
        } else {
            None
        };
        let has_cache = !is_modified && decompiler::cached_source_path(hash, &entry_path).is_some();
        let tab = Self::create_tab(&entry_path, &bytes, Some(hash), mem_cached.as_ref());
        self.editor.open_tab(tab);
        if entry_path.ends_with(".class") {
            if is_modified && mem_cached.is_none() {
                // 已修改但无内存缓存（首次保存后 tab 未关闭过），重新反编译
                self.start_single_decompile(&entry_path, false);
            } else if !is_modified && !has_cache {
                self.start_single_decompile(&entry_path, true);
            }
        }
    }

    /// 保存当前聚焦 tab 的修改到 JAR 内存，并触发单文件反编译
    pub fn save_active_tab(&mut self) {
        let Some(tab) = self.editor.focused_tab_mut() else {
            return;
        };
        if !tab.is_modified {
            return;
        }
        let Some(entry_path) = tab.entry_path.clone() else {
            return;
        };
        // 纯文本文件：直接将编辑后的文本写回 JAR
        if tab.is_text {
            let new_bytes = tab.decompiled.as_bytes().to_vec();
            tab.raw_bytes = new_bytes.clone();
            tab.is_modified = false;
            if let Some(jar) = &mut self.jar {
                jar.put(&entry_path, new_bytes);
            }
            log::info!("Saved text: {entry_path}");
            return;
        }
        // class 文件保存需要 Vineflower（保存后要重新反编译刷新视图）
        if decompiler::vineflower_version().is_none() {
            self.toasts.error(t!("status.vineflower_not_found"));
            return;
        }
        let Some(cs) = &tab.class_structure else {
            return;
        };
        match pervius_java_bridge::save::apply_structure(
            &tab.raw_bytes,
            cs,
            self.jar.as_ref().map(|j| j.path.as_path()),
        ) {
            Ok(new_bytes) => {
                tab.raw_bytes = new_bytes.clone();
                tab.is_modified = false;
                // 就地翻转 modified → saved，不重建 class structure
                let mut saved_set: HashSet<SavedMember> = HashSet::new();
                if let Some(cs) = &mut tab.class_structure {
                    if cs.info.modified || cs.info.saved {
                        cs.info.saved = true;
                        saved_set.insert(SavedMember::ClassInfo);
                    }
                    cs.info.modified = false;
                    for f in &mut cs.fields {
                        if f.modified || f.saved {
                            f.saved = true;
                            saved_set
                                .insert(SavedMember::Field(f.name.clone(), f.descriptor.clone()));
                        }
                        f.modified = false;
                    }
                    for m in &mut cs.methods {
                        if m.modified || m.saved {
                            m.saved = true;
                            saved_set
                                .insert(SavedMember::Method(m.name.clone(), m.descriptor.clone()));
                        }
                        m.modified = false;
                    }
                }
                // 持久化 saved 成员（跨 tab 关闭/重开保留）
                self.editor
                    .saved_members
                    .insert(entry_path.clone(), saved_set);
                if let Some(jar) = &mut self.jar {
                    jar.put(&entry_path, new_bytes);
                }
                log::info!("Saved: {entry_path}");
                self.start_single_decompile(&entry_path, false);
            }
            Err(e) => {
                log::error!("Save failed: {entry_path}: {e}");
                self.toasts.error(t!("editor.save_failed", error = e));
            }
        }
    }

    /// 从 JAR 条目创建编辑器 tab
    ///
    /// `mem_cached` 为已修改条目的内存反编译缓存，优先于磁盘缓存。
    fn create_tab(
        entry_path: &str,
        bytes: &[u8],
        jar_hash: Option<&str>,
        mem_cached: Option<&CachedSource>,
    ) -> EditorTab {
        let file_name = entry_path.rsplit('/').next().unwrap_or(entry_path);
        let title = file_name.strip_suffix(".class").unwrap_or(file_name);
        if file_name.ends_with(".class") {
            let cached = mem_cached
                .cloned()
                .or_else(|| jar_hash.and_then(|h| decompiler::cached_source(h, entry_path)));
            let lang = match &cached {
                Some(c) if c.is_kotlin => Language::Kotlin,
                _ => Language::Java,
            };
            let mut tab = EditorTab::new_class(title, entry_path, bytes.to_vec(), lang);
            if let Some(c) = cached {
                tab.set_decompiled(c.source, lang, c.line_mapping);
                tab.active_view = ActiveView::Decompiled;
            }
            tab
        } else if Self::is_binary(bytes) {
            EditorTab::new_binary(title, entry_path, bytes.to_vec())
        } else {
            let text = Self::decode_text(bytes);
            let lang = Language::from_filename(file_name);
            EditorTab::new_text(title, entry_path, text, bytes.to_vec(), lang)
        }
    }

    /// 判断字节内容是否为二进制文件（前 8KB 内含 null 字节即视为二进制）
    fn is_binary(bytes: &[u8]) -> bool {
        let check_len = bytes.len().min(8192);
        bytes[..check_len].contains(&0)
    }

    /// 将字节解码为文本（自动检测编码）
    fn decode_text(bytes: &[u8]) -> String {
        // UTF-8 快速路径
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.to_string();
        }
        // 非 UTF-8: 用 chardetng 检测编码后转换
        let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Deny);
        detector.feed(bytes, true);
        let encoding = detector.guess(None, chardetng::Utf8Detection::Allow);
        let (text, _, _) = encoding.decode(bytes);
        text.into_owned()
    }
}
