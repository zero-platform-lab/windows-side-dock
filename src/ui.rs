use crate::app::{ContextMenuTarget, LauncherApp};
use crate::layout::{context_menu_screen_position, directional_tooltip};
use crate::model::{IconKind, LauncherItem};
use crate::platform::load_shell_icon;
use crate::theme::draw_icon;
use eframe::egui::{self, Color32};
use std::path::{Path, PathBuf};
use std::time::Instant;

impl LauncherApp {
    pub(crate) fn icon_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, index: usize) {
        let item = &self.items[index];
        let selected = index == self.selected;
        let size = 40.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
        let hovered = response.hovered();
        ui.painter().rect_filled(
            rect,
            8.0,
            if item.active {
                Color32::from_rgba_unmultiplied(80, 132, 220, 110)
            } else if selected {
                Color32::from_rgba_unmultiplied(72, 118, 210, 210)
            } else if hovered {
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
                Color32::from_rgb(104, 220, 132),
            );
        }
        if let Some(image) = &item.icon {
            let texture = self
                .textures
                .entry(item.command.clone())
                .or_insert_with(|| {
                    ctx.load_texture(
                        format!("shell-icon:{}", item.command),
                        image.clone(),
                        egui::TextureOptions::LINEAR,
                    )
                });
            ui.painter().image(
                texture.id(),
                rect.shrink(5.0),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            draw_icon(ui.painter(), rect.shrink(10.0), item.fallback_icon);
        }
        directional_tooltip(&response, &item.name, self.popup_direction.alignment(ctx));
        if response.secondary_clicked() {
            if let Some(position) = context_menu_screen_position(
                self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT,
            ) {
                self.context_menu =
                    Some((ContextMenuTarget::Pinned(index), position, Instant::now()));
            }
        }
        if response.clicked() {
            self.selected = index;
            self.launch(index);
        }
    }

    pub(crate) fn running_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, index: usize) {
        let Some(item) = self.running.get(index).cloned() else {
            return;
        };
        let size = 40.0;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
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
            Color32::from_rgb(104, 220, 132),
        );
        if let Some(image) = &item.icon {
            let key = format!("running:{}", item.command);
            let texture = self.textures.entry(key.clone()).or_insert_with(|| {
                ctx.load_texture(key, image.clone(), egui::TextureOptions::LINEAR)
            });
            ui.painter().image(
                texture.id(),
                rect.shrink(5.0),
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            draw_icon(ui.painter(), rect.shrink(10.0), item.fallback_icon);
        }
        let window_count = item.windows.len();
        let tooltip = if window_count > 1 {
            format!("{}（{}個のウィンドウ）", item.name, window_count)
        } else {
            format!("{}（実行中）", item.name)
        };
        directional_tooltip(&response, &tooltip, self.popup_direction.alignment(ctx));
        if response.secondary_clicked() {
            if let Some(position) = context_menu_screen_position(
                self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT,
            ) {
                self.context_menu =
                    Some((ContextMenuTarget::Running(index), position, Instant::now()));
            }
        }
        if response.clicked() {
            self.activate_running(index, ctx);
        }
    }
}

pub(crate) fn item(
    name: &str,
    command: &str,
    fallback_icon: IconKind,
    icon_source: &str,
) -> LauncherItem {
    LauncherItem {
        name: name.into(),
        command: command.into(),
        fallback_icon,
        icon: load_shell_icon(icon_source),
        windows: Vec::new(),
        active: false,
    }
}

pub(crate) fn normalized_executable_path(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .to_owned()
}

pub(crate) fn dock_directory() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}
