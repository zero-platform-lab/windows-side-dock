use crate::config::PopupDirection;
use crate::platform::{LocalTime, Platform};
use eframe::egui::{self, Color32};
use std::time::{Duration, Instant};

pub(crate) const SETTINGS_WIDTH: f32 = 380.0;
pub(crate) const TOOLTIP_WIDTH: f32 = 220.0;
pub(crate) const CONTEXT_MENU_WIDTH: f32 = 210.0;
pub(crate) const WINDOW_PICKER_WIDTH: f32 = 480.0;
const DOCK_WIDTH: f32 = 54.0;
/// 作業領域が分からないときの高さ。
const DOCK_FALLBACK_HEIGHT: f32 = 800.0;
const DOCK_MARGIN: f32 = 12.0;

/// 基準範囲の左右どちらかへ、幅 `width` のポップアップを `gap` だけ離して置くときの左端X座標。
fn beside_x(anchor_left: f32, anchor_right: f32, open_left: bool, width: f32, gap: f32) -> f32 {
    if open_left {
        anchor_left - width - gap
    } else {
        anchor_right + gap
    }
}

/// Dockが画面の右半分にあれば、ポップアップは左へ開く。Dockが見つからなければ左へ開く。
pub(crate) fn popup_should_open_left(platform: &dyn Platform) -> bool {
    platform
        .dock_rect()
        .is_none_or(|dock| dock.center().x > platform.screen_width() / 2.0)
}

pub(crate) fn popup_alignment(
    platform: &dyn Platform,
    direction: PopupDirection,
) -> egui::RectAlign {
    let open_left = match direction {
        PopupDirection::Auto => popup_should_open_left(platform),
        PopupDirection::Left => true,
        PopupDirection::Right => false,
    };
    if open_left {
        egui::RectAlign::LEFT
    } else {
        egui::RectAlign::RIGHT
    }
}

pub(crate) fn settings_dialog_position(
    platform: &dyn Platform,
    direction: PopupDirection,
) -> egui::Pos2 {
    let Some(dock) = platform.dock_rect() else {
        return egui::pos2(100.0, 100.0);
    };
    let open_left = popup_alignment(platform, direction) == egui::RectAlign::LEFT;
    let x = beside_x(dock.left(), dock.right(), open_left, SETTINGS_WIDTH, 12.0);
    egui::pos2(x, dock.top())
}

/// 横位置はDockの左右の端、縦位置はカーソルの高さを基準にする。
/// Dockが見つからない場合はカーソル位置を基準にする。
fn beside_dock_at_cursor(
    platform: &dyn Platform,
    open_left: bool,
    width: f32,
) -> Option<egui::Pos2> {
    let cursor = platform.cursor_position()?;
    let (left, right) = platform
        .dock_rect()
        .map_or((cursor.x, cursor.x), |dock| (dock.left(), dock.right()));
    let x = beside_x(left, right, open_left, width, 8.0);
    Some(egui::pos2(x, cursor.y))
}

pub(crate) fn context_menu_screen_position(
    platform: &dyn Platform,
    open_left: bool,
) -> Option<egui::Pos2> {
    beside_dock_at_cursor(platform, open_left, CONTEXT_MENU_WIDTH)
}

pub(crate) fn window_picker_screen_position(
    platform: &dyn Platform,
    open_left: bool,
) -> Option<egui::Pos2> {
    beside_dock_at_cursor(platform, open_left, WINDOW_PICKER_WIDTH)
}

/// ツールチップはDockの外側、対象アイコンの高さに出す。`item_center_y` はDock内の座標。
fn tooltip_screen_position(
    platform: &dyn Platform,
    item_center_y: f32,
    open_left: bool,
) -> Option<egui::Pos2> {
    let dock = platform.dock_rect()?;
    let x = beside_x(dock.left(), dock.right(), open_left, TOOLTIP_WIDTH, 8.0);
    let y = dock.top() + item_center_y - 18.0;
    Some(egui::pos2(x, y.max(0.0)))
}

/// 子Viewportのツールチップ。表示中は位置を固定し、最初の20msは非表示にしてちらつきを防ぐ。
pub(crate) fn directional_tooltip(
    platform: &dyn Platform,
    response: &egui::Response,
    text: &str,
    alignment: egui::RectAlign,
) {
    let state_id = response.id.with("child-tooltip-state");
    let pointer_pressed = response
        .ctx
        .input(|input| input.pointer.any_down() || input.pointer.any_click());
    if pointer_pressed || !egui::Tooltip::should_show_tooltip(response, false) {
        response
            .ctx
            .data_mut(|data| data.remove::<(Instant, egui::Pos2)>(state_id));
        return;
    }
    let Some(initial_position) = tooltip_screen_position(
        platform,
        response.rect.center().y,
        alignment == egui::RectAlign::LEFT,
    ) else {
        return;
    };
    let (opened_at, position) = response.ctx.data_mut(|data| {
        *data.get_temp_mut_or_insert_with(state_id, || (Instant::now(), initial_position))
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

/// 時計に表示する日付、曜日、時刻。
pub(crate) fn format_date_time(time: LocalTime) -> (String, String, String) {
    const WEEKDAYS: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];
    let weekday = WEEKDAYS.get(usize::from(time.weekday)).unwrap_or(&"―");
    (
        format!("{:02}/{:02}", time.month, time.day),
        format!("（{weekday}）"),
        format!("{:02}:{:02}", time.hour, time.minute),
    )
}

/// 起動時のDockの位置とサイズ。作業領域の右上に置き、高さは上下の余白を除いた作業領域いっぱいにする。
pub(crate) fn dock_geometry(work_area: Option<egui::Rect>) -> ([f32; 2], [f32; 2]) {
    let Some(area) = work_area else {
        return ([0.0, 60.0], [DOCK_WIDTH, DOCK_FALLBACK_HEIGHT]);
    };
    let height = area.height() - DOCK_MARGIN * 2.0;
    (
        [
            area.right() - DOCK_WIDTH - DOCK_MARGIN,
            area.top() + DOCK_MARGIN,
        ],
        [DOCK_WIDTH, height],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::fake::FakePlatform;

    fn dock_at(left: f32) -> FakePlatform {
        FakePlatform {
            dock: Some(egui::Rect::from_min_size(
                egui::pos2(left, 12.0),
                egui::vec2(54.0, 800.0),
            )),
            cursor: Some(egui::pos2(left + 27.0, 300.0)),
            ..FakePlatform::default()
        }
    }

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
    fn opens_toward_the_wider_side_of_the_screen() {
        assert!(popup_should_open_left(&dock_at(1854.0)));
        assert!(!popup_should_open_left(&dock_at(12.0)));
        let missing = FakePlatform {
            dock: None,
            ..FakePlatform::default()
        };
        assert!(popup_should_open_left(&missing));
    }

    #[test]
    fn honours_fixed_popup_direction() {
        let right_dock = dock_at(1854.0);
        let left_dock = dock_at(12.0);
        assert_eq!(
            popup_alignment(&right_dock, PopupDirection::Auto),
            egui::RectAlign::LEFT
        );
        assert_eq!(
            popup_alignment(&left_dock, PopupDirection::Auto),
            egui::RectAlign::RIGHT
        );
        assert_eq!(
            popup_alignment(&left_dock, PopupDirection::Left),
            egui::RectAlign::LEFT
        );
        assert_eq!(
            popup_alignment(&right_dock, PopupDirection::Right),
            egui::RectAlign::RIGHT
        );
    }

    #[test]
    fn places_settings_dialog_next_to_the_dock() {
        let platform = dock_at(1854.0);
        assert_eq!(
            settings_dialog_position(&platform, PopupDirection::Auto),
            egui::pos2(1854.0 - SETTINGS_WIDTH - 12.0, 12.0)
        );
        assert_eq!(
            settings_dialog_position(&platform, PopupDirection::Right),
            egui::pos2(1920.0, 12.0)
        );
        let missing = FakePlatform {
            dock: None,
            ..FakePlatform::default()
        };
        assert_eq!(
            settings_dialog_position(&missing, PopupDirection::Auto),
            egui::pos2(100.0, 100.0)
        );
    }

    #[test]
    fn anchors_menus_to_the_dock_edge_at_cursor_height() {
        let platform = dock_at(1854.0);
        assert_eq!(
            context_menu_screen_position(&platform, true),
            Some(egui::pos2(1854.0 - CONTEXT_MENU_WIDTH - 8.0, 300.0))
        );
        assert_eq!(
            window_picker_screen_position(&platform, false),
            Some(egui::pos2(1916.0, 300.0))
        );
    }

    #[test]
    fn falls_back_to_cursor_without_dock_and_gives_up_without_cursor() {
        let no_dock = FakePlatform {
            dock: None,
            cursor: Some(egui::pos2(500.0, 40.0)),
            ..FakePlatform::default()
        };
        assert_eq!(
            context_menu_screen_position(&no_dock, false),
            Some(egui::pos2(508.0, 40.0))
        );
        let no_cursor = FakePlatform {
            cursor: None,
            ..FakePlatform::default()
        };
        assert_eq!(window_picker_screen_position(&no_cursor, true), None);
    }

    #[test]
    fn positions_tooltip_outside_the_dock_and_on_screen() {
        let platform = dock_at(1854.0);
        assert_eq!(
            tooltip_screen_position(&platform, 100.0, true),
            Some(egui::pos2(1854.0 - TOOLTIP_WIDTH - 8.0, 94.0))
        );
        assert_eq!(
            tooltip_screen_position(&platform, 0.0, false),
            Some(egui::pos2(1916.0, 0.0))
        );
        let missing = FakePlatform {
            dock: None,
            ..FakePlatform::default()
        };
        assert_eq!(tooltip_screen_position(&missing, 10.0, true), None);
    }

    #[test]
    fn formats_clock_text() {
        let time = LocalTime {
            month: 9,
            day: 3,
            weekday: 6,
            hour: 7,
            minute: 5,
        };
        assert_eq!(
            format_date_time(time),
            ("09/03".into(), "（土）".into(), "07:05".into())
        );
        let invalid = LocalTime { weekday: 9, ..time };
        assert_eq!(format_date_time(invalid).1, "（―）");
    }

    #[test]
    fn fills_the_work_area_height() {
        let full_hd = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1920.0, 1032.0));
        assert_eq!(
            dock_geometry(Some(full_hd)),
            ([1854.0, 12.0], [54.0, 1008.0])
        );
        let short = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1280.0, 600.0));
        assert_eq!(dock_geometry(Some(short)), ([1214.0, 12.0], [54.0, 576.0]));
        assert_eq!(dock_geometry(None), ([0.0, 60.0], [54.0, 800.0]));
        let platform = FakePlatform {
            work_area: Some(full_hd),
            ..FakePlatform::default()
        };
        assert_eq!(
            dock_geometry(platform.work_area()),
            ([1854.0, 12.0], [54.0, 1008.0])
        );
    }
}
