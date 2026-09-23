# Host smoke for Stable `amber compile` on Windows. See docs/COMPILE_CONTRACT.md.
param(
    [Parameter(Mandatory = $true)]
    [string]$Amber
)

$ErrorActionPreference = "Stop"
if (-not (Test-Path -LiteralPath $Amber)) {
    throw "amber binary not found: $Amber"
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("sea-smoke-" + [guid]::NewGuid().ToString("n"))
New-Item -ItemType Directory -Path $root | Out-Null
try {
    $smoke = Join-Path $root "smoke.js"
    $js = @'
const path = require("path");
if (typeof path.basename !== "function") {
  throw new Error("path.basename missing");
}
const args = process.argv.slice(2).join(",");
console.log("SEA_SMOKE_OK:" + path.basename("file.txt") + ":" + args);
'@
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($smoke, $js, $utf8)

    $out = Join-Path $root "sea-smoke.exe"
    & $Amber compile $smoke -o $out
    if ($LASTEXITCODE -ne 0) {
        throw "amber compile failed with exit $LASTEXITCODE"
    }
    if (-not (Test-Path -LiteralPath $out)) {
        throw "compile did not write $out"
    }

    $got = & $out hello
    if ($LASTEXITCODE -ne 0) {
        throw "standalone binary failed: $got"
    }
    $gotText = ($got | Out-String)
    if ($gotText -notmatch "SEA_SMOKE_OK:file.txt:hello") {
        throw "unexpected stdout: $gotText"
    }

    $missing = Join-Path $root "missing.js"
    $failOut = Join-Path $root "should-not-exist.exe"
    $prev = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $failText = (& $Amber compile $missing -o $failOut 2>&1 | Out-String)
    $failCode = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($failCode -eq 0) {
        throw "missing entry should fail: $failText"
    }
    if ($failText -notmatch "error: amber compile:") {
        throw "missing stable prefix: $failText"
    }
    if ($failText -notmatch "entry file not found") {
        throw "missing diagnostic body: $failText"
    }
    if (Test-Path -LiteralPath $failOut) {
        throw "failed compile wrote $failOut"
    }

    Write-Host "SEA compile smoke passed on $([Environment]::OSVersion.VersionString)"
}
finally {
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
# The missing-entry check is supposed to fail. PowerShell keeps that native
# exit code, which would fail CI after the success message. Exit explicitly.
exit 0
