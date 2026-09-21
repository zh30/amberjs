# Amber Windows installer. Tries amber-vX-x86_64-pc-windows-msvc.zip, then legacy bee- zip.
param(
    [string]$Version = $env:AMBER_VERSION,
    [string]$InstallDir = $(if ($env:AMBER_INSTALL_DIR) { $env:AMBER_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "amber\bin" }),
    [string]$Repo = $(if ($env:AMBER_REPO) { $env:AMBER_REPO } else { "zh30/amberjs" })
)

$ErrorActionPreference = "Stop"
$Target = "x86_64-pc-windows-msvc"

if (-not $Version) {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $Version = $release.tag_name
}

if ($Version -notmatch '^v') {
    $Version = "v$Version"
}

if ($env:AMBER_RELEASE_BASE) {
    $releaseRoot = $env:AMBER_RELEASE_BASE.TrimEnd("/") + "/$Version"
} else {
    $releaseRoot = "https://github.com/$Repo/releases/download/$Version"
}

$urls = @(
    "$releaseRoot/amber-$Version-$Target.zip",
    "$releaseRoot/bee-$Version-$Target.zip"
)

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("amber-" + [guid]::NewGuid().ToString("n") + ".zip")
$downloaded = $false
$tried = New-Object System.Collections.Generic.List[string]

foreach ($url in $urls) {
    [void]$tried.Add($url)
    Write-Host "Downloading $url"
    try {
        Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing
        $downloaded = $true
        break
    } catch {
        Write-Host "amber install: not found: $url"
    }
}

if (-not $downloaded) {
    throw "amber install: download failed (tried: $($tried -join ' '))"
}

$extract = Join-Path ([System.IO.Path]::GetTempPath()) ("amber-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $extract | Out-Null
Expand-Archive -Path $tmp -DestinationPath $extract -Force

$src = Get-ChildItem -Path $extract -Recurse -Filter "amber.exe" | Select-Object -First 1
if (-not $src) {
    $src = Get-ChildItem -Path $extract -Recurse -Filter "bee.exe" | Select-Object -First 1
}
if (-not $src) {
    throw "amber.exe or bee.exe not found in archive"
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Copy-Item $src.FullName (Join-Path $InstallDir "amber.exe") -Force

$envPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($envPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$InstallDir;$envPath", "User")
}

Write-Host "Amber $Version installed to $InstallDir\amber.exe"
Write-Host "Open a new terminal and run: amber --version"
