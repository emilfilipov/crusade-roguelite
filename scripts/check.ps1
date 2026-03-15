#!/usr/bin/env pwsh
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Write-Host "Running cargo fmt..."
cargo fmt --all

Write-Host "Running cargo clippy..."
cargo clippy --all-targets --all-features -- -D warnings

Write-Host "Running cargo test..."
cargo test --all-targets --all-features

Write-Host "Running release build..."
cargo build --release --target x86_64-pc-windows-msvc

Write-Host "All checks completed successfully."
