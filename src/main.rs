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
#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod win32_tray;

use app::LauncherApp;
use config::ConfigStore;
use eframe::egui;
use layout::dock_geometry;
use platform::Platform;
use std::rc::Rc;
use theme::{app_icon, japanese_fonts, JAPANESE_FONT_PATH};

/// OSの実装と、タスクトレイの操作をその実装へ届ける待ち行列。
#[cfg(windows)]
fn system_platform() -> (Rc<dyn Platform>, win32_tray::TrayQueue) {
    let tray_actions = win32_tray::TrayQueue::default();
    let platform = win32::WindowsPlatform {
        tray_actions: tray_actions.clone(),
    };
    (Rc::new(platform), tray_actions)
}

#[cfg(not(windows))]
fn system_platform() -> (Rc<dyn Platform>, ()) {
    (Rc::new(platform::NullPlatform), ())
}

#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
fn install_tray(
    ctx: &egui::Context,
    queue: win32_tray::TrayQueue,
) -> Option<Box<dyn std::any::Any>> {
    win32_tray::install_tray(ctx.clone(), queue)
        .map(|tray| Box::new(tray) as Box<dyn std::any::Any>)
}

#[cfg(not(windows))]
fn install_tray(_ctx: &egui::Context, _queue: ()) -> Option<Box<dyn std::any::Any>> {
    None
}

// eframeのイベントループを起動するだけで、テストからは実行できない。
#[cfg_attr(coverage_nightly, coverage(off))]
fn main() -> eframe::Result {
    let (platform, tray_actions) = system_platform();
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
            .with_title("Windows Side Dock")
            .with_icon(app_icon()),
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
            let mut app = LauncherApp::new(
                platform,
                ConfigStore::from_env(),
                &windows_dir,
                &local_app_data,
            );
            app.tray = install_tray(&cc.egui_ctx, tray_actions);
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_operating_system_platform() {
        let (platform, tray_actions) = system_platform();
        assert_eq!(Rc::strong_count(&platform), 1);
        tray_actions
            .lock()
            .unwrap()
            .push_back(platform::TrayAction::Quit);
        assert_eq!(
            platform.take_tray_action(),
            Some(platform::TrayAction::Quit)
        );
        assert_eq!(platform.take_tray_action(), None);
    }
}
