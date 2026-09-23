use super::*;
use crate::app::new_item;
use crate::config::{temp_root, ConfigStore};
use crate::model::{LauncherItem, RunningWindow};
use crate::platform::fake::FakePlatform;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

fn app_with(platform: FakePlatform, test: &str) -> (LauncherApp, Rc<FakePlatform>) {
    let platform = Rc::new(platform);
    let config = ConfigStore::new(Some(temp_root(test)));
    let app = LauncherApp::new(platform.clone(), config, r"C:\Windows", r"C:\Local");
    (app, platform)
}

fn harness(app: LauncherApp) -> Harness<'static, LauncherApp> {
    Harness::builder()
        .with_size(egui::vec2(54.0, 800.0))
        .build_state(|ctx, app: &mut LauncherApp| app.show(ctx), app)
}

fn dock(test: &str) -> (Harness<'static, LauncherApp>, Rc<FakePlatform>) {
    let (app, platform) = app_with(FakePlatform::default(), test);
    (harness(app), platform)
}

fn window(handle: isize) -> RunningWindow {
    RunningWindow {
        handle,
        title: format!("window {handle}"),
    }
}

fn viewport_input<'a>(
    harness: &'a mut Harness<'static, LauncherApp>,
) -> &'a mut egui::ViewportInfo {
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
}

fn secondary_click_at(harness: &mut Harness<'static, LauncherApp>, pos: egui::Pos2) {
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.step();
}

fn sent_commands(harness: &Harness<'static, LauncherApp>) -> Vec<egui::ViewportCommand> {
    harness
        .output()
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .map(|output| output.commands.clone())
        .unwrap_or_default()
}

/// 1フレームだけ進め、そのフレームでDockが出したウィンドウ操作を返す。
fn step_commands(harness: &mut Harness<'static, LauncherApp>) -> Vec<egui::ViewportCommand> {
    harness.step();
    sent_commands(harness)
}

/// キーを押した1フレームだけを処理し、そのフレームのウィンドウ操作を返す。
fn press(
    harness: &mut Harness<'static, LauncherApp>,
    key: egui::Key,
) -> Vec<egui::ViewportCommand> {
    harness.input_mut().events.push(egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    step_commands(harness)
}

/// ラベルの付いた部品をドラッグし、途中の各フレームで出たウィンドウ操作をまとめて返す。
fn drag(
    harness: &mut Harness<'static, LauncherApp>,
    label: &str,
    delta: egui::Vec2,
) -> Vec<egui::ViewportCommand> {
    let start = harness.get_by_label(label).rect().center();
    let mut commands = Vec::new();
    harness
        .input_mut()
        .events
        .push(egui::Event::PointerMoved(start));
    commands.extend(step_commands(harness));
    harness.input_mut().events.push(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    commands.extend(step_commands(harness));
    for step in 1..=4 {
        let pos = start + delta * (step as f32 / 4.0);
        harness
            .input_mut()
            .events
            .push(egui::Event::PointerMoved(pos));
        commands.extend(step_commands(harness));
    }
    harness.input_mut().events.push(egui::Event::PointerButton {
        pos: start + delta,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    commands.extend(step_commands(harness));
    commands
}

#[test]
fn shows_clock_and_refreshes_running_apps_once_per_second() {
    let (app, platform) = app_with(FakePlatform::default(), "dock-refresh");
    platform.windows.replace(vec![(
        1,
        r"C:\Apps\Chrome.exe".into(),
        "Google Chrome".into(),
    )]);
    let mut harness = harness(app);
    assert!(harness.query_by_label("09/03").is_some());
    assert!(harness.query_by_label("（水）").is_some());
    assert!(harness.query_by_label("07:05").is_some());
    assert_eq!(harness.state().running.len(), 1);
    platform.windows.replace(Vec::new());
    harness.step();
    assert_eq!(harness.state().running.len(), 1);
}

#[test]
fn refreshes_running_apps_once_the_poll_interval_passes() {
    let (app, platform) = app_with(FakePlatform::default(), "dock-poll");
    let mut harness = harness(app);
    platform
        .windows
        .replace(vec![(1, r"C:\Apps\Chrome.exe".into(), "Chrome".into())]);
    harness.state_mut().last_refresh = Some(Instant::now() - Duration::from_secs(2));
    harness.step();
    assert_eq!(harness.state().running.len(), 1);
}

#[test]
fn refreshes_running_apps_only_when_windows_change() {
    let platform = FakePlatform {
        window_changes: Cell::new(Some(false)),
        ..Default::default()
    };
    let (app, platform) = app_with(platform, "dock-watch");
    let mut harness = harness(app);
    let first = harness.state().last_refresh;
    assert!(first.is_some());
    platform
        .windows
        .replace(vec![(1, r"C:\Apps\Chrome.exe".into(), "Chrome".into())]);
    harness.state_mut().last_refresh = Some(Instant::now() - Duration::from_secs(2));
    harness.step();
    assert!(harness.state().running.is_empty());
    platform.window_changes.set(Some(true));
    harness.step();
    assert_eq!(harness.state().running.len(), 1);
}

#[test]
fn undoes_maximize_and_fullscreen() {
    let (mut harness, _platform) = dock("dock-window-state");
    let viewport = viewport_input(&mut harness);
    viewport.fullscreen = Some(true);
    viewport.maximized = Some(true);
    harness.step();
    let commands = sent_commands(&harness);
    assert!(commands.contains(&egui::ViewportCommand::Fullscreen(false)));
    assert!(commands.contains(&egui::ViewportCommand::Maximized(false)));
}

#[test]
fn registers_dropped_files_with_paths() {
    let (mut harness, _platform) = dock("dock-drop");
    harness.input_mut().dropped_files = vec![
        egui::DroppedFile {
            path: Some(r"C:\Games\Steam.lnk".into()),
            ..Default::default()
        },
        egui::DroppedFile::default(),
    ];
    harness.step();
    assert_eq!(harness.state().items.last().unwrap().name, "Steam");
    assert_eq!(harness.state().items.len(), 5);
}

#[test]
fn moves_selection_with_arrow_keys_and_launches_with_enter() {
    let (mut harness, platform) = dock("dock-keys");
    press(&mut harness, egui::Key::ArrowLeft);
    assert_eq!(harness.state().selected, 3);
    press(&mut harness, egui::Key::ArrowRight);
    assert_eq!(harness.state().selected, 0);
    press(&mut harness, egui::Key::Enter);
    assert_eq!(platform.calls(), [r"open C:\Windows\explorer.exe"]);
}

#[test]
fn closes_on_escape() {
    let (mut harness, _platform) = dock("dock-escape");
    assert!(press(&mut harness, egui::Key::Escape).contains(&egui::ViewportCommand::Close));
}

#[test]
fn opens_process_tool_menu_from_clock() {
    let (mut harness, _platform) = dock("dock-menus");
    harness.get_by_label("07:05").click_secondary();
    harness.step();
    assert_eq!(
        harness.state().context_menu.map(|menu| menu.0),
        Some(ContextMenuTarget::Clock)
    );
}

#[test]
fn opens_dock_menu_from_move_handle() {
    let (mut harness, _platform) = dock("dock-handle-menu");
    harness.get_by_label("移動ハンドル").click_secondary();
    harness.step();
    assert_eq!(
        harness.state().context_menu.map(|menu| menu.0),
        Some(ContextMenuTarget::Handle)
    );
}

#[test]
fn opens_dock_menu_from_empty_background() {
    let (mut harness, _platform) = dock("dock-background");
    secondary_click_at(&mut harness, egui::pos2(27.0, 700.0));
    assert_eq!(
        harness.state().context_menu.map(|menu| menu.0),
        Some(ContextMenuTarget::Handle)
    );
}

#[test]
fn moves_the_dock_with_the_native_window_drag() {
    let (mut harness, _platform) = dock("dock-drag");
    let commands = drag(&mut harness, "移動ハンドル", egui::vec2(-30.0, 20.0));
    assert!(commands.contains(&egui::ViewportCommand::StartDrag));
}

#[test]
fn starts_resizing_from_the_grip() {
    let (mut harness, _platform) = dock("dock-resize");
    let commands = drag(&mut harness, "サイズ変更", egui::vec2(0.0, 20.0));
    assert!(commands.contains(&egui::ViewportCommand::BeginResize(
        egui::ResizeDirection::South
    )));
}

#[test]
fn launches_pinned_icons_and_opens_their_menu() {
    let (mut harness, platform) = dock("dock-icons");
    harness.get_by_label("メモ帳").click();
    harness.run();
    assert_eq!(harness.state().selected, 2);
    assert_eq!(platform.calls(), [r"open C:\Windows\System32\notepad.exe"]);
    harness.get_by_label("メモ帳").click_secondary();
    harness.step();
    assert_eq!(
        harness.state().context_menu.map(|menu| menu.0),
        Some(ContextMenuTarget::Pinned(2))
    );
}

#[test]
fn draws_running_state_and_shell_icons() {
    let (app, platform) = app_with(
        FakePlatform {
            icons: true,
            foreground: 1,
            ..FakePlatform::default()
        },
        "dock-running",
    );
    platform.windows.replace(vec![
        (1, r"C:\Windows\explorer.exe".into(), "Downloads".into()),
        (2, r"C:\Apps\Chrome.exe".into(), "Google Chrome".into()),
        (3, r"C:\Apps\Code.exe".into(), "a - Code".into()),
        (4, r"C:\Apps\Code.exe".into(), "b - Code".into()),
    ]);
    let mut harness = harness(app);
    harness.get_by_label("Code").hover();
    harness.step();
    let state = harness.state();
    assert!(state.items[0].active);
    assert_eq!(state.running.len(), 2);
    assert!(state
        .textures
        .contains_key(r"shell-icon:C:\Windows\explorer.exe"));
    assert!(state.textures.contains_key(r"running:C:\Apps\Chrome.exe"));
}

#[test]
fn highlights_active_running_apps() {
    let (app, platform) = app_with(
        FakePlatform {
            foreground: 2,
            ..FakePlatform::default()
        },
        "dock-running-active",
    );
    platform.windows.replace(vec![(
        2,
        r"C:\Apps\Chrome.exe".into(),
        "Google Chrome".into(),
    )]);
    let harness = harness(app);
    assert!(harness.state().running[0].active);
}

#[test]
fn activates_running_apps_and_opens_their_menu() {
    let (app, platform) = app_with(FakePlatform::default(), "dock-running-click");
    platform.windows.replace(vec![(
        2,
        r"C:\Apps\Chrome.exe".into(),
        "Google Chrome".into(),
    )]);
    let mut harness = harness(app);
    harness.get_by_label("Google Chrome").click();
    harness.run();
    assert_eq!(platform.calls(), ["foreground 2"]);
    harness.get_by_label("Google Chrome").click_secondary();
    harness.step();
    assert_eq!(
        harness.state().context_menu.map(|menu| menu.0),
        Some(ContextMenuTarget::Running(0))
    );
}

#[test]
fn shows_tooltips_beside_the_dock() {
    let (app, platform) = app_with(FakePlatform::default(), "dock-tooltip");
    platform.windows.replace(vec![(
        2,
        r"C:\Apps\Chrome.exe".into(),
        "Google Chrome".into(),
    )]);
    let mut harness = harness(app);
    harness
        .ctx
        .memory_mut(|memory| memory.set_everything_is_visible(true));
    harness.step();
    harness.step();
    assert!(harness.query_by_label("Google Chrome（実行中）").is_some());
    std::thread::sleep(Duration::from_millis(25));
    harness.step();
    assert!(harness.query_by_label("Google Chrome（実行中）").is_some());
}

#[test]
fn describes_running_apps_by_window_count() {
    let mut item = new_item(
        &FakePlatform::default(),
        "Code",
        "code.exe",
        crate::model::IconKind::File,
        "",
    );
    item.windows = vec![window(1)];
    assert_eq!(crate::ui::running_tooltip(&item), "Code（実行中）");
    item.windows.push(window(2));
    assert_eq!(crate::ui::running_tooltip(&item), "Code（2個のウィンドウ）");
}

#[test]
fn skips_tooltips_when_dock_position_is_unknown() {
    let (app, _platform) = app_with(
        FakePlatform {
            dock: None,
            ..FakePlatform::default()
        },
        "dock-tooltip-unknown",
    );
    let mut harness = harness(app);
    harness
        .ctx
        .memory_mut(|memory| memory.set_everything_is_visible(true));
    harness.step();
    assert!(harness.query_by_label("メモ帳").is_some());
    assert_eq!(harness.query_all_by_label("メモ帳").count(), 1);
}

fn settings(test: &str) -> (Harness<'static, LauncherApp>, Rc<FakePlatform>) {
    let (mut app, platform) = app_with(FakePlatform::default(), test);
    app.show_settings = true;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 700.0))
        .build_state(|ctx, app: &mut LauncherApp| app.show(ctx), app);
    harness.run();
    (harness, platform)
}

fn click(harness: &mut Harness<'static, LauncherApp>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

#[test]
fn saves_popup_direction_and_process_tool_from_settings() {
    let (mut harness, _platform) = settings("settings-radios");
    click(&mut harness, "常に右");
    click(&mut harness, "Process Explorer");
    let state = harness.state();
    assert_eq!(state.popup_direction, PopupDirection::Right);
    assert_eq!(state.config.load_popup_direction(), PopupDirection::Right);
    assert_eq!(
        state.config.load_process_tool(),
        ProcessTool::ProcessExplorer
    );
    assert!(harness
        .query_by_label("実行ファイルのパスを設定してください")
        .is_some());
}

#[test]
fn edits_process_explorer_path_by_typing() {
    let (mut harness, _platform) = settings("settings-type");
    click(&mut harness, "Process Explorer");
    let field = harness.get_by_role(egui::accesskit::Role::TextInput);
    field.click();
    harness.run();
    harness
        .get_by_role(egui::accesskit::Role::TextInput)
        .type_text(r"C:\none.exe");
    harness.run();
    assert_eq!(harness.state().process_explorer_path, r"C:\none.exe");
    assert_eq!(
        harness.state().config.load_process_explorer_path(),
        r"C:\none.exe"
    );
    assert!(harness
        .query_by_label("指定されたファイルが見つかりません")
        .is_some());
}

#[test]
fn chooses_process_explorer_from_file_dialog_and_test_launches_it() {
    let root = temp_root("settings-choose-file");
    std::fs::create_dir_all(&root).unwrap();
    let file = root.join("procexp.exe");
    std::fs::write(&file, "").unwrap();
    let (mut app, platform) = app_with(
        FakePlatform {
            chosen_file: Some(file.to_string_lossy().into_owned()),
            ..FakePlatform::default()
        },
        "settings-choose",
    );
    app.show_settings = true;
    app.set_process_tool(ProcessTool::ProcessExplorer);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 700.0))
        .build_state(|ctx, app: &mut LauncherApp| app.show(ctx), app);
    harness.run();
    click(&mut harness, "エクスプローラーから選択…");
    click(&mut harness, "Process Explorerをテスト起動");
    assert_eq!(
        platform.calls(),
        [
            "choose".to_owned(),
            format!("open {}", file.to_string_lossy())
        ]
    );
}

#[test]
fn keeps_path_when_file_dialog_is_cancelled() {
    let (mut app, platform) = app_with(FakePlatform::default(), "settings-cancel");
    app.show_settings = true;
    app.process_tool = ProcessTool::ProcessExplorer;
    app.process_explorer_path = r"C:\old.exe".into();
    app.monitor_status = Some("起動できませんでした".into());
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 700.0))
        .build_state(|ctx, app: &mut LauncherApp| app.show(ctx), app);
    harness.run();
    assert!(harness.query_by_label("起動できませんでした").is_some());
    click(&mut harness, "エクスプローラーから選択…");
    assert_eq!(harness.state().process_explorer_path, r"C:\old.exe");
    assert_eq!(platform.calls(), ["choose"]);
}

#[test]
fn closes_settings_with_button_or_window_close() {
    let (mut harness, _platform) = settings("settings-close");
    click(&mut harness, "閉じる");
    assert!(!harness.state().show_settings);

    harness.state_mut().show_settings = true;
    harness.run();
    viewport_input(&mut harness)
        .events
        .push(egui::ViewportEvent::Close);
    harness.step();
    assert!(!harness.state().show_settings);
}

#[test]
fn draws_every_builtin_icon_shape() {
    let (mut harness, _platform) = dock("dock-shapes");
    let state = harness.state_mut();
    state.items.push(new_item(
        &FakePlatform::default(),
        "file",
        "file.exe",
        crate::model::IconKind::File,
        "",
    ));
    state.items[4].windows = vec![window(5)];
    state.selected = 4;
    harness.run();
    assert!(harness.query_by_label("file").is_some());
    let _: Option<&LauncherItem> = harness.state().items.get(4);
}

#[test]
fn opens_settings_from_gear_button_and_highlights_on_hover() {
    let (mut harness, _platform) = dock("dock-gear");
    harness.get_by_label("Dock 設定").hover();
    harness.step();
    harness.step();
    harness.get_by_label("Dock 設定").click();
    harness.step();
    assert!(harness.state().show_settings);
}

#[test]
fn runs_as_an_eframe_app() {
    let (app, _platform) = app_with(FakePlatform::default(), "dock-eframe");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(54.0, 800.0))
        .build_eframe(|_creation_context| app);
    harness.step();
    assert!(harness.query_by_label("時計").is_some());
}

#[test]
fn opens_dock_menu_from_gear_button() {
    let (mut harness, _platform) = dock("dock-gear-menu");
    harness.get_by_label("Dock 設定").click_secondary();
    harness.step();
    assert_eq!(
        harness.state().context_menu.map(|menu| menu.0),
        Some(ContextMenuTarget::Handle)
    );
    assert!(!harness.state().show_settings);
}

#[test]
fn handles_tray_menu_actions() {
    use crate::platform::TrayAction;
    let (app, platform) = app_with(FakePlatform::default(), "dock-tray");
    platform
        .tray_actions
        .replace([TrayAction::OpenSettings, TrayAction::LaunchProcessTool].into());
    let mut harness = harness(app);
    assert!(harness.state().show_settings);
    assert_eq!(platform.calls(), ["open taskmgr.exe"]);
    platform
        .tray_actions
        .borrow_mut()
        .push_back(TrayAction::Quit);
    assert!(step_commands(&mut harness).contains(&egui::ViewportCommand::Close));
}
