use crate::config::DockSide;
use crate::model::{group_windows, IconKind, LauncherItem, RunningWindow};
use eframe::egui;
use std::collections::HashMap;

/// Windowsのローカル時刻のうち、Dockの時計に使う値。`weekday` は日曜を0とする。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocalTime {
    pub(crate) month: u16,
    pub(crate) day: u16,
    pub(crate) weekday: u16,
    pub(crate) hour: u16,
    pub(crate) minute: u16,
    pub(crate) second: u16,
}

/// タスクトレイのメニューとアイコンのクリックから届く操作。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TrayAction {
    /// Dockをしまう・引き出す。
    ToggleCollapsed,
    /// Dockを指定した画面の端へ移す。
    MoveTo(DockSide),
    OpenSettings,
    LaunchProcessTool,
    Quit,
}

/// 実行ファイルやショートカットに対する、エクスプローラーの右クリックメニューと同じ操作。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FileAction {
    RunAsAdmin,
    OpenLocation,
    Properties,
}

/// OSへの問い合わせと操作。実装は `win32.rs`、テストでは記録用の偽物を使う。
/// 判断を伴う処理はここへ置かず、呼び出し側の関数でテストする。
pub(crate) trait Platform {
    fn open_target(&self, target: &str) -> bool;
    fn file_action(&self, action: FileAction, path: &str) -> bool;
    /// 表示中でタイトルを持つ他プロセスのウィンドウ `(ハンドル, 実行ファイル, タイトル)`。
    fn visible_windows(&self) -> Vec<(isize, String, String)>;
    fn foreground_window(&self) -> isize;
    fn is_minimized(&self, window: isize) -> bool;
    fn minimize(&self, window: isize);
    fn restore(&self, window: isize);
    fn set_foreground(&self, window: isize);
    fn close_window(&self, window: isize);
    fn load_icon(&self, path: &str) -> Option<egui::ColorImage>;
    fn cursor_position(&self) -> Option<egui::Pos2>;
    /// Dock本体のウィンドウの画面座標。
    fn dock_rect(&self) -> Option<egui::Rect>;
    fn work_area(&self) -> Option<egui::Rect>;
    fn local_time(&self) -> LocalTime;
    fn choose_executable(&self) -> Option<String>;
    /// Dockの実行ファイルがあるフォルダー。
    fn install_directory(&self) -> Option<std::path::PathBuf>;
    /// `HKEY_CURRENT_USER` 配下のキーが存在するか。
    fn registry_key_exists(&self, key: &str) -> bool;
    /// `HKEY_CURRENT_USER\key\subkey` の文字列値を書き込む。キーがなければ作られる。
    fn set_registry_string(&self, key: &str, subkey: &str, name: &str, value: &str);
    /// タスクトレイから届いた操作を古い順に1つ取り出す。
    fn take_tray_action(&self) -> Option<TrayAction>;
    /// ほかのアプリのウィンドウの変化を見張れていれば、前回の呼び出しから変化があったか。
    /// 見張れていなければ `None` で、呼び出し側が定期的に確認する。
    fn take_window_changes(&self) -> Option<bool>;
    /// Dockのウィンドウがあるモニターの `side` の端を `width` 物理ピクセルだけ確保し、
    /// Dockを置ける範囲を返す。`width` に `None` を渡すと確保をやめる。確保できなければ `None`。
    fn reserve_edge(&self, side: DockSide, width: Option<f32>) -> Option<egui::Rect>;
}

/// 実行ファイルのパスごとに取り出したアイコン。取り出せなかったことも覚えておく。
pub(crate) type IconCache = HashMap<String, Option<egui::ColorImage>>;

/// 実行中のアプリの一覧。アイコンの取り出しは重いため、一度取り出したものは `icons` から使う。
pub(crate) fn running_apps(platform: &dyn Platform, icons: &mut IconCache) -> Vec<LauncherItem> {
    group_windows(platform.visible_windows(), platform.foreground_window())
        .into_iter()
        .map(|group| LauncherItem {
            icon: icons
                .entry(group.command.clone())
                .or_insert_with(|| platform.load_icon(&group.command))
                .clone(),
            name: group.name,
            command: group.command,
            fallback_icon: IconKind::File,
            windows: group.windows,
            active: group.active,
        })
        .collect()
}

/// タスクバーのボタンと同じ動作。前面のウィンドウなら最小化し、それ以外は前面へ出す。
/// 通常表示中のウィンドウを復元するとちらつくため、最小化中のときだけ復元する。
pub(crate) fn activate_windows(platform: &dyn Platform, windows: &[RunningWindow]) {
    let Some(window) = windows.first() else {
        return;
    };
    if platform.foreground_window() == window.handle {
        platform.minimize(window.handle);
        return;
    }
    if platform.is_minimized(window.handle) {
        platform.restore(window.handle);
    }
    platform.set_foreground(window.handle);
}

pub(crate) fn close_windows(platform: &dyn Platform, windows: &[RunningWindow]) {
    for window in windows {
        platform.close_window(window.handle);
    }
}

/// OSを持たない環境向けの何もしない実装。Windows以外でのビルドと起動だけを支える。
#[cfg(not(windows))]
pub(crate) struct NullPlatform;

#[cfg(not(windows))]
impl Platform for NullPlatform {
    fn open_target(&self, _target: &str) -> bool {
        false
    }
    fn file_action(&self, _action: FileAction, _path: &str) -> bool {
        false
    }
    fn visible_windows(&self) -> Vec<(isize, String, String)> {
        Vec::new()
    }
    fn foreground_window(&self) -> isize {
        0
    }
    fn is_minimized(&self, _window: isize) -> bool {
        false
    }
    fn minimize(&self, _window: isize) {}
    fn restore(&self, _window: isize) {}
    fn set_foreground(&self, _window: isize) {}
    fn close_window(&self, _window: isize) {}
    fn load_icon(&self, _path: &str) -> Option<egui::ColorImage> {
        None
    }
    fn cursor_position(&self) -> Option<egui::Pos2> {
        None
    }
    fn dock_rect(&self) -> Option<egui::Rect> {
        None
    }
    fn work_area(&self) -> Option<egui::Rect> {
        None
    }
    fn local_time(&self) -> LocalTime {
        LocalTime {
            month: 1,
            day: 1,
            weekday: 0,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
    fn choose_executable(&self) -> Option<String> {
        None
    }
    fn install_directory(&self) -> Option<std::path::PathBuf> {
        None
    }
    fn registry_key_exists(&self, _key: &str) -> bool {
        false
    }
    fn set_registry_string(&self, _key: &str, _subkey: &str, _name: &str, _value: &str) {}
    fn take_tray_action(&self) -> Option<TrayAction> {
        None
    }
    fn take_window_changes(&self) -> Option<bool> {
        None
    }
    fn reserve_edge(&self, _side: DockSide, _width: Option<f32>) -> Option<egui::Rect> {
        None
    }
}

#[cfg(test)]
#[path = "platform_fake.rs"]
pub(crate) mod fake;

#[cfg(test)]
mod tests {
    use super::fake::FakePlatform;
    use super::*;

    fn window(handle: isize) -> RunningWindow {
        RunningWindow {
            handle,
            title: format!("window {handle}"),
        }
    }

    #[test]
    fn builds_running_apps_with_icons() {
        let platform = FakePlatform {
            icons: true,
            foreground: 2,
            ..FakePlatform::default()
        };
        platform.windows.replace(vec![
            (
                1,
                r"C:\Apps\Code.exe".into(),
                "a - Visual Studio Code".into(),
            ),
            (
                2,
                r"C:\Apps\Code.exe".into(),
                "b - Visual Studio Code".into(),
            ),
        ]);
        let mut icons = IconCache::new();
        let apps = running_apps(&platform, &mut icons);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Visual Studio Code");
        assert!(apps[0].icon.is_some());
        assert!(apps[0].active);
        assert_eq!(apps[0].windows.len(), 2);
        assert!(icons[r"C:\Apps\Code.exe"].is_some());
    }

    #[test]
    fn reuses_icons_already_loaded() {
        let platform = FakePlatform::default();
        platform
            .windows
            .replace(vec![(1, r"C:\Apps\Code.exe".into(), "Code".into())]);
        let cached = egui::ColorImage::filled([1, 1], egui::Color32::BLUE);
        let mut icons = IconCache::from([(r"C:\Apps\Code.exe".to_owned(), Some(cached.clone()))]);
        let apps = running_apps(&platform, &mut icons);
        assert_eq!(apps[0].icon, Some(cached));
    }

    #[test]
    fn minimizes_the_foreground_window() {
        let platform = FakePlatform {
            foreground: 7,
            ..FakePlatform::default()
        };
        activate_windows(&platform, &[window(7), window(8)]);
        assert_eq!(platform.calls(), ["minimize 7"]);
    }

    #[test]
    fn restores_only_minimized_windows_before_focusing() {
        let platform = FakePlatform {
            minimized: vec![5],
            ..FakePlatform::default()
        };
        activate_windows(&platform, &[window(5)]);
        activate_windows(&platform, &[window(6)]);
        assert_eq!(
            platform.calls(),
            ["restore 5", "foreground 5", "foreground 6"]
        );
    }

    #[test]
    fn ignores_empty_window_list() {
        let platform = FakePlatform::default();
        activate_windows(&platform, &[]);
        close_windows(&platform, &[]);
        assert!(platform.calls().is_empty());
    }

    #[test]
    fn closes_every_window() {
        let platform = FakePlatform::default();
        close_windows(&platform, &[window(1), window(2)]);
        assert_eq!(platform.calls(), ["close 1", "close 2"]);
    }
}
