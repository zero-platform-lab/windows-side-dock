//! Dockを「画面の端を使うバー」（AppBar。タスクバーと同じしくみ）としてWindowsに登録し、
//! 画面の左右どちらかの端を確保する。最大化したウィンドウは確保した分の手前で止まる。
//! OSの作業領域を実際に変えるため、自動テストとカバレッジ計測の対象外にしている
//! （`main.rs` の `coverage(off)`）。

use crate::config::DockSide;
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows_sys::Win32::UI::Shell::{
    SHAppBarMessage, ABE_LEFT, ABE_RIGHT, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS, APPBARDATA,
};
use windows_sys::Win32::UI::WindowsAndMessaging::WM_APP;

/// AppBarの通知を受けるメッセージ。今は通知を使わないが、登録には番号が要る。
const APPBAR_CALLBACK: u32 = WM_APP + 0x51;

static REGISTERED: AtomicBool = AtomicBool::new(false);

fn appbar_data(window: HWND, side: DockSide) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: window,
        uCallbackMessage: APPBAR_CALLBACK,
        uEdge: match side {
            DockSide::Left => ABE_LEFT,
            DockSide::Right => ABE_RIGHT,
        },
        ..Default::default()
    }
}

fn monitor_of(window: HWND) -> Option<MONITORINFO> {
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    (unsafe { GetMonitorInfoW(monitor, &mut info) } != 0).then_some(info)
}

/// `window` のあるモニターの `side` の端を `width` ピクセル確保し、Dockを置ける範囲を返す。
/// 横はWindowsが決めた確保範囲、縦は作業領域（タスクバーを除いた範囲）。
pub(crate) fn reserve_edge(window: HWND, side: DockSide, width: i32) -> Option<egui::Rect> {
    if window.is_null() {
        return None;
    }
    let mut data = appbar_data(window, side);
    if !REGISTERED.swap(true, Ordering::Relaxed)
        && unsafe { SHAppBarMessage(ABM_NEW, &mut data) } == 0
    {
        REGISTERED.store(false, Ordering::Relaxed);
        return None;
    }
    let monitor = monitor_of(window)?;
    let screen = monitor.rcMonitor;
    data.rc = match side {
        DockSide::Left => RECT {
            right: screen.left + width,
            ..screen
        },
        DockSide::Right => RECT {
            left: screen.right - width,
            ..screen
        },
    };
    unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut data) };
    // ほかのバーに合わせて外側の端が動いた場合も、幅は保つ。
    match side {
        DockSide::Left => data.rc.right = data.rc.left + width,
        DockSide::Right => data.rc.left = data.rc.right - width,
    }
    unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) };
    // 確保した後の作業領域の縦の範囲に合わせる。
    let work = monitor_of(window)?.rcWork;
    Some(egui::Rect::from_min_max(
        egui::pos2(data.rc.left as f32, work.top as f32),
        egui::pos2(data.rc.right as f32, work.bottom as f32),
    ))
}

/// 確保をやめて、作業領域を元に戻す。
pub(crate) fn release(window: HWND) {
    if window.is_null() || !REGISTERED.swap(false, Ordering::Relaxed) {
        return;
    }
    let mut data = appbar_data(window, DockSide::Right);
    unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) };
}
