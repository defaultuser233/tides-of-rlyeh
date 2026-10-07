# Build the Windows release binary.
#
#   powershell -ExecutionPolicy Bypass -File build-windows.ps1
#
# Produces: target\x86_64-pc-windows-msvc\release\tides-of-rlyeh.exe

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

# `+crt-static` is also set in `.cargo/config.toml`, but a `RUSTFLAGS` in the
# environment *overrides* that file — and `build-linux.ps1` sets one for its own
# link step, which stays in the shell it ran in. Appending rather than assigning
# is the same convention that script uses: it cannot drop flags the caller set,
# and it cannot be silently defeated by them either.
$env:RUSTFLAGS = "-C target-feature=+crt-static $env:RUSTFLAGS"

Write-Host "==> Building for Windows (x86_64-pc-windows-msvc)" -ForegroundColor Cyan
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$exe = "target\x86_64-pc-windows-msvc\release\tides-of-rlyeh.exe"
if (-not (Test-Path $exe)) { throw "Build finished but $exe is missing" }

$size = (Get-Item $exe).Length / 1MB
Write-Host ("==> Built: {0}  ({1:N1} MB)" -f $exe, $size) -ForegroundColor Green
