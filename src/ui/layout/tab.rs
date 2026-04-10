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
                self.start_single_decompile(&entry_path, bytes, false);
            } else if !is_modified && !has_cache {
                self.start_single_decompile(&entry_path, bytes, true);
            }
        }
    }

    /// 保存当前聚焦 tab 的修改
    ///
    /// 独立文件直接写回磁盘；JAR 条目写入 JAR 内存并触发重反编译。
    pub fn save_active_tab(&mut self) {
        let Some(tab) = self.editor.focused_tab_mut() else {
            return;
        };
        if !tab.is_modified {
            return;
        }
        // 独立文件：直接写回磁盘，不参与 JAR modified 管理
        if let Some(disk_path) = tab.standalone_path.clone() {
            let new_bytes = if tab.is_text {
                tab.decompiled.as_bytes().to_vec()
            } else if tab.is_class {
                let Some(cs) = &tab.class_structure else {
                    return;
                };
                match pervius_java_bridge::save::apply_structure(&tab.raw_bytes, cs, None) {
                    Ok(b) => b,
                    Err(e) => {
                        log::error!("Save failed: {}: {e}", disk_path.display());
                        self.toasts.error(t!("editor.save_failed", error = e));
                        return;
                    }
                }
            } else {
                tab.raw_bytes.clone()
            };
            if let Err(e) = std::fs::write(&disk_path, &new_bytes) {
                log::error!("Write failed: {}: {e}", disk_path.display());
                self.toasts.error(t!("layout.write_file_failed", error = e));
                return;
            }
            tab.raw_bytes = new_bytes.clone();
            tab.is_modified = false;
            // 独立 class 文件也需要重建 class structure
            if tab.is_class {
                if let Ok(new_cs) = pervius_java_bridge::bytecode::disassemble(&new_bytes) {
                    tab.class_structure = Some(new_cs);
                }
            }
            let is_class = tab.is_class;
            let entry_path = tab.entry_path.clone();
            log::info!("Saved standalone: {}", disk_path.display());
            // class 文件保存后重新反编译刷新视图（tab 借用在此之前结束）
            if is_class {
                if let Some(ep) = entry_path {
                    self.start_single_decompile(&ep, new_bytes, false);
                }
            }
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
                // 收集 saved 成员（modified 或已 saved 的）
                let mut saved_set: HashSet<SavedMember> = HashSet::new();
                if let Some(cs) = &tab.class_structure {
                    if cs.info.modified || cs.info.saved {
                        saved_set.insert(SavedMember::ClassInfo);
                    }
                    for f in &cs.fields {
                        if f.modified || f.saved {
                            saved_set
                                .insert(SavedMember::Field(f.name.clone(), f.descriptor.clone()));
                        }
                    }
                    for m in &cs.methods {
                        if m.modified || m.saved {
                            saved_set
                                .insert(SavedMember::Method(m.name.clone(), m.descriptor.clone()));
                        }
                    }
                }
                // 从新字节重建 class structure，保持与 raw_bytes 同步
                // （不重建会导致二次保存时所有方法被误判为 CHANGED）
                match pervius_java_bridge::bytecode::disassemble(&new_bytes) {
                    Ok(mut new_cs) => {
                        if saved_set.contains(&SavedMember::ClassInfo) {
                            new_cs.info.saved = true;
                        }
                        for f in &mut new_cs.fields {
                            if saved_set
                                .contains(&SavedMember::Field(f.name.clone(), f.descriptor.clone()))
                            {
                                f.saved = true;
                            }
                        }
                        for m in &mut new_cs.methods {
                            if saved_set.contains(&SavedMember::Method(
                                m.name.clone(),
                                m.descriptor.clone(),
                            )) {
                                m.saved = true;
                            }
                        }
                        tab.class_structure = Some(new_cs);
                    }
                    Err(e) => {
                        log::warn!("Failed to rebuild class structure after save: {e}");
                    }
                }
                // 持久化 saved 成员（跨 tab 关闭/重开保留）
                self.editor
                    .saved_members
                    .insert(entry_path.clone(), saved_set);
                if let Some(jar) = &mut self.jar {
                    jar.put(&entry_path, new_bytes.clone());
                }
                log::info!("Saved: {entry_path}");
                self.start_single_decompile(&entry_path, new_bytes, false);
            }
            Err(e) => {
                log::error!("Save failed: {entry_path}: {e}");
                self.toasts.error(t!("editor.save_failed", error = e));
            }
        }
    }

    /// 从条目路径和字节创建编辑器 tab
    ///
    /// `mem_cached` 为已修改条目的内存反编译缓存，优先于磁盘缓存。
    pub(super) fn create_tab(
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
