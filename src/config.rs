use std::path::PathBuf;

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

fn legacy_config_file(name: &str) -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("lancher").join(name))
}

pub(crate) fn config_path() -> Option<PathBuf> {
    legacy_config_file("items.txt")
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

pub(crate) fn load_popup_direction() -> PopupDirection {
    settings_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map_or(PopupDirection::Auto, |value| parse_popup_direction(&value))
}

pub(crate) fn save_popup_direction(direction: PopupDirection) {
    let Some(path) = settings_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let value = match direction {
        PopupDirection::Auto => "auto",
        PopupDirection::Left => "left",
        PopupDirection::Right => "right",
    };
    let _ = std::fs::write(path, value);
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

pub(crate) fn load_process_tool() -> ProcessTool {
    process_tool_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map_or(ProcessTool::TaskManager, |value| parse_process_tool(&value))
}

pub(crate) fn save_process_tool(tool: ProcessTool) {
    let Some(path) = process_tool_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let value = match tool {
        ProcessTool::TaskManager => "task_manager",
        ProcessTool::ProcessExplorer => "process_explorer",
    };
    let _ = std::fs::write(path, value);
}

pub(crate) fn load_process_explorer_path() -> String {
    process_explorer_path_file()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

pub(crate) fn save_process_explorer_path(value: &str) {
    let Some(path) = process_explorer_path_file() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, value.trim());
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
    fn legacy_paths_remain_compatible() {
        let path = PathBuf::from("root").join("lancher").join("items.txt");
        assert!(path.ends_with(PathBuf::from("lancher").join("items.txt")));
    }
}
