use crate::config::{ConfigStore, PopupDirection, ProcessTool};
use crate::layout::{popup_alignment, window_picker_screen_position};
use crate::model::{
    assign_running, item_name_for_path, registered_entries, IconKind, LauncherItem, RunningWindow,
    BUILTIN_ITEM_COUNT,
};
use crate::platform::{activate_windows, running_apps, Platform};
use crate::shell_menu::sync_process_tool_menu;
use crate::ui::normalized_executable_path;
use eframe::egui;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ContextMenuTarget {
    Handle,
    Clock,
    Pinned(usize),
    Running(usize),
}

pub(crate) struct LauncherApp {
    pub(crate) platform: Rc<dyn Platform>,
    pub(crate) config: ConfigStore,
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
    /// `windows_dir` は `%WINDIR%`、`local_app_data` は `%LOCALAPPDATA%`。標準アイコンのパスに使う。
    pub(crate) fn new(
        platform: Rc<dyn Platform>,
        config: ConfigStore,
        windows_dir: &str,
        local_app_data: &str,
    ) -> Self {
        config.migrate_legacy();
        let builtin = [
            (
                "ファイルエクスプローラー",
                format!(r"{windows_dir}\explorer.exe"),
                IconKind::Folder,
                format!(r"{windows_dir}\explorer.exe"),
            ),
            (
                "ターミナル",
                format!(r"{local_app_data}\Microsoft\WindowsApps\wt.exe"),
                IconKind::Terminal,
                format!(r"{local_app_data}\Microsoft\WindowsApps\wt.exe"),
            ),
            (
                "メモ帳",
                format!(r"{windows_dir}\System32\notepad.exe"),
                IconKind::Note,
                format!(r"{windows_dir}\System32\notepad.exe"),
            ),
            (
                "Windows 設定",
                "ms-settings:".to_owned(),
                IconKind::Settings,
                format!(r"{windows_dir}\ImmersiveControlPanel\SystemSettings.exe"),
            ),
        ];
        let items = builtin
            .iter()
            .map(|(name, command, icon, source)| {
                new_item(platform.as_ref(), name, command, *icon, source)
            })
            .collect();
        let mut app = Self {
            items,
            running: Vec::new(),
            selected: 0,
            textures: HashMap::new(),
            last_refresh: Instant::now() - Duration::from_secs(2),
            show_settings: false,
            font_size: 13.0,
            popup_direction: config.load_popup_direction(),
            drag_origin: None,
            context_menu: None,
            window_picker: None,
            confirm_close_all: false,
            process_tool: config.load_process_tool(),
            process_explorer_path: config.load_process_explorer_path(),
            monitor_status: None,
            platform,
            config,
        };
        app.load_registered();
        sync_process_tool_menu(
            app.platform.as_ref(),
            app.process_tool,
            &app.process_explorer_path,
        );
        app
    }

    pub(crate) fn alignment(&self) -> egui::RectAlign {
        popup_alignment(self.platform.as_ref(), self.popup_direction)
    }

    pub(crate) fn opens_left(&self) -> bool {
        self.alignment() == egui::RectAlign::LEFT
    }

    fn load_registered(&mut self) {
        for (name, command) in self.config.load_registered_items() {
            if !self.items.iter().any(|item| item.command == command) {
                let item = new_item(
                    self.platform.as_ref(),
                    &name,
                    &command,
                    IconKind::File,
                    &command,
                );
                self.items.push(item);
            }
        }
    }

    pub(crate) fn save_registered(&self) {
        self.config
            .save_registered_items(registered_entries(&self.items));
    }

    pub(crate) fn add_path(&mut self, path: &Path) {
        let Some(command) = path.to_str() else { return };
        if self.items.iter().any(|item| item.command == command) {
            return;
        }
        let item = new_item(
            self.platform.as_ref(),
            item_name_for_path(path),
            command,
            IconKind::File,
            command,
        );
        self.items.push(item);
        self.save_registered();
    }

    /// ピン留め項目を起動する。Windows 設定だけは単一ウィンドウとして扱い、開いていれば前面へ出す。
    pub(crate) fn launch(&mut self, index: usize) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        if item.command.eq_ignore_ascii_case("ms-settings:") && !item.windows.is_empty() {
            activate_windows(self.platform.as_ref(), &item.windows);
            return;
        }
        let _ = self.platform.open_target(&item.command);
    }

    pub(crate) fn refresh_running(&mut self) {
        self.running = assign_running(&mut self.items, running_apps(self.platform.as_ref()));
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

    /// ユーザー登録項目のピン留めを外す。標準アイコンは外せない。
    pub(crate) fn unpin(&mut self, index: usize) {
        if index < BUILTIN_ITEM_COUNT || index >= self.items.len() {
            return;
        }
        self.items.remove(index);
        self.selected = self.selected.min(self.items.len() - 1);
        self.save_registered();
        self.refresh_running();
    }

    pub(crate) fn activate_running(&mut self, index: usize) {
        let Some(item) = self.running.get(index) else {
            return;
        };
        let name = item.name.clone();
        let windows = item.windows.clone();
        self.open_or_activate_windows(name, windows);
    }

    /// ウィンドウが1つなら前面へ出し、複数ならウィンドウ選択画面を開く。
    pub(crate) fn open_or_activate_windows(&mut self, name: String, windows: Vec<RunningWindow>) {
        if windows.len() <= 1 {
            activate_windows(self.platform.as_ref(), &windows);
        } else if let Some(position) =
            window_picker_screen_position(self.platform.as_ref(), self.opens_left())
        {
            self.window_picker = Some((name, windows, position));
            self.confirm_close_all = false;
        }
    }

    pub(crate) fn open_context_menu(&mut self, target: ContextMenuTarget) {
        if let Some(position) =
            crate::layout::context_menu_screen_position(self.platform.as_ref(), self.opens_left())
        {
            self.context_menu = Some((target, position, Instant::now()));
            self.confirm_close_all = false;
        }
    }

    pub(crate) fn process_tool_ready(&self) -> bool {
        self.process_tool == ProcessTool::TaskManager
            || Path::new(&normalized_executable_path(&self.process_explorer_path)).is_file()
    }

    /// 設定されたプロセスツールを起動する。失敗したら理由を表示し、必要なら設定画面を開く。
    pub(crate) fn launch_process_tool(&mut self) -> bool {
        let (target, name) = match self.process_tool {
            ProcessTool::TaskManager => ("taskmgr.exe".to_owned(), "タスク マネージャー"),
            ProcessTool::ProcessExplorer => (
                normalized_executable_path(&self.process_explorer_path),
                "Process Explorer",
            ),
        };
        if !self.process_tool_ready() {
            self.monitor_status = Some("Process Explorerの実行ファイルが見つかりません".into());
            self.show_settings = true;
            return false;
        }
        let opened = self.platform.open_target(&target);
        self.monitor_status = (!opened).then(|| format!("{name}を起動できませんでした"));
        if !opened && self.process_tool == ProcessTool::ProcessExplorer {
            self.show_settings = true;
        }
        opened
    }

    pub(crate) fn set_popup_direction(&mut self, direction: PopupDirection) {
        self.popup_direction = direction;
        self.config.save_popup_direction(direction);
    }

    pub(crate) fn set_process_tool(&mut self, tool: ProcessTool) {
        self.process_tool = tool;
        self.config.save_process_tool(tool);
        self.sync_process_tool_menu();
    }

    pub(crate) fn set_process_explorer_path(&mut self, path: String) {
        self.process_explorer_path = path;
        self.config
            .save_process_explorer_path(&self.process_explorer_path);
        self.sync_process_tool_menu();
        self.monitor_status = None;
    }

    fn sync_process_tool_menu(&self) {
        sync_process_tool_menu(
            self.platform.as_ref(),
            self.process_tool,
            &self.process_explorer_path,
        );
    }
}

pub(crate) fn new_item(
    platform: &dyn Platform,
    name: &str,
    command: &str,
    fallback_icon: IconKind,
    icon_source: &str,
) -> LauncherItem {
    LauncherItem {
        name: name.into(),
        command: command.into(),
        fallback_icon,
        icon: platform.load_icon(icon_source),
        windows: Vec::new(),
        active: false,
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
