//! 平台特定逻辑
//!
//! @author sky

use eframe::egui;

/// 构建平台适配的 ViewportBuilder
pub fn viewport(title: &str, size: [f32; 2]) -> egui::ViewportBuilder {
    let mut vp = egui::ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(size)
        .with_min_inner_size([640.0, 400.0]);
    // macOS: 保留原生交通灯，内容延伸到标题栏
    #[cfg(target_os = "macos")]
    {
        vp = vp
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
            .with_titlebar_buttons_shown(true);
    }
    // Windows / Linux: 完全自绘标题栏
    #[cfg(not(target_os = "macos"))]
    {
        vp = vp.with_decorations(false);
    }
    vp
}

/// Windows 11 圆角（Build 22000+），旧版静默忽略
#[cfg(target_os = "windows")]
pub fn enable_rounded_corners(title: &str) {
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    unsafe extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: isize,
            dw_attribute: u32,
            pv_attribute: *const c_void,
            cb_attribute: u32,
        ) -> i32;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
    }

    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;

    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
        if hwnd == 0 {
            return;
        }
        let preference = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const u32 as *const c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}
