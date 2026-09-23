use crate::model::{group_windows, IconKind, LauncherItem, RunningWindow};
use eframe::egui;

/// Windowsのローカル時刻のうち、Dockの時計に使う値。`weekday` は日曜を0とする。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LocalTime {
    pub(crate) month: u16,
    pub(crate) day: u16,
    pub(crate) weekday: u16,
    pub(crate) hour: u16,
    pub(crate) minute: u16,
}

/// OSへの問い合わせと操作。実装は `win32.rs`、テストでは記録用の偽物を使う。
/// 判断を伴う処理はここへ置かず、呼び出し側の関数でテストする。
pub(crate) trait Platform {
    fn open_target(&self, target: &str) -> bool;
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
    fn screen_width(&self) -> f32;
    fn work_area(&self) -> Option<egui::Rect>;
    fn local_time(&self) -> LocalTime;
    fn choose_executable(&self) -> Option<String>;
    /// Dockの実行ファイルがあるフォルダー。
    fn install_directory(&self) -> Option<std::path::PathBuf>;
    /// `HKEY_CURRENT_USER` 配下のキーが存在するか。
    fn registry_key_exists(&self, key: &str) -> bool;
    /// `HKEY_CURRENT_USER\key\subkey` の文字列値を書き込む。キーがなければ作られる。
    fn set_registry_string(&self, key: &str, subkey: &str, name: &str, value: &str);
}

pub(crate) fn running_apps(platform: &dyn Platform) -> Vec<LauncherItem> {
    group_windows(platform.visible_windows(), platform.foreground_window())
        .into_iter()
        .map(|group| LauncherItem {
            icon: platform.load_icon(&group.command),
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
    fn screen_width(&self) -> f32 {
        0.0
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
        let apps = running_apps(&platform);
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Visual Studio Code");
        assert!(apps[0].icon.is_some());
        assert!(apps[0].active);
        assert_eq!(apps[0].windows.len(), 2);
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
