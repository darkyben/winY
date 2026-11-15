param([switch]$DebugBuild)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    $cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
    if (!(Test-Path $cargo)) { throw 'Installer Rust avec scripts/setup.ps1.' }
    if (!(Test-Path 'upstream/crates/gpui/Cargo.toml')) {
        git submodule update --init --depth 1
        if ($LASTEXITCODE -ne 0) { throw 'Échec du téléchargement de GPUI.' }
    }
    if ($DebugBuild) { & $cargo build --locked } else { & $cargo build --release --locked }
    if ($LASTEXITCODE -ne 0) { throw 'Échec de compilation.' }
} finally { Pop-Location }
