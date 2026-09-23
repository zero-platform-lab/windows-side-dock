//! ほかのアプリのウィンドウの変化をWindowsから知らせてもらう。知らせを受けたら描画を起こし、
//! 実行中アプリの一覧を数え直させる。何も起きていない間はDockを描き直さずに済む。
//! OSのイベントとメッセージループに依存するため、自動テストとカバレッジ計測の対象外にしている
//! （`main.rs` の `coverage(off)`）。

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetAncestor, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE,
    EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GA_ROOT, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS,
};

/// 見張るイベントの範囲。表示・非表示・破棄、タイトル変更、前面の切り替え、最小化、
/// 仮想デスクトップなどによる隠し（クローク）を含む。
const EVENT_RANGES: [(u32, u32); 5] = [
    (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
    (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
    // 破棄・表示・非表示は連番。
    (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
    (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
    (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
];
/// タイトルの変化をまとめて反映する間隔。作業中の表示をタイトルで動かし続けるアプリがあり、
/// 変化のたびに描き直すと負荷が下がらない。
const TITLE_CHANGE_DELAY: Duration = Duration::from_secs(1);
const OBJID_WINDOW: i32 = 0;
const CHILDID_SELF: i32 = 0;

static WATCHING: AtomicBool = AtomicBool::new(false);
static CHANGED: AtomicBool = AtomicBool::new(false);
static CONTEXT: OnceLock<egui::Context> = OnceLock::new();

unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    // ボタンやツールチップなど、ウィンドウの中の部品の変化は無視する。
    if window.is_null() || object != OBJID_WINDOW || child != CHILDID_SELF {
        return;
    }
    if unsafe { GetAncestor(window, GA_ROOT) } != window {
        return;
    }
    CHANGED.store(true, Ordering::Relaxed);
    if let Some(ctx) = CONTEXT.get() {
        if event == EVENT_OBJECT_NAMECHANGE {
            // 待っている間の変化は1回の描画にまとまる。
            ctx.request_repaint_after(TITLE_CHANGE_DELAY);
        } else {
            ctx.request_repaint();
        }
    }
}

/// 見張りを始める。登録したスレッド（eframeのメインスレッド）のメッセージループで知らせが届く。
/// 1つでも登録に失敗したら、呼び出し側は従来どおり定期的に確認する。
pub(crate) fn watch_windows(ctx: &egui::Context) {
    let _ = CONTEXT.set(ctx.clone());
    let all = EVENT_RANGES.iter().all(|&(min, max)| {
        let hook = unsafe {
            SetWinEventHook(
                min,
                max,
                std::ptr::null_mut(),
                Some(on_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            )
        };
        !hook.is_null()
    });
    WATCHING.store(all, Ordering::Relaxed);
}

/// 見張れていれば、前回の呼び出しから変化があったか。見張れていなければ `None`。
pub(crate) fn take_changes() -> Option<bool> {
    WATCHING
        .load(Ordering::Relaxed)
        .then(|| CHANGED.swap(false, Ordering::Relaxed))
}
