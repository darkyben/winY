$ErrorActionPreference = 'Stop'
winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --accept-package-agreements --accept-source-agreements --silent --override '--wait --quiet --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
if ($LASTEXITCODE -notin @(0, -1978335189)) { throw 'Vérifier l’installation des Build Tools C++ et du SDK Windows.' }
$installer = Join-Path $env:TEMP 'rustup-init.exe'
Invoke-WebRequest 'https://win.rustup.rs/x86_64' -OutFile $installer
& $installer -y --profile minimal --default-toolchain stable
if ($LASTEXITCODE -ne 0) { throw 'Échec de rustup.' }
& "$env:USERPROFILE\.cargo\bin\rustup.exe" component add rustfmt clippy
if ($LASTEXITCODE -ne 0) { throw 'Échec des composants Rust.' }
