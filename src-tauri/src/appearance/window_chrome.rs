//! Per-OS window chrome setup applied at app startup.
//!
//! macOS: rounded corners + drop shadow via Cocoa, dock icon override, vibrancy
//!        is wired separately in `vibrancy.rs`. The window itself is configured
//!        with `decorations: false` + `transparent: true` via tauri.macos.conf.json.
//! Windows: DWM rounded corners (Win11 only; no-op on Win10). The window uses
//!        native decorations and is non-transparent — Mica/Acrylic translucency
//!        is applied by `vibrancy.rs` via window-vibrancy.
//! Linux:  no-op. Native GTK chrome with an opaque background.
//!
//! This module is the single entry-point — `apply(app)` does the right thing
//! for the current target.

use tauri::Manager;

pub fn apply(app: &tauri::App) {
    apply_inner(app);
}

#[cfg(target_os = "macos")]
fn apply_inner(app: &tauri::App) {
    use cocoa::appkit::{NSApp, NSApplication, NSImage};
    use cocoa::base::nil;
    use cocoa::foundation::NSData;

    if let Some(win) = app.get_webview_window("main") {
        use objc::{runtime::Object, sel, sel_impl};
        let ns_win: *mut Object = match win.ns_window() {
            Ok(p) => p as *mut Object,
            Err(_) => return,
        };
        unsafe {
            let _: () = objc::msg_send![ns_win, setHasShadow: true];
            let content_view: *mut Object = objc::msg_send![ns_win, contentView];
            let _: () = objc::msg_send![content_view, setWantsLayer: true];
            let layer: *mut Object = objc::msg_send![content_view, layer];
            let _: () = objc::msg_send![layer, setCornerRadius: 10.0_f64];
            let _: () = objc::msg_send![layer, setMasksToBounds: true];
        }
    }

    // Override dock icon with the bundled PNG.
    let icon_data = include_bytes!("../../icons/icon.png");
    unsafe {
        let ns_data = NSData::dataWithBytes_length_(
            nil,
            icon_data.as_ptr() as *const std::ffi::c_void,
            icon_data.len() as u64,
        );
        let ns_image = NSImage::initWithData_(NSImage::alloc(nil), ns_data);
        NSApp().setApplicationIconImage_(ns_image);
    }
}

#[cfg(target_os = "windows")]
fn apply_inner(app: &tauri::App) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    };

    let win = match app.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };
    let hwnd_raw = match win.hwnd() {
        Ok(h) => h,
        Err(_) => return,
    };
    // Bridge Tauri's HWND (windows-rs version may differ from ours) via the
    // raw pointer. HWND is a pointer-shaped handle on every windows-rs version.
    let hwnd = HWND(hwnd_raw.0 as *mut _);
    let preference = DWMWCP_ROUND;
    // Silently ignored on Win10 — the attribute is a Win11+ feature.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &preference as *const _ as *const _,
            std::mem::size_of::<i32>() as u32,
        );
    }
}

#[cfg(target_os = "linux")]
fn apply_inner(_app: &tauri::App) {
    // No reliable cross-distro chrome customisation. Native GTK chrome.
}
