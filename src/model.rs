use std::path::Path;

#[derive(Clone, Copy)]
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
        assert_eq!(titles, ["a.rs - Visual Studio Code", "b.rs - Visual Studio Code"]);
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

    #[test]
    fn names_group_from_executable_when_title_is_blank() {
        let groups = group_windows(vec![window(1, r"C:\Tools\procexp64.exe", " ")], 0);
        assert_eq!(groups[0].name, "procexp64");
    }
}
