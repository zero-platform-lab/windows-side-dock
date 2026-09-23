use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum IconKind {
    Folder,
    Terminal,
    Note,
    Settings,
    File,
}

#[derive(Clone)]
pub(crate) struct RunningWindow {
    pub(crate) handle: isize,
    pub(crate) title: String,
}

#[derive(Clone)]
pub(crate) struct LauncherItem {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) fallback_icon: IconKind,
    pub(crate) icon: Option<eframe::egui::ColorImage>,
    pub(crate) windows: Vec<RunningWindow>,
    pub(crate) active: bool,
}

fn normalized_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn same_application(pinned: &LauncherItem, running: &LauncherItem) -> bool {
    if pinned.command.eq_ignore_ascii_case(&running.command) {
        return true;
    }
    if pinned.command.eq_ignore_ascii_case("ms-settings:")
        && Path::new(&running.command)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("SystemSettings.exe"))
    {
        return true;
    }
    let pinned_name = normalized_name(&pinned.name);
    let running_name = normalized_name(&running.name);
    running_name.chars().count() >= 4 && pinned_name.contains(&running_name)
}

pub(crate) fn friendly_window_name(title: &str, fallback: &str) -> String {
    title
        .rsplit_once(" - ")
        .map(|(_, application)| application.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            let trimmed = title.trim();
            if trimmed.is_empty() {
                fallback
            } else {
                trimmed
            }
        })
        .to_owned()
}

/// コマンドに合う代わりのアイコンと、アイコンを取り出すファイル。
/// Windows 設定はURIなので、設定アプリの実行ファイルからアイコンを取り出す。
pub(crate) fn fallback_icon(command: &str, windows_dir: &str) -> (IconKind, String) {
    let lower = command.to_ascii_lowercase();
    if lower == "ms-settings:" {
        let source = format!(r"{windows_dir}\ImmersiveControlPanel\SystemSettings.exe");
        return (IconKind::Settings, source);
    }
    let kind = if lower.ends_with(r"\explorer.exe") {
        IconKind::Folder
    } else if lower.ends_with(r"\wt.exe") {
        IconKind::Terminal
    } else if lower.ends_with(r"\notepad.exe") {
        IconKind::Note
    } else {
        IconKind::File
    };
    (kind, command.to_owned())
}

/// ピン留めの `(名前, コマンド)` を並び順どおりに返す。
pub(crate) fn pinned_entries(items: &[LauncherItem]) -> impl Iterator<Item = (&str, &str)> {
    items
        .iter()
        .map(|item| (item.name.as_str(), item.command.as_str()))
}

/// ドロップされたファイルの表示名。拡張子を除いたファイル名を使う。
pub(crate) fn item_name_for_path(path: &Path) -> &str {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("アプリ")
}

/// 実行中アプリをピン留め項目へ割り当て、どのピン留め項目にも該当しないものを返す。
pub(crate) fn assign_running(
    items: &mut [LauncherItem],
    discovered: Vec<LauncherItem>,
) -> Vec<LauncherItem> {
    for pinned in items.iter_mut() {
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
    discovered
        .into_iter()
        .filter(|running| !items.iter().any(|pinned| same_application(pinned, running)))
        .collect()
}

pub(crate) struct WindowGroup {
    pub(crate) name: String,
    pub(crate) command: String,
    pub(crate) windows: Vec<RunningWindow>,
    pub(crate) active: bool,
}

/// 列挙順を保ったまま、実行ファイルのパスごとにウィンドウをまとめる。
pub(crate) fn group_windows(
    windows: Vec<(isize, String, String)>,
    foreground: isize,
) -> Vec<WindowGroup> {
    let mut groups: Vec<WindowGroup> = Vec::new();
    for (handle, command, title) in windows {
        let active = handle == foreground;
        if let Some(existing) = groups
            .iter_mut()
            .find(|group| group.command.eq_ignore_ascii_case(&command))
        {
            existing.windows.push(RunningWindow { handle, title });
            existing.active |= active;
        } else {
            let executable_name = Path::new(&command)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("アプリ")
                .to_owned();
            groups.push(WindowGroup {
                name: friendly_window_name(&title, &executable_name),
                command,
                windows: vec![RunningWindow { handle, title }],
                active,
            });
        }
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, command: &str) -> LauncherItem {
        LauncherItem {
            name: name.into(),
            command: command.into(),
            fallback_icon: IconKind::File,
            icon: None,
            windows: Vec::new(),
            active: false,
        }
    }

    #[test]
    fn matches_commands_without_case_sensitivity() {
        let pinned = item("VS Code", r"C:\Apps\Code.exe");
        let running = item("Code", r"c:\apps\CODE.EXE");
        assert!(same_application(&pinned, &running));
    }

    #[test]
    fn matches_windows_settings_protocol_to_system_settings() {
        let pinned = item("Windows 設定", "ms-settings:");
        let running = item(
            "Settings",
            r"C:\Windows\ImmersiveControlPanel\SystemSettings.exe",
        );
        assert!(same_application(&pinned, &running));
    }

    #[test]
    fn matches_registered_name_after_normalization() {
        let pinned = item("Visual Studio Code", r"C:\Portable\launcher.exe");
        let running = item("Studio-Code", r"C:\Apps\Code.exe");
        assert!(same_application(&pinned, &running));
    }

    #[test]
    fn rejects_short_or_unrelated_names() {
        let pinned = item("メモ帳", r"C:\Apps\memo.exe");
        let running = item("メモ", r"C:\Other\memo.exe");
        assert!(!same_application(&pinned, &running));
    }

    #[test]
    fn derives_application_name_from_window_title() {
        assert_eq!(
            friendly_window_name("notes.txt - Visual Studio Code", "Code"),
            "Visual Studio Code"
        );
    }

    #[test]
    fn falls_back_for_an_empty_window_title() {
        assert_eq!(friendly_window_name("   ", "notepad"), "notepad");
    }

    #[test]
    fn uses_whole_title_when_it_has_no_application_suffix() {
        assert_eq!(friendly_window_name("  電卓  ", "calc"), "電卓");
        assert_eq!(friendly_window_name("draft - ", "notepad"), "draft -");
    }

    fn window(handle: isize, command: &str, title: &str) -> (isize, String, String) {
        (handle, command.into(), title.into())
    }

    #[test]
    fn groups_windows_by_executable_ignoring_case() {
        let groups = group_windows(
            vec![
                window(1, r"C:\Apps\Code.exe", "a.rs - Visual Studio Code"),
                window(2, r"C:\Windows\explorer.exe", "Downloads"),
                window(3, r"c:\apps\CODE.EXE", "b.rs - Visual Studio Code"),
            ],
            0,
        );
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Visual Studio Code");
        assert_eq!(groups[0].command, r"C:\Apps\Code.exe");
        let titles: Vec<_> = groups[0].windows.iter().map(|w| w.title.as_str()).collect();
        assert_eq!(
            titles,
            ["a.rs - Visual Studio Code", "b.rs - Visual Studio Code"]
        );
        assert_eq!(groups[1].name, "Downloads");
    }

    #[test]
    fn marks_group_active_when_any_window_is_foreground() {
        let groups = group_windows(
            vec![
                window(1, r"C:\Apps\Code.exe", "a"),
                window(2, r"C:\Apps\Code.exe", "b"),
                window(3, r"C:\Apps\Other.exe", "c"),
            ],
            2,
        );
        assert!(groups[0].active);
        assert!(!groups[1].active);
    }

    fn running(name: &str, command: &str, handles: &[isize], active: bool) -> LauncherItem {
        LauncherItem {
            windows: handles
                .iter()
                .map(|&handle| RunningWindow {
                    handle,
                    title: name.into(),
                })
                .collect(),
            active,
            ..item(name, command)
        }
    }

    #[test]
    fn pinned_entries_keep_every_item_in_order() {
        let items = [
            item("エクスプローラー", r"C:\Windows\explorer.exe"),
            item("Windows 設定", "ms-settings:"),
            item("Code", r"C:\Apps\Code.exe"),
        ];
        let entries: Vec<_> = pinned_entries(&items).collect();
        assert_eq!(
            entries,
            [
                ("エクスプローラー", r"C:\Windows\explorer.exe"),
                ("Windows 設定", "ms-settings:"),
                ("Code", r"C:\Apps\Code.exe"),
            ]
        );
    }

    #[test]
    fn picks_fallback_icons_for_well_known_commands() {
        let windows = r"C:\Windows";
        assert_eq!(
            fallback_icon("ms-settings:", windows),
            (
                IconKind::Settings,
                r"C:\Windows\ImmersiveControlPanel\SystemSettings.exe".to_owned()
            )
        );
        for (command, kind) in [
            (r"C:\Windows\EXPLORER.EXE", IconKind::Folder),
            (r"C:\Local\Microsoft\WindowsApps\wt.exe", IconKind::Terminal),
            (r"C:\Windows\System32\notepad.exe", IconKind::Note),
            (r"C:\Apps\Code.exe", IconKind::File),
        ] {
            assert_eq!(fallback_icon(command, windows), (kind, command.to_owned()));
        }
    }

    #[test]
    fn names_dropped_file_without_extension() {
        assert_eq!(
            item_name_for_path(Path::new(r"C:\Users\me\Desktop\Steam.lnk")),
            "Steam"
        );
        assert_eq!(item_name_for_path(Path::new(r"C:\")), "アプリ");
    }

    #[test]
    fn assigns_running_windows_to_pinned_items() {
        let mut items = [
            item("Code", r"C:\Apps\Code.exe"),
            item("Steam", r"C:\Steam\steam.exe"),
        ];
        items[1].windows = vec![RunningWindow {
            handle: 99,
            title: "stale".into(),
        }];
        items[1].active = true;
        let unpinned = assign_running(
            &mut items,
            vec![
                running("Code", r"c:\apps\code.exe", &[1, 2], true),
                running("Chrome", r"C:\Chrome\chrome.exe", &[3], false),
            ],
        );

        let handles: Vec<_> = items[0].windows.iter().map(|w| w.handle).collect();
        assert_eq!(handles, [1, 2]);
        assert!(items[0].active);
        assert!(items[1].windows.is_empty());
        assert!(!items[1].active);
        assert_eq!(unpinned.len(), 1);
        assert_eq!(unpinned[0].name, "Chrome");
    }

    #[test]
    fn names_group_from_executable_when_title_is_blank() {
        let groups = group_windows(vec![window(1, r"C:\Tools\procexp64.exe", " ")], 0);
        assert_eq!(groups[0].name, "procexp64");
    }
}
