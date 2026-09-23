use super::*;
use crate::app::new_item;
use crate::config::{four_pins_store, temp_root, DockSide};
use crate::model::{IconKind, LauncherItem};
use crate::platform::fake::FakePlatform;
use egui_kittest::kittest::Queryable;
use egui_kittest::Harness;
use std::rc::Rc;
use std::time::Instant;

fn window(handle: isize) -> RunningWindow {
    RunningWindow {
        handle,
        title: format!("window {handle}"),
    }
}

fn running_item(name: &str, handles: &[isize]) -> LauncherItem {
    LauncherItem {
        windows: handles.iter().map(|&handle| window(handle)).collect(),
        ..new_item(&FakePlatform::default(), name, name, IconKind::File, "")
    }
}

fn app(test: &str, platform: FakePlatform) -> (LauncherApp, Rc<FakePlatform>) {
    let platform = Rc::new(platform);
    let config = four_pins_store(test);
    let mut app = LauncherApp::new(platform.clone(), config, r"C:\Windows");
    app.add_path(std::path::Path::new(r"C:\Apps\Code.exe"));
    app.running = vec![
        running_item("Chrome", &[21]),
        running_item("Code", &[31, 32]),
    ];
    (app, platform)
}

/// 開いてから十分時間がたったメニュー。表示待ちとフォーカス猶予の対象外になる。
fn open_menu(app: &mut LauncherApp, target: ContextMenuTarget) {
    app.context_menu = Some((
        target,
        egui::pos2(1636.0, 300.0),
        Instant::now() - Duration::from_secs(1),
    ));
}

fn harness(app: LauncherApp) -> Harness<'static, LauncherApp> {
    Harness::builder()
        .with_size(egui::vec2(480.0, 480.0))
        .build_state(
            |ctx, app: &mut LauncherApp| {
                app.show_context_menu_viewport(ctx);
                app.show_window_picker_viewport(ctx);
            },
            app,
        )
}

fn menu_harness(
    test: &str,
    platform: FakePlatform,
    target: ContextMenuTarget,
) -> (Harness<'static, LauncherApp>, Rc<FakePlatform>) {
    let (mut app, platform) = app(test, platform);
    open_menu(&mut app, target);
    (harness(app), platform)
}

fn click(harness: &mut Harness<'static, LauncherApp>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

fn existing_file(test: &str) -> String {
    let root = temp_root(test);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("procexp.exe");
    std::fs::write(&path, "").unwrap();
    path.to_string_lossy().into_owned()
}

#[test]
fn sizes_menus_by_target_and_window_count() {
    use ContextMenuTarget::*;
    assert_eq!(menu_size(Handle, None), (210.0, 156.0));
    assert_eq!(menu_size(Clock, None), (210.0, 54.0));
    assert_eq!(menu_size(Pinned(0), Some(0)), (210.0, 208.0));
    assert_eq!(menu_size(Pinned(0), Some(1)), (430.0, 256.0));
    assert_eq!(menu_size(Pinned(0), Some(3)), (430.0, 354.0));
    assert_eq!(menu_size(Running(0), None), (210.0, 208.0));
    assert_eq!(menu_size(Running(0), Some(1)), (430.0, 222.0));
    assert_eq!(menu_size(Running(0), Some(20)), (430.0, 500.0));
}

#[test]
fn widens_menus_toward_the_open_side() {
    let origin = egui::pos2(1636.0, 300.0);
    assert_eq!(
        menu_position(origin, 430.0, true),
        egui::pos2(1416.0, 300.0)
    );
    assert_eq!(menu_position(origin, 430.0, false), origin);
}

#[test]
fn handle_menu_collapses_and_expands_the_dock() {
    let (mut harness, _platform) = menu_harness(
        "menu-collapse",
        FakePlatform::default(),
        ContextMenuTarget::Handle,
    );
    click(&mut harness, "Dockをしまう");
    assert!(harness.state().collapsed);
    assert!(harness.state().context_menu.is_none());
    open_menu(harness.state_mut(), ContextMenuTarget::Handle);
    harness.run();
    click(&mut harness, "Dockを引き出す");
    assert!(!harness.state().collapsed);
}

#[test]
fn handle_menu_opens_install_folder() {
    let (mut harness, platform) = menu_harness(
        "menu-folder",
        FakePlatform::default(),
        ContextMenuTarget::Handle,
    );
    click(&mut harness, "Windows Side Dockの場所を開く");
    assert_eq!(platform.calls(), [r"open C:\Dock"]);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn handle_menu_closes_even_without_install_folder() {
    let platform = FakePlatform {
        install_directory: None,
        ..FakePlatform::default()
    };
    let (mut harness, platform) =
        menu_harness("menu-folder-none", platform, ContextMenuTarget::Handle);
    click(&mut harness, "Windows Side Dockの場所を開く");
    assert!(platform.calls().is_empty());
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn handle_menu_launches_process_tool_and_opens_settings() {
    let (mut harness, platform) = menu_harness(
        "menu-handle",
        FakePlatform::default(),
        ContextMenuTarget::Handle,
    );
    harness.run();
    assert!(harness.state().context_menu.is_some());
    click(&mut harness, "タスク マネージャー");
    assert_eq!(platform.calls(), ["open taskmgr.exe"]);
    assert!(harness.state().context_menu.is_none());

    open_menu(harness.state_mut(), ContextMenuTarget::Handle);
    harness.run();
    click(&mut harness, "Dock 設定");
    assert!(harness.state().show_settings);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn clock_menu_asks_for_process_explorer_path() {
    let (mut app, platform) = app("menu-clock", FakePlatform::default());
    app.process_tool = ProcessTool::ProcessExplorer;
    open_menu(&mut app, ContextMenuTarget::Clock);
    let mut harness = harness(app);
    harness.run();
    click(&mut harness, "Process Explorer");
    assert!(harness.state().context_menu.is_some());
    click(&mut harness, "パスを設定…");
    assert!(harness.state().show_settings);
    assert!(harness.state().context_menu.is_none());
    assert!(platform.calls().is_empty());
}

#[test]
fn clock_menu_launches_ready_process_explorer() {
    let (mut app, platform) = app("menu-clock-ready", FakePlatform::default());
    app.process_tool = ProcessTool::ProcessExplorer;
    app.process_explorer_path = existing_file("menu-clock-ready-file");
    open_menu(&mut app, ContextMenuTarget::Clock);
    let mut harness = harness(app);
    harness.run();
    assert!(harness.query_by_label("パスを設定…").is_none());
    click(&mut harness, "Process Explorer");
    assert_eq!(platform.calls().len(), 1);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn pinned_menu_launches_idle_items() {
    let (mut harness, platform) = menu_harness(
        "menu-pinned",
        FakePlatform::default(),
        ContextMenuTarget::Pinned(0),
    );
    harness.run();
    assert!(harness.query_by_label("ウィンドウへ移動").is_none());
    assert!(harness.query_by_label("ファイルエクスプローラー").is_some());
    assert!(harness.query_by_label("プロパティ").is_some());
    assert!(harness.query_by_label("ピン留めを外す").is_some());
    click(&mut harness, "起動");
    assert_eq!(platform.calls(), [r"open C:\Windows\explorer.exe"]);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn pinned_menu_switches_to_the_only_window() {
    let (mut app, platform) = app("menu-pinned-one", FakePlatform::default());
    app.items[4].windows = vec![window(41)];
    open_menu(&mut app, ContextMenuTarget::Pinned(4));
    let mut harness = harness(app);
    harness.run();
    assert!(harness.query_by_label("すべて閉じる").is_none());
    click(&mut harness, "window 41");
    assert_eq!(platform.calls(), ["foreground 41"]);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn pinned_menu_starts_new_instance_of_running_app() {
    let (mut app, platform) = app("menu-pinned-new", FakePlatform::default());
    app.items[4].windows = vec![window(41)];
    open_menu(&mut app, ContextMenuTarget::Pinned(4));
    let mut harness = harness(app);
    click(&mut harness, "新しく起動");
    assert_eq!(platform.calls(), [r"open C:\Apps\Code.exe"]);
}

#[test]
fn pinned_menu_closes_all_windows_after_confirmation() {
    let (mut app, platform) = app("menu-pinned-many", FakePlatform::default());
    app.items[4].windows = vec![window(41), window(42)];
    open_menu(&mut app, ContextMenuTarget::Pinned(4));
    let mut harness = harness(app);
    click(&mut harness, "すべて閉じる");
    assert!(harness.state().confirm_close_all);
    assert!(harness.state().context_menu.is_some());
    click(&mut harness, "本当にすべて閉じる");
    assert_eq!(platform.calls(), ["close 41", "close 42"]);
    assert!(harness.state().context_menu.is_none());
    assert!(!harness.state().confirm_close_all);
}

#[test]
fn pinned_menu_switches_to_one_of_many_windows() {
    let (mut app, platform) = app("menu-pinned-pick", FakePlatform::default());
    app.items[4].windows = vec![window(41), window(42)];
    open_menu(&mut app, ContextMenuTarget::Pinned(4));
    let mut harness = harness(app);
    click(&mut harness, "window 42");
    assert_eq!(platform.calls(), ["foreground 42"]);
}

#[test]
fn pinned_menu_runs_as_admin_opens_location_and_shows_properties() {
    let (mut harness, platform) = menu_harness(
        "menu-file-actions",
        FakePlatform::default(),
        ContextMenuTarget::Pinned(4),
    );
    for label in ["管理者として実行", "ファイルの場所を開く", "プロパティ"] {
        open_menu(harness.state_mut(), ContextMenuTarget::Pinned(4));
        harness.run();
        click(&mut harness, label);
        assert!(harness.state().context_menu.is_none());
    }
    assert_eq!(
        platform.calls(),
        [
            r"RunAsAdmin C:\Apps\Code.exe",
            r"OpenLocation C:\Apps\Code.exe",
            r"Properties C:\Apps\Code.exe",
        ]
    );
}

#[test]
fn running_menu_shows_file_actions() {
    let (mut app, platform) = app("menu-running-file-actions", FakePlatform::default());
    app.running[0].command = r"C:\Apps\Chrome.exe".into();
    open_menu(&mut app, ContextMenuTarget::Running(0));
    let mut harness = harness(app);
    click(&mut harness, "プロパティ");
    assert_eq!(platform.calls(), [r"Properties C:\Apps\Chrome.exe"]);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn settings_menu_has_no_file_actions() {
    let (mut harness, _platform) = menu_harness(
        "menu-settings-uri",
        FakePlatform::default(),
        ContextMenuTarget::Pinned(3),
    );
    harness.run();
    assert!(harness.query_by_label("プロパティ").is_none());
    assert!(harness.query_by_label("ピン留めを外す").is_some());
}

#[test]
fn pinned_menu_unpins_registered_items() {
    let (mut harness, _platform) = menu_harness(
        "menu-unpin",
        FakePlatform::default(),
        ContextMenuTarget::Pinned(4),
    );
    click(&mut harness, "ピン留めを外す");
    assert_eq!(harness.state().items.len(), 4);
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn menus_for_removed_items_close_immediately() {
    for target in [ContextMenuTarget::Pinned(9), ContextMenuTarget::Running(9)] {
        let (mut harness, _platform) = menu_harness("menu-stale", FakePlatform::default(), target);
        harness.run();
        assert!(harness.state().context_menu.is_none());
    }
}

#[test]
fn running_menu_switches_windows_and_pins_apps() {
    let (mut harness, platform) = menu_harness(
        "menu-running",
        FakePlatform::default(),
        ContextMenuTarget::Running(0),
    );
    click(&mut harness, "window 21");
    assert_eq!(platform.calls(), ["foreground 21"]);

    open_menu(harness.state_mut(), ContextMenuTarget::Running(0));
    harness.run();
    click(&mut harness, "ピン留めする");
    assert_eq!(harness.state().items.last().unwrap().name, "Chrome");
    assert!(harness.state().context_menu.is_none());
}

#[test]
fn running_menu_lists_every_window() {
    let (mut harness, platform) = menu_harness(
        "menu-running-many",
        FakePlatform::default(),
        ContextMenuTarget::Running(1),
    );
    harness.run();
    assert!(harness.query_by_label("すべて閉じる").is_some());
    click(&mut harness, "window 32");
    assert_eq!(platform.calls(), ["foreground 32"]);
}

#[test]
fn menu_stays_hidden_until_first_frame_is_drawn() {
    let (mut app, _platform) = app("menu-reveal", FakePlatform::default());
    app.dock_side = DockSide::Left;
    app.context_menu = Some((
        ContextMenuTarget::Pinned(4),
        egui::pos2(0.0, 0.0),
        Instant::now(),
    ));
    let mut harness = harness(app);
    harness.step();
    assert!(harness.state().context_menu.is_some());
}

#[test]
fn menu_closes_on_escape_and_on_focus_loss() {
    let (mut harness, _platform) = menu_harness(
        "menu-escape",
        FakePlatform::default(),
        ContextMenuTarget::Clock,
    );
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().context_menu.is_none());

    open_menu(harness.state_mut(), ContextMenuTarget::Clock);
    set_focus(&mut harness, true);
    harness.step();
    assert!(harness.state().context_menu.is_some());
    set_focus(&mut harness, false);
    harness.step();
    assert!(harness.state().context_menu.is_none());
    assert!(!harness.state().context_menu_focused);
}

fn set_focus(harness: &mut Harness<'static, LauncherApp>, focused: bool) {
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .focused = Some(focused);
}

#[test]
fn menu_stays_open_until_it_has_had_focus() {
    let (mut app, _platform) = app("menu-grace", FakePlatform::default());
    // 開いてから時間がたっていても、まだフォーカスを受け取っていなければ閉じない。
    open_menu(&mut app, ContextMenuTarget::Clock);
    let mut harness = harness(app);
    set_focus(&mut harness, false);
    harness.step();
    harness.step();
    assert!(harness.state().context_menu.is_some());
}

fn picker_harness(test: &str) -> (Harness<'static, LauncherApp>, Rc<FakePlatform>) {
    let (mut app, platform) = app(test, FakePlatform::default());
    app.window_picker = Some((
        "Code".into(),
        vec![window(31), window(32)],
        egui::pos2(0.0, 0.0),
    ));
    (harness(app), platform)
}

#[test]
fn window_picker_switches_to_the_chosen_window() {
    let (mut harness, platform) = picker_harness("picker-choose");
    harness.run();
    assert!(harness
        .query_by_label("2個のウィンドウ — 現在のタイトルで選択")
        .is_some());
    click(&mut harness, "2.  window 32");
    assert_eq!(platform.calls(), ["foreground 32"]);
    assert!(harness.state().window_picker.is_none());
}

#[test]
fn window_picker_confirms_before_closing_everything() {
    let (mut harness, platform) = picker_harness("picker-close");
    click(&mut harness, "すべて閉じる");
    click(&mut harness, "キャンセル");
    assert!(!harness.state().confirm_close_all);
    click(&mut harness, "すべて閉じる");
    click(&mut harness, "本当にすべて閉じる");
    assert_eq!(platform.calls(), ["close 31", "close 32"]);
    assert!(harness.state().window_picker.is_none());
}

#[test]
fn window_picker_closes_on_escape() {
    let (mut harness, _platform) = picker_harness("picker-escape");
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.state().window_picker.is_none());
}

#[test]
fn window_picker_closes_when_its_window_is_closed() {
    let (mut harness, _platform) = picker_harness("picker-close-request");
    harness.run();
    harness
        .input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    harness.step();
    assert!(harness.state().window_picker.is_none());
}

#[test]
fn fits_menu_to_its_contents_before_showing_it() {
    let (mut app, _platform) = app("menu-fit", FakePlatform::default());
    open_menu(&mut app, ContextMenuTarget::Clock);
    let mut harness = harness(app);
    harness.step();
    let size = harness.state().context_menu_size.unwrap();
    assert!(size.x < CONTEXT_MENU_WIDTH && size.y < 54.0);
    harness.step();
    assert_eq!(harness.state().context_menu_size, Some(size));
}

#[test]
fn limits_menu_height_and_keeps_long_window_lists_scrollable() {
    let (mut app, platform) = app("menu-tall", FakePlatform::default());
    app.items[4].windows = (1..=20).map(window).collect();
    open_menu(&mut app, ContextMenuTarget::Pinned(4));
    let mut harness = harness(app);
    harness.step();
    assert!(harness.state().context_menu_size.unwrap().y <= 500.0);
    click(&mut harness, "window 1");
    assert_eq!(platform.calls(), ["foreground 1"]);
}
