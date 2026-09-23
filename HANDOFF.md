# Windows Side Dock 引き継ぎ書

## 概要

Windows向けの縦型Dock／ランチャー。Rust、`eframe 0.33`、`egui`、`windows-sys`で実装している。

- リポジトリ名: `windows-side-dock`
- Cargoパッケージ名: `windows-side-dock`
- 実行ファイル: `target\release\windows-side-dock.exe`
- メインウィンドウ名: `Windows Side Dock`
- 設定画面名: `Windows Side Dock 設定`

## ビルドと起動

```powershell
cargo check
cargo test
cargo build --release
Start-Process .\target\release\windows-side-dock.exe
```

MSI生成:

```powershell
.\scripts\build-installer.ps1
```

生成先は`dist\windows-side-dock-<version>-x64.msi`。WiX Toolset 6を使用する。

リリースビルドでは `windows_subsystem = "windows"` が有効になるため、PowerShell／コンソール画面は表示されない。実行中のexeをビルドし直す場合は、先に同じパスのプロセスを終了する必要がある。

## 現在の主な機能

- 画面右上を初期位置とする、移動・縦サイズ変更可能な枠なしDock
- 日付、曜日、時刻表示
- BIZ UDPゴシックの利用
- ブラックメタリック背景とシルバーのDock設定アイコン
- Windows Shellから取得した純正アプリアイコン
- ファイル／ショートカットのドラッグ＆ドロップ登録
- ピン留め領域と実行中アプリ領域の分離
- 実行中ウィンドウの列挙、前面化、最小化
- 複数ウィンドウを現在のタイトルで直接選択
- 複数ウィンドウの「すべて閉じる」（2段階確認）
- 子Viewportによるツールチップ、右クリックメニュー、設定画面
- ポップアップ方向の自動／常に左／常に右設定
- Task Manager／Process Explorerの切り替え
- Process Explorerのファイル選択ダイアログ
- Windows設定の `ms-settings:` 起動と既存ウィンドウへの移動
- Dock背景の右クリックメニュー

## 右クリック操作

- アプリアイコン: 起動、ウィンドウタイトル選択、ピン留め／解除、すべて閉じる
- 時計: 設定で選択したTask ManagerまたはProcess Explorer
- Dockの空白背景: Windows Side Dockの場所、プロセスツール、Dock設定
- 移動ハンドル: Dock設定

右クリックメニューとツールチップは、親Viewport内へ制限されないよう子Viewportで実装している。この方針は過去のユーザー判断によるものなので、egui標準Popupへ安易に戻さないこと。

## 設定ファイル

アプリ名変更前との互換性を維持するため、保存先のフォルダー名は現在も `lancher` のまま。

- `%LOCALAPPDATA%\lancher\items.txt`
- `%LOCALAPPDATA%\lancher\settings.txt`
- `%LOCALAPPDATA%\lancher\process_tool.txt`
- `%LOCALAPPDATA%\lancher\process_explorer_path.txt`

保存先を変更する場合は、旧フォルダーからの移行処理を先に実装すること。

## Windows右クリックメニュー

デスクトップおよびエクスプローラーの空白背景に「Windows Side Dock」サブメニューを追加している。登録先はユーザー単位の次のレジストリ。

- `HKCU\Software\Classes\DesktopBackground\Shell\WindowsSideDock`
- `HKCU\Software\Classes\Directory\Background\Shell\WindowsSideDock`

MSI（`installer\Package.wxs`）が登録・解除する項目は次の2つ。

- `01Folder`: Windows Side Dockの場所を開く
- `02ProcessTool`: プロセスツール（MSIの初期値はタスク マネージャー）

Dock本体（`src/shell_menu.rs`）が起動時と設定変更時に `02ProcessTool` の `MUIVerb` と `command` を書き換え、Dock設定のTask Manager／Process Explorerに合わせる。Process Explorerのパスが空ならタスク マネージャーに戻す。キーが存在しない（MSI未インストール）場合は何もしないため、開発ビルドを直接起動してもメニューは作られない。値の名前はMSIと同じなので、アンインストール時はMSIがまとめて削除する。

Windows 11では「その他のオプションを確認」側に表示される場合がある。

0.1.4で旧 `02TaskManager` を廃止し、手動登録だった `02ProcessTool` をMSI管理下へ移した（メニュー3項目の重複を解消、実機確認済み）。

## 重要な実装上の注意

- ピン留めアイコンの通常クリックは、原則として実行中でも新しいインスタンスを起動する。
- Windows設定だけは単一ウィンドウとして扱い、既に開いていれば前面へ移動する。
- 通常表示中のウィンドウへ移動するときは `SW_RESTORE` を呼ばない。最小化中だけ復元する。無条件復元はちらつきの原因になる。
- 子Viewportは最初の未描画フレームを非表示にしてから表示する。ツールチップと右クリックメニューのちらつき対策。
- ツールチップ表示中は位置を固定し、クリック成立フレームでは生成しない。
- 右クリックメニューとウィンドウ選択画面は、横位置をDockの端、縦位置をカーソルの高さに合わせる（`layout::beside_dock_at_cursor`）。カーソル基準に戻すとDockに重なる。
- Dockは最大化／全画面化を解除し、独自ドラッグで移動する。Windowsスナップによる不自然な挙動を避けるため。
- Process Explorerは現在 `E:\Downloads\ProcessExplorer\procexp.exe` が設定されている。

## テストとカバレッジ

- 現在の自動テスト: 23件（`model.rs`、`config.rs`、`layout.rs`、`ui.rs`、`shell_menu.rs`）
- 全体行カバレッジ: 15.26%（`cargo llvm-cov --summary-only`）
- `model.rs` 行カバレッジ: 100%
- `config.rs` 行カバレッジ: 58.92%
- `layout.rs` 行カバレッジ: 11.43%
- `cargo-llvm-cov 0.9.1` はインストール済み

テスト済み: アプリ同一判定、ウィンドウのグループ化（`model::group_windows`）、タイトル正規化、設定値と `items.txt` の読み書き、`lancher` 保存先パス、ポップアップ位置計算（`layout::beside_x`）、実行ファイルパス正規化。

テスト方針: Win32 APIやファイルI/Oを呼ぶ関数から純粋関数を切り出し、そちらをテストする。ポップアップ幅は `layout.rs` の `*_WIDTH` 定数を唯一の値とし、表示サイズと位置計算の両方で使うこと（以前、ウィンドウ選択画面の幅だけ変更され、左表示時にDockへ120px重なる不具合があった）。

残り: 旧設定からの移行処理（保存先変更時に実装してテストする）、`app.rs` の状態操作。

## 技術的負債と次の作業

`src/main.rs` は41行で、起動処理だけを持つ。全Rustソースを400行未満へ分割済み。

- `model.rs`: `LauncherItem`、`RunningWindow`、各enum
- `config.rs`: 設定と登録項目の永続化
- `platform.rs`: Win32 API、Shell起動、ウィンドウ列挙、アイコン取得
- `app.rs`: アプリ状態と操作
- `dock.rs`: Dock本体と設定画面
- `context_menu.rs`: 右クリックメニューとウィンドウ選択
- `layout.rs`: 子Viewportの位置・サイズ計算
- `shell_menu.rs`: Windows背景メニューのプロセスツール項目の同期
- `ui.rs`: アイコン操作と項目生成
- `theme.rs`: フォント、色、独自アイコン描画

優先度が高い未完了事項:

1. `app.rs` の状態操作（登録・ピン留め・実行中判定）のテスト追加
2. 設定保存先を `windows-side-dock` へ安全に移行

GitHub Releasesを使った自動更新はユーザー判断により対象外（2026-09-23）。更新は新しいMSIを手動で実行する方式とする。

## インストールとアップグレード

- MSI定義: `installer\Package.wxs`
- ビルドスクリプト: `scripts\build-installer.ps1`
- インストール範囲: ユーザー単位
- インストール先: `%LOCALAPPDATA%\Programs\Windows Side Dock`
- Package ID: `ZeroPlatformLab.WindowsSideDock`（変更しないこと）
- バージョン元: `Cargo.toml`
- 現在のバージョン: `0.1.5`
- `build-installer.ps1` はUTF-8のため、Windows PowerShell 5.1ではなくPowerShell 7（`pwsh`）で実行すること
- `MajorUpgrade`で旧版を置換し、ダウングレードを拒否
- 同一バージョンの開発用再インストールを許可
- ユーザー設定フォルダーはMSI管理対象外なのでアップグレード／アンインストールで保持
- デスクトップとフォルダー背景の右クリックメニューはMSIが登録・解除
- 新規インストール／アップグレード完了後にアプリを自動起動

0.1.0をインストール後に0.1.1を適用する実機アップグレードテスト済み。両方とも`msiexec`終了コード0。
さらに、起動中の0.1.1へ0.1.2を適用し、既存プロセス2個が終了して新プロセス1個が自動起動することを確認済み。

## Gitと作業ツリー

- 変更は小さく分けてコミットする方針
- 直近の機能コミット: 背景メニューのプロセスツール同期（0.1.4）
- `windows-side-dock-screenshot.png` はユーザー指示によりGitへ追加しない
- 既存のユーザー変更を破棄する `git reset --hard` 等は使用しない
