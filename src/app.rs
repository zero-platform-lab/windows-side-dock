use crate::config::{
    config_path, load_popup_direction, load_process_explorer_path, load_process_tool,
    PopupDirection, ProcessTool,
};
use crate::layout::{popup_alignment, window_picker_screen_position};
use crate::model::{same_application, IconKind, LauncherItem, RunningWindow};
use crate::platform::{activate_taskbar_item, open_target, running_apps};
use crate::ui::item;
use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(crate) enum ContextMenuTarget {
    Handle,
    Clock,
    Pinned(usize),
    Running(usize),
}

impl PopupDirection {
    pub(crate) fn alignment(self, ctx: &egui::Context) -> egui::RectAlign {
        match self {
            Self::Auto => popup_alignment(ctx),
            Self::Left => egui::RectAlign::LEFT,
            Self::Right => egui::RectAlign::RIGHT,
        }
    }
}

pub(crate) struct LauncherApp {
    pub(crate) items: Vec<LauncherItem>,
    pub(crate) running: Vec<LauncherItem>,
    pub(crate) selected: usize,
    pub(crate) textures: HashMap<String, egui::TextureHandle>,
    pub(crate) last_refresh: Instant,
    pub(crate) show_settings: bool,
    pub(crate) font_size: f32,
    pub(crate) popup_direction: PopupDirection,
    pub(crate) drag_origin: Option<egui::Pos2>,
    pub(crate) context_menu: Option<(ContextMenuTarget, egui::Pos2, Instant)>,
    pub(crate) window_picker: Option<(String, Vec<RunningWindow>, egui::Pos2)>,
    pub(crate) confirm_close_all: bool,
    pub(crate) process_tool: ProcessTool,
    pub(crate) process_explorer_path: String,
    pub(crate) monitor_status: Option<String>,
}

impl LauncherApp {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn save_registered(&self) {
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

    pub(crate) fn add_path(&mut self, path: &Path) {
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

    pub(crate) fn launch(&mut self, index: usize) {
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

    pub(crate) fn refresh_running(&mut self) {
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

    pub(crate) fn pin_running(&mut self, index: usize) {
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

    pub(crate) fn activate_running(&mut self, index: usize, ctx: &egui::Context) {
        let Some(item) = self.running.get(index) else {
            return;
        };
        let name = item.name.clone();
        let windows = item.windows.clone();
        self.open_or_activate_windows(name, windows, ctx);
    }

    pub(crate) fn open_or_activate_windows(
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
