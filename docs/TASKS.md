# TASKS.md

## Planning Notes
- Board style: Jira-like backlog with task keys, dependencies, implementation steps, and acceptance criteria.
- Primary stack: Rust + Bevy.
- Primary target: Windows (`x86_64-pc-windows-msvc`).
- Distribution: Steam-ready, but local Windows installer is required from early milestones.
- Scope limits and expansion gates are tracked in `docs/SYSTEM_SCOPE_MAP.md`.

## Global Delivery Rules (Apply to Every Task)
1. Run full quality loop before closing a task:
   - `cargo fmt --all`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --all-targets --all-features`
   - `cargo build --release --target x86_64-pc-windows-msvc`
2. If any check/test/build fails, investigate and fix, then rerun the full loop until green.
3. Any logic that can be unit tested must have unit tests.
4. Do not close a task with failing tests.
5. After successful build/test loop, push changes to repository.
6. When expanding a previously limited system, update `docs/SYSTEM_SCOPE_MAP.md` and add/update task cards in this file.
7. Documentation-only changes (including `.md` files) must still be committed and pushed; no local-only markdown drift.

## Status Legend
- `TODO`: not started
- `IN PROGRESS`: active
- `BLOCKED`: waiting on dependency/decision
- `DONE`: implemented, tested, and pushed

---

## Active Backlog
- No active tasks at the moment.

---

## Cycle 2026-03-20 (Completed)

### CRU-079 - Modal Stability + Scroll UX
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `none`
- Goal: Make in-run/archive modal interactions reliable and prevent content clipping.
- Implementation:
  1. Add scrollable viewport/content system for long modal content.
  2. Apply to `Archive`/`Bestiary` and `Skill Book` screens.
  3. Ensure close actions clear modal state deterministically.
  4. Add paused `Escape` behavior to resume run.
- Acceptance Criteria:
  - Archive and Skill Book are scrollable in both in-run and main-menu contexts.
  - `Escape` in pause resumes run.
  - Modal close button always exits modal.

### CRU-080 - Inventory Grid Layout + Tiered Equipment Rows
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-079`
- Goal: Replace text inventory scaffold with slot-grid presentation.
- Implementation:
  1. Render bag drops as 1-item-per-slot grid.
  2. Render equipment rows for commander and tiers (`Tier 0..5`).
  3. Add commander slots (`Banner`, `Instrument`, `Chant`) and unit-tier slots (`Melee Weapon`, `Ranged Weapon`, `Armor`).
- Acceptance Criteria:
  - Inventory screen visually presents bag + equipment as slot grids.
  - Empty slots render correctly and consistently.

### CRU-081 - Stats Table Readability Pass
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-079`
- Goal: Make stats easier to scan via table formatting and value coloring.
- Implementation:
  1. Replace pipe-delimited text rows with table rows.
  2. Color bonus values: positive green, negative red, neutral default.
- Acceptance Criteria:
  - Stats panel uses table layout with clear base/bonus/final columns.
  - Bonus color coding is visible and accurate.

### CRU-082 - Skill Book Cumulative Effect Aggregation
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-079`
- Goal: Show cumulative effect totals instead of only last picked roll text.
- Implementation:
  1. Extend skill book entry model with accumulated value tracking.
  2. Render cumulative descriptions per upgrade kind.
  3. Preserve conditional active/inactive indicators and unmet requirement details.
- Acceptance Criteria:
  - Skill Book shows aggregated totals that match runtime-stacked upgrades.

### CRU-083 - Tier-0 Rescue Pool Expansion (Infantry/Archer/Priest)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `none`
- Goal: Make archer and priest tier-0 and rescue-spawn eligible.
- Implementation:
  1. Add priest recruit/rescuable variants to model and rescue mappings.
  2. Set archer/priest to tier-0 in tier rules and rescue config.
  3. Update config validation and rescue sequencing tests.
- Acceptance Criteria:
  - Rescue pool spawns infantry, archer, and priest variants.
  - Tier-0 validation still passes for rescue config.

### CRU-084 - Promotion Budget + Unit Upgrade Grid Table
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-083`
- Goal: Keep level-cost accounting correct for specialization promotions and improve upgrade-screen clarity.
- Implementation:
  1. Preserve per-unit accumulated level cost on promotions.
  2. Support allowed specialization path costs (infantry -> archer/priest costs `+1`).
  3. Reformat unit-upgrade panel options into grid-table style columns.
- Acceptance Criteria:
  - Promotion budget remains correct after repeated promotions.
  - Unit upgrade options display in table-like rows with clear affordability.

### CRU-085 - Mob's Mercy Rescue Progress HUD Sync
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `none`
- Goal: Keep rescue progress bars aligned with active rescue-time modifiers.
- Implementation:
  1. Use effective rescue duration (including conditional multipliers) in rescue HUD ratio calculations.
- Acceptance Criteria:
  - Rescue progress bars complete at the same time as actual rescue completion when Mercy is active.

### CRU-086 - QA + Test Coverage for Cycle Changes
- Status: `DONE`
- Type: `QA`
- Priority: `P0`
- Depends on: `CRU-079`, `CRU-080`, `CRU-081`, `CRU-082`, `CRU-083`, `CRU-084`, `CRU-085`
- Goal: Validate behavior and keep repository green.
- Implementation:
  1. Update/extend unit tests for new rescue pool, skill book accumulation, and roster-cost behavior.
  2. Run full quality loop and installer packaging.
- Acceptance Criteria:
  - `fmt`, `clippy`, `test`, release build, and installer packaging all pass.

### CRU-087 - Documentation Refresh for Cycle
- Status: `DONE`
- Type: `Docs`
- Priority: `P1`
- Depends on: `CRU-079`, `CRU-080`, `CRU-081`, `CRU-082`, `CRU-083`, `CRU-084`, `CRU-085`
- Goal: Keep docs aligned with runtime behavior.
- Implementation:
  1. Update `docs/SYSTEMS_REFERENCE.md` for modal scroll, pause/Escape, inventory/stats/skillbook updates, rescue pool changes, and promotion cost semantics.
  2. Update `docs/SYSTEM_SCOPE_MAP.md` where scope wording changed.
- Acceptance Criteria:
  - System/reference docs reflect shipped behavior from this cycle.

---

## Task Card Template
### CRU-XXX - <Title>
- Status: `TODO`
- Type: `<Gameplay|UI|Core|Balance|Release|QA|Docs>`
- Priority: `<P0|P1|P2>`
- Depends on: `<none|CRU-###,...>`
- Goal: `<one clear outcome sentence>`
- Context:
  - Why this task exists.
  - Runtime constraints or known pitfalls.
  - Exact files/systems expected to change.
- Implementation:
  1. `<step 1>`
  2. `<step 2>`
  3. `<step 3>`
- Unit Tests Required:
  - `<test case 1>`
  - `<test case 2>`
- Acceptance Criteria:
  - `<observable runtime result 1>`
  - `<observable runtime result 2>`
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md` (if scope gate changes)
  - `docs/ASSET_SOURCES.md` (if assets/source usage changes)
