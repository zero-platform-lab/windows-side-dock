[CmdletBinding()]
param(
    # 未実行の行を一覧表示する代わりにHTMLレポートを開く。
    [switch]$Html
)

# C1（分岐）カバレッジを測る。nightlyツールチェーン、llvm-tools、cargo-llvm-covが必要。
# 古いテスト実行ファイルが残っていると行番号がずれた結果になるため、毎回消してから測る。
$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    cargo llvm-cov clean --workspace
    if ($Html) {
        cargo +nightly llvm-cov --branch --html --open
    } else {
        cargo +nightly llvm-cov --branch --show-missing-lines
    }
    if ($LASTEXITCODE -ne 0) {
        throw "カバレッジの計測に失敗しました。"
    }
} finally {
    Pop-Location
}
