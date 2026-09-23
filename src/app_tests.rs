use super::*;
use crate::config::temp_root;
use crate::config::DockSide;
use crate::platform::fake::FakePlatform;
use std::path::PathBuf;

struct Fixture {
    app: LauncherApp,
    platform: Rc<FakePlatform>,
    root: PathBuf,
}

/// 0.1.21から更新した状態（標準アイコンだった4つを引き継ぐ）で始める。
fn fixture(test: &str, platform: FakePlatform) -> Fixture {
    let root = temp_root(test);
    std::fs::create_dir_all(root.join("windows-side-dock")).unwrap();
    std::fs::write(root.join("windows-side-dock").join("items.txt"), "").unwrap();
    let platform = Rc::new(platform);
    let app = LauncherApp::new(
        platform.clone(),
        ConfigStore::new(Some(root.clone())),
        r"C:\Windows",
        r"C:\Local",
    );
    Fixture {
        app,
        platform,
        root,
    }
}

fn window(handle: isize) -> RunningWindow {
    RunningWindow {
        handle,
        title: format!("window {handle}"),
    }
}

/// 実在するファイル。Process Explorerの設定が有効かどうかの判定に使う。
fn existing_file(root: &Path) -> String {
    std::fs::create_dir_all(root).unwrap();
    let path = root.join("procexp.exe");
    std::fs::write(&path, "").unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn starts_with_explorer_and_settings_on_first_run() {
    let root = temp_root("app-first-run");
    let config = ConfigStore::new(Some(root.clone()));
    let app = LauncherApp::new(
        Rc::new(FakePlatform::default()),
        config,
        r"C:\Windows",
        r"C:\Local",
    );
    let commands: Vec<_> = app.items.iter().map(|item| item.command.as_str()).collect();
    assert_eq!(commands, [r"C:\Windows\explorer.exe", "ms-settings:"]);
    assert_eq!(app.items[1].fallback_icon, IconKind::Settings);
    assert_eq!(app.config.load_pinned_items().unwrap().len(), 2);
}

#[test]
fn keeps_saved_pins_even_when_every_pin_was_removed() {
    let root = temp_root("app-saved-pins");
    let config = ConfigStore::new(Some(root.clone()));
    config.save_pinned_items(std::iter::empty());
    let app = LauncherApp::new(
        Rc::new(FakePlatform::default()),
        config,
        r"C:\Windows",
        r"C:\Local",
    );
    assert!(app.items.is_empty());
}

#[test]
fn carries_over_the_former_builtin_icons_and_old_pins_without_duplicates() {
    let root = temp_root("app-start");
    std::fs::create_dir_all(root.join("windows-side-dock")).unwrap();
    std::fs::write(
        root.join("windows-side-dock").join("items.txt"),
        "Code|C:\\Apps\\Code.exe\nメモ帳|C:\\Windows\\System32\\notepad.exe",
    )
    .unwrap();
    let config = ConfigStore::new(Some(root.clone()));
    config.save_process_tool(ProcessTool::ProcessExplorer);
    config.save_process_explorer_path(r"E:\procexp.exe");
    let platform = Rc::new(FakePlatform {
        icons: true,
        registry_keys: vec![
            r"Software\Classes\DesktopBackground\Shell\WindowsSideDock\shell\02ProcessTool".into(),
        ],
        ..FakePlatform::default()
    });
    let app = LauncherApp::new(platform.clone(), config, r"C:\Windows", r"C:\Local");

    let commands: Vec<_> = app.items.iter().map(|item| item.command.as_str()).collect();
    assert_eq!(
        commands,
        [
            r"C:\Windows\explorer.exe",
            r"C:\Local\Microsoft\WindowsApps\wt.exe",
            r"C:\Windows\System32\notepad.exe",
            "ms-settings:",
            r"C:\Apps\Code.exe",
        ]
    );
    assert!(app.items.iter().all(|item| item.icon.is_some()));
    assert_eq!(app.config.load_pinned_items().unwrap().len(), 5);
    assert_eq!(app.process_tool, ProcessTool::ProcessExplorer);
    assert_eq!(
        platform.registry_values.borrow().values().next().unwrap(),
        r#""E:\procexp.exe""#
    );
}

#[test]
fn adds_dropped_files_once() {
    let mut f = fixture("app-add", FakePlatform::default());
    f.app.add_path(Path::new(r"C:\Games\Steam.lnk"));
    f.app.add_path(Path::new(r"C:\Games\Steam.lnk"));
    assert_eq!(f.app.items.len(), 5);
    assert_eq!(f.app.items[4].name, "Steam");
    assert_eq!(
        f.app.config.load_pinned_items().unwrap().last(),
        Some(&("Steam".to_owned(), r"C:\Games\Steam.lnk".to_owned()))
    );
}

#[test]
fn ignores_dropped_paths_that_are_not_unicode() {
    use std::os::windows::ffi::OsStringExt;
    let mut f = fixture("app-add-invalid", FakePlatform::default());
    let invalid = std::ffi::OsString::from_wide(&[0xD800]);
    f.app.add_path(Path::new(&invalid));
    assert_eq!(f.app.items.len(), 4);
}

#[test]
fn launches_items_and_reuses_open_settings_window() {
    let mut f = fixture("app-launch", FakePlatform::default());
    f.app.launch(0);
    f.app.launch(3);
    f.app.items[3].windows = vec![window(9)];
    f.app.launch(3);
    f.app.launch(99);
    assert_eq!(
        f.platform.calls(),
        [
            r"open C:\Windows\explorer.exe",
            "open ms-settings:",
            "foreground 9"
        ]
    );
}

#[test]
fn refreshes_running_apps_and_pins_them() {
    let f = fixture("app-pin", FakePlatform::default());
    let mut app = f.app;
    f.platform.windows.replace(vec![
        (1, r"C:\Windows\explorer.exe".into(), "Downloads".into()),
        (2, r"C:\Apps\Chrome.exe".into(), "Google Chrome".into()),
    ]);
    app.refresh_running();
    assert_eq!(app.items[0].windows.len(), 1);
    assert_eq!(app.running.len(), 1);

    app.pin_running(5);
    app.pin_running(0);
    assert_eq!(app.items.len(), 5);
    assert_eq!(app.items[4].command, r"C:\Apps\Chrome.exe");
    assert!(app.items[4].windows.len() == 1);
    assert!(app.running.is_empty());
    assert_eq!(app.config.load_pinned_items().unwrap().len(), 5);
}

#[test]
fn unpins_any_item_down_to_an_empty_dock() {
    let mut f = fixture("app-unpin", FakePlatform::default());
    f.app.add_path(Path::new(r"C:\a.exe"));
    f.app.selected = 5;
    f.app.unpin(9);
    assert_eq!(f.app.items.len(), 5);
    f.app.unpin(0);
    assert_eq!(f.app.items.len(), 4);
    assert_eq!(f.app.selected, 3);
    assert_eq!(
        f.app.config.load_pinned_items().unwrap().last(),
        Some(&("a".to_owned(), r"C:\a.exe".to_owned()))
    );
    while !f.app.items.is_empty() {
        f.app.unpin(0);
    }
    assert_eq!(f.app.selected, 0);
    assert_eq!(f.app.config.load_pinned_items(), Some(Vec::new()));
}

#[test]
fn activates_single_window_or_opens_picker() {
    let mut f = fixture("app-activate", FakePlatform::default());
    f.app.running = vec![
        LauncherItem {
            windows: vec![window(1)],
            ..new_item(f.platform.as_ref(), "One", "one.exe", IconKind::File, "")
        },
        LauncherItem {
            windows: vec![window(2), window(3)],
            ..new_item(f.platform.as_ref(), "Two", "two.exe", IconKind::File, "")
        },
    ];
    f.app.confirm_close_all = true;
    f.app.activate_running(0);
    f.app.activate_running(1);
    f.app.activate_running(7);
    assert_eq!(f.platform.calls(), ["foreground 1"]);
    let (name, windows, position) = f.app.window_picker.clone().unwrap();
    assert_eq!(name, "Two");
    assert_eq!(windows.len(), 2);
    assert_eq!(position, egui::pos2(1854.0 - 480.0 - 8.0, 300.0));
    assert!(!f.app.confirm_close_all);
}

#[test]
fn skips_picker_and_menu_when_cursor_is_unknown() {
    let mut f = fixture(
        "app-no-cursor",
        FakePlatform {
            cursor: None,
            ..FakePlatform::default()
        },
    );
    f.app
        .open_or_activate_windows("Two".into(), vec![window(2), window(3)]);
    f.app.open_context_menu(ContextMenuTarget::Clock);
    assert!(f.app.window_picker.is_none());
    assert!(f.app.context_menu.is_none());
}

#[test]
fn opens_context_menu_beside_the_dock() {
    let mut f = fixture("app-menu", FakePlatform::default());
    f.app.confirm_close_all = true;
    f.app.open_context_menu(ContextMenuTarget::Pinned(1));
    let (target, position, _) = f.app.context_menu.unwrap();
    assert_eq!(target, ContextMenuTarget::Pinned(1));
    assert_eq!(position, egui::pos2(1854.0 - 210.0 - 8.0, 300.0));
    assert!(!f.app.confirm_close_all);
}

#[test]
fn launches_task_manager_and_reports_failure() {
    let mut f = fixture(
        "app-taskmgr",
        FakePlatform {
            open_succeeds: false,
            ..FakePlatform::default()
        },
    );
    assert!(f.app.process_tool_ready());
    assert!(!f.app.launch_process_tool());
    assert_eq!(
        f.app.monitor_status.as_deref(),
        Some("タスク マネージャーを起動できませんでした")
    );
    assert!(!f.app.show_settings);
    assert_eq!(f.platform.calls(), ["open taskmgr.exe"]);
}

#[test]
fn launches_configured_process_explorer() {
    let mut f = fixture("app-procexp", FakePlatform::default());
    let path = existing_file(&f.root);
    f.app.set_process_tool(ProcessTool::ProcessExplorer);
    f.app.set_process_explorer_path(format!("\"{path}\""));
    assert!(f.app.process_tool_ready());
    assert!(f.app.launch_process_tool());
    assert_eq!(f.app.monitor_status, None);
    assert_eq!(f.platform.calls(), [format!("open {path}")]);
}

#[test]
fn opens_settings_when_process_explorer_cannot_start() {
    let mut f = fixture(
        "app-procexp-fail",
        FakePlatform {
            open_succeeds: false,
            ..FakePlatform::default()
        },
    );
    let path = existing_file(&f.root);
    f.app.set_process_tool(ProcessTool::ProcessExplorer);
    f.app.set_process_explorer_path(path);
    assert!(!f.app.launch_process_tool());
    assert_eq!(
        f.app.monitor_status.as_deref(),
        Some("Process Explorerを起動できませんでした")
    );
    assert!(f.app.show_settings);
}

#[test]
fn asks_for_a_valid_process_explorer_path() {
    let mut f = fixture("app-procexp-missing", FakePlatform::default());
    f.app.set_process_tool(ProcessTool::ProcessExplorer);
    f.app
        .set_process_explorer_path(r"C:\missing\procexp.exe".into());
    assert!(!f.app.process_tool_ready());
    assert!(!f.app.launch_process_tool());
    assert_eq!(
        f.app.monitor_status.as_deref(),
        Some("Process Explorerの実行ファイルが見つかりません")
    );
    assert!(f.app.show_settings);
    assert!(f.platform.calls().is_empty());
}

#[test]
fn saves_settings_changes() {
    let mut f = fixture("app-settings", FakePlatform::default());
    f.app.monitor_status = Some("old".into());
    f.app.set_dock_side(DockSide::Left);
    f.app.set_process_tool(ProcessTool::ProcessExplorer);
    f.app.set_process_explorer_path(r"E:\p.exe".into());
    assert_eq!(f.app.alignment(), egui::RectAlign::RIGHT);
    assert!(!f.app.opens_left());
    assert_eq!(f.app.monitor_status, None);
    assert_eq!(f.app.config.load_dock_side(), DockSide::Left);
    assert_eq!(
        f.app.config.load_process_tool(),
        ProcessTool::ProcessExplorer
    );
    assert_eq!(f.app.config.load_process_explorer_path(), r"E:\p.exe");
}
