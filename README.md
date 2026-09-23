# Windows Side Dock

Rustと`eframe/egui`で作ったWindows向けのDock型アプリランチャーです。

## 機能

- アイコンをクリックしてアプリ、ファイル、フォルダーを起動
- 左右キーで選択し、`Enter`で起動
- ファイルやショートカットをウィンドウへドロップして登録
- 最近起動した項目を右側へ移動
- `Esc`で終了

## 起動

```powershell
cargo run
```

初期状態ではエクスプローラー、ターミナル、メモ帳、Windows設定が登録されています。
追加した項目は`%LOCALAPPDATA%\lancher\items.txt`に保存されます。

## 配布用ビルド

```powershell
cargo build --release
```

実行ファイルは`target\release\windows-side-dock.exe`に作成されます。
