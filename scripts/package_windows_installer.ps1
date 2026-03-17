#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$installerScript = Join-Path $repoRoot "installer/crusade_roguelite.iss"
$installerVersionFile = Join-Path $repoRoot "installer/version.json"
$targetExe = Join-Path $repoRoot "target/x86_64-pc-windows-msvc/release/crusade_roguelite.exe"

if (-not (Test-Path $installerVersionFile)) {
    @{ installer_patch = 1 } | ConvertTo-Json | Set-Content $installerVersionFile
}

$versionState = Get-Content $installerVersionFile -Raw | ConvertFrom-Json
if ($null -eq $versionState.installer_patch) {
    throw "Invalid installer version file, expected installer_patch: $installerVersionFile"
}

$patch = [int]$versionState.installer_patch
if ($patch -lt 0) {
    throw "installer_patch must be >= 0 in $installerVersionFile"
}

$appVersion = "0.0.$patch"
$outputVersionTag = "0_0_$patch"

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

Write-Host "Packaging installer version $appVersion..."
& $iscc.Source "/DMyAppVersion=$appVersion" "/DMyOutputVersionTag=$outputVersionTag" $installerScript
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup compilation failed with exit code $LASTEXITCODE"
}

$nextPatch = $patch + 1
@{ installer_patch = $nextPatch } | ConvertTo-Json | Set-Content $installerVersionFile

Write-Host "Installer packaging completed."
