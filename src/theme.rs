use crate::model::IconKind;
use eframe::egui::{self, Color32};
use std::sync::{Arc, LazyLock};

/// アプリのアイコン。`scripts/make-icon.py` が書き出した64px四方のRGBA。
static APP_ICON: LazyLock<Arc<egui::IconData>> = LazyLock::new(|| {
    Arc::new(egui::IconData {
        rgba: include_bytes!("../assets/icon-64.rgba").to_vec(),
        width: 64,
        height: 64,
    })
});

/// ウィンドウのタイトルバーやAlt+Tabに出すアイコン。
pub(crate) fn app_icon() -> Arc<egui::IconData> {
    APP_ICON.clone()
}

pub(crate) fn draw_icon(painter: &egui::Painter, rect: egui::Rect, icon: IconKind) {
    draw_icon_colored(painter, rect, icon, Color32::from_rgb(238, 242, 250));
}

/// 左寄せの文字を持つ横長ボタン。通常は利用可能な幅いっぱいに広がり、
/// 寸法を測るパスでは文字に合わせた幅を返してメニューが必要以上に広がらないようにする。
pub(crate) fn left_aligned_button(ui: &mut egui::Ui, text: &str, height: f32) -> egui::Response {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let width = if ui.is_sizing_pass() {
        let text_width = ui
            .painter()
            .layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x;
        (text_width + 18.0).min(ui.available_width())
    } else {
        ui.available_width()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, text));
    let visuals = ui.style().interact(&response);
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );
    ui.painter().with_clip_rect(rect.shrink(7.0)).text(
        egui::pos2(rect.left() + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        font,
        visuals.text_color(),
    );
    response
}

pub(crate) fn draw_icon_colored(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: IconKind,
    color: Color32,
) {
    let stroke = egui::Stroke::new(2.1_f32, color);
    let center = rect.center();
    match icon {
        IconKind::Folder => {
            let body = egui::Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() + 5.0),
                egui::pos2(rect.right(), rect.bottom() - 2.0),
            );
            painter.rect_stroke(body, 3.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 2.0, rect.top() + 5.0),
                    egui::pos2(center.x - 1.0, rect.top() + 5.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 3.0, rect.top() + 5.0),
                    egui::pos2(rect.left() + 7.0, rect.top() + 1.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 7.0, rect.top() + 1.0),
                    egui::pos2(center.x, rect.top() + 1.0),
                ],
                stroke,
            );
        }
        IconKind::Terminal => {
            painter.rect_stroke(rect, 4.0, stroke, egui::StrokeKind::Middle);
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 5.0, center.y - 5.0),
                    egui::pos2(rect.left() + 10.0, center.y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 10.0, center.y),
                    egui::pos2(rect.left() + 5.0, center.y + 5.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(center.x + 1.0, center.y + 5.0),
                    egui::pos2(rect.right() - 5.0, center.y + 5.0),
                ],
                stroke,
            );
        }
        IconKind::Note | IconKind::File => {
            painter.rect_stroke(
                rect.shrink2(egui::vec2(3.0, 0.0)),
                3.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            for offset in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 7.0, center.y + offset),
                        egui::pos2(rect.right() - 7.0, center.y + offset),
                    ],
                    stroke,
                );
            }
        }
        IconKind::Settings => {
            let radius = rect.width().min(rect.height()) * 0.32;
            let inner = rect.width().min(rect.height()) * 0.12;
            let tooth = rect.width().min(rect.height()) * 0.48;
            let tooth_stroke = egui::Stroke::new(3.2_f32, color);
            painter.circle_stroke(center, radius, stroke);
            painter.circle_stroke(center, inner, stroke);
            for angle in (0..8).map(|i| i as f32 * std::f32::consts::TAU / 8.0) {
                let direction = egui::vec2(angle.cos(), angle.sin());
                painter.line_segment(
                    [
                        center + direction * (radius * 0.88),
                        center + direction * tooth,
                    ],
                    tooth_stroke,
                );
            }
        }
    }
}

/// BIZ UDPゴシック（BIZ-UDGothicR.ttc の2番目の書体）。
pub(crate) const JAPANESE_FONT_PATH: &str = r"C:\Windows\Fonts\BIZ-UDGothicR.ttc";

/// 日本語フォントを先頭に置いたフォント設定。フォントファイルを読めなければ `None`。
pub(crate) fn japanese_fonts(path: &str) -> Option<egui::FontDefinitions> {
    let bytes = std::fs::read(path).ok()?;
    let mut fonts = egui::FontDefinitions::default();
    let mut biz_udp = egui::FontData::from_owned(bytes);
    biz_udp.index = 1;
    fonts.font_data.insert("japanese".into(), biz_udp.into());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "japanese".into());
    Some(fonts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_icon_matches_its_pixel_size() {
        let icon = app_icon();
        assert_eq!((icon.width, icon.height), (64, 64));
        assert_eq!(icon.rgba.len(), 64 * 64 * 4);
        assert!(Arc::ptr_eq(&icon, &app_icon()));
    }

    #[test]
    fn puts_japanese_font_first_when_available() {
        let root = crate::config::temp_root("font");
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("font.ttc");
        std::fs::write(&path, b"font").unwrap();
        let fonts = japanese_fonts(&path.to_string_lossy()).unwrap();
        assert_eq!(
            fonts.families[&egui::FontFamily::Proportional][0],
            "japanese"
        );
        assert_eq!(fonts.font_data["japanese"].index, 1);
        assert!(japanese_fonts(&root.join("missing.ttc").to_string_lossy()).is_none());
    }
}
