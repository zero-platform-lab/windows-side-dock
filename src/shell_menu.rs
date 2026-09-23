use crate::config::ProcessTool;
use crate::ui::normalized_executable_path;

/// デスクトップ／フォルダー背景の「Windows Side Dock」サブメニュー（MSIが作成する）。
#[cfg_attr(not(windows), allow(dead_code))]
const MENU_ROOTS: [&str; 2] = [
    r"Software\Classes\DesktopBackground\Shell\WindowsSideDock\shell\02ProcessTool",
    r"Software\Classes\Directory\Background\Shell\WindowsSideDock\shell\02ProcessTool",
];

/// 背景メニューのプロセスツール項目に表示する名前と実行コマンド。
/// Process Explorerのパスが未設定ならタスク マネージャーに戻す。
fn process_tool_entry(tool: ProcessTool, process_explorer_path: &str) -> (&'static str, String) {
    let path = normalized_executable_path(process_explorer_path);
    match tool {
        ProcessTool::ProcessExplorer if !path.is_empty() => {
            ("Process Explorer", format!("\"{path}\""))
        }
        _ => ("タスク マネージャー", "taskmgr.exe".to_owned()),
    }
}

/// Dockの設定に合わせて背景メニューのプロセスツール項目を書き換える。
/// MSIでインストールされていない（項目のキーがない）場合は何もしない。
#[cfg(windows)]
pub(crate) fn sync_process_tool_menu(tool: ProcessTool, process_explorer_path: &str) {
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegSetKeyValueW, HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ,
    };

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    let (label, command) = process_tool_entry(tool, process_explorer_path);
    for root in MENU_ROOTS {
        let root = wide(root);
        let mut key: HKEY = std::ptr::null_mut();
        if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, root.as_ptr(), 0, KEY_SET_VALUE, &mut key) }
            != 0
        {
            continue;
        }
        for (subkey, name, value) in [("", "MUIVerb", label), ("command", "", command.as_str())] {
            let subkey = wide(subkey);
            let name = wide(name);
            let data = wide(value);
            unsafe {
                RegSetKeyValueW(
                    key,
                    subkey.as_ptr(),
                    name.as_ptr(),
                    REG_SZ,
                    data.as_ptr().cast(),
                    (data.len() * 2) as u32,
                );
            }
        }
        unsafe { RegCloseKey(key) };
    }
}

#[cfg(not(windows))]
pub(crate) fn sync_process_tool_menu(_tool: ProcessTool, _process_explorer_path: &str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_task_manager_by_default() {
        assert_eq!(
            process_tool_entry(ProcessTool::TaskManager, r"E:\Tools\procexp.exe"),
            ("タスク マネージャー", "taskmgr.exe".to_owned())
        );
    }

    #[test]
    fn quotes_configured_process_explorer_path() {
        assert_eq!(
            process_tool_entry(ProcessTool::ProcessExplorer, " \"E:\\Tools\\procexp.exe\" "),
            ("Process Explorer", "\"E:\\Tools\\procexp.exe\"".to_owned())
        );
    }

    #[test]
    fn falls_back_when_process_explorer_path_is_missing() {
        assert_eq!(
            process_tool_entry(ProcessTool::ProcessExplorer, "  "),
            ("タスク マネージャー", "taskmgr.exe".to_owned())
        );
    }
}
