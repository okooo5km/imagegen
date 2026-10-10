# Install prebuilt imagegen for Windows x64/ARM64 without language runtimes.
$ErrorActionPreference = 'Stop'
$version = if ($env:IMAGEGEN_VERSION) { $env:IMAGEGEN_VERSION } else { 'latest' }
if ($version -ne 'latest' -and $version -notmatch '^v\d+\.\d+\.\d+([-.][A-Za-z0-9.-]+)?$') { throw 'Invalid IMAGEGEN_VERSION' }
$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$target = switch ($arch) {
    'X64' { 'x86_64-pc-windows-msvc' }
    'Arm64' { 'aarch64-pc-windows-msvc' }
    default { throw "Unsupported architecture: $arch" }
}
$archive = "imagegen-$target.zip"
$base = if ($version -eq 'latest') {
    'https://github.com/okooo5km/imagegen/releases/latest/download'
} else {
    "https://github.com/okooo5km/imagegen/releases/download/$version"
}
$bin = if ($env:IMAGEGEN_INSTALL_DIR) { $env:IMAGEGEN_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'imagegen\bin' }
$work = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $work | Out-Null
try {
    $file = Join-Path $work $archive
    $checksum = "$file.sha256"
    if ($env:IMAGEGEN_ARCHIVE_DIR) {
        Copy-Item -LiteralPath (Join-Path $env:IMAGEGEN_ARCHIVE_DIR $archive) -Destination $file
        Copy-Item -LiteralPath (Join-Path $env:IMAGEGEN_ARCHIVE_DIR "$archive.sha256") -Destination $checksum
    } else {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$archive" -OutFile $file
        Invoke-WebRequest -UseBasicParsing -Uri "$base/$archive.sha256" -OutFile $checksum
    }
    $expected = ((Get-Content -LiteralPath $checksum -Raw).Trim() -split '\s+')[0]
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash
    if ($expected -notmatch '^[a-fA-F0-9]{64}$' -or $expected -ne $actual) { throw 'Checksum mismatch; nothing installed.' }
    Expand-Archive -LiteralPath $file -DestinationPath (Join-Path $work 'unpacked')
    $binary = Join-Path $work 'unpacked\imagegen.exe'
    if (-not (Test-Path -LiteralPath $binary)) { throw 'Archive contains no imagegen.exe.' }
    New-Item -ItemType Directory -Force -Path $bin | Out-Null
    $stage = Join-Path $bin '.imagegen-new.exe'
    Copy-Item -LiteralPath $binary -Destination $stage -Force
    Move-Item -LiteralPath $stage -Destination (Join-Path $bin 'imagegen.exe') -Force
    $env:PATH = "$bin;$env:PATH"
    if ($env:IMAGEGEN_NO_MODIFY_PATH -ne '1') {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (($userPath -split ';') -notcontains $bin) {
            [Environment]::SetEnvironmentVariable('Path', "$bin;$userPath", 'User')
        }
    }
    & (Join-Path $bin 'imagegen.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'Installed binary failed to run.' }
    Write-Host "Installed: $bin\imagegen.exe"
} finally {
    Remove-Item -LiteralPath $work -Recurse -Force -ErrorAction SilentlyContinue
}
