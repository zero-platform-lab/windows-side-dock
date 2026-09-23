//! Windows 11 の右クリックメニュー（デスクトップとフォルダーの背景）に「Windows Side Dock」を出す
//! シェル拡張。`IExplorerCommand` を実装したCOMサーバーで、スパースパッケージ
//! （`installer/sparse/AppxManifest.xml`）がエクスプローラーに登録する。
//!
//! 項目の名前とコマンドは、MSIが作りDockが設定に合わせて書き換える従来メニューのレジストリから読む。
//! そのためDockの設定（タスク マネージャー／Process Explorer）の切り替えがそのまま反映される。

pub mod command_line;

#[cfg(windows)]
mod com;

#[cfg(windows)]
pub use com::CLSID_WINDOWS_SIDE_DOCK_MENU;
