$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$executable = Join-Path $repo 'target\release\winy.exe'
$output = Join-Path $repo 'target\buffer-benchmark.json'
$process = Start-Process -FilePath $executable -ArgumentList '--benchmark-buffer' -RedirectStandardOutput $output -WindowStyle Hidden -Wait -PassThru
if ($process.ExitCode -ne 0) { throw 'Échec du benchmark.' }
Write-Output "Exécutable : $((Get-Item $executable).Length) octets"
Get-Content -LiteralPath $output
