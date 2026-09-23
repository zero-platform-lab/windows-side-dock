use crate::config::ProcessTool;
use crate::platform::Platform;
use crate::ui::normalized_executable_path;

/// デスクトップ／フォルダー背景の「Windows Side Dock」サブメニュー（MSIが作成する）。
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
pub(crate) fn sync_process_tool_menu(
    platform: &dyn Platform,
    tool: ProcessTool,
    process_explorer_path: &str,
) {
    let (label, command) = process_tool_entry(tool, process_explorer_path);
    for root in MENU_ROOTS {
        if !platform.registry_key_exists(root) {
            continue;
        }
        platform.set_registry_string(root, "", "MUIVerb", label);
        platform.set_registry_string(root, "command", "", &command);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::fake::FakePlatform;

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

    #[test]
    fn updates_only_installed_menu_entries() {
        let platform = FakePlatform {
            registry_keys: vec![MENU_ROOTS[1].to_owned()],
            ..FakePlatform::default()
        };
        sync_process_tool_menu(&platform, ProcessTool::ProcessExplorer, r"E:\procexp.exe");
        let values = platform.registry_values.borrow();
        let written: Vec<_> = values.iter().collect();
        assert_eq!(
            written,
            [
                (
                    &format!("{}|command|", MENU_ROOTS[1]),
                    &r#""E:\procexp.exe""#.to_owned()
                ),
                (
                    &format!("{}||MUIVerb", MENU_ROOTS[1]),
                    &"Process Explorer".to_owned()
                ),
            ]
        );
    }
}
