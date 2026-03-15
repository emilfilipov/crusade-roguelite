#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$installerScript = Join-Path $repoRoot "installer/crusade_roguelite.iss"
$targetExe = Join-Path $repoRoot "target/x86_64-pc-windows-msvc/release/crusade_roguelite.exe"

Write-Host "Building Windows release first..."
cargo build --release --target x86_64-pc-windows-msvc

if (-not (Test-Path $targetExe)) {
    throw "Expected build output not found: $targetExe"
}

$iscc = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
if (-not $iscc) {
    $candidatePaths = @(
        "$env:LOCALAPPDATA\\Programs\\Inno Setup 6\\ISCC.exe",
        "C:\\Program Files\\Inno Setup 6\\ISCC.exe",
        "C:\\Program Files (x86)\\Inno Setup 6\\ISCC.exe"
    )
    foreach ($candidate in $candidatePaths) {
        if (Test-Path $candidate) {
            $iscc = @{ Source = $candidate }
            break
        }
    }
    if (-not $iscc) {
        throw "Inno Setup compiler (ISCC.exe) not found in PATH or known install paths."
    }
}

Write-Host "Packaging installer..."
& $iscc.Source $installerScript

Write-Host "Installer packaging completed."
