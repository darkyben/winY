param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (!$SkipBuild) { & (Join-Path $PSScriptRoot 'build.ps1') }
$destination = Join-Path $env:LOCALAPPDATA 'Programs\winY'
$executable = Join-Path $destination 'winy.exe'
$wasInstalled = Test-Path -LiteralPath $executable
$startup = Join-Path ([Environment]::GetFolderPath('Startup')) 'winY Hotkey.lnk'
$runKey = Get-ItemProperty -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue
$runEntry = $runKey.winY
$running = Get-CimInstance Win32_Process -Filter "name='winy.exe'" | Where-Object { $_.ExecutablePath -eq $executable }
if ($running | Where-Object { $_.CommandLine -notlike '*--hotkey-launcher*' }) { throw 'Fermer la fenêtre winY avant de mettre à jour son exécutable.' }
$enableLauncher = !$wasInstalled -or (Test-Path -LiteralPath $startup) -or [bool]$runEntry -or [bool]($running | Where-Object { $_.CommandLine -like '*--hotkey-launcher*' })
$running | ForEach-Object { Stop-Process -Id $_.ProcessId }
New-Item -ItemType Directory -Force -Path $destination | Out-Null
Copy-Item -LiteralPath (Join-Path $repo 'target\release\winy.exe') -Destination $executable -Force
Copy-Item -LiteralPath (Join-Path $repo 'LICENSE') -Destination (Join-Path $destination 'LICENSE-GPL.txt') -Force
Copy-Item -LiteralPath (Join-Path $repo 'upstream\LICENSE-APACHE') -Destination (Join-Path $destination 'LICENSE-GPUI.txt') -Force
Copy-Item -LiteralPath (Join-Path $repo 'upstream\assets\fonts\lilex\OFL.txt') -Destination (Join-Path $destination 'LICENSE-Lilex.txt') -Force
foreach ($themeFamily in @('one', 'ayu', 'gruvbox')) {
    Copy-Item -LiteralPath (Join-Path $repo "upstream\assets\themes\$themeFamily\LICENSE") -Destination (Join-Path $destination "LICENSE-Theme-$themeFamily.txt") -Force
}
Copy-Item -LiteralPath (Join-Path $repo 'assets\LICENSE-Unicode.txt') -Destination (Join-Path $destination 'LICENSE-Unicode.txt') -Force
 $themeNotices = Join-Path $destination 'theme-attributions'
New-Item -ItemType Directory -Force -Path $themeNotices | Out-Null
Get-ChildItem -LiteralPath (Join-Path $repo 'assets\themes') | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $themeNotices -Recurse -Force
}
$shell = New-Object -ComObject WScript.Shell
if (!$wasInstalled) {
$startup = Join-Path ([Environment]::GetFolderPath('Startup')) 'winY Hotkey.lnk'
$shortcut = $shell.CreateShortcut($startup)
$shortcut.TargetPath = $executable
$shortcut.Arguments = '--hotkey-launcher'
$shortcut.WorkingDirectory = $destination
$shortcut.IconLocation = "$executable,0"
$shortcut.Description = 'Lance winY avec Win+Y'
$shortcut.Save()
}
$shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Programs')) 'winY.lnk'))
$shortcut.TargetPath = $executable
$shortcut.WorkingDirectory = $destination
$shortcut.IconLocation = "$executable,0"
$shortcut.Save()
if ($enableLauncher) { Start-Process -FilePath $executable -ArgumentList '--hotkey-launcher' -WindowStyle Hidden }
Write-Output "Installé : $executable"
Write-Output "Win+Y launcher enabled: $enableLauncher. Configure it in Ctrl+Shift+P > Win+Y."
