//! 画面の端に確保するDockの場所と、Dockをしまう・引き出す操作。
//! 端を確保すると最大化したウィンドウはDockの手前で止まる。しまうと確保はつまみの幅だけになる。

use crate::app::{ContextMenuTarget, LauncherApp};
use crate::config::DockSide;
use crate::layout::{directional_tooltip, DOCK_MARGIN, DOCK_WIDTH};
use eframe::egui::{self, Color32};

/// しまったときに画面の端に残すつまみの幅。
pub(crate) const COLLAPSED_WIDTH: f32 = 12.0;
const MIN_DOCK_HEIGHT: f32 = 220.0;

/// 確保する幅（論理ポイント）。引き出しているときはDockと左右の余白、しまっているときはつまみの幅。
pub(crate) fn reserved_width(collapsed: bool) -> f32 {
    if collapsed {
        COLLAPSED_WIDTH
    } else {
        DOCK_WIDTH + DOCK_MARGIN * 2.0
    }
}

/// 確保した範囲 `edge`（物理ピクセル）の中に置くDockの位置と大きさ（論理ポイント）。
/// `scale` は1論理ポイントあたりの物理ピクセル数。
pub(crate) fn dock_in_edge(
    edge: egui::Rect,
    collapsed: bool,
    scale: f32,
) -> (egui::Pos2, egui::Vec2) {
    let edge = egui::Rect::from_min_max(edge.min / scale, edge.max / scale);
    if collapsed {
        (edge.min, egui::vec2(COLLAPSED_WIDTH, edge.height()))
    } else {
        (
            edge.min + egui::vec2(DOCK_MARGIN, DOCK_MARGIN),
            egui::vec2(DOCK_WIDTH, edge.height() - DOCK_MARGIN * 2.0),
        )
    }
}

/// 端を確保できなかったときの代わりの範囲。作業領域の `side` の端を使う。
fn fallback_edge(work_area: egui::Rect, side: DockSide, width: f32) -> egui::Rect {
    let mut edge = work_area;
    match side {
        DockSide::Left => edge.max.x = work_area.left() + width,
        DockSide::Right => edge.min.x = work_area.right() - width,
    }
    edge
}

/// つまみは画面の内側だけ角を丸める。
fn tab_corners(side: DockSide) -> egui::CornerRadius {
    match side {
        DockSide::Left => egui::CornerRadius {
            ne: 6,
            se: 6,
            ..Default::default()
        },
        DockSide::Right => egui::CornerRadius {
            nw: 6,
            sw: 6,
            ..Default::default()
        },
    }
}

impl LauncherApp {
    /// Dockを置く端を変える。次のフレームで確保し直して移る。
    pub(crate) fn set_dock_side(&mut self, side: DockSide) {
        self.dock_side = side;
        self.config.save_dock_side(side);
        self.applied_collapsed = None;
    }

    pub(crate) fn set_collapsed(&mut self, collapsed: bool) {
        self.collapsed = collapsed;
        self.context_menu = None;
        self.window_picker = None;
    }

    /// しまう・引き出すが変わったときだけ、画面の端の確保とDockの位置・大きさを合わせる。
    pub(crate) fn apply_edge(&mut self, ctx: &egui::Context) {
        if self.applied_collapsed == Some(self.collapsed) {
            return;
        }
        self.applied_collapsed = Some(self.collapsed);
        let scale = ctx.native_pixels_per_point().unwrap_or(1.0);
        let width = reserved_width(self.collapsed) * scale;
        let side = self.dock_side;
        let Some(edge) = self
            .platform
            .reserve_edge(side, Some(width))
            .or_else(|| Some(fallback_edge(self.platform.work_area()?, side, width)))
        else {
            return;
        };
        let (position, size) = dock_in_edge(edge, self.collapsed, scale);
        let root = egui::ViewportId::ROOT;
        for command in [
            egui::ViewportCommand::MinInnerSize(egui::vec2(size.x, MIN_DOCK_HEIGHT)),
            egui::ViewportCommand::MaxInnerSize(egui::vec2(size.x, 4000.0)),
            egui::ViewportCommand::InnerSize(size),
            egui::ViewportCommand::OuterPosition(position),
        ] {
            ctx.send_viewport_cmd_to(root, command);
        }
    }

    /// 終了時に画面の端の確保をやめ、作業領域を元に戻す。
    pub(crate) fn release_edge(&mut self) {
        self.platform.reserve_edge(self.dock_side, None);
    }

    /// しまっている間に画面の端に出すつまみ。クリックで引き出し、右クリックでDockのメニューを開く。
    pub(crate) fn show_collapsed_tab(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(8, 9, 12, 235))
                    .stroke(egui::Stroke::new(
                        1.0_f32,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 55),
                    ))
                    .corner_radius(tab_corners(self.dock_side)),
            )
            .show(ctx, |ui| {
                let area = ui.max_rect();
                let tab = ui.interact(area, ui.id().with("collapsed-tab"), egui::Sense::click());
                tab.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Dockを引き出す")
                });
                let center = area.center();
                ui.painter().line_segment(
                    [
                        center - egui::vec2(0.0, 18.0),
                        center + egui::vec2(0.0, 18.0),
                    ],
                    egui::Stroke::new(
                        3.0_f32,
                        Color32::from_gray(if tab.hovered() { 200 } else { 130 }),
                    ),
                );
                directional_tooltip(
                    self.platform.as_ref(),
                    &tab,
                    "Dockを引き出す",
                    self.alignment(),
                );
                if tab.clicked() {
                    self.set_collapsed(false);
                }
                if tab.secondary_clicked() {
                    self.open_context_menu(ContextMenuTarget::Handle);
                }
            });
    }

    /// Dockをしまうボタン。画面の端へ押し込む向きの矢印を描く。
    pub(crate) fn collapse_button(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(40.0, 16.0), egui::Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "Dockをしまう")
        });
        let color = Color32::from_gray(if response.hovered() { 220 } else { 145 });
        // 矢印はDockのある端を向く。
        let toward = match self.dock_side {
            DockSide::Left => -1.0,
            DockSide::Right => 1.0,
        };
        let center = rect.center();
        for offset in [-3.0, 3.0] {
            let tip = center + egui::vec2((offset + 2.0) * toward, 0.0);
            for rise in [-4.0, 4.0] {
                ui.painter().line_segment(
                    [tip + egui::vec2(-4.0 * toward, rise), tip],
                    egui::Stroke::new(1.5_f32, color),
                );
            }
        }
        if response.clicked() {
            self.set_collapsed(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f32, top: f32, right: f32, bottom: f32) -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(right, bottom))
    }

    #[test]
    fn reserves_the_dock_with_margins_or_just_the_tab() {
        assert_eq!(reserved_width(false), 78.0);
        assert_eq!(reserved_width(true), COLLAPSED_WIDTH);
    }

    #[test]
    fn places_the_dock_inside_the_reserved_edge() {
        let edge = rect(1842.0, 0.0, 1920.0, 1032.0);
        assert_eq!(
            dock_in_edge(edge, false, 1.0),
            (egui::pos2(1854.0, 12.0), egui::vec2(54.0, 1008.0))
        );
        assert_eq!(
            dock_in_edge(rect(1908.0, 0.0, 1920.0, 1032.0), true, 1.0),
            (egui::pos2(1908.0, 0.0), egui::vec2(12.0, 1032.0))
        );
        // 拡大率200%では物理ピクセルを半分にして論理ポイントへ直す。
        assert_eq!(
            dock_in_edge(rect(3684.0, 0.0, 3840.0, 2064.0), false, 2.0),
            (egui::pos2(1854.0, 12.0), egui::vec2(54.0, 1008.0))
        );
    }

    #[test]
    fn falls_back_to_the_work_area_edge() {
        let work_area = rect(0.0, 0.0, 1920.0, 1032.0);
        assert_eq!(
            fallback_edge(work_area, DockSide::Right, 78.0),
            rect(1842.0, 0.0, 1920.0, 1032.0)
        );
        assert_eq!(
            fallback_edge(work_area, DockSide::Left, 78.0),
            rect(0.0, 0.0, 78.0, 1032.0)
        );
    }

    #[test]
    fn rounds_only_the_inner_corners_of_the_tab() {
        assert_eq!(tab_corners(DockSide::Right).nw, 6);
        assert_eq!(tab_corners(DockSide::Right).ne, 0);
        assert_eq!(tab_corners(DockSide::Left).ne, 6);
        assert_eq!(tab_corners(DockSide::Left).nw, 0);
    }
}
