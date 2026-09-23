use crate::config::PopupDirection;
use eframe::egui::{self, Color32};
use std::time::{Duration, Instant};

pub(crate) const SETTINGS_WIDTH: f32 = 380.0;
pub(crate) const TOOLTIP_WIDTH: f32 = 220.0;
pub(crate) const CONTEXT_MENU_WIDTH: f32 = 210.0;
pub(crate) const WINDOW_PICKER_WIDTH: f32 = 480.0;

/// 基準範囲の左右どちらかへ、幅 `width` のポップアップを `gap` だけ離して置くときの左端X座標。
fn beside_x(anchor_left: f32, anchor_right: f32, open_left: bool, width: f32, gap: f32) -> f32 {
    if open_left {
        anchor_left - width - gap
    } else {
        anchor_right + gap
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn center_in_right_half(left: i32, right: i32, screen_width: i32) -> bool {
    (left + right) / 2 > screen_width / 2
}

#[cfg(windows)]
pub(crate) fn popup_should_open_left(_ctx: &egui::Context) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetSystemMetrics, GetWindowRect,
    };

    let title: Vec<u16> = std::ffi::OsStr::new("Windows Side Dock")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    if window.is_null() {
        return true;
    }
    let mut rect = RECT::default();
    if unsafe { GetWindowRect(window, &mut rect) } == 0 {
        return true;
    }
    let screen_width = unsafe { GetSystemMetrics(0) };
    center_in_right_half(rect.left, rect.right, screen_width)
}

#[cfg(windows)]
pub(crate) fn launcher_window_position() -> Option<egui::Pos2> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect};

    let title: Vec<u16> = std::ffi::OsStr::new("Windows Side Dock")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    let mut rect = RECT::default();
    if window.is_null() || unsafe { GetWindowRect(window, &mut rect) } == 0 {
        None
    } else {
        Some(egui::pos2(rect.left as f32, rect.top as f32))
    }
}

#[cfg(not(windows))]
pub(crate) fn launcher_window_position() -> Option<egui::Pos2> {
    None
}

#[cfg(not(windows))]
pub(crate) fn popup_should_open_left(ctx: &egui::Context) -> bool {
    ctx.input(|input| {
        let viewport = input.viewport();
        match (viewport.outer_rect, viewport.monitor_size) {
            (Some(rect), Some(monitor)) => rect.center().x > monitor.x / 2.0,
            _ => true,
        }
    })
}

pub(crate) fn popup_alignment(ctx: &egui::Context) -> egui::RectAlign {
    if popup_should_open_left(ctx) {
        egui::RectAlign::LEFT
    } else {
        egui::RectAlign::RIGHT
    }
}

#[cfg(windows)]
pub(crate) fn settings_dialog_position(
    ctx: &egui::Context,
    direction: PopupDirection,
) -> egui::Pos2 {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect};

    let title: Vec<u16> = std::ffi::OsStr::new("Windows Side Dock")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    let mut rect = RECT::default();
    if !window.is_null() && unsafe { GetWindowRect(window, &mut rect) } != 0 {
        let open_left = match direction {
            PopupDirection::Auto => popup_should_open_left(ctx),
            PopupDirection::Left => true,
            PopupDirection::Right => false,
        };
        let x = beside_x(
            rect.left as f32,
            rect.right as f32,
            open_left,
            SETTINGS_WIDTH,
            12.0,
        );
        egui::pos2(x, rect.top as f32)
    } else {
        egui::pos2(100.0, 100.0)
    }
}

#[cfg(not(windows))]
pub(crate) fn settings_dialog_position(
    _ctx: &egui::Context,
    _direction: PopupDirection,
) -> egui::Pos2 {
    egui::pos2(100.0, 100.0)
}

pub(crate) fn directional_tooltip(
    response: &egui::Response,
    text: &str,
    alignment: egui::RectAlign,
) {
    let state_id = response.id.with("child-tooltip-state");
    if response
        .ctx
        .input(|input| input.pointer.any_down() || input.pointer.any_click())
    {
        response
            .ctx
            .data_mut(|data| data.remove::<(Instant, egui::Pos2)>(state_id));
        return;
    }
    if !egui::Tooltip::should_show_tooltip(response, false) {
        response
            .ctx
            .data_mut(|data| data.remove::<(Instant, egui::Pos2)>(state_id));
        return;
    }
    let Some(initial_position) =
        tooltip_screen_position(response, alignment == egui::RectAlign::LEFT)
    else {
        return;
    };
    let (opened_at, position) = response.ctx.data_mut(|data| {
        if let Some(state) = data.get_temp::<(Instant, egui::Pos2)>(state_id) {
            state
        } else {
            let state = (Instant::now(), initial_position);
            data.insert_temp(state_id, state);
            state
        }
    });
    let ready = opened_at.elapsed() >= Duration::from_millis(20);
    let tooltip_text = text.to_owned();
    response.ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of(("launcher-tooltip", response.id)),
        egui::ViewportBuilder::default()
            .with_title("Launcher tooltip")
            .with_inner_size([TOOLTIP_WIDTH, 36.0])
            .with_position(position)
            .with_decorations(false)
            .with_resizable(false)
            .with_transparent(true)
            .with_taskbar(false)
            .with_mouse_passthrough(true)
            .with_visible(ready)
            .with_always_on_top(),
        |tooltip_ctx, _class| {
            if !ready {
                tooltip_ctx.request_repaint_after(Duration::from_millis(20));
            }
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(14, 16, 21, 248))
                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(85)))
                        .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(10, 7)),
                )
                .show(tooltip_ctx, |ui| {
                    ui.label(&tooltip_text);
                });
        },
    );
}

#[cfg(windows)]
pub(crate) fn context_menu_screen_position(open_left: bool) -> Option<egui::Pos2> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return None;
    }
    let x = beside_x(
        cursor.x as f32,
        cursor.x as f32,
        open_left,
        CONTEXT_MENU_WIDTH,
        8.0,
    );
    Some(egui::pos2(x, cursor.y as f32))
}

#[cfg(windows)]
pub(crate) fn window_picker_screen_position(open_left: bool) -> Option<egui::Pos2> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return None;
    }
    let x = beside_x(
        cursor.x as f32,
        cursor.x as f32,
        open_left,
        WINDOW_PICKER_WIDTH,
        8.0,
    );
    Some(egui::pos2(x, cursor.y as f32))
}

#[cfg(not(windows))]
pub(crate) fn context_menu_screen_position(_open_left: bool) -> Option<egui::Pos2> {
    Some(egui::pos2(100.0, 100.0))
}

#[cfg(not(windows))]
pub(crate) fn window_picker_screen_position(_open_left: bool) -> Option<egui::Pos2> {
    Some(egui::pos2(100.0, 100.0))
}

#[cfg(windows)]
pub(crate) fn tooltip_screen_position(
    response: &egui::Response,
    open_left: bool,
) -> Option<egui::Pos2> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect};

    let title: Vec<u16> = std::ffi::OsStr::new("Windows Side Dock")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let window = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
    let mut rect = RECT::default();
    if window.is_null() || unsafe { GetWindowRect(window, &mut rect) } == 0 {
        return None;
    }
    let x = beside_x(
        rect.left as f32,
        rect.right as f32,
        open_left,
        TOOLTIP_WIDTH,
        8.0,
    );
    let y = rect.top as f32 + response.rect.center().y - 18.0;
    Some(egui::pos2(x, y.max(0.0)))
}

#[cfg(not(windows))]
pub(crate) fn tooltip_screen_position(
    response: &egui::Response,
    open_left: bool,
) -> Option<egui::Pos2> {
    let x = beside_x(
        response.rect.left(),
        response.rect.right(),
        open_left,
        TOOLTIP_WIDTH,
        8.0,
    );
    Some(egui::pos2(x, response.rect.top()))
}

#[cfg(windows)]
pub(crate) fn current_date_time() -> (String, String, String) {
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    let mut time = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut time) };
    let weekdays = ["日", "月", "火", "水", "木", "金", "土"];
    (
        format!("{:02}/{:02}", time.wMonth, time.wDay),
        format!("（{}）", weekdays[time.wDayOfWeek as usize]),
        format!("{:02}:{:02}", time.wHour, time.wMinute),
    )
}

#[cfg(not(windows))]
pub(crate) fn current_date_time() -> (String, String, String) {
    ("--/--".into(), "（―）".into(), "--:--".into())
}

#[cfg(windows)]
pub(crate) fn dock_geometry() -> ([f32; 2], [f32; 2]) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA};
    let mut area = RECT::default();
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            &mut area as *mut _ as *mut std::ffi::c_void,
            0,
        )
    };
    if ok != 0 {
        let width = 54.0;
        let height = 800.0_f32.min((area.bottom - area.top) as f32 - 24.0);
        (
            [area.right as f32 - width - 12.0, area.top as f32 + 12.0],
            [width, height],
        )
    } else {
        ([0.0, 60.0], [54.0, 800.0])
    }
}

#[cfg(not(windows))]
pub(crate) fn dock_geometry() -> ([f32; 2], [f32; 2]) {
    ([0.0, 60.0], [54.0, 800.0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_popup_beside_anchor_without_overlap() {
        for width in [
            SETTINGS_WIDTH,
            TOOLTIP_WIDTH,
            CONTEXT_MENU_WIDTH,
            WINDOW_PICKER_WIDTH,
        ] {
            let left = beside_x(1800.0, 1854.0, true, width, 8.0);
            assert_eq!(left + width + 8.0, 1800.0);
            assert_eq!(beside_x(1800.0, 1854.0, false, width, 8.0), 1862.0);
        }
    }

    #[test]
    fn detects_which_half_of_screen_holds_the_dock() {
        assert!(center_in_right_half(1854, 1908, 1920));
        assert!(!center_in_right_half(12, 66, 1920));
        assert!(!center_in_right_half(930, 990, 1920));
    }
}
