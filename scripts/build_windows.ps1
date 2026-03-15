#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "Building Windows release binary..."
cargo build --release --target x86_64-pc-windows-msvc

Write-Host "Build completed."
