# FACTION_ONBOARDING_CHECKLIST.md

## Purpose
This checklist is the regression gate for onboarding faction content through data-first identity paths.

## Checklist
1. Add or update faction overrides in:
   - `assets/data/units.json`
   - `assets/data/enemies.json`
   - `assets/data/heroes.json`
   - `assets/data/items.json`
2. Ensure all generic unit IDs used by the faction resolve through `UnitKind::from_faction_and_unit_id`.
3. Ensure all hero subtypes are present and each subtype has exactly `10` heroes for the faction.
4. Validate map faction keys are in the supported faction registry and at least one map allows the default start faction.
5. Run the full quality loop:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-targets --all-features`
   - `cargo build --release --target x86_64-pc-windows-msvc`
6. Confirm runtime identity guard test passes:
   - `model::tests::runtime_modules_avoid_faction_specific_unit_variants_outside_identity_bridge`

