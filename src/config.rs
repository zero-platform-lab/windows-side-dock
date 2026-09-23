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

const CONFIG_DIR: &str = "windows-side-dock";
/// アプリ名変更前（0.1.5以前）の保存先。移行元としてだけ読む。
const LEGACY_CONFIG_DIR: &str = "lancher";
const CONFIG_FILES: [&str; 4] = [
    "items.txt",
    "settings.txt",
    "process_tool.txt",
    "process_explorer_path.txt",
];

fn config_file_in(local_app_data: &Path, name: &str) -> PathBuf {
    local_app_data.join(CONFIG_DIR).join(name)
}

fn config_file(name: &str) -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| config_file_in(Path::new(&root), name))
}

/// 旧フォルダーの設定を新フォルダーへコピーする。新フォルダーに既にあるファイルは上書きしない。
/// 旧フォルダーは戻せるように残し、失敗したファイルは次回起動時に再試行される。
fn migrate_legacy_config_in(local_app_data: &Path) {
    for name in CONFIG_FILES {
        let target = config_file_in(local_app_data, name);
        let source = local_app_data.join(LEGACY_CONFIG_DIR).join(name);
        if target.exists() || !source.is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::copy(&source, &target);
    }
}

pub(crate) fn migrate_legacy_config() {
    if let Some(root) = std::env::var_os("LOCALAPPDATA") {
        migrate_legacy_config_in(Path::new(&root));
    }
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
    config_file("items.txt")
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
    config_file("settings.txt")
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
    config_file("process_tool.txt")
}

fn process_explorer_path_file() -> Option<PathBuf> {
    config_file("process_explorer_path.txt")
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
    fn stores_settings_under_application_folder() {
        let root = Path::new(r"C:\Users\me\AppData\Local");
        assert_eq!(
            config_file_in(root, "items.txt"),
            root.join("windows-side-dock").join("items.txt")
        );
    }

    fn temp_root(test: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("wsd-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn copies_legacy_settings_and_keeps_originals() {
        let root = temp_root("migrate-copy");
        let legacy = root.join("lancher");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("items.txt"), "Code|C:\\Code.exe").unwrap();
        std::fs::write(legacy.join("process_tool.txt"), "process_explorer").unwrap();
        std::fs::write(legacy.join("unrelated.txt"), "x").unwrap();

        migrate_legacy_config_in(&root);

        let read = |name| std::fs::read_to_string(config_file_in(&root, name)).ok();
        assert_eq!(read("items.txt").as_deref(), Some("Code|C:\\Code.exe"));
        assert_eq!(
            read("process_tool.txt").as_deref(),
            Some("process_explorer")
        );
        assert_eq!(read("settings.txt"), None);
        assert_eq!(read("unrelated.txt"), None);
        assert!(legacy.join("items.txt").is_file());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migration_never_overwrites_new_settings() {
        let root = temp_root("migrate-keep");
        std::fs::create_dir_all(root.join("lancher")).unwrap();
        std::fs::write(root.join("lancher").join("settings.txt"), "left").unwrap();
        write_setting(Some(config_file_in(&root, "settings.txt")), "right");

        migrate_legacy_config_in(&root);
        migrate_legacy_config_in(&root);

        assert_eq!(
            read_setting(Some(config_file_in(&root, "settings.txt"))).as_deref(),
            Some("right")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migration_without_legacy_folder_creates_nothing() {
        let root = temp_root("migrate-none");
        std::fs::create_dir_all(&root).unwrap();
        migrate_legacy_config_in(&root);
        assert!(!root.join("windows-side-dock").exists());
        let _ = std::fs::remove_dir_all(root);
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
        let root = temp_root("write-setting");
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
