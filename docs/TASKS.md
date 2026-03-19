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

## CRU-034 - Enemy Movement Stabilization (Jitter Fix)
- Status: `DONE`
- Type: `Gameplay/AI`
- Priority: `P0`
- Depends on: none
- Goal: Remove enemy micro-jitter while chasing/engaging targets.
- Context:
  - Enemy movement oscillated around melee range thresholds.
  - Unit-level pixel snapping amplified visible jitter.
- Implementation:
  1. Added chase hysteresis (`stop`/`resume` distances) via per-enemy movement state.
  2. Kept camera pixel snapping but removed unit transform snapping.
  3. Added tests for hysteresis behavior.
- Unit Tests Required:
  - Hysteresis transition test.
- Acceptance Criteria:
  - Enemies hold stable melee standoff and no longer rapidly start/stop each frame.

## CRU-035 - XP Drop Visibility and Pickup Delay
- Status: `DONE`
- Type: `Gameplay/Core`
- Priority: `P0`
- Depends on: none
- Goal: Ensure XP packs are visible, enemy drops linger briefly, then home to commander.
- Context:
  - Players reported no visible ambient packs and no obvious enemy drop feedback.
  - Enemy kill drops should not instantly home in the same frame.
- Implementation:
  1. Added per-pack pickup delay timer.
  2. Enemy death drop events now spawn packs with delay (`0.45s`) before pickup/homing.
  3. Ambient pack spawn now centers around commander position for better visibility.
  4. Homing speed now derives from commander base move speed and stays slightly faster.
- Unit Tests Required:
  - Pickup delay tick-down test.
  - Homing speed > commander base speed test.
- Acceptance Criteria:
  - Ambient packs are regularly visible in the play area.
  - Enemy drops remain on ground briefly, then home after pickup trigger.

## CRU-036 - Morale/Cohesion Vertical Meter Direction
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: none
- Goal: Ensure morale/cohesion bars deplete from top to bottom.
- Context:
  - Meter container axis settings were ambiguous for vertical anchoring.
- Implementation:
  1. Set bar container to column flow with bottom anchoring for fill.
  2. Kept runtime fill ratio updates unchanged.
- Unit Tests Required:
  - Existing HUD ratio tests remain valid.
- Acceptance Criteria:
  - Lower values visually reduce fill from top downward.

## CRU-037 - Desert Floor and Foliage Rendering Refresh
- Status: `DONE`
- Type: `Visual`
- Priority: `P1`
- Depends on: none
- Goal: Restore readable desert battlefield floor using available asset packs.
- Context:
  - Background floor looked missing/placeholder after asset path changes.
- Implementation:
  1. Switched floor texture source to Ishtar dirt/sand-like tile.
  2. Replaced single stretched floor sprite with tiled background grid.
  3. Added sparse deterministic foliage/debris overlay tiles.
- Unit Tests Required:
  - Deterministic foliage placement helper test.
- Acceptance Criteria:
  - Battlefield has visible textured floor and sparse decorative variation.

## CRU-038 - GameOver Overlay with Restart/Main Menu
- Status: `DONE`
- Type: `UI/Flow`
- Priority: `P0`
- Depends on: none
- Goal: On defeat, pause gameplay and show `Restart` / `Main Menu`.
- Context:
  - Previous defeat flow returned directly to menu without player choice.
- Implementation:
  1. Defeat now transitions `InRun -> GameOver`.
  2. Added `GameOver` overlay UI with two actions.
  3. `Restart` sends `StartRunEvent` and returns to `InRun` with fresh run state.
  4. `Main Menu` transitions to `MainMenu`.
- Unit Tests Required:
  - Existing core state tests still pass.
- Acceptance Criteria:
  - Player can restart immediately after defeat without re-entering main menu.

## CRU-039 - Enemy Collision Activation
- Status: `DONE`
- Type: `Gameplay/Physics`
- Priority: `P1`
- Depends on: none
- Goal: Activate collision behavior so enemies do not stack into one point.
- Context:
  - Collision module existed but plugin was not registered in app wiring.
- Implementation:
  1. Registered `CollisionPlugin` in runtime plugin setup.
  2. Preserved existing rule set for enemy-enemy and inner-ring friendly interactions.
- Unit Tests Required:
  - Existing collision tests remain valid.
- Acceptance Criteria:
  - Enemy bodies maintain separation instead of collapsing into a single stack.

## CRU-040 - Infantry Knight Range Balance Pass
- Status: `DONE`
- Type: `Balance`
- Priority: `P2`
- Depends on: none
- Goal: Slightly increase infantry knight attack range.
- Context:
  - Requested micro-buff for frontline feel and contact consistency.
- Implementation:
  1. Increased `recruit_infantry_knight.attack_range` in `assets/data/units.json`.
- Unit Tests Required:
  - Config loader/validation tests.
- Acceptance Criteria:
  - Knight range increase is active in runtime and validated by config tests.

## CRU-041 - Floor Artifact Cleanup (Opaque Foliage Squares)
- Status: `DONE`
- Type: `Visual`
- Priority: `P1`
- Depends on: none
- Goal: Remove remaining square floor artifacts while keeping light battlefield variation.
- Context:
  - Decorative overlay used an opaque tile that created visible square stamps.
  - Fix should preserve deterministic placement logic and avoid introducing new map seams.
- Implementation:
  1. Switched decorative overlay source to a transparent Ishtar detail tile.
  2. Tuned overlay render size/alpha for subtle, non-blocky floor variation.
  3. Kept deterministic placement helper unchanged.
- Unit Tests Required:
  - Existing deterministic placement test remains valid.
- Acceptance Criteria:
  - Circled floor artifacts from issue screenshot are no longer visible.
  - Background remains textured and readable.

## CRU-042 - Randomized Batched Enemy Spawning
- Status: `DONE`
- Type: `Gameplay/AI`
- Priority: `P0`
- Depends on: none
- Goal: Spawn enemies in wave batches at random map positions rather than ring-border bursts.
- Context:
  - Ring-border all-at-once spawning caused predictable pressure and clutter spikes.
  - Needed wave pacing while preserving infinite-wave scaling and deterministic behavior for tests.
- Implementation:
  1. Added queued pending wave batches in `WaveRuntime`.
  2. Added wave-scaled batch size + interval processing system.
  3. Replaced border ring spawn points with pseudo-random playable-area spawn positions (with commander distance guard).
- Unit Tests Required:
  - Batch size/interval scaling tests.
  - Random spawn bounds/distance validity tests.
- Acceptance Criteria:
  - Enemies no longer appear only along map border.
  - Wave enemies arrive in staggered groups instead of one frame burst.

## CRU-043 - In-Run Escape Pause Overlay
- Status: `DONE`
- Type: `UI/Flow`
- Priority: `P0`
- Depends on: none
- Goal: Make `Escape` open a pause menu only during live matches.
- Context:
  - Prior flow allowed Escape-to-resume without menu and lacked explicit in-match pause actions.
  - Request requires centered pause menu similar to defeat menu.
- Implementation:
  1. Added `Paused` overlay with buttons: `Resume`, `Restart`, `Main Menu / Quit`.
  2. Added pause-menu action handler:
     - `Resume` -> `InRun`
     - `Restart` -> resets run + sends `StartRunEvent`
     - `Main Menu / Quit` -> `MainMenu`
  3. Removed Escape resume toggle from `Paused`; Escape now only opens pause from `InRun`.
- Unit Tests Required:
  - Existing core state tests remain valid.
- Acceptance Criteria:
  - Escape has effect only during `InRun`.
  - Pausing shows centered menu with required button order and behavior.

## CRU-044 - Enemy-In-Formation Damage Vulnerability
- Status: `DONE`
- Type: `Gameplay/Combat`
- Priority: `P1`
- Depends on: none
- Goal: Increase friendly damage against enemies that are inside the current player formation footprint.
- Context:
  - Intended to reward aggressive melee positioning and tighter formation play.
  - Must be deterministic and testable, with no effect when commander is alone.
- Implementation:
  1. Added formation-context extraction (commander position + active recruit count) in combat target snapshot flow.
  2. Added square-formation bounds helper and inside-bounds check.
  3. Applied `1.2x` multiplier to friendly outgoing damage when enemy target is inside those bounds.
- Unit Tests Required:
  - Formation context extraction test.
  - Inside-formation multiplier test (`1.2` inside / `1.0` outside).
  - Bounds helper behavior test for zero-recruit case.
- Acceptance Criteria:
  - Enemies inside formation take 20% increased incoming friendly damage.
  - Commander-only state does not receive this bonus.

## CRU-045 - Banner Visibility and Minimap Tracking Pass
- Status: `DONE`
- Type: `Gameplay/UI`
- Priority: `P1`
- Depends on: none
- Goal: Improve banner readability and tactical navigation by surfacing banner/rescue information on minimap.
- Context:
  - Banner was frequently hidden behind commander stack.
  - Dropped-banner sprite readability was weak in combat clutter.
  - Players need fast directional cues for dropped banner and rescuable retinue.
- Implementation:
  1. Added vertical follow offset for banner while attached to commander.
  2. Switched dropped-banner visual to standard upright banner asset for stronger silhouette.
  3. Added minimap markers for dropped banner and rescuable retinue entities.
- Unit Tests Required:
  - Banner follow-translation helper test.
  - Existing minimap position conversion tests remain valid.
- Acceptance Criteria:
  - Banner is clearly visible during movement.
  - Dropped banner is easy to spot in world and on minimap.
  - Rescuables are visible on minimap.

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
