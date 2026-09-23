use crate::app::{ContextMenuTarget, LauncherApp};
use crate::config::{
    choose_process_explorer_file, save_popup_direction, save_process_explorer_path,
    save_process_tool, PopupDirection, ProcessTool,
};
use crate::layout::{
    context_menu_screen_position, current_date_time, directional_tooltip, launcher_window_position,
    settings_dialog_position,
};
use crate::model::IconKind;
use crate::theme::draw_icon_colored;
use crate::ui::normalized_executable_path;
use eframe::egui::{self, Color32, Key};
use eframe::{App, Frame};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

impl App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let (is_fullscreen, is_maximized) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.fullscreen.unwrap_or(false),
                viewport.maximized.unwrap_or(false),
            )
        });
        if is_fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
        }
        if is_maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        }
        if self.last_refresh.elapsed() >= Duration::from_secs(1) {
            self.refresh_running();
            self.last_refresh = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
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
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|input| input.key_pressed(Key::ArrowRight)) {
            self.selected = (self.selected + 1) % self.items.len();
        }
        if ctx.input(|input| input.key_pressed(Key::ArrowLeft)) {
            self.selected = self.selected.checked_sub(1).unwrap_or(self.items.len() - 1);
        }
        if ctx.input(|input| input.key_pressed(Key::Enter)) {
            self.launch(self.selected);
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(8, 9, 12, 250))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 55),
                    ))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(6, 8)),
            )
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let background_response = ui.interact(
                    area,
                    ui.id().with("launcher-background-context"),
                    egui::Sense::click(),
                );
                let bands = 28;
                for band in 0..bands {
                    let t = band as f32 / (bands - 1) as f32;
                    let shine = (1.0 - (t * 2.0 - 1.0).abs()) * 12.0;
                    let value = (9.0 + shine) as u8;
                    let top = egui::lerp(area.top()..=area.bottom(), t);
                    let bottom =
                        egui::lerp(area.top()..=area.bottom(), (band + 1) as f32 / bands as f32);
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            egui::pos2(area.left(), top),
                            egui::pos2(area.right(), bottom),
                        ),
                        0.0,
                        Color32::from_rgb(value, value + 1, value + 4),
                    );
                }
                ui.vertical_centered(|ui| {
                    let (date, weekday, time) = current_date_time();
                    let clock_response = ui
                        .vertical_centered(|ui| {
                            for text in [date, weekday, time] {
                                ui.label(
                                    egui::RichText::new(text)
                                        .size(11.0)
                                        .color(Color32::from_rgb(225, 229, 238)),
                                );
                            }
                        })
                        .response
                        .interact(egui::Sense::click());
                    if clock_response.secondary_clicked() {
                        if let Some(position) = context_menu_screen_position(
                            self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT,
                        ) {
                            self.context_menu =
                                Some((ContextMenuTarget::Clock, position, Instant::now()));
                        }
                    }
                    ui.add_space(3.0);
                    let (handle, drag) =
                        ui.allocate_exact_size(egui::vec2(40.0, 14.0), egui::Sense::drag());
                    for offset in [-6.0, 0.0, 6.0] {
                        ui.painter().circle_filled(
                            egui::pos2(handle.center().x + offset, handle.center().y),
                            1.5,
                            Color32::from_gray(145),
                        );
                    }
                    if drag.drag_started() {
                        self.drag_origin = launcher_window_position();
                    }
                    if drag.dragged() {
                        if let Some(origin) = self.drag_origin {
                            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                                origin + drag.drag_delta(),
                            ));
                        }
                    }
                    if drag.drag_stopped() {
                        self.drag_origin = None;
                    }
                    if drag.secondary_clicked() {
                        if let Some(position) = context_menu_screen_position(
                            self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT,
                        ) {
                            self.context_menu =
                                Some((ContextMenuTarget::Handle, position, Instant::now()));
                        }
                    }
                    ui.add_space(3.0);
                    let (settings_rect, settings_response) =
                        ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
                    ui.painter().rect_filled(
                        settings_rect,
                        8.0,
                        if settings_response.hovered() {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 32)
                        } else {
                            Color32::from_rgba_unmultiplied(255, 255, 255, 12)
                        },
                    );
                    draw_icon_colored(
                        ui.painter(),
                        settings_rect.shrink(10.0),
                        IconKind::Settings,
                        Color32::from_rgb(188, 194, 204),
                    );
                    directional_tooltip(
                        &settings_response,
                        "Dock 設定",
                        self.popup_direction.alignment(ctx),
                    );
                    if settings_response.clicked() {
                        self.show_settings = true;
                        ctx.request_repaint();
                    }
                    ui.add_space(5.0);
                    ui.separator();
                    ui.add_space(5.0);
                    for index in 0..self.items.len() {
                        self.icon_button(ui, ctx, index);
                    }
                    if !self.running.is_empty() {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);
                        let scroll_height = (ui.available_height() - 18.0).max(40.0);
                        egui::ScrollArea::vertical()
                            .id_salt("running-items")
                            .max_height(scroll_height)
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    let count = self.running.len();
                                    for index in 0..count {
                                        self.running_button(ui, ctx, index);
                                    }
                                })
                            });
                    }
                    let (resize_rect, resize) =
                        ui.allocate_exact_size(egui::vec2(40.0, 12.0), egui::Sense::drag());
                    ui.painter().line_segment(
                        [
                            egui::pos2(resize_rect.center().x - 8.0, resize_rect.center().y),
                            egui::pos2(resize_rect.center().x + 8.0, resize_rect.center().y),
                        ],
                        egui::Stroke::new(2.0_f32, Color32::from_gray(120)),
                    );
                    if resize.drag_started() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(
                            egui::ResizeDirection::South,
                        ));
                    }
                });
                if background_response.secondary_clicked() {
                    if let Some(position) = context_menu_screen_position(
                        self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT,
                    ) {
                        self.context_menu =
                            Some((ContextMenuTarget::Handle, position, Instant::now()));
                        self.confirm_close_all = false;
                    }
                }
            });

        if self.show_settings {
            let position = settings_dialog_position(ctx, self.popup_direction);
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("launcher-settings"),
                egui::ViewportBuilder::default()
                    .with_title("Windows Side Dock 設定")
                    .with_inner_size([380.0, 430.0])
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
                    egui::CentralPanel::default().show(settings_ctx, |ui| {
                        ui.heading("Windows Side Dock 設定");
                        ui.separator();
                        ui.label("UIフォント");
                        ui.add_enabled(false, egui::Button::new("BIZ UDPゴシック"));
                        ui.add_space(8.0);
                        ui.label("文字サイズ");
                        ui.add(egui::Slider::new(&mut self.font_size, 10.0..=20.0).suffix(" px"));
                        ui.add_space(8.0);
                        ui.label("ポップアップの方向");
                        let direction_changed = ui
                            .radio_value(&mut self.popup_direction, PopupDirection::Auto, "自動")
                            .changed()
                            | ui.radio_value(
                                &mut self.popup_direction,
                                PopupDirection::Left,
                                "常に左",
                            )
                            .changed()
                            | ui.radio_value(
                                &mut self.popup_direction,
                                PopupDirection::Right,
                                "常に右",
                            )
                            .changed();
                        if direction_changed {
                            save_popup_direction(self.popup_direction);
                        }
                        ui.add_space(12.0);
                        ui.label("システムモニター");
                        let tool_changed = ui
                            .radio_value(
                                &mut self.process_tool,
                                ProcessTool::TaskManager,
                                "タスク マネージャー",
                            )
                            .changed()
                            | ui.radio_value(
                                &mut self.process_tool,
                                ProcessTool::ProcessExplorer,
                                "Process Explorer",
                            )
                            .changed();
                        if tool_changed {
                            save_process_tool(self.process_tool);
                        }
                        if self.process_tool == ProcessTool::ProcessExplorer {
                            ui.label("Process Explorerのパス");
                            let path_response = ui.add(
                                egui::TextEdit::singleline(&mut self.process_explorer_path)
                                    .hint_text(r"C:\Tools\ProcessExplorer\procexp64.exe"),
                            );
                            if path_response.changed() {
                                save_process_explorer_path(&self.process_explorer_path);
                                self.monitor_status = None;
                            }
                            if ui.button("エクスプローラーから選択…").clicked() {
                                if let Some(path) = choose_process_explorer_file() {
                                    self.process_explorer_path = path;
                                    save_process_explorer_path(&self.process_explorer_path);
                                    self.monitor_status = None;
                                }
                            }
                            let normalized =
                                normalized_executable_path(&self.process_explorer_path);
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
                        if let Some(message) = &self.monitor_status {
                            ui.colored_label(Color32::from_rgb(255, 140, 100), message);
                        }
                        settings_ctx.style_mut(|style| {
                            if let Some(font) = style.text_styles.get_mut(&egui::TextStyle::Body) {
                                font.size = self.font_size;
                            }
                            if let Some(font) = style.text_styles.get_mut(&egui::TextStyle::Button)
                            {
                                font.size = self.font_size;
                            }
                        });
                        ui.add_space(10.0);
                        if ui.button("閉じる").clicked() {
                            self.show_settings = false;
                            settings_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                },
            );
        }
        self.show_context_menu_viewport(ctx);
        self.show_window_picker_viewport(ctx);
    }
}
