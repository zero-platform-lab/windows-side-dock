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

2026-09-23時点で、ユーザー単位の次のレジストリへ手動登録済み。

- `HKCU\Software\Classes\DesktopBackground\Shell\WindowsSideDock`
- `HKCU\Software\Classes\Directory\Background\Shell\WindowsSideDock`

デスクトップおよびエクスプローラーの空白背景に「Windows Side Dock」サブメニューを追加している。現在の登録内容は次の2項目。

- Windows Side Dockの場所を開く
- Process Explorer

Windows 11では「その他のオプションを確認」側に表示される場合がある。現状、この登録処理はアプリのコードやインストーラーへ組み込まれていない。また、Process Explorerの設定変更時にレジストリは自動更新されない。

## 重要な実装上の注意

- ピン留めアイコンの通常クリックは、原則として実行中でも新しいインスタンスを起動する。
- Windows設定だけは単一ウィンドウとして扱い、既に開いていれば前面へ移動する。
- 通常表示中のウィンドウへ移動するときは `SW_RESTORE` を呼ばない。最小化中だけ復元する。無条件復元はちらつきの原因になる。
- 子Viewportは最初の未描画フレームを非表示にしてから表示する。ツールチップと右クリックメニューのちらつき対策。
- ツールチップ表示中は位置を固定し、クリック成立フレームでは生成しない。
- Dockは最大化／全画面化を解除し、独自ドラッグで移動する。Windowsスナップによる不自然な挙動を避けるため。
- Process Explorerは現在 `E:\Downloads\ProcessExplorer\procexp.exe` が設定されている。

## テストとカバレッジ

- 現在の自動テスト: 6件（`model.rs`）
- 全体行カバレッジ: 4.80%
- `model.rs` 行カバレッジ: 98.75%
- `cargo-llvm-cov 0.9.1` はインストール済み

優先してテストすべき対象:

1. アプリ名と実行ファイルの同一判定
2. ウィンドウのグループ化
3. 設定値の読み書きと旧設定移行
4. ポップアップ位置計算
5. タイトル正規化

## 技術的負債と次の作業

`src/main.rs` は約2,000行あり、責務を持ちすぎている。挙動を固定するテストを追加してから、次の単位へ分割する。

- `model.rs`: `LauncherItem`、`RunningWindow`、各enum
- `config.rs`: 設定と登録項目の永続化
- `windows.rs`: Win32 API、Shell起動、ウィンドウ列挙、アイコン取得
- `app.rs`: アプリ状態と操作
- `ui/`: Dock、設定、右クリックメニュー、ウィンドウ選択
- `theme.rs`: フォント、色、独自アイコン描画

優先度が高い未完了事項:

1. テスト追加とカバレッジ計測
2. `main.rs` の段階的分割
3. Windows背景メニューの登録／解除をアプリ設定へ統合
4. Process Explorer切り替え時のWindows背景メニュー自動更新
5. 設定保存先を `windows-side-dock` へ安全に移行

## Gitと作業ツリー

- 変更は小さく分けてコミットする方針
- 直近の機能コミット: `37b9200 feat: add background context menu actions`
- `windows-side-dock-screenshot.png` はユーザー指示によりGitへ追加しない
- 既存のユーザー変更を破棄する `git reset --hard` 等は使用しない
