#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod model;
mod platform;

use config::{
    choose_process_explorer_file, config_path, load_popup_direction, load_process_explorer_path,
    load_process_tool, save_popup_direction, save_process_explorer_path, save_process_tool,
    PopupDirection, ProcessTool,
};
use eframe::egui::{self, Color32, Key};
use eframe::{App, Frame};
use model::{same_application, IconKind, LauncherItem, RunningWindow};
use platform::{
    activate_taskbar_item, close_all_windows, load_shell_icon, open_target, running_apps,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum ContextMenuTarget {
    Handle,
    Clock,
    Pinned(usize),
    Running(usize),
}

impl PopupDirection {
    fn alignment(self, ctx: &egui::Context) -> egui::RectAlign {
        match self {
            Self::Auto => popup_alignment(ctx),
            Self::Left => egui::RectAlign::LEFT,
            Self::Right => egui::RectAlign::RIGHT,
        }
    }
}

struct LauncherApp {
    items: Vec<LauncherItem>,
    running: Vec<LauncherItem>,
    selected: usize,
    textures: HashMap<String, egui::TextureHandle>,
    last_refresh: Instant,
    show_settings: bool,
    font_size: f32,
    popup_direction: PopupDirection,
    drag_origin: Option<egui::Pos2>,
    context_menu: Option<(ContextMenuTarget, egui::Pos2, Instant)>,
    window_picker: Option<(String, Vec<RunningWindow>, egui::Pos2)>,
    confirm_close_all: bool,
    process_tool: ProcessTool,
    process_explorer_path: String,
    monitor_status: Option<String>,
}

impl LauncherApp {
    fn new() -> Self {
        let windows = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let mut app = Self {
            items: vec![
                item(
                    "ファイルエクスプローラー",
                    &format!(r"{windows}\explorer.exe"),
                    IconKind::Folder,
                    &format!(r"{windows}\explorer.exe"),
                ),
                item(
                    "ターミナル",
                    &format!(r"{local}\Microsoft\WindowsApps\wt.exe"),
                    IconKind::Terminal,
                    &format!(r"{local}\Microsoft\WindowsApps\wt.exe"),
                ),
                item(
                    "メモ帳",
                    &format!(r"{windows}\System32\notepad.exe"),
                    IconKind::Note,
                    &format!(r"{windows}\System32\notepad.exe"),
                ),
                item(
                    "Windows 設定",
                    "ms-settings:",
                    IconKind::Settings,
                    &format!(r"{windows}\ImmersiveControlPanel\SystemSettings.exe"),
                ),
            ],
            running: Vec::new(),
            selected: 0,
            textures: HashMap::new(),
            last_refresh: Instant::now() - Duration::from_secs(2),
            show_settings: false,
            font_size: 13.0,
            popup_direction: load_popup_direction(),
            drag_origin: None,
            context_menu: None,
            window_picker: None,
            confirm_close_all: false,
            process_tool: load_process_tool(),
            process_explorer_path: load_process_explorer_path(),
            monitor_status: None,
        };
        app.load_registered();
        app
    }

    fn load_registered(&mut self) {
        let Some(path) = config_path() else { return };
        let Ok(contents) = std::fs::read_to_string(path) else {
            return;
        };
        for line in contents.lines() {
            if let Some((name, command)) = line.split_once('|') {
                if !self.items.iter().any(|item| item.command == command) {
                    self.items
                        .push(item(name, command, IconKind::File, command));
                }
            }
        }
    }

    fn save_registered(&self) {
        let Some(path) = config_path() else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let text = self
            .items
            .iter()
            .skip(4)
            .map(|item| {
                format!(
                    "{}|{}",
                    item.name.replace('|', " "),
                    item.command.replace('|', " ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let _ = std::fs::write(path, text);
    }

    fn add_path(&mut self, path: &Path) {
        let Some(command) = path.to_str() else { return };
        if self.items.iter().any(|item| item.command == command) {
            return;
        }
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("アプリ");
        self.items
            .push(item(name, command, IconKind::File, command));
        self.save_registered();
    }

    fn launch(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }
        if self.items[index]
            .command
            .eq_ignore_ascii_case("ms-settings:")
            && !self.items[index].windows.is_empty()
        {
            activate_taskbar_item(&self.items[index].windows);
            return;
        }
        let command = self.items[index].command.clone();
        let _ = open_target(&command);
    }

    fn refresh_running(&mut self) {
        let discovered = running_apps();
        for pinned in &mut self.items {
            pinned.windows.clear();
            pinned.active = false;
            if let Some(running) = discovered
                .iter()
                .find(|running| same_application(pinned, running))
            {
                pinned.windows = running.windows.clone();
                pinned.active = running.active;
            }
        }
        self.running = discovered
            .into_iter()
            .filter(|running| {
                !self
                    .items
                    .iter()
                    .any(|pinned| same_application(pinned, running))
            })
            .collect();
    }

    fn pin_running(&mut self, index: usize) {
        let Some(item) = self.running.get(index).cloned() else {
            return;
        };
        self.items.push(LauncherItem {
            windows: Vec::new(),
            active: false,
            ..item
        });
        self.save_registered();
        self.refresh_running();
    }

    fn activate_running(&mut self, index: usize, ctx: &egui::Context) {
        let Some(item) = self.running.get(index) else {
            return;
        };
        let name = item.name.clone();
        let windows = item.windows.clone();
        self.open_or_activate_windows(name, windows, ctx);
    }

    fn open_or_activate_windows(
        &mut self,
        name: String,
        windows: Vec<RunningWindow>,
        ctx: &egui::Context,
    ) {
        if windows.len() <= 1 {
            activate_taskbar_item(&windows);
        } else if let Some(position) = window_picker_screen_position(
            self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT,
        ) {
            self.window_picker = Some((name, windows, position));
            self.confirm_close_all = false;
        }
    }
}

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

impl LauncherApp {
    fn launch_process_tool(&mut self) -> bool {
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

    fn show_window_picker_viewport(&mut self, ctx: &egui::Context) {
        let Some((name, windows, position)) = self.window_picker.clone() else {
            return;
        };
        let height = (windows.len() as f32 * 38.0 + 116.0).min(420.0);
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("launcher-window-picker"),
            egui::ViewportBuilder::default()
                .with_title(format!("{name} のウィンドウ"))
                .with_inner_size([480.0, height])
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

    fn show_context_menu_viewport(&mut self, ctx: &egui::Context) {
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
        let width = if window_count > 0 { 430.0 } else { 210.0 };
        let position =
            if width > 210.0 && self.popup_direction.alignment(ctx) == egui::RectAlign::LEFT {
                egui::pos2(position.x - (width - 210.0), position.y)
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

    fn icon_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, index: usize) {
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

    fn running_button(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, index: usize) {
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

fn item(name: &str, command: &str, fallback_icon: IconKind, icon_source: &str) -> LauncherItem {
    LauncherItem {
        name: name.into(),
        command: command.into(),
        fallback_icon,
        icon: load_shell_icon(icon_source),
        windows: Vec::new(),
        active: false,
    }
}

fn normalized_executable_path(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| character == '"' || character == '\'')
        .to_owned()
}

fn dock_directory() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn draw_icon(painter: &egui::Painter, rect: egui::Rect, icon: IconKind) {
    draw_icon_colored(painter, rect, icon, Color32::from_rgb(238, 242, 250));
}

fn left_aligned_button(ui: &mut egui::Ui, text: &str, height: f32) -> egui::Response {
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

fn draw_icon_colored(painter: &egui::Painter, rect: egui::Rect, icon: IconKind, color: Color32) {
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

#[cfg(windows)]
fn popup_should_open_left(_ctx: &egui::Context) -> bool {
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
    (rect.left + rect.right) / 2 > screen_width / 2
}

#[cfg(windows)]
fn launcher_window_position() -> Option<egui::Pos2> {
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
fn launcher_window_position() -> Option<egui::Pos2> {
    None
}

#[cfg(not(windows))]
fn popup_should_open_left(ctx: &egui::Context) -> bool {
    ctx.input(|input| {
        let viewport = input.viewport();
        match (viewport.outer_rect, viewport.monitor_size) {
            (Some(rect), Some(monitor)) => rect.center().x > monitor.x / 2.0,
            _ => true,
        }
    })
}

fn popup_alignment(ctx: &egui::Context) -> egui::RectAlign {
    if popup_should_open_left(ctx) {
        egui::RectAlign::LEFT
    } else {
        egui::RectAlign::RIGHT
    }
}

#[cfg(windows)]
fn settings_dialog_position(ctx: &egui::Context, direction: PopupDirection) -> egui::Pos2 {
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
        let x = if open_left {
            rect.left as f32 - 392.0
        } else {
            rect.right as f32 + 12.0
        };
        egui::pos2(x, rect.top as f32)
    } else {
        egui::pos2(100.0, 100.0)
    }
}

#[cfg(not(windows))]
fn settings_dialog_position(_ctx: &egui::Context, _direction: PopupDirection) -> egui::Pos2 {
    egui::pos2(100.0, 100.0)
}

fn directional_tooltip(response: &egui::Response, text: &str, alignment: egui::RectAlign) {
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
            .with_inner_size([220.0, 36.0])
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
fn context_menu_screen_position(open_left: bool) -> Option<egui::Pos2> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return None;
    }
    let x = if open_left {
        cursor.x as f32 - 218.0
    } else {
        cursor.x as f32 + 8.0
    };
    Some(egui::pos2(x, cursor.y as f32))
}

#[cfg(windows)]
fn window_picker_screen_position(open_left: bool) -> Option<egui::Pos2> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) } == 0 {
        return None;
    }
    let x = if open_left {
        cursor.x as f32 - 368.0
    } else {
        cursor.x as f32 + 8.0
    };
    Some(egui::pos2(x, cursor.y as f32))
}

#[cfg(not(windows))]
fn context_menu_screen_position(_open_left: bool) -> Option<egui::Pos2> {
    Some(egui::pos2(100.0, 100.0))
}

#[cfg(not(windows))]
fn window_picker_screen_position(_open_left: bool) -> Option<egui::Pos2> {
    Some(egui::pos2(100.0, 100.0))
}

#[cfg(windows)]
fn tooltip_screen_position(response: &egui::Response, open_left: bool) -> Option<egui::Pos2> {
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
    let x = if open_left {
        rect.left as f32 - 228.0
    } else {
        rect.right as f32 + 8.0
    };
    let y = rect.top as f32 + response.rect.center().y - 18.0;
    Some(egui::pos2(x, y.max(0.0)))
}

#[cfg(not(windows))]
fn tooltip_screen_position(response: &egui::Response, open_left: bool) -> Option<egui::Pos2> {
    let x = if open_left {
        response.rect.left() - 228.0
    } else {
        response.rect.right() + 8.0
    };
    Some(egui::pos2(x, response.rect.top()))
}

#[cfg(windows)]
fn current_date_time() -> (String, String, String) {
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
fn current_date_time() -> (String, String, String) {
    ("--/--".into(), "（―）".into(), "--:--".into())
}

fn configure_font(ctx: &egui::Context) {
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

#[cfg(windows)]
fn dock_geometry() -> ([f32; 2], [f32; 2]) {
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
fn dock_geometry() -> ([f32; 2], [f32; 2]) {
    ([0.0, 60.0], [54.0, 800.0])
}

fn main() -> eframe::Result {
    let (position, size) = dock_geometry();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(size)
            .with_min_inner_size([54.0, 220.0])
            .with_position(position)
            .with_decorations(false)
            .with_resizable(true)
            .with_fullscreen(false)
            .with_maximized(false)
            .with_transparent(true)
            .with_title("Windows Side Dock"),
        ..Default::default()
    };
    eframe::run_native(
        "Windows Side Dock",
        options,
        Box::new(|cc| {
            configure_font(&cc.egui_ctx);
            Ok(Box::new(LauncherApp::new()))
        }),
    )
}
