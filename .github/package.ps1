$ErrorActionPreference = 'Stop'
$target = $env:IMAGEGEN_TARGET
New-Item -ItemType Directory -Force dist, package | Out-Null
Copy-Item "target/$target/release/imagegen.exe" package/imagegen.exe
Copy-Item LICENSE, NOTICE package/
$archive = "imagegen-$target.zip"
Compress-Archive -Path package/* -DestinationPath "dist/$archive" -Force
$hash = (Get-FileHash -Algorithm SHA256 "dist/$archive").Hash.ToLower()
[IO.File]::WriteAllText((Join-Path $PWD "dist/$archive.sha256"), "$hash  $archive`n")
$env:IMAGEGEN_ARCHIVE_DIR = Join-Path $PWD 'dist'
$env:IMAGEGEN_INSTALL_DIR = Join-Path $env:RUNNER_TEMP "imagegen installed $target"
$env:IMAGEGEN_CONFIG_DIR = Join-Path $env:RUNNER_TEMP "imagegen config $target"
$env:IMAGEGEN_NO_MODIFY_PATH = '1'
Remove-Item Env:APIMART_API_KEY, Env:APIMART_IMAGE_MODEL -ErrorAction SilentlyContinue
& ./installers/install.ps1
& imagegen doctor
if ($LASTEXITCODE -ne 0) { throw 'Doctor failed' }
& imagegen config set aspect_ratio 16:9
if ($LASTEXITCODE -ne 0) { throw 'Config failed' }
& imagegen generate --prompt 'Portable installation smoke' --dry-run
if ($LASTEXITCODE -ne 0) { throw 'Dry run failed' }
Copy-Item "dist/$archive" "dist/$archive.good"
Add-Content "dist/$archive" 'tamper'
$rejected = $false
try { & ./installers/install.ps1 } catch { $rejected = $true }
if (-not $rejected) { throw 'Corrupt archive was accepted' }
Move-Item "dist/$archive.good" "dist/$archive" -Force
& imagegen --version
if ($LASTEXITCODE -ne 0) { throw 'Existing binary was damaged' }
# Also exercise the Windows PowerShell shipped with Windows, as documented.
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File ./installers/install.ps1
if ($LASTEXITCODE -ne 0) { throw 'Windows PowerShell installation failed' }
