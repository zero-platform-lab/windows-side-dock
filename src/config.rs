use std::path::PathBuf;

/// Dockを置く画面の端。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DockSide {
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
const ITEMS_FILE: &str = "items.txt";
const PROCESS_TOOL_FILE: &str = "process_tool.txt";
const PROCESS_EXPLORER_PATH_FILE: &str = "process_explorer_path.txt";
/// 0.1.16で追加。旧保存先には存在しないため移行対象に含めない。
const ALWAYS_ON_TOP_FILE: &str = "always_on_top.txt";
/// 0.1.18で追加。旧保存先には存在しないため移行対象に含めない。
const DOCK_SIDE_FILE: &str = "dock_side.txt";
const CONFIG_FILES: [&str; 3] = [ITEMS_FILE, PROCESS_TOOL_FILE, PROCESS_EXPLORER_PATH_FILE];

/// `%LOCALAPPDATA%\windows-side-dock` 以下の設定ファイル。
/// 保存先が決まらない（`LOCALAPPDATA` がない）場合は読み込みも保存もしない。
pub(crate) struct ConfigStore {
    local_app_data: Option<PathBuf>,
}

impl ConfigStore {
    pub(crate) fn new(local_app_data: Option<PathBuf>) -> Self {
        Self { local_app_data }
    }

    pub(crate) fn from_env() -> Self {
        Self::new(std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
    }

    fn read(&self, name: &str) -> Option<String> {
        let root = self.local_app_data.as_ref()?;
        std::fs::read_to_string(root.join(CONFIG_DIR).join(name)).ok()
    }

    fn write(&self, name: &str, value: &str) {
        let Some(root) = &self.local_app_data else {
            return;
        };
        let directory = root.join(CONFIG_DIR);
        let _ = std::fs::create_dir_all(&directory);
        let _ = std::fs::write(directory.join(name), value);
    }

    /// 旧フォルダーの設定を新フォルダーへコピーする。新フォルダーに既にあるファイルは上書きしない。
    /// 旧フォルダーは戻せるように残し、失敗したファイルは次回起動時に再試行される。
    pub(crate) fn migrate_legacy(&self) {
        let Some(root) = &self.local_app_data else {
            return;
        };
        let directory = root.join(CONFIG_DIR);
        for name in CONFIG_FILES {
            let source = root.join(LEGACY_CONFIG_DIR).join(name);
            let target = directory.join(name);
            if target.exists() || !source.is_file() {
                continue;
            }
            let _ = std::fs::create_dir_all(&directory);
            let _ = std::fs::copy(&source, &target);
        }
    }

    pub(crate) fn load_registered_items(&self) -> Vec<(String, String)> {
        self.read(ITEMS_FILE)
            .map_or_else(Vec::new, |contents| parse_registered_items(&contents))
    }

    pub(crate) fn save_registered_items<'a>(
        &self,
        items: impl Iterator<Item = (&'a str, &'a str)>,
    ) {
        self.write(ITEMS_FILE, &format_registered_items(items));
    }

    pub(crate) fn load_process_tool(&self) -> ProcessTool {
        self.read(PROCESS_TOOL_FILE)
            .map_or(ProcessTool::TaskManager, |value| parse_process_tool(&value))
    }

    pub(crate) fn save_process_tool(&self, tool: ProcessTool) {
        self.write(PROCESS_TOOL_FILE, process_tool_value(tool));
    }

    pub(crate) fn load_process_explorer_path(&self) -> String {
        self.read(PROCESS_EXPLORER_PATH_FILE)
            .map(|value| value.trim().to_owned())
            .unwrap_or_default()
    }

    pub(crate) fn save_process_explorer_path(&self, value: &str) {
        self.write(PROCESS_EXPLORER_PATH_FILE, value.trim());
    }

    /// Dockをほかのウィンドウより常に手前に出すか。既定は出さない。
    pub(crate) fn load_always_on_top(&self) -> bool {
        self.read(ALWAYS_ON_TOP_FILE)
            .is_some_and(|value| value.trim() == "on")
    }

    pub(crate) fn save_always_on_top(&self, enabled: bool) {
        self.write(ALWAYS_ON_TOP_FILE, if enabled { "on" } else { "off" });
    }

    /// Dockを置く画面の端。既定は右。
    pub(crate) fn load_dock_side(&self) -> DockSide {
        match self.read(DOCK_SIDE_FILE).as_deref().map(str::trim) {
            Some("left") => DockSide::Left,
            _ => DockSide::Right,
        }
    }

    pub(crate) fn save_dock_side(&self, side: DockSide) {
        let value = match side {
            DockSide::Left => "left",
            DockSide::Right => "right",
        };
        self.write(DOCK_SIDE_FILE, value);
    }
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

/// テスト用の一時フォルダー。テストごとに名前を分け、前回の残りは消してから返す。
#[cfg(test)]
pub(crate) fn temp_root(test: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("wsd-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn store(root: &Path) -> ConfigStore {
        ConfigStore::new(Some(root.to_path_buf()))
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
        for tool in [ProcessTool::TaskManager, ProcessTool::ProcessExplorer] {
            assert_eq!(parse_process_tool(process_tool_value(tool)), tool);
        }
    }

    #[test]
    fn saves_every_setting_under_application_folder() {
        let root = temp_root("store-round-trip");
        let config = store(&root);
        config.save_registered_items([("Code", r"C:\Code.exe")].into_iter());
        config.save_process_tool(ProcessTool::ProcessExplorer);
        config.save_process_explorer_path("  E:\\procexp.exe \n");
        config.save_always_on_top(true);
        config.save_dock_side(DockSide::Left);

        let reloaded = store(&root);
        assert_eq!(
            reloaded.load_registered_items(),
            [("Code".to_owned(), r"C:\Code.exe".to_owned())]
        );
        assert_eq!(reloaded.load_process_tool(), ProcessTool::ProcessExplorer);
        assert!(reloaded.load_always_on_top());
        assert_eq!(reloaded.load_dock_side(), DockSide::Left);
        reloaded.save_dock_side(DockSide::Right);
        assert_eq!(reloaded.load_dock_side(), DockSide::Right);
        reloaded.save_always_on_top(false);
        assert!(!reloaded.load_always_on_top());
        assert_eq!(reloaded.load_process_explorer_path(), r"E:\procexp.exe");
        assert!(root
            .join("windows-side-dock")
            .join("dock_side.txt")
            .is_file());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn uses_defaults_when_files_are_missing() {
        let root = temp_root("store-defaults");
        let config = store(&root);
        assert!(config.load_registered_items().is_empty());
        assert_eq!(config.load_process_tool(), ProcessTool::TaskManager);
        assert!(!config.load_always_on_top());
        assert_eq!(config.load_dock_side(), DockSide::Right);
        assert_eq!(config.load_process_explorer_path(), "");
    }

    #[test]
    fn does_nothing_without_local_app_data() {
        let config = ConfigStore::new(None);
        config.save_process_tool(ProcessTool::ProcessExplorer);
        config.migrate_legacy();
        assert_eq!(config.load_process_tool(), ProcessTool::TaskManager);
    }

    #[test]
    fn reads_local_app_data_from_environment() {
        let config = ConfigStore::from_env();
        assert_eq!(
            config.local_app_data,
            std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
        );
    }

    #[test]
    fn copies_legacy_settings_and_keeps_originals() {
        let root = temp_root("migrate-copy");
        let legacy = root.join("lancher");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("items.txt"), "Code|C:\\Code.exe").unwrap();
        std::fs::write(legacy.join("process_tool.txt"), "process_explorer").unwrap();
        std::fs::write(legacy.join("unrelated.txt"), "x").unwrap();

        store(&root).migrate_legacy();

        let read = |name| store(&root).read(name);
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
        std::fs::write(
            root.join("lancher").join("process_tool.txt"),
            "task_manager",
        )
        .unwrap();
        let config = store(&root);
        config.save_process_tool(ProcessTool::ProcessExplorer);

        config.migrate_legacy();
        config.migrate_legacy();

        assert_eq!(config.load_process_tool(), ProcessTool::ProcessExplorer);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migration_without_legacy_folder_creates_nothing() {
        let root = temp_root("migrate-none");
        std::fs::create_dir_all(&root).unwrap();
        store(&root).migrate_legacy();
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
}
