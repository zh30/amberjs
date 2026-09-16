# Locate OpenSSL for openssl-sys on GitHub windows-latest.
# Prefer vcpkg (binary-cached, ~seconds) over Chocolatey (Program Files).
$ErrorActionPreference = "Stop"

$vcpkgRoot = $env:VCPKG_INSTALLATION_ROOT
if (-not $vcpkgRoot) {
    $vcpkgRoot = "C:\vcpkg"
}

$dirs = @(
    (Join-Path $vcpkgRoot "installed\x64-windows-static-md"),
    (Join-Path $vcpkgRoot "installed\x64-windows"),
    "C:\Program Files\OpenSSL-Win64",
    "C:\Program Files\OpenSSL",
    "C:\Program Files (x86)\OpenSSL-Win64"
)
$found = $dirs | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $found) {
    throw "OpenSSL directory not found (tried vcpkg + Chocolatey paths)"
}

$lib = Get-ChildItem -Path $found -Recurse -Filter "libcrypto.lib" -ErrorAction SilentlyContinue |
    Select-Object -First 1
if (-not $lib) {
    throw "libcrypto.lib not found under $found"
}

$libDir = $lib.Directory.FullName
$inc = Join-Path $found "include"
if (-not (Test-Path $inc)) {
    throw "OpenSSL include dir missing: $inc"
}

Add-Content -Path $env:GITHUB_ENV -Value "OPENSSL_DIR=$found"
Add-Content -Path $env:GITHUB_ENV -Value "OPENSSL_LIB_DIR=$libDir"
Add-Content -Path $env:GITHUB_ENV -Value "OPENSSL_INCLUDE_DIR=$inc"
if ($found -match "static") {
    Add-Content -Path $env:GITHUB_ENV -Value "OPENSSL_STATIC=1"
}
Write-Host "OpenSSL dir=$found"
Write-Host "OpenSSL lib=$libDir"
Write-Host "OpenSSL include=$inc"
Get-ChildItem $libDir | ForEach-Object { Write-Host ("  " + $_.Name) }
