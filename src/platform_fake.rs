//! テスト用の偽のOS。操作を `calls` に記録し、問い合わせの戻り値はフィールドで指定する。

use super::{LocalTime, Platform, TrayAction};
use eframe::egui;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};

pub(crate) struct FakePlatform {
    pub(crate) calls: RefCell<Vec<String>>,
    pub(crate) open_succeeds: bool,
    pub(crate) windows: RefCell<Vec<(isize, String, String)>>,
    pub(crate) foreground: isize,
    pub(crate) minimized: Vec<isize>,
    pub(crate) icons: bool,
    pub(crate) cursor: Option<egui::Pos2>,
    pub(crate) dock: Option<egui::Rect>,
    pub(crate) screen_width: f32,
    pub(crate) work_area: Option<egui::Rect>,
    pub(crate) time: LocalTime,
    pub(crate) chosen_file: Option<String>,
    pub(crate) install_directory: Option<std::path::PathBuf>,
    /// 存在するレジストリキー。
    pub(crate) registry_keys: Vec<String>,
    /// 書き込まれた値。キーは `key|subkey|name`。
    pub(crate) registry_values: RefCell<BTreeMap<String, String>>,
    pub(crate) tray_actions: RefCell<VecDeque<TrayAction>>,
    /// `None` なら見張れない環境。`Some` なら取り出すたびに `Some(false)` へ戻る。
    pub(crate) window_changes: Cell<Option<bool>>,
    /// 画面の右端を確保できたときに返す範囲。`None` なら確保に失敗する。
    pub(crate) edge: Option<egui::Rect>,
    /// `reserve_right_edge` に渡された幅。
    pub(crate) reservations: RefCell<Vec<Option<f32>>>,
}

impl Default for FakePlatform {
    fn default() -> Self {
        Self {
            calls: RefCell::default(),
            open_succeeds: true,
            windows: RefCell::default(),
            foreground: 0,
            minimized: Vec::new(),
            icons: false,
            cursor: Some(egui::pos2(1880.0, 300.0)),
            dock: Some(egui::Rect::from_min_max(
                egui::pos2(1854.0, 12.0),
                egui::pos2(1908.0, 812.0),
            )),
            screen_width: 1920.0,
            work_area: None,
            time: LocalTime {
                month: 9,
                day: 3,
                weekday: 3,
                hour: 7,
                minute: 5,
                second: 0,
            },
            chosen_file: None,
            install_directory: Some(std::path::PathBuf::from(r"C:\Dock")),
            registry_keys: Vec::new(),
            registry_values: RefCell::default(),
            tray_actions: RefCell::default(),
            window_changes: Cell::new(None),
            edge: None,
            reservations: RefCell::default(),
        }
    }
}

impl FakePlatform {
    fn record(&self, call: String) {
        self.calls.borrow_mut().push(call);
    }

    pub(crate) fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

impl Platform for FakePlatform {
    fn open_target(&self, target: &str) -> bool {
        self.record(format!("open {target}"));
        self.open_succeeds
    }
    fn visible_windows(&self) -> Vec<(isize, String, String)> {
        self.windows.borrow().clone()
    }
    fn foreground_window(&self) -> isize {
        self.foreground
    }
    fn is_minimized(&self, window: isize) -> bool {
        self.minimized.contains(&window)
    }
    fn minimize(&self, window: isize) {
        self.record(format!("minimize {window}"));
    }
    fn restore(&self, window: isize) {
        self.record(format!("restore {window}"));
    }
    fn set_foreground(&self, window: isize) {
        self.record(format!("foreground {window}"));
    }
    fn close_window(&self, window: isize) {
        self.record(format!("close {window}"));
    }
    fn load_icon(&self, _path: &str) -> Option<egui::ColorImage> {
        self.icons
            .then(|| egui::ColorImage::filled([2, 2], egui::Color32::RED))
    }
    fn cursor_position(&self) -> Option<egui::Pos2> {
        self.cursor
    }
    fn dock_rect(&self) -> Option<egui::Rect> {
        self.dock
    }
    fn screen_width(&self) -> f32 {
        self.screen_width
    }
    fn work_area(&self) -> Option<egui::Rect> {
        self.work_area
    }
    fn local_time(&self) -> LocalTime {
        self.time
    }
    fn choose_executable(&self) -> Option<String> {
        self.record("choose".into());
        self.chosen_file.clone()
    }
    fn install_directory(&self) -> Option<std::path::PathBuf> {
        self.install_directory.clone()
    }
    fn registry_key_exists(&self, key: &str) -> bool {
        self.registry_keys.iter().any(|existing| existing == key)
    }
    fn take_tray_action(&self) -> Option<TrayAction> {
        self.tray_actions.borrow_mut().pop_front()
    }
    fn reserve_right_edge(&self, width: Option<f32>) -> Option<egui::Rect> {
        self.reservations.borrow_mut().push(width);
        self.edge
    }
    fn take_window_changes(&self) -> Option<bool> {
        let changes = self.window_changes.get();
        if changes.is_some() {
            self.window_changes.set(Some(false));
        }
        changes
    }
    fn set_registry_string(&self, key: &str, subkey: &str, name: &str, value: &str) {
        self.registry_values
            .borrow_mut()
            .insert(format!("{key}|{subkey}|{name}"), value.to_owned());
    }
}
