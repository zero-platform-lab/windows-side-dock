//! タスクトレイのアイコンとメニュー。OSのトレイとメッセージループに依存するため、
//! 自動テストとカバレッジ計測の対象外にしている（`main.rs` の `coverage(off)`）。
//! メニュー操作は `TrayQueue` に積み、`Platform::take_tray_action` 経由でアプリが処理する。

use crate::platform::TrayAction;
use eframe::egui;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub(crate) type TrayQueue = Arc<Mutex<VecDeque<TrayAction>>>;

const TOGGLE_ID: &str = "toggle-dock";
const SETTINGS_ID: &str = "settings";
const PROCESS_TOOL_ID: &str = "process-tool";
const QUIT_ID: &str = "quit";

/// Dock本体のウィンドウ。非表示中はeguiの描画が止まるため、表示の切り替えはここで直接行う。
fn dock_window() -> windows_sys::Win32::Foundation::HWND {
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;
    let title: Vec<u16> = "Windows Side Dock".encode_utf16().chain(Some(0)).collect();
    unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) }
}

fn show_dock(visible: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetForegroundWindow, ShowWindow, SW_HIDE, SW_SHOW,
    };
    let window = dock_window();
    if window.is_null() {
        return;
    }
    unsafe {
        ShowWindow(window, if visible { SW_SHOW } else { SW_HIDE });
        if visible {
            SetForegroundWindow(window);
        }
    }
}

fn toggle_dock() {
    use windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible;
    let window = dock_window();
    show_dock(window.is_null() || unsafe { IsWindowVisible(window) } == 0);
}

/// Dockを表示してから操作を積み、アプリに処理させる。
fn request(queue: &TrayQueue, ctx: &egui::Context, action: TrayAction) {
    show_dock(true);
    if let Ok(mut queue) = queue.lock() {
        queue.push_back(action);
    }
    ctx.request_repaint();
}

/// 黒い角丸の縦長バーに、アイコンを表す明るい点を並べたトレイアイコン。
fn tray_icon_image() -> Icon {
    const SIZE: u32 = 32;
    let mut rgba = vec![0_u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let inside_bar = (9..23).contains(&x) && (2..30).contains(&y);
            let dot =
                (13..19).contains(&x) && [6, 13, 20].iter().any(|top| (*top..top + 5).contains(&y));
            let color = if dot {
                [104, 220, 132, 255]
            } else if inside_bar {
                [36, 40, 48, 255]
            } else {
                [0, 0, 0, 0]
            };
            let offset = ((y * SIZE + x) * 4) as usize;
            rgba[offset..offset + 4].copy_from_slice(&color);
        }
    }
    Icon::from_rgba(rgba, SIZE, SIZE).expect("トレイアイコンの画像サイズが不正です")
}

/// トレイにアイコンを登録する。戻り値を破棄するとアイコンも消えるため、アプリが保持すること。
pub(crate) fn install_tray(ctx: egui::Context, queue: TrayQueue) -> Option<TrayIcon> {
    let menu = Menu::new();
    let items = [
        MenuItem::with_id(TOGGLE_ID, "Dockを表示／隠す", true, None),
        MenuItem::with_id(SETTINGS_ID, "Dock 設定", true, None),
        MenuItem::with_id(PROCESS_TOOL_ID, "システムモニター", true, None),
    ];
    let quit = MenuItem::with_id(QUIT_ID, "終了", true, None);
    let separator = PredefinedMenuItem::separator();
    menu.append_items(&[&items[0], &items[1], &items[2], &separator, &quit])
        .ok()?;

    let menu_queue = queue.clone();
    let menu_ctx = ctx.clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| match event.id().as_ref() {
        TOGGLE_ID => toggle_dock(),
        SETTINGS_ID => request(&menu_queue, &menu_ctx, TrayAction::OpenSettings),
        PROCESS_TOOL_ID => request(&menu_queue, &menu_ctx, TrayAction::LaunchProcessTool),
        QUIT_ID => request(&menu_queue, &menu_ctx, TrayAction::Quit),
        _ => {}
    }));
    TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            toggle_dock();
        }
    }));

    TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("Windows Side Dock")
        .with_icon(tray_icon_image())
        .build()
        .ok()
}
