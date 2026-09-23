#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod context_menu;
mod dock;
mod layout;
mod model;
mod platform;
mod theme;
mod ui;

use app::LauncherApp;
use eframe::egui;
use layout::dock_geometry;
use theme::configure_font;

fn main() -> eframe::Result {
    let (position, size) = dock_geometry();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([54.0, 220.0])
            .with_position(position)
            .with_decorations(false)
            .with_resizable(true)
            .with_fullscreen(false)
            .with_maximized(false)
            .with_transparent(true)
            .with_title("Windows Side Dock"),
        ..Default::default()
    };
    eframe::run_native(
        "Windows Side Dock",
        options,
        Box::new(|cc| {
            configure_font(&cc.egui_ctx);
            Ok(Box::new(LauncherApp::new()))
        }),
    )
}
