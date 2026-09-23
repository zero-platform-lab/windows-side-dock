use crate::app::{ContextMenuTarget, LauncherApp};
use crate::config::ProcessTool;
use crate::layout::{CONTEXT_MENU_WIDTH, WINDOW_PICKER_WIDTH};
use crate::model::RunningWindow;
use crate::platform::{activate_windows, close_windows, FileAction};
use crate::theme::left_aligned_button;
use crate::ui::normalized_executable_path;
use eframe::egui::{self, Color32, Key};
use std::time::Duration;

/// ウィンドウ一覧を含むメニューの幅。
const WINDOW_MENU_WIDTH: f32 = 430.0;
/// 開いた直後の未描画フレームを隠す時間。子Viewportのちらつき対策。
pub(crate) const MENU_REVEAL_DELAY: Duration = Duration::from_millis(16);

/// メニュー枠の内側の余白と枠線を合わせた、中身の大きさに足す分。
const MENU_CHROME: egui::Vec2 = egui::vec2(8.0 * 2.0 + 2.0, 7.0 * 2.0 + 2.0);

/// 右クリックメニューの中身を測るときの幅と、高さの上限。実際の大きさは中身に合わせる。
/// `window_count` は対象アプリのウィンドウ数（対象が消えていれば `None`）。
fn menu_size(target: ContextMenuTarget, window_count: Option<usize>) -> (f32, f32) {
    let (single, multiple_base) = match target {
        ContextMenuTarget::Handle => return (CONTEXT_MENU_WIDTH, 156.0),
        ContextMenuTarget::Clock => return (CONTEXT_MENU_WIDTH, 54.0),
        ContextMenuTarget::Pinned(_) => (256.0, 252.0),
        ContextMenuTarget::Running(_) => (222.0, 218.0),
    };
    match window_count {
        Some(count) if count > 1 => (
            WINDOW_MENU_WIDTH,
            (multiple_base + count as f32 * 34.0).min(500.0),
        ),
        Some(1) => (WINDOW_MENU_WIDTH, single),
        _ => (CONTEXT_MENU_WIDTH, 208.0),
    }
}

/// アイコンのメニューの先頭に、何のアプリのメニューかを示す名前を出す。
fn menu_title(ui: &mut egui::Ui, name: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(name)
                .strong()
                .color(Color32::from_rgb(225, 229, 238)),
        )
        .truncate(),
    );
    ui.separator();
}

/// 左へ開くときは、幅が広がった分だけ左へずらしてDockに重ならないようにする。
fn menu_position(position: egui::Pos2, width: f32, opens_left: bool) -> egui::Pos2 {
    if opens_left {
        egui::pos2(position.x - (width - CONTEXT_MENU_WIDTH), position.y)
    } else {
        position
    }
}

fn close_requested(ctx: &egui::Context) -> bool {
    ctx.input(|input| input.viewport().close_requested() || input.key_pressed(Key::Escape))
}

impl LauncherApp {
    fn target_window_count(&self, target: ContextMenuTarget) -> Option<usize> {
        match target {
            ContextMenuTarget::Pinned(index) => self.items.get(index),
            ContextMenuTarget::Running(index) => self.running.get(index),
            ContextMenuTarget::Handle | ContextMenuTarget::Clock => None,
        }
        .map(|item| item.windows.len())
    }

    fn process_tool_label(&self) -> &'static str {
        match self.process_tool {
            ProcessTool::TaskManager => "タスク マネージャー",
            ProcessTool::ProcessExplorer => "Process Explorer",
        }
    }

    /// プロセスツールの起動ボタン。押されたら `true`。
    fn process_tool_button(&mut self, ui: &mut egui::Ui) -> bool {
        let clicked = ui
            .add_enabled(
                self.process_tool_ready(),
                egui::Button::new(self.process_tool_label()),
            )
            .clicked();
        if clicked {
            self.launch_process_tool();
        }
        clicked
    }

    /// 「すべて閉じる」の2段階確認。閉じたら `true`。
    fn close_all_buttons(&mut self, ui: &mut egui::Ui, windows: &[RunningWindow]) -> bool {
        if !self.confirm_close_all {
            if ui.button("すべて閉じる").clicked() {
                self.confirm_close_all = true;
            }
            return false;
        }
        let confirmed = ui
            .button(
                egui::RichText::new("本当にすべて閉じる").color(Color32::from_rgb(255, 120, 120)),
            )
            .clicked();
        if confirmed {
            close_windows(self.platform.as_ref(), windows);
        }
        confirmed
    }

    /// アプリのウィンドウをタイトルで選ぶ一覧。いずれかを選ぶか、すべて閉じたら `true`。
    fn window_menu(
        &mut self,
        ui: &mut egui::Ui,
        windows: &[RunningWindow],
        list_height: f32,
    ) -> bool {
        ui.label("ウィンドウへ移動");
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .max_height(list_height)
            .show(ui, |ui| {
                for window in windows {
                    if left_aligned_button(ui, &window.title, 30.0)
                        .on_hover_text(&window.title)
                        .clicked()
                    {
                        chosen = Some(window.clone());
                    }
                }
            });
        if let Some(window) = &chosen {
            activate_windows(self.platform.as_ref(), std::slice::from_ref(window));
        }
        let closed_all = windows.len() > 1 && self.close_all_buttons(ui, windows);
        chosen.is_some() || closed_all
    }

    /// 右クリックメニューの中身。メニューを閉じるべきなら `true`。
    fn context_menu_contents(
        &mut self,
        ui: &mut egui::Ui,
        target: ContextMenuTarget,
        height: f32,
    ) -> bool {
        match target {
            ContextMenuTarget::Handle => {
                let label = if self.collapsed {
                    "Dockを引き出す"
                } else {
                    "Dockをしまう"
                };
                if ui.button(label).clicked() {
                    self.set_collapsed(!self.collapsed);
                    return true;
                }
                if ui.button("Windows Side Dockの場所を開く").clicked() {
                    if let Some(directory) = self.platform.install_directory() {
                        let _ = self.platform.open_target(&directory.to_string_lossy());
                    }
                    return true;
                }
                if self.process_tool_button(ui) {
                    return true;
                }
                ui.separator();
                let settings = ui.button("Dock 設定").clicked();
                self.show_settings |= settings;
                settings
            }
            ContextMenuTarget::Clock => {
                if self.process_tool_button(ui) {
                    return true;
                }
                let set_path =
                    !self.process_tool_ready() && ui.small_button("パスを設定…").clicked();
                self.show_settings |= set_path;
                set_path
            }
            ContextMenuTarget::Pinned(index) => {
                let Some(item) = self.items.get(index) else {
                    return true;
                };
                let windows = item.windows.clone();
                let command = item.command.clone();
                menu_title(ui, &item.name);
                let launch_label = if windows.is_empty() {
                    "起動"
                } else {
                    "新しく起動"
                };
                if ui.button(launch_label).clicked() {
                    self.launch(index);
                    return true;
                }
                if !windows.is_empty() && self.window_menu(ui, &windows, (height - 245.0).max(68.0))
                {
                    return true;
                }
                ui.separator();
                if self.file_action_buttons(ui, &command) {
                    return true;
                }
                let unpin = ui.button("ピン留めを外す").clicked();
                if unpin {
                    self.unpin(index);
                }
                unpin
            }
            ContextMenuTarget::Running(index) => {
                let Some(item) = self.running.get(index) else {
                    return true;
                };
                let windows = item.windows.clone();
                let command = item.command.clone();
                menu_title(ui, &item.name);
                if self.window_menu(ui, &windows, (height - 211.0).max(68.0)) {
                    return true;
                }
                ui.separator();
                if self.file_action_buttons(ui, &command) {
                    return true;
                }
                let pin = ui.button("ピン留めする").clicked();
                if pin {
                    self.pin_running(index);
                }
                pin
            }
        }
    }

    /// エクスプローラーの右クリックメニューと同じ操作のボタン。押されたら `true`。
    /// Windows 設定（`ms-settings:`）とエクスプローラーには出さない。設定はファイルでなく、
    /// エクスプローラーは管理者として起動しても通常の権限で開き直し、場所やプロパティも
    /// Windowsフォルダーの explorer.exe を指すだけで役に立たない（2026-09-23 ユーザー判断）。
    fn file_action_buttons(&mut self, ui: &mut egui::Ui, command: &str) -> bool {
        let path = normalized_executable_path(command);
        let explorer = path.to_ascii_lowercase().ends_with(r"\explorer.exe");
        if explorer || !std::path::Path::new(&path).is_absolute() {
            return false;
        }
        for (label, action) in [
            ("管理者として実行", FileAction::RunAsAdmin),
            ("ファイルの場所を開く", FileAction::OpenLocation),
            ("プロパティ", FileAction::Properties),
        ] {
            if ui.button(label).clicked() {
                self.platform.file_action(action, &path);
                return true;
            }
        }
        ui.separator();
        false
    }

    pub(crate) fn show_context_menu_viewport(&mut self, ctx: &egui::Context) {
        let Some((target, position, opened_at)) = self.context_menu else {
            return;
        };
        let (max_width, max_height) = menu_size(target, self.target_window_count(target));
        let measured = self.context_menu_size;
        // 幅は中身に合わせ、高さだけ上限で抑える（それを超える分はウィンドウ一覧がスクロールする）。
        let size = measured.map_or(egui::vec2(max_width, max_height), |size| {
            egui::vec2(size.x, size.y.min(max_height))
        });
        let position = menu_position(position, size.x, self.opens_left());
        // 中身を測り終え、最初の描画が済むまでは表示しない。
        let revealed = measured.is_some() && opened_at.elapsed() >= MENU_REVEAL_DELAY;
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("launcher-context-menu"),
            egui::ViewportBuilder::default()
                .with_title("Launcher menu")
                .with_inner_size(size)
                .with_position(position)
                .with_decorations(false)
                .with_resizable(false)
                .with_transparent(true)
                .with_taskbar(false)
                .with_always_on_top()
                .with_visible(revealed)
                .with_active(true),
            |menu_ctx, _class| {
                if !revealed {
                    menu_ctx.request_repaint_after(MENU_REVEAL_DELAY);
                }
                // 一度フォーカスを受け取ってから失ったときだけ閉じる。起動直後の最初のメニューは
                // ウィンドウの作成に時間がかかり、フォーカスを受け取る前に閉じてしまっていた。
                let focused = menu_ctx.input(|input| input.viewport().focused);
                self.context_menu_focused |= focused == Some(true);
                let lost_focus = self.context_menu_focused && focused == Some(false);
                close = close_requested(menu_ctx) || lost_focus;
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(Color32::from_rgba_unmultiplied(12, 14, 19, 252))
                            .stroke(egui::Stroke::new(1.0_f32, Color32::from_gray(80)))
                            .corner_radius(8.0)
                            .inner_margin(egui::Margin::symmetric(8, 7)),
                    )
                    .show(menu_ctx, |ui| {
                        let builder = if measured.is_none() {
                            egui::UiBuilder::new().sizing_pass()
                        } else {
                            egui::UiBuilder::new()
                        };
                        let contents = ui.scope_builder(builder, |ui| {
                            self.context_menu_contents(ui, target, max_height)
                        });
                        if measured.is_none() {
                            self.context_menu_size =
                                Some(contents.response.rect.size() + MENU_CHROME);
                        }
                        close |= contents.inner;
                    });
                if close {
                    menu_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    // 設定画面を開いた場合などに、Dock本体をすぐ描き直す。
                    menu_ctx.request_repaint();
                }
            },
        );
        if close {
            self.context_menu = None;
            self.context_menu_size = None;
            self.context_menu_focused = false;
            self.confirm_close_all = false;
        }
    }

    pub(crate) fn show_window_picker_viewport(&mut self, ctx: &egui::Context) {
        let Some((name, windows, position)) = self.window_picker.clone() else {
            return;
        };
        let height = (windows.len() as f32 * 38.0 + 116.0).min(420.0);
        let mut close = false;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("launcher-window-picker"),
            egui::ViewportBuilder::default()
                .with_title(format!("{name} のウィンドウ"))
                .with_inner_size([WINDOW_PICKER_WIDTH, height])
                .with_position(position)
                .with_resizable(false)
                .with_taskbar(false)
                .with_always_on_top()
                .with_active(true),
            |picker_ctx, _class| {
                close = close_requested(picker_ctx);
                egui::CentralPanel::default().show(picker_ctx, |ui| {
                    ui.heading(&name);
                    ui.label(format!(
                        "{}個のウィンドウ — 現在のタイトルで選択",
                        windows.len()
                    ));
                    ui.separator();
                    let mut chosen = None;
                    egui::ScrollArea::vertical()
                        .max_height((height - 105.0).max(60.0))
                        .show(ui, |ui| {
                            for (index, window) in windows.iter().enumerate() {
                                let label = format!("{}.  {}", index + 1, window.title);
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 32.0],
                                        egui::Button::new(label),
                                    )
                                    .on_hover_text(&window.title)
                                    .clicked()
                                {
                                    chosen = Some(window.clone());
                                }
                            }
                        });
                    if let Some(window) = &chosen {
                        activate_windows(self.platform.as_ref(), std::slice::from_ref(window));
                    }
                    ui.separator();
                    let closed_all = ui
                        .horizontal(|ui| {
                            let closed = self.close_all_buttons(ui, &windows);
                            if self.confirm_close_all && ui.button("キャンセル").clicked() {
                                self.confirm_close_all = false;
                            }
                            closed
                        })
                        .inner;
                    close |= chosen.is_some() || closed_all;
                });
                if close {
                    picker_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            },
        );
        if close {
            self.window_picker = None;
            self.confirm_close_all = false;
        }
    }
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;
