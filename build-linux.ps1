# Cross-compile the Linux binary **from Windows**, with no Linux toolchain.
#
#   powershell -ExecutionPolicy Bypass -File build-linux.ps1
#
# Produces: target\x86_64-unknown-linux-musl\release\rust-fps
#
# The result is a statically linked x86_64 ELF (static-pie). It needs no glibc
# and no shared libraries, so it runs on any x86_64 Linux with a desktop
# session. macroquad loads X11/GL at runtime with dlopen, so there is no
# link-time dependency on any .so either.
#
# How this works: the rust-std component for the musl target ships a
# self-contained libc, and rustc can link it with the `rust-lld` that is already
# bundled inside the Rust sysroot. `rust-lld` is not on PATH by default, so this
# script puts it there and points the linker at it.

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$target = "x86_64-unknown-linux-musl"

# Locate the rust-lld shipped with the active toolchain.
$hostTriple = ((rustc -vV | Select-String '^host:') -replace '^host:\s*', '').Trim()
$sysroot = (rustc --print sysroot).Trim()
$lldDir = Join-Path $sysroot "lib\rustlib\$hostTriple\bin"
$lld = Join-Path $lldDir "rust-lld.exe"

if (-not (Test-Path $lld)) {
    throw "rust-lld.exe not found at $lld -- is the Rust toolchain complete?"
}
Write-Host "==> Using linker: $lld" -ForegroundColor DarkGray

$env:PATH = "$lldDir;$env:PATH"
# Append rather than replace: overwriting would silently drop any flags the
# caller set, and the assignment persists for the rest of the shell session.
$env:RUSTFLAGS = "-C linker=rust-lld $env:RUSTFLAGS"

Write-Host "==> Ensuring target $target is installed" -ForegroundColor Cyan
rustup target add $target
if ($LASTEXITCODE -ne 0) { throw "rustup target add failed" }

Write-Host "==> Building for Linux" -ForegroundColor Cyan
# `--no-default-features` disables the `sound` feature on purpose.
#
# The audio stack is quad-snd -> audrey -> alsa-sys, and alsa-sys declares
# `#[link(name = "asound")]`. Linux distributions ship ALSA as a shared object
# (libasound.so.2) and generally not as a static library, so a fully static musl
# link has nothing to resolve `-lasound` against and fails with
# "unable to find library -lasound". This is a property of ALSA packaging, not
# of the game. Cross-compiled Linux builds are therefore silent; build natively
# on Linux with ./build.sh to get audio.
cargo build --release --no-default-features --target $target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$bin = "target\$target\release\rust-fps"
if (-not (Test-Path $bin)) { throw "Build finished but $bin is missing" }

# Prove the artefact really is a Linux ELF and not a mislabelled PE.
$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $bin))[0..3]
$magic = ($bytes | ForEach-Object { $_.ToString("x2") }) -join " "
if ($magic -ne "7f 45 4c 46") {
    throw "Output is not an ELF file (magic was: $magic)"
}

$size = (Get-Item $bin).Length / 1MB
Write-Host ("==> Built: {0}  ({1:N1} MB)" -f $bin, $size) -ForegroundColor Green
Write-Host "    ELF magic verified: $magic" -ForegroundColor Green
Write-Host ""
Write-Host "Transfer the file to a Linux box and:" -ForegroundColor DarkGray
Write-Host "    chmod +x rust-fps && ./rust-fps" -ForegroundColor DarkGray
