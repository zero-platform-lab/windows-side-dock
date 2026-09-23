use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PopupDirection {
    Auto,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ProcessTool {
    TaskManager,
    ProcessExplorer,
}

/// 旧アプリ名との互換性のため、保存先フォルダーは `lancher` のまま使う。
fn config_file_in(local_app_data: &Path, name: &str) -> PathBuf {
    local_app_data.join("lancher").join(name)
}

fn legacy_config_file(name: &str) -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| config_file_in(Path::new(&root), name))
}

fn read_setting(path: Option<PathBuf>) -> Option<String> {
    path.and_then(|path| std::fs::read_to_string(path).ok())
}

fn write_setting(path: Option<PathBuf>, value: &str) {
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, value);
}

fn config_path() -> Option<PathBuf> {
    legacy_config_file("items.txt")
}

/// `items.txt` の各行 `名前|コマンド` を読み取る。区切りのない行は無視する。
fn parse_registered_items(contents: &str) -> Vec<(String, String)> {
    contents
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(name, command)| (name.to_owned(), command.to_owned()))
        .collect()
}

fn format_registered_items<'a>(items: impl Iterator<Item = (&'a str, &'a str)>) -> String {
    items
        .map(|(name, command)| format!("{}|{}", name.replace('|', " "), command.replace('|', " ")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn load_registered_items() -> Vec<(String, String)> {
    read_setting(config_path()).map_or_else(Vec::new, |contents| parse_registered_items(&contents))
}

pub(crate) fn save_registered_items<'a>(items: impl Iterator<Item = (&'a str, &'a str)>) {
    write_setting(config_path(), &format_registered_items(items));
}

fn settings_path() -> Option<PathBuf> {
    legacy_config_file("settings.txt")
}

fn parse_popup_direction(value: &str) -> PopupDirection {
    match value.trim() {
        "left" => PopupDirection::Left,
        "right" => PopupDirection::Right,
        _ => PopupDirection::Auto,
    }
}

fn popup_direction_value(direction: PopupDirection) -> &'static str {
    match direction {
        PopupDirection::Auto => "auto",
        PopupDirection::Left => "left",
        PopupDirection::Right => "right",
    }
}

pub(crate) fn load_popup_direction() -> PopupDirection {
    read_setting(settings_path())
        .map_or(PopupDirection::Auto, |value| parse_popup_direction(&value))
}

pub(crate) fn save_popup_direction(direction: PopupDirection) {
    write_setting(settings_path(), popup_direction_value(direction));
}

fn process_tool_path() -> Option<PathBuf> {
    legacy_config_file("process_tool.txt")
}

fn process_explorer_path_file() -> Option<PathBuf> {
    legacy_config_file("process_explorer_path.txt")
}

fn parse_process_tool(value: &str) -> ProcessTool {
    match value.trim() {
        "process_explorer" => ProcessTool::ProcessExplorer,
        _ => ProcessTool::TaskManager,
    }
}

fn process_tool_value(tool: ProcessTool) -> &'static str {
    match tool {
        ProcessTool::TaskManager => "task_manager",
        ProcessTool::ProcessExplorer => "process_explorer",
    }
}

pub(crate) fn load_process_tool() -> ProcessTool {
    read_setting(process_tool_path())
        .map_or(ProcessTool::TaskManager, |value| parse_process_tool(&value))
}

pub(crate) fn save_process_tool(tool: ProcessTool) {
    write_setting(process_tool_path(), process_tool_value(tool));
}

pub(crate) fn load_process_explorer_path() -> String {
    read_setting(process_explorer_path_file())
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

pub(crate) fn save_process_explorer_path(value: &str) {
    write_setting(process_explorer_path_file(), value.trim());
}

#[cfg(windows)]
pub(crate) fn choose_process_explorer_file() -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

    let owner_title: Vec<u16> = std::ffi::OsStr::new("Windows Side Dock 設定")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let dialog_title: Vec<u16> = std::ffi::OsStr::new("Process Explorerを選択")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let filter: Vec<u16> = "実行ファイル (*.exe)\0*.exe\0すべてのファイル\0*.*\0\0"
        .encode_utf16()
        .collect();
    let mut file_buffer = vec![0_u16; 32768];
    let mut options = OPENFILENAMEW::default();
    options.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    options.hwndOwner = unsafe { FindWindowW(std::ptr::null(), owner_title.as_ptr()) };
    options.lpstrFilter = filter.as_ptr();
    options.lpstrFile = file_buffer.as_mut_ptr();
    options.nMaxFile = file_buffer.len() as u32;
    options.lpstrTitle = dialog_title.as_ptr();
    options.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST;

    if unsafe { GetOpenFileNameW(&mut options) } == 0 {
        return None;
    }
    let length = file_buffer.iter().position(|&value| value == 0)?;
    Some(String::from_utf16_lossy(&file_buffer[..length]))
}

#[cfg(not(windows))]
pub(crate) fn choose_process_explorer_file() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_popup_direction_with_safe_default() {
        assert_eq!(parse_popup_direction("left\n"), PopupDirection::Left);
        assert_eq!(parse_popup_direction("right"), PopupDirection::Right);
        assert_eq!(parse_popup_direction("unexpected"), PopupDirection::Auto);
    }

    #[test]
    fn parses_process_tool_with_safe_default() {
        assert_eq!(
            parse_process_tool("process_explorer\n"),
            ProcessTool::ProcessExplorer
        );
        assert_eq!(parse_process_tool("task_manager"), ProcessTool::TaskManager);
        assert_eq!(parse_process_tool("unknown"), ProcessTool::TaskManager);
    }

    #[test]
    fn stored_values_round_trip() {
        for direction in [
            PopupDirection::Auto,
            PopupDirection::Left,
            PopupDirection::Right,
        ] {
            assert_eq!(
                parse_popup_direction(popup_direction_value(direction)),
                direction
            );
        }
        for tool in [ProcessTool::TaskManager, ProcessTool::ProcessExplorer] {
            assert_eq!(parse_process_tool(process_tool_value(tool)), tool);
        }
    }

    #[test]
    fn legacy_paths_remain_compatible() {
        let root = Path::new(r"C:\Users\me\AppData\Local");
        assert_eq!(
            config_file_in(root, "items.txt"),
            root.join("lancher").join("items.txt")
        );
    }

    #[test]
    fn parses_registered_items_and_skips_malformed_lines() {
        let items =
            parse_registered_items("Code|C:\\Apps\\Code.exe\r\nbroken line\nURL|https://a/b|c\n");
        assert_eq!(
            items,
            [
                ("Code".to_owned(), r"C:\Apps\Code.exe".to_owned()),
                ("URL".to_owned(), "https://a/b|c".to_owned()),
            ]
        );
    }

    #[test]
    fn registered_items_round_trip_and_replace_separators() {
        let text =
            format_registered_items([("A|B", r"C:\a.exe"), ("メモ帳", r"C:\x|y.exe")].into_iter());
        assert_eq!(text, "A B|C:\\a.exe\nメモ帳|C:\\x y.exe");
        assert_eq!(
            parse_registered_items(&text),
            [
                ("A B".to_owned(), r"C:\a.exe".to_owned()),
                ("メモ帳".to_owned(), r"C:\x y.exe".to_owned()),
            ]
        );
    }

    #[test]
    fn writes_settings_into_missing_folders() {
        let root = std::env::temp_dir().join(format!("wsd-config-test-{}", std::process::id()));
        let path = config_file_in(&root, "settings.txt");
        write_setting(Some(path.clone()), "left");
        assert_eq!(read_setting(Some(path)).as_deref(), Some("left"));
        assert_eq!(
            read_setting(Some(config_file_in(&root, "missing.txt"))),
            None
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
