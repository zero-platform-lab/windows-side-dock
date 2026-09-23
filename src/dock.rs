use crate::app::{ContextMenuTarget, LauncherApp};
use crate::config::{DockSide, PopupDirection, ProcessTool};
use crate::edge::inner_corners;
use crate::layout::{
    directional_tooltip, format_date_time, next_repaint, settings_dialog_position, POLL_INTERVAL,
    SETTINGS_WIDTH,
};
use crate::model::IconKind;
use crate::theme::{app_icon, draw_icon_colored};
use crate::ui::normalized_executable_path;
use eframe::egui::{self, Color32, Key};
use eframe::{App, Frame};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// ボタン以外の操作部品を、支援技術とテストから名前で見つけられるようにする。
fn label_widget(response: &egui::Response, name: &str) {
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
}

impl App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.show(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.release_edge();
    }
}

impl LauncherApp {
    /// 1フレーム分の処理。Dock本体と、開いている設定画面・メニューを描く。
    pub(crate) fn show(&mut self, ctx: &egui::Context) {
        self.keep_window_state(ctx);
        self.apply_window_level(ctx);
        self.apply_edge(ctx);
        // ウィンドウの変化を見張れていれば変化があったときだけ、見張れなければ一定間隔で数え直す。
        let changes = self.platform.take_window_changes();
        let due = match changes {
            Some(changed) => changed || self.last_refresh.is_none(),
            None => self
                .last_refresh
                .is_none_or(|at| at.elapsed() >= POLL_INTERVAL),
        };
        if due {
            self.refresh_running();
            self.last_refresh = Some(Instant::now());
        }
        ctx.request_repaint_after(next_repaint(self.platform.local_time(), changes.is_some()));
        self.handle_tray_actions(ctx);
        self.handle_input(ctx);
        if self.collapsed {
            self.show_collapsed_tab(ctx);
        } else {
            self.show_dock(ctx);
        }
        if self.show_settings {
            self.show_settings_viewport(ctx);
        }
        self.show_context_menu_viewport(ctx);
        self.show_window_picker_viewport(ctx);
    }

    /// 「常に手前に表示」の設定が変わったときだけ、Dockのウィンドウへ反映する。
    fn apply_window_level(&mut self, ctx: &egui::Context) {
        if self.applied_always_on_top == Some(self.always_on_top) {
            return;
        }
        let level = if self.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd_to(
            egui::ViewportId::ROOT,
            egui::ViewportCommand::WindowLevel(level),
        );
        self.applied_always_on_top = Some(self.always_on_top);
    }

    /// Windowsのスナップなどで最大化・全画面化されたら元に戻す。
    fn keep_window_state(&self, ctx: &egui::Context) {
        let (fullscreen, maximized) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.fullscreen.unwrap_or(false),
                viewport.maximized.unwrap_or(false),
            )
        });
        if fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect()
        });
        for path in dropped {
            self.add_path(&path);
        }
        let pressed = |key| ctx.input(|input| input.key_pressed(key));
        if pressed(Key::Escape) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let count = self.items.len();
        if pressed(Key::ArrowRight) {
            self.selected = (self.selected + 1) % count;
        }
        if pressed(Key::ArrowLeft) {
            self.selected = (self.selected + count - 1) % count;
        }
        if pressed(Key::Enter) {
            self.launch(self.selected);
        }
    }

    fn show_dock(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(8, 9, 12, 250))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 55),
                    ))
                    .corner_radius(inner_corners(self.dock_side, 10))
                    .inner_margin(egui::Margin::symmetric(6, 8)),
            )
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let background = ui.interact(
                    area,
                    ui.id().with("launcher-background-context"),
                    egui::Sense::click(),
                );
                paint_metallic_background(ui, area);
                ui.vertical_centered(|ui| self.dock_contents(ui));
                if background.secondary_clicked() {
                    self.open_context_menu(ContextMenuTarget::Handle);
                }
            });
    }

    fn dock_contents(&mut self, ui: &mut egui::Ui) {
        let (date, weekday, time) = format_date_time(self.platform.local_time());
        let clock = ui
            .vertical_centered(|ui| {
                // 選択可能な文字列は右クリックを先に受け取ってしまうため、時計の文字は選択不可にする。
                for text in [date, weekday, time] {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(text)
                                .size(11.0)
                                .color(Color32::from_rgb(225, 229, 238)),
                        )
                        .selectable(false),
                    );
                }
            })
            .response
            .interact(egui::Sense::click());
        label_widget(&clock, "時計");
        if clock.secondary_clicked() {
            self.open_context_menu(ContextMenuTarget::Clock);
        }
        ui.add_space(3.0);
        self.collapse_button(ui);
        ui.add_space(3.0);
        self.settings_button(ui);
        ui.add_space(5.0);
        ui.separator();
        ui.add_space(5.0);
        for index in 0..self.items.len() {
            self.icon_button(ui, index);
        }
        if !self.running.is_empty() {
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);
            let scroll_height = (ui.available_height() - 6.0).max(40.0);
            egui::ScrollArea::vertical()
                .id_salt("running-items")
                .max_height(scroll_height)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    for index in 0..self.running.len() {
                        self.running_button(ui, index);
                    }
                });
        }
    }

    fn settings_button(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
        label_widget(&response, "Dock 設定");
        ui.painter().rect_filled(
            rect,
            8.0,
            if response.hovered() {
                Color32::from_rgba_unmultiplied(255, 255, 255, 32)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 12)
            },
        );
        draw_icon_colored(
            ui.painter(),
            rect.shrink(10.0),
            IconKind::Settings,
            Color32::from_rgb(188, 194, 204),
        );
        directional_tooltip(
            self.platform.as_ref(),
            &response,
            "Dock 設定",
            self.alignment(),
        );
        if response.clicked() {
            self.show_settings = true;
            ui.ctx().request_repaint();
        }
        if response.secondary_clicked() {
            self.open_context_menu(ContextMenuTarget::Handle);
        }
    }

    fn show_settings_viewport(&mut self, ctx: &egui::Context) {
        let position = settings_dialog_position(self.platform.as_ref(), self.popup_direction);
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("launcher-settings"),
            egui::ViewportBuilder::default()
                .with_title("Windows Side Dock 設定")
                .with_icon(app_icon())
                .with_inner_size([SETTINGS_WIDTH, 560.0])
                .with_min_inner_size([360.0, 400.0])
                .with_position(position)
                .with_resizable(false)
                .with_taskbar(false)
                .with_always_on_top()
                .with_active(true),
            |settings_ctx, _class| {
                if settings_ctx.input(|input| input.viewport().close_requested()) {
                    self.show_settings = false;
                }
                egui::CentralPanel::default().show(settings_ctx, |ui| self.settings_contents(ui));
                settings_ctx.style_mut(|style| {
                    for text_style in [egui::TextStyle::Body, egui::TextStyle::Button] {
                        style
                            .text_styles
                            .entry(text_style)
                            .and_modify(|font| font.size = self.font_size);
                    }
                });
                if !self.show_settings {
                    settings_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            },
        );
    }

    fn settings_contents(&mut self, ui: &mut egui::Ui) {
        ui.heading("Windows Side Dock 設定");
        ui.separator();
        ui.label("UIフォント");
        ui.add_enabled(false, egui::Button::new("BIZ UDPゴシック"));
        ui.add_space(8.0);
        ui.label("文字サイズ");
        ui.add(egui::Slider::new(&mut self.font_size, 10.0..=20.0).suffix(" px"));
        ui.add_space(8.0);
        ui.label("Dockの位置");
        let mut side = self.dock_side;
        ui.radio_value(&mut side, DockSide::Right, "右端");
        ui.radio_value(&mut side, DockSide::Left, "左端");
        if side != self.dock_side {
            self.set_dock_side(side);
        }
        ui.add_space(8.0);
        ui.label("ポップアップの方向");
        let mut direction = self.popup_direction;
        ui.radio_value(&mut direction, PopupDirection::Auto, "自動");
        ui.radio_value(&mut direction, PopupDirection::Left, "常に左");
        ui.radio_value(&mut direction, PopupDirection::Right, "常に右");
        if direction != self.popup_direction {
            self.set_popup_direction(direction);
        }
        ui.add_space(8.0);
        let mut always_on_top = self.always_on_top;
        if ui
            .checkbox(&mut always_on_top, "Dockを常に手前に表示")
            .changed()
        {
            self.set_always_on_top(always_on_top);
        }
        ui.add_space(12.0);
        ui.label("システムモニター");
        let mut tool = self.process_tool;
        ui.radio_value(&mut tool, ProcessTool::TaskManager, "タスク マネージャー");
        ui.radio_value(&mut tool, ProcessTool::ProcessExplorer, "Process Explorer");
        if tool != self.process_tool {
            self.set_process_tool(tool);
        }
        if self.process_tool == ProcessTool::ProcessExplorer {
            self.process_explorer_settings(ui);
        }
        if let Some(message) = &self.monitor_status {
            ui.colored_label(Color32::from_rgb(255, 140, 100), message);
        }
        ui.add_space(10.0);
        if ui.button("閉じる").clicked() {
            self.show_settings = false;
        }
    }

    fn process_explorer_settings(&mut self, ui: &mut egui::Ui) {
        ui.label("Process Explorerのパス");
        let mut path = self.process_explorer_path.clone();
        let edited = ui
            .add(
                egui::TextEdit::singleline(&mut path)
                    .hint_text(r"C:\Tools\ProcessExplorer\procexp64.exe"),
            )
            .changed();
        if edited {
            self.set_process_explorer_path(path);
        }
        if ui.button("エクスプローラーから選択…").clicked() {
            if let Some(chosen) = self.platform.choose_executable() {
                self.set_process_explorer_path(chosen);
            }
        }
        let normalized = normalized_executable_path(&self.process_explorer_path);
        if normalized.is_empty() {
            ui.colored_label(
                Color32::from_rgb(255, 175, 90),
                "実行ファイルのパスを設定してください",
            );
        } else if !Path::new(&normalized).is_file() {
            ui.colored_label(
                Color32::from_rgb(255, 120, 120),
                "指定されたファイルが見つかりません",
            );
        } else if ui.button("Process Explorerをテスト起動").clicked() {
            self.launch_process_tool();
        }
    }
}

/// 上下の端が暗く、中央がわずかに明るいブラックメタリックの背景。
fn paint_metallic_background(ui: &egui::Ui, area: egui::Rect) {
    const BANDS: usize = 28;
    for band in 0..BANDS {
        let t = band as f32 / (BANDS - 1) as f32;
        let shine = (1.0 - (t * 2.0 - 1.0).abs()) * 12.0;
        let value = (9.0 + shine) as u8;
        let top = egui::lerp(area.top()..=area.bottom(), t);
        let bottom = egui::lerp(area.top()..=area.bottom(), (band + 1) as f32 / BANDS as f32);
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(area.left(), top),
                egui::pos2(area.right(), bottom),
            ),
            0.0,
            Color32::from_rgb(value, value + 1, value + 4),
        );
    }
}

#[cfg(test)]
#[path = "dock_tests.rs"]
mod tests;
