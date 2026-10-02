# Build the Windows release binary.
#
#   powershell -ExecutionPolicy Bypass -File build-windows.ps1
#
# Produces: target\x86_64-pc-windows-msvc\release\tides-of-rlyeh.exe

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "==> Building for Windows (x86_64-pc-windows-msvc)" -ForegroundColor Cyan
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$exe = "target\x86_64-pc-windows-msvc\release\tides-of-rlyeh.exe"
if (-not (Test-Path $exe)) { throw "Build finished but $exe is missing" }

$size = (Get-Item $exe).Length / 1MB
Write-Host ("==> Built: {0}  ({1:N1} MB)" -f $exe, $size) -ForegroundColor Green
