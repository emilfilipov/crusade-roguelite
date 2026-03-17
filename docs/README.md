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

## Runtime Logs
- Windows log file path:
  - `%LOCALAPPDATA%\CrusadeRoguelite\logs\crusade_roguelite.log`
- If `%LOCALAPPDATA%` is unavailable, fallback path:
  - `<project-or-launch-dir>\logs\crusade_roguelite.log`

## Runtime Asset Discovery
- Runtime looks for an `assets` directory:
  1. next to the executable
  2. then by walking parent directories (to support direct runs from `target/.../release`)
- This allows both:
  - installed builds (`assets` next to `.exe`)
  - local dev/release runs from project build folders

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

## Installer Versioning
- Installer versions use `0.0.x` and are driven by `installer/version.json` (`installer_patch`).
- Each successful `./scripts/package_windows_installer.ps1` run:
  1. builds installer version `0.0.<installer_patch>`
  2. writes output file `installer/dist/crusade_roguelite_installer_0_0_<installer_patch>.exe`
  3. increments `installer_patch` for the next package build

## Documentation
- System scope map: `docs/SYSTEM_SCOPE_MAP.md`
- Task backlog: `docs/TASKS.md`
- Art requirements: `docs/requirements.md`
- Full technical system reference: `docs/SYSTEMS_REFERENCE.md`
- External asset pack sources/licenses: `docs/ASSET_SOURCES.md`
