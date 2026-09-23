#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

mod app;
mod config;
mod context_menu;
mod dock;
mod layout;
mod model;
mod platform;
mod shell_menu;
mod theme;
mod ui;
// OSを実際に操作する層は自動テストできないため、カバレッジ計測から外す。
#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod win32;

use app::LauncherApp;
use config::ConfigStore;
use eframe::egui;
use layout::dock_geometry;
use platform::Platform;
use std::rc::Rc;
use theme::{japanese_fonts, JAPANESE_FONT_PATH};

#[cfg(windows)]
fn system_platform() -> Rc<dyn Platform> {
    Rc::new(win32::WindowsPlatform)
}

#[cfg(not(windows))]
fn system_platform() -> Rc<dyn Platform> {
    Rc::new(platform::NullPlatform)
}

// eframeのイベントループを起動するだけで、テストからは実行できない。
#[cfg_attr(coverage_nightly, coverage(off))]
fn main() -> eframe::Result {
    let platform = system_platform();
    let (position, size) = dock_geometry(platform.work_area());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([54.0, 220.0])
            // 幅を固定し、ドラッグ中のWindowsのスナップで横に広がらないようにする。
            .with_max_inner_size([54.0, 4000.0])
            .with_position(position)
            .with_decorations(false)
            .with_resizable(true)
            .with_fullscreen(false)
            .with_maximized(false)
            .with_transparent(true)
            .with_title("Windows Side Dock"),
        ..Default::default()
    };
    let windows_dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_default();
    eframe::run_native(
        "Windows Side Dock",
        options,
        Box::new(move |cc| {
            if let Some(fonts) = japanese_fonts(JAPANESE_FONT_PATH) {
                cc.egui_ctx.set_fonts(fonts);
            }
            Ok(Box::new(LauncherApp::new(
                platform,
                ConfigStore::from_env(),
                &windows_dir,
                &local_app_data,
            )))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_operating_system_platform() {
        let platform = system_platform();
        assert_eq!(Rc::strong_count(&platform), 1);
    }
}
