# Crusade Roguelite

Small 2D survivor-like prototype where the player controls a squad centered around commander-led formation play.

## Stack
- Rust (stable)
- Bevy
- Windows-first build target (`x86_64-pc-windows-msvc`)

## Run
```powershell
cargo run
```

## Quality Loop
```powershell
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo build --release --target x86_64-pc-windows-msvc
```

## Helper Scripts
```powershell
./scripts/check.ps1
./scripts/build_windows.ps1
./scripts/package_windows_installer.ps1
```

## Documentation
- System scope map: `docs/SYSTEM_SCOPE_MAP.md`
- Task backlog: `docs/TASKS.md`
- Art requirements: `docs/requirements.md`
- Full technical system reference: `docs/SYSTEMS_REFERENCE.md`
