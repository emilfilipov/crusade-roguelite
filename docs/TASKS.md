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

## CRU-029 - Drop Transit-To-Commander Consumption
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: none
- Goal: Convert drop collection so friendly contact only starts homing; effect applies only when drop reaches commander.
- Context:
  - Current flow consumes XP immediately at pickup.
  - Future drop types should reuse the same transit mechanic.
  - Changes expected in `src/drops.rs`, event handling, and UI readability hooks.
- Implementation:
  1. Add drop transit state/component and commander-homing movement.
  2. Trigger transit when any friendly touches a drop.
  3. Consume drop payload only on commander contact.
- Unit Tests Required:
  - Transit movement helper tests.
  - Commander-contact consumption tests.
- Acceptance Criteria:
  - Friendly pickup starts homing animation/state.
  - XP is granted only on commander arrival.

## CRU-030 - Morale System Refactor (Per-Unit Morale, Friendly + Enemy)
- Status: `DONE`
- Type: `Gameplay/Core`
- Priority: `P0`
- Depends on: `CRU-029`
- Goal: Replace `morale_weight` placeholder with active per-unit morale that influences combat.
- Context:
  - Existing `morale_weight` is a data placeholder and not active in runtime formulas.
  - Needs deterministic morale debuffs for both friendlies and enemies below 50% morale.
  - Needs event-driven morale changes from damage, kills, and deaths.
- Implementation:
  1. Introduce unit `Morale` component and migrate spawn/data pipelines.
  2. Apply morale-based damage/attack-speed debuffs below 50% threshold.
  3. Wire morale changes from hit/death/kill events for both teams.
- Unit Tests Required:
  - Morale multiplier threshold tests.
  - Morale event adjustment tests.
- Acceptance Criteria:
  - All combat units have active morale values.
  - Low-morale units fight less effectively.

## CRU-031 - Cohesion/Banner Recovery Loop v2
- Status: `DONE`
- Type: `Gameplay/Balance`
- Priority: `P0`
- Depends on: `CRU-030`
- Goal: Tie banner behavior directly to low-cohesion state and add timed recovery channel loop.
- Context:
  - Existing banner drop condition is casualty/cohesion formula-driven and rarely visible.
  - New loop requires automatic drop on low cohesion, delayed pickup, channel UI, and cohesion reset on recovery.
  - Banner drop effect should be movement-speed penalty only for friendlies.
- Implementation:
  1. Trigger auto-drop at low cohesion tier and apply friendly move-speed penalty while dropped.
  2. Add 10s pickup lockout + 5s pickup channel with progress state.
  3. Restore cohesion to 60-79 tier target on successful pickup.
- Unit Tests Required:
  - Drop/recovery threshold and timer tests.
  - Cohesion restore tests.
- Acceptance Criteria:
  - Banner consistently enters recoverable failure loop at low cohesion.
  - Recovery progress is visible and functional.

## CRU-032 - HUD Expansion (Bottom-Left Vertical Morale/Cohesion + Banner Pickup Bar)
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-030`, `CRU-031`
- Goal: Expose average army morale and cohesion with vertical bars; show banner pickup progress under XP bar.
- Context:
  - Current HUD lacks persistent morale readout.
  - Rescue progress bars already live under XP bar and need to coexist with banner recovery progress.
  - Must stay readable with minimal UI style changes.
- Implementation:
  1. Add average army morale snapshot and vertical bar UI widgets.
  2. Add cohesion vertical bar next to morale bar.
  3. Render banner pickup progress alongside rescue progress bars.
- Unit Tests Required:
  - Morale average helper tests.
  - Progress ratio/formatter tests for banner pickup bar.
- Acceptance Criteria:
  - Bottom-left bars update live for morale/cohesion.
  - Banner pickup channel appears under XP bar only while active.

## CRU-033 - Oasis/Deprecated Data Cleanup
- Status: `DONE`
- Type: `Core/Docs`
- Priority: `P1`
- Depends on: none
- Goal: Remove deprecated oasis gameplay/config remnants from active MVP schema/runtime.
- Context:
  - Oasis system is intentionally out of current gameplay loop.
  - Remaining config/fields/asset references create confusion.
  - Docs must match cleaned runtime schema.
- Implementation:
  1. Remove oasis fields from runtime config schema and validation.
  2. Remove oasis runtime asset handle references not used by gameplay.
  3. Update docs (`SYSTEMS_REFERENCE`, `SYSTEM_SCOPE_MAP`, `requirements`, `ASSET_SOURCES`).
- Unit Tests Required:
  - Config load tests for updated map schema.
  - Validation tests for required map fields.
- Acceptance Criteria:
  - No active oasis fields are required by runtime config loader.
  - Documentation reflects oasis as deferred/not active.

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
