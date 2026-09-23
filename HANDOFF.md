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

0.1.6から保存先は `%LOCALAPPDATA%\windows-side-dock\`。

- `items.txt`: ユーザー登録項目（`名前|コマンド` を1行ずつ）
- `settings.txt`: ポップアップ方向
- `process_tool.txt`: Task Manager／Process Explorer
- `process_explorer_path.txt`: Process Explorerのパス

起動時に `config::migrate_legacy_config` が旧保存先 `%LOCALAPPDATA%\lancher\` から上記4ファイルをコピーする。新フォルダーに既にあるファイルは上書きしない。旧フォルダーは削除せず残している（0.1.6への移行は実機確認済み。4ファイルとも内容一致）。旧フォルダーを消す処理を入れる場合は、十分な期間を置いてからにすること。

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
- 右クリックメニューとウィンドウ選択画面は、横位置をDockの端、縦位置をカーソルの高さに合わせる（`layout::beside_dock_at_cursor`）。カーソル基準に戻すとDockに重なる。0.1.5で実機確認済み。
- Dockは最大化／全画面化を解除し、独自ドラッグで移動する。Windowsスナップによる不自然な挙動を避けるため。
- Process Explorerは現在 `E:\Downloads\ProcessExplorer\procexp.exe` が設定されている。

## テストとカバレッジ

ユーザー指示により、技術的にテストできない部分を除いてC1（分岐）カバレッジ100%を維持する（2026-09-23）。

- 計測: `.\scripts\coverage.ps1`（HTMLで見る場合は `-Html`）。nightly、llvm-tools、`cargo-llvm-cov 0.9.1` が必要で、いずれもインストール済み
- 現在: 自動テスト110件。分岐 258/258、行・リージョン・関数とも100%
- スクリプトは毎回 `cargo llvm-cov clean` してから測る。古いテスト実行ファイルが残ると、行番号のずれた誤った結果になるため
- 計測対象外（`coverage(off)`）: `win32.rs`（OSを実際に操作する層）と `main()`（eframe起動）だけ。ここへ判断ロジックを置かないこと

テストの仕組み:

- OSへの問い合わせと操作はすべて `platform::Platform` トレイト経由。本番は `win32::WindowsPlatform`、テストは `platform_fake.rs` の `FakePlatform`（操作を `calls()` に記録）
- 設定ファイルは `config::ConfigStore` 経由。テストは `config::temp_root` の一時フォルダーを使う
- 画面は `egui_kittest` でテストする。部品はアクセシビリティ名（`get_by_label`）で探すので、文字のないクリック部品には `widget_info` で名前を付けること（`dock::label_widget`、`ui::icon_slot`、`theme::left_aligned_button`）
- テスト環境では子Viewportが本体に埋め込まれて描かれる。メニューを開いた直後やホバー中は再描画要求が続くため、`harness.run()` ではなく `harness.step()` を使う
- キー入力やドラッグはイベントごとに1フレームずつ処理される。ウィンドウ操作（`ViewportCommand`）はそのフレームの出力で確認する（`dock_tests.rs` の `press`、`drag`）

ポップアップ幅は `layout.rs` の `*_WIDTH` 定数を唯一の値とし、表示サイズと位置計算の両方で使うこと（以前、ウィンドウ選択画面の幅だけ変更され、左表示時にDockへ120px重なる不具合があった）。

## 技術的負債と次の作業

本体のRustソースは400行未満に保っている（テスト専用ファイルは除く）。

- `model.rs`: `LauncherItem`、`RunningWindow`、ウィンドウのグループ化、実行中アプリの割り当て
- `config.rs`: `ConfigStore`（設定と登録項目の永続化、旧保存先からの移行）
- `platform.rs`: `Platform` トレイトと、それを使うウィンドウ操作の判断
- `win32.rs`: `Platform` のWin32実装（計測対象外）
- `app.rs`: アプリ状態と操作
- `dock.rs`: Dock本体と設定画面
- `context_menu.rs`: 右クリックメニューとウィンドウ選択
- `layout.rs`: 子Viewportの位置・サイズ計算、時計の表示、ツールチップ
- `shell_menu.rs`: Windows背景メニューのプロセスツール項目の同期
- `ui.rs`: アイコンボタン
- `theme.rs`: フォント、色、独自アイコン描画
- `*_tests.rs`、`platform_fake.rs`: テスト専用

優先度が高い未完了事項:

なし（2026-09-23時点）。

GitHub Releasesを使った自動更新はユーザー判断により対象外（2026-09-23）。更新は新しいMSIを手動で実行する方式とする。

## インストールとアップグレード

- MSI定義: `installer\Package.wxs`
- ビルドスクリプト: `scripts\build-installer.ps1`
- インストール範囲: ユーザー単位
- インストール先: `%LOCALAPPDATA%\Programs\Windows Side Dock`
- Package ID: `ZeroPlatformLab.WindowsSideDock`（変更しないこと）
- バージョン元: `Cargo.toml`
- 現在のバージョン: `0.1.7`
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
- 直近の機能コミット: テスト用のOS抽象化とC1カバレッジ100%（0.1.7）
- `windows-side-dock-screenshot.png` はユーザー指示によりGitへ追加しない
- 既存のユーザー変更を破棄する `git reset --hard` 等は使用しない
