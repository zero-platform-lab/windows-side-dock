use crate::model::IconKind;
use eframe::egui::{self, Color32};

pub(crate) fn draw_icon(painter: &egui::Painter, rect: egui::Rect, icon: IconKind) {
    draw_icon_colored(painter, rect, icon, Color32::from_rgb(238, 242, 250));
}

pub(crate) fn left_aligned_button(ui: &mut egui::Ui, text: &str, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::click(),
    );
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
        egui::TextStyle::Button.resolve(ui.style()),
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

pub(crate) fn configure_font(ctx: &egui::Context) {
    let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\BIZ-UDGothicR.ttc") else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    let mut biz_udp = egui::FontData::from_owned(bytes);
    biz_udp.index = 1;
    fonts.font_data.insert("japanese".into(), biz_udp.into());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "japanese".into());
    ctx.set_fonts(fonts);
}
