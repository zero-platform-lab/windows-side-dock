[CmdletBinding()]
param(
    # 出力するMSIXのパス。既定は dist\windows-side-dock-shell.msix。
    [string]$OutputPath
)

# 右クリックメニュー用のスパースパッケージ（MSIX）を作る。中身はマニフェストとロゴだけ。
# 署名はしない（GitHub Actionsで署名する）。未署名のMSIXはこのPCに登録できない。
$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $projectRoot "Cargo.toml") -Raw
$versionMatch = [regex]::Match($manifest, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"')
if (-not $versionMatch.Success) {
    throw "Cargo.toml からバージョンを取得できません。"
}
$version = "$($versionMatch.Groups['version'].Value).0"
if (-not $OutputPath) {
    $OutputPath = Join-Path $projectRoot "dist\windows-side-dock-shell.msix"
}

$sdk = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$makeappx = Get-ChildItem -Path $sdk -Recurse -Filter makeappx.exe -ErrorAction Stop |
    Where-Object { $_.FullName -match '\\x64\\' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $makeappx) {
    throw "makeappx.exe が見つかりません（Windows SDK が必要です）。"
}

$layout = Join-Path ([IO.Path]::GetTempPath()) "wsd-sparse-$PID"
New-Item -ItemType Directory -Path $layout -Force | Out-Null
try {
    $template = Get-Content -LiteralPath (Join-Path $projectRoot "installer\sparse\AppxManifest.xml") -Raw
    Set-Content -LiteralPath (Join-Path $layout "AppxManifest.xml") -Value $template.Replace('$(Version)', $version) -Encoding utf8
    Copy-Item (Join-Path $projectRoot "assets\logo-44.png"), (Join-Path $projectRoot "assets\logo-150.png") $layout
    New-Item -ItemType Directory -Path (Split-Path -Parent $OutputPath) -Force | Out-Null
    & $makeappx.FullName pack /o /nv /d $layout /p $OutputPath | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "MSIXの作成に失敗しました。"
    }
} finally {
    Remove-Item -LiteralPath $layout -Recurse -Force -ErrorAction SilentlyContinue
}
Get-Item -LiteralPath $OutputPath
