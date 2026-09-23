use crate::app::{ContextMenuTarget, LauncherApp};
use crate::config::ProcessTool;
use crate::layout::{CONTEXT_MENU_WIDTH, WINDOW_PICKER_WIDTH};
use crate::platform::{activate_taskbar_item, close_all_windows, open_target};
use crate::theme::left_aligned_button;
use crate::ui::{dock_directory, normalized_executable_path};
use eframe::egui::{self, Color32, Key};
use std::path::Path;
use std::time::Duration;

impl LauncherApp {
    pub(crate) fn launch_process_tool(&mut self) -> bool {
        match self.process_tool {
            ProcessTool::TaskManager => {
                let opened = open_target("taskmgr.exe");
                self.monitor_status =
                    (!opened).then(|| "タスク マネージャーを起動できませんでした".into());
                opened
            }
            ProcessTool::ProcessExplorer => {
                let path = normalized_executable_path(&self.process_explorer_path);
                if path.is_empty() || !Path::new(&path).is_file() {
                    self.monitor_status =
                        Some("Process Explorerの実行ファイルが見つかりません".into());
                    self.show_settings = true;
                    return false;
                }
                let opened = open_target(&path);
                self.monitor_status =
                    (!opened).then(|| "Process Explorerを起動できませんでした".into());
                if !opened {
                    self.show_settings = true;
                }
                opened
            }
        }
    }

    pub(crate) fn show_window_picker_viewport(&mut self, ctx: &egui::Context) {
        let Some((name, windows, position)) = self.window_picker.clone() else {
            return;
        };
        let height = (windows.len() as f32 * 38.0 + 116.0).min(420.0);
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("launcher-window-picker"),
            egui::ViewportBuilder::default()
                .with_title(format!("{name} のウィンドウ"))
                .with_inner_size([WINDOW_PICKER_WIDTH, height])
                .with_position(position)
                .with_resizable(false)
                .with_taskbar(false)
                .with_always_on_top()
                .with_active(true),
            |picker_ctx, _class| {
                if picker_ctx.input(|input| {
                    input.viewport().close_requested() || input.key_pressed(Key::Escape)
                }) {
                    close = true;
                }
                egui::CentralPanel::default().show(picker_ctx, |ui| {
                    ui.heading(&name);
                    ui.label(format!(
                        "{}個のウィンドウ — 現在のタイトルで選択",
                        windows.len()
                    ));
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height((height - 105.0).max(60.0))
                        .show(ui, |ui| {
                            for (index, window) in windows.iter().enumerate() {
                                let label = format!("{}.  {}", index + 1, window.title);
                                let response = ui
                                    .add_sized(
                                        [ui.available_width(), 32.0],
                                        egui::Button::new(label),
                                    )
                                    .on_hover_text(&window.title);
                                if response.clicked() {
                                    activate_taskbar_item(std::slice::from_ref(window));
                                    close = true;
                                }
                            }
                        });
                    ui.separator();
                    if self.confirm_close_all {
                        ui.horizontal(|ui| {
                            if ui
                                .button(
                                    egui::RichText::new("本当にすべて閉じる")
                                        .color(Color32::from_rgb(255, 120, 120)),
                                )
                                .clicked()
                            {
                                close_all_windows(&windows);
                                close = true;
                            }
                            if ui.button("キャンセル").clicked() {
                                self.confirm_close_all = false;
                            }
                        });
                    } else if ui.button("すべて閉じる").clicked() {
                        self.confirm_close_all = true;
                    }
                });
                if close {
                    picker_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            },
        );
        if close {
            self.window_picker = None;
            self.confirm_close_all = false;
        }
    }

    pub(crate) fn show_context_menu_viewport(&mut self, ctx: &egui::Context) {
        let Some((target, position, opened_at)) = self.context_menu else {
            return;
        };
        let height = match target {
            ContextMenuTarget::Handle => 132.0,
            ContextMenuTarget::Clock => 54.0,
            ContextMenuTarget::Pinned(index) => {
                match self.items.get(index).map(|item| item.windows.len()) {
                    Some(count) if count > 1 => (146.0 + count as f32 * 34.0).min(420.0),
                    Some(1) => 150.0,
                    _ => 102.0,
                }
            }
            ContextMenuTarget::Running(index) => {
                match self.running.get(index).map(|item| item.windows.len()) {
                    Some(count) if count > 1 => (112.0 + count as f32 * 34.0).min(420.0),
                    Some(1) => 116.0,
                    _ => 102.0,
                }
            }
        };
        let window_count = match target {
            ContextMenuTarget::Pinned(index) => {
                self.items.get(index).map_or(0, |item| item.windows.len())
            }
            ContextMenuTarget::Running(index) => {
                self.running.get(index).map_or(0, |item| item.windows.len())
            }
            _ => 0,
        };
        let width = if window_count > 0 {
            430.0
        } else {
            CONTEXT_MENU_WIDTH
        };
        let position = if width > CONTEXT_MENU_WIDTH
            && self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT
        {
            egui::pos2(position.x - (width - CONTEXT_MENU_WIDTH), position.y)
        } else {
            position
        };
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("launcher-context-menu"),
            egui::ViewportBuilder::default()
                .with_title("Launcher menu")
                .with_inner_size([width, height])
                .with_position(position)
                .with_decorations(false)
                .with_resizable(false)
                .with_transparent(true)
                .with_taskbar(false)
                .with_always_on_top()
                .with_visible(opened_at.elapsed() >= Duration::from_millis(16))
                .with_active(true),
            |menu_ctx, _class| {
                if opened_at.elapsed() < Duration::from_millis(16) {
                    menu_ctx.request_repaint_after(Duration::from_millis(16));
                }
                if menu_ctx.input(|input| {
                    input.viewport().close_requested()
                        || input.key_pressed(Key::Escape)
                        || (opened_at.elapsed() > Duration::from_millis(200)
                            && input.viewport().focused == Some(false))
                }) {
                    close = true;
                }
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(12, 14, 19, 252))
                            .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(80)))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(8, 7)),
                    )
                    .show(menu_ctx, |ui| match target {
                        ContextMenuTarget::Handle => {
                            if ui.button("Windows Side Dockの場所を開く").clicked() {
                                if let Some(directory) = dock_directory() {
                                    let _ = open_target(&directory.to_string_lossy());
                                }
                                close = true;
                            }
                            let tool_label = match self.process_tool {
                                ProcessTool::TaskManager => "タスク マネージャー",
                                ProcessTool::ProcessExplorer => "Process Explorer",
                            };
                            let tool_ready = self.process_tool == ProcessTool::TaskManager
                                || Path::new(self.process_explorer_path.trim()).is_file();
                            if ui
                                .add_enabled(tool_ready, egui::Button::new(tool_label))
                                .clicked()
                            {
                                self.launch_process_tool();
                                close = true;
                            }
                            ui.separator();
                            if ui.button("Dock 設定").clicked() {
                                self.show_settings = true;
                                menu_ctx.request_repaint();
                                close = true;
                            }
                        }
                        ContextMenuTarget::Clock => {
                            let tool_label = match self.process_tool {
                                ProcessTool::TaskManager => "タスク マネージャー",
                                ProcessTool::ProcessExplorer => "Process Explorer",
                            };
                            let tool_ready = self.process_tool == ProcessTool::TaskManager
                                || Path::new(self.process_explorer_path.trim()).is_file();
                            if ui
                                .add_enabled(tool_ready, egui::Button::new(tool_label))
                                .clicked()
                            {
                                self.launch_process_tool();
                                close = true;
                            }
                            if !tool_ready && ui.small_button("パスを設定…").clicked() {
                                self.show_settings = true;
                                menu_ctx.request_repaint();
                                close = true;
                            }
                        }
                        ContextMenuTarget::Pinned(index) => {
                            if index >= self.items.len() {
                                close = true;
                                return;
                            }
                            let is_running = !self.items[index].windows.is_empty();
                            if ui
                                .button(if is_running {
                                    "新しく起動"
                                } else {
                                    "起動"
                                })
                                .clicked()
                            {
                                self.launch(index);
                                close = true;
                            }
                            if is_running {
                                let windows = self.items[index].windows.clone();
                                ui.label("ウィンドウへ移動");
                                if windows.len() == 1 {
                                    let window = &windows[0];
                                    if left_aligned_button(ui, &window.title, 30.0)
                                        .on_hover_text(&window.title)
                                        .clicked()
                                    {
                                        activate_taskbar_item(&windows);
                                        close = true;
                                    }
                                } else {
                                    egui::ScrollArea::vertical()
                                        .max_height((height - 139.0).max(68.0))
                                        .show(ui, |ui| {
                                            for window in &windows {
                                                if left_aligned_button(ui, &window.title, 30.0)
                                                    .on_hover_text(&window.title)
                                                    .clicked()
                                                {
                                                    activate_taskbar_item(std::slice::from_ref(
                                                        window,
                                                    ));
                                                    close = true;
                                                }
                                            }
                                        });
                                    if self.confirm_close_all {
                                        if ui
                                            .button(
                                                egui::RichText::new("本当にすべて閉じる")
                                                    .color(Color32::from_rgb(255, 120, 120)),
                                            )
                                            .clicked()
                                        {
                                            close_all_windows(&windows);
                                            close = true;
                                        }
                                    } else if ui.button("すべて閉じる").clicked() {
                                        self.confirm_close_all = true;
                                    }
                                }
                            }
                            ui.separator();
                            if index >= 4 {
                                if ui.button("ピン留めを外す").clicked() {
                                    self.items.remove(index);
                                    self.selected =
                                        self.selected.min(self.items.len().saturating_sub(1));
                                    self.save_registered();
                                    self.refresh_running();
                                    close = true;
                                }
                            } else {
                                ui.add_enabled(false, egui::Button::new("標準アイコン"));
                            }
                        }
                        ContextMenuTarget::Running(index) => {
                            if index >= self.running.len() {
                                close = true;
                                return;
                            }
                            let windows = self.running[index].windows.clone();
                            ui.label("ウィンドウへ移動");
                            if windows.len() == 1 {
                                let window = &windows[0];
                                if left_aligned_button(ui, &window.title, 30.0)
                                    .on_hover_text(&window.title)
                                    .clicked()
                                {
                                    activate_taskbar_item(&windows);
                                    close = true;
                                }
                            } else {
                                egui::ScrollArea::vertical()
                                    .max_height((height - 105.0).max(68.0))
                                    .show(ui, |ui| {
                                        for window in &windows {
                                            if left_aligned_button(ui, &window.title, 30.0)
                                                .on_hover_text(&window.title)
                                                .clicked()
                                            {
                                                activate_taskbar_item(std::slice::from_ref(window));
                                                close = true;
                                            }
                                        }
                                    });
                                if self.confirm_close_all {
                                    if ui
                                        .button(
                                            egui::RichText::new("本当にすべて閉じる")
                                                .color(Color32::from_rgb(255, 120, 120)),
                                        )
                                        .clicked()
                                    {
                                        close_all_windows(&windows);
                                        close = true;
                                    }
                                } else if ui.button("すべて閉じる").clicked() {
                                    self.confirm_close_all = true;
                                }
                            }
                            ui.separator();
                            if ui.button("ピン留めする").clicked() {
                                self.pin_running(index);
                                close = true;
                            }
                        }
                    });
                if close {
                    menu_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            },
        );
        if close {
            self.context_menu = None;
            self.confirm_close_all = false;
        }
    }
}
