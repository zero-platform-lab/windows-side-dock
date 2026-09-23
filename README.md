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

初めて起動したときは、エクスプローラーとWindows設定がピン留めされています。どの項目もピン留めを外せます。
ピン留めは`%LOCALAPPDATA%\windows-side-dock\pinned.txt`に保存されます。

## ダウンロード

[Releases](https://github.com/zero-platform-lab/windows-side-dock/releases)から、署名済みの`windows-side-dock-<version>-x64.msi`を入手できます。
署名は自己署名の証明書（Zero Platform Lab）によるもので、同梱の`signing.cer`を信頼ストアへ登録するまでは「不明な発行元」と表示されます。

## 配布用ビルド

```powershell
cargo build --release
```

実行ファイルは`target\release\windows-side-dock.exe`に作成されます。

## MSIインストーラー

WiX Toolset 6がインストールされたWindows環境で次を実行します。

```powershell
.\scripts\build-installer.ps1
```

`dist\windows-side-dock-<version>-x64.msi`が生成されます。バージョンは`Cargo.toml`から取得します。新しいバージョンのMSIは既存版を置き換え、`%LOCALAPPDATA%\windows-side-dock`のユーザー設定を保持します。
インストールまたはアップグレードが正常完了すると、Windows Side Dockを自動起動します。

リリースごとに、Windows Installerが比較する先頭3桁のバージョンを必ず増やしてください（例: `0.1.1` → `0.1.2`）。

署名付きのMSIは、`Cargo.toml`の版と同じ`v<version>`タグをpushするとGitHub Actions（`.github/workflows/release.yml`）が作り、Releaseに出します。
