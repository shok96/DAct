# Quick release build for Windows.
# Usage:  .\scripts\build.ps1
#         .\scripts\build.ps1 -Test
#         .\scripts\build.ps1 -Run

param(
    [switch]$Test,
    [switch]$Run
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Need-Cmd($Name, $Hint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Error "$Name is not on PATH. $Hint"
    }
}

Need-Cmd cargo "Install Rust from https://rustup.rs and reopen the terminal."

if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Error "rustc is not on PATH. Install the stable MSVC toolchain: rustup toolchain install stable-x86_64-pc-windows-msvc"
}

Write-Host "Building DAct (release)..."
if ($Test) {
    cargo test
}
cargo build --release

$Exe = Join-Path $Root "target\release\dact.exe"
if (-not (Test-Path $Exe)) {
    Write-Error "Build finished but $Exe was not found."
}

$Dist = Join-Path $Root "dist"
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
$Dest = Join-Path $Dist "dact.exe"
try {
    Copy-Item -Force $Exe $Dest
    $Out = Resolve-Path $Dest
} catch {
    Write-Warning "Could not update dist\dact.exe (close the running app and rebuild to refresh it)."
    $Out = Resolve-Path $Exe
}

Write-Host "Ready: $Out"

if ($Run) {
    & $Out
}
