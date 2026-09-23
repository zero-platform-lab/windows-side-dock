use crate::app::{ContextMenuTarget, LauncherApp};
use crate::layout::directional_tooltip;
use crate::model::LauncherItem;
use crate::theme::draw_icon;
use eframe::egui::{self, Color32};

const ICON_SIZE: f32 = 40.0;
const RUNNING_DOT: Color32 = Color32::from_rgb(104, 220, 132);

impl LauncherApp {
    pub(crate) fn icon_button(&mut self, ui: &mut egui::Ui, index: usize) {
        let item = &self.items[index];
        let selected = index == self.selected;
        let (rect, response) = icon_slot(ui, &item.name);
        ui.painter().rect_filled(
            rect,
            8.0,
            if item.active {
                Color32::from_rgba_unmultiplied(80, 132, 220, 110)
            } else if selected {
                Color32::from_rgba_unmultiplied(72, 118, 210, 210)
            } else if response.hovered() {
                Color32::from_rgba_unmultiplied(255, 255, 255, 32)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 16)
            },
        );
        if selected {
            ui.painter().circle_filled(
                egui::pos2(rect.center().x, rect.bottom() - 5.0),
                2.0,
                Color32::WHITE,
            );
        }
        if !item.windows.is_empty() {
            ui.painter().circle_filled(
                egui::pos2(rect.left() + 4.0, rect.center().y),
                2.0,
                RUNNING_DOT,
            );
        }
        self.paint_item_icon(ui, rect, index, false);
        let name = self.items[index].name.clone();
        directional_tooltip(self.platform.as_ref(), &response, &name, self.alignment());
        if response.secondary_clicked() {
            self.open_context_menu(ContextMenuTarget::Pinned(index));
        }
        if response.clicked() {
            self.selected = index;
            self.launch(index);
        }
    }

    pub(crate) fn running_button(&mut self, ui: &mut egui::Ui, index: usize) {
        let item = &self.running[index];
        let (rect, response) = icon_slot(ui, &item.name);
        ui.painter().rect_filled(
            rect,
            8.0,
            if item.active {
                Color32::from_rgba_unmultiplied(80, 132, 220, 110)
            } else if response.hovered() {
                Color32::from_rgba_unmultiplied(126, 211, 146, 42)
            } else {
                Color32::from_rgba_unmultiplied(126, 211, 146, 16)
            },
        );
        ui.painter().circle_filled(
            egui::pos2(rect.left() + 4.0, rect.center().y),
            2.0,
            RUNNING_DOT,
        );
        let tooltip = running_tooltip(item);
        self.paint_item_icon(ui, rect, index, true);
        directional_tooltip(
            self.platform.as_ref(),
            &response,
            &tooltip,
            self.alignment(),
        );
        if response.secondary_clicked() {
            self.open_context_menu(ContextMenuTarget::Running(index));
        }
        if response.clicked() {
            self.activate_running(index);
        }
    }

    /// Shellから取得したアイコンを描く。取得できなかった項目は独自アイコンで代用する。
    fn paint_item_icon(&mut self, ui: &egui::Ui, rect: egui::Rect, index: usize, running: bool) {
        let item = if running {
            &self.running[index]
        } else {
            &self.items[index]
        };
        let Some(image) = &item.icon else {
            draw_icon(ui.painter(), rect.shrink(10.0), item.fallback_icon);
            return;
        };
        let key = if running {
            format!("running:{}", item.command)
        } else {
            format!("shell-icon:{}", item.command)
        };
        let texture = self.textures.entry(key.clone()).or_insert_with(|| {
            ui.ctx()
                .load_texture(key, image.clone(), egui::TextureOptions::LINEAR)
        });
        ui.painter().image(
            texture.id(),
            rect.shrink(5.0),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
}

/// アイコン1個分の領域。支援技術とテストから名前で見つけられるようにボタンとして登録する。
fn icon_slot(ui: &mut egui::Ui, name: &str) -> (egui::Rect, egui::Response) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ICON_SIZE, ICON_SIZE), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
    (rect, response)
}

pub(crate) fn running_tooltip(item: &LauncherItem) -> String {
    match item.windows.len() {
        count if count > 1 => format!("{}（{}個のウィンドウ）", item.name, count),
        _ => format!("{}（実行中）", item.name),
    }
}

pub(crate) fn normalized_executable_path(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_whitespace_and_quotes_from_executable_path() {
        assert_eq!(
            normalized_executable_path(" \"E:\\Tools\\procexp.exe\"\n"),
            r"E:\Tools\procexp.exe"
        );
        assert_eq!(
            normalized_executable_path(r"'C:\a b\x.exe'"),
            r"C:\a b\x.exe"
        );
        assert_eq!(normalized_executable_path("   "), "");
    }
}
