[CmdletBinding()]
param(
    [string]$Configuration = "release"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $projectRoot "Cargo.toml"
$manifest = Get-Content -LiteralPath $manifestPath -Raw
$versionMatch = [regex]::Match($manifest, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"')
if (-not $versionMatch.Success) {
    throw "Cargo.toml からバージョンを取得できません。"
}

$version = $versionMatch.Groups['version'].Value
$cargoArguments = @("build")
if ($Configuration -eq "release") {
    $cargoArguments += "--release"
}
& cargo @cargoArguments
if ($LASTEXITCODE -ne 0) {
    throw "cargo build に失敗しました。"
}

$sourceDirectory = Join-Path $projectRoot "target\$Configuration"
$executable = Join-Path $sourceDirectory "windows-side-dock.exe"
if (-not (Test-Path -LiteralPath $executable)) {
    throw "実行ファイルが見つかりません: $executable"
}

$distDirectory = Join-Path $projectRoot "dist"
New-Item -ItemType Directory -Path $distDirectory -Force | Out-Null
$outputPath = Join-Path $distDirectory "windows-side-dock-$version-x64.msi"
$packageSource = Join-Path $projectRoot "installer\Package.wxs"

& wix build $packageSource `
    -d "Version=$version" `
    -d "SourceDir=$sourceDirectory" `
    -arch x64 `
    -out $outputPath
if ($LASTEXITCODE -ne 0) {
    throw "WiX MSIの生成に失敗しました。"
}

Get-Item -LiteralPath $outputPath
