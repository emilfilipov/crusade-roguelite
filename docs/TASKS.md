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

## CRU-057 - Diamond Slot Ordering Around Commander
- Status: `DONE`
- Type: `Gameplay/Formation`
- Priority: `P1`
- Depends on: `CRU-055`
- Goal: Ensure diamond formation distributes units in a visibly different ordered pattern around the commander.
- Context:
  - Rotated coordinates alone were valid geometrically but could still look too similar in fill order.
- Implementation:
  1. Added explicit diamond slot comparator by ring distance first.
  2. Added clockwise slot ordering per ring around the commander (starting near top).
  3. Kept the same diamond geometry while improving assignment/readability.
- Unit Tests Required:
  - Ring-order monotonic test for diamond offsets.
- Acceptance Criteria:
  - Diamond formation unit ordering around commander is distinct and predictable.

## CRU-053 - Formation Skillbar Runtime (10 Slots, 1..0 Activation)
- Status: `DONE`
- Type: `Gameplay/UI`
- Priority: `P0`
- Depends on: none
- Goal: Add a bottom-center skillbar that hosts active skills/formations with keyboard activation.
- Context:
  - Formations are now player-facing active selections and need a deterministic runtime owner.
  - Commander starts in `Square`, so slot `1` must be pre-populated and active by default.
- Implementation:
  1. Added `FormationSkillBar` resource with fixed capacity (`10`) and active-slot tracking.
  2. Added hotkey activation (`1..0`) for slot selection during `InRun`.
  3. Enforced exclusive active formation selection through `ActiveFormation`.
- Unit Tests Required:
  - Default skillbar state test (square in slot 1, active).
  - Activation test (switch to diamond when slot is selected).
  - Duplicate/full-slot rejection tests.
- Acceptance Criteria:
  - Skillbar exists with square active on run start.
  - Pressing `1..0` activates the corresponding skill if present.
  - Only one formation can be active at once.

## CRU-054 - One-Time Skillbar Upgrade Entries + Draft Filtering
- Status: `DONE`
- Type: `Gameplay/Progression`
- Priority: `P0`
- Depends on: `CRU-053`
- Goal: Support one-time active-skill upgrades and remove skillbar-bound cards when the bar is full.
- Context:
  - Formation unlock cards should not reappear after pick.
  - Skillbar additions should not be offered when no slot is available.
- Implementation:
  1. Extended `UpgradeConfig` with `one_time`, `adds_to_skillbar`, and `formation_id`.
  2. Added one-time tracker resource reset on run start.
  3. Filtered draft pool by one-time history and skillbar capacity/contents.
  4. Added `unlock_formation_diamond` to `assets/data/upgrades.json`.
- Unit Tests Required:
  - One-time card exclusion after acquisition.
  - Skillbar-full exclusion for `adds_to_skillbar` upgrades.
- Acceptance Criteria:
  - Picked one-time upgrades do not return in future drafts.
  - Skillbar-bound upgrades are absent when skillbar is full.

## CRU-055 - Diamond Formation Gameplay Modifiers
- Status: `DONE`
- Type: `Gameplay/Balance`
- Priority: `P0`
- Depends on: `CRU-053`
- Goal: Add Diamond formation with offense bonus while moving, speed bonus, and defense penalty.
- Context:
  - Square was normalized to neutral baseline (`x1`) to make Diamond a clear tactical choice.
- Implementation:
  1. Expanded `formations.json` with `diamond` and per-formation runtime fields:
     - `offense_while_moving_multiplier`
     - `move_speed_multiplier`
  2. Applied formation move-speed multiplier in commander movement.
  3. Applied moving offense bonus through combat multiplier path.
  4. Applied defense multiplier in friendly effective armor path.
- Unit Tests Required:
  - Moving offense helper test.
  - Formation bounds and switching behavior tests.
- Acceptance Criteria:
  - Diamond is selectable and changes combat/movement behavior as designed.
  - Square remains neutral baseline.

## CRU-056 - Formation Icons + Skillbar HUD Rendering
- Status: `DONE`
- Type: `UI/Visual`
- Priority: `P1`
- Depends on: `CRU-053`
- Goal: Add clear visual identifiers for formations in cards and the skillbar.
- Context:
  - Requested simple dot-pattern icons (dice-like readability).
- Implementation:
  1. Added generated icons:
     - `assets/sprites/skills/formation_square.png`
     - `assets/sprites/skills/formation_diamond.png`
  2. Loaded formation icons into `ArtAssets`.
  3. Added bottom-center skillbar HUD with 10 slots, active highlight, and key labels.
  4. Routed formation upgrade card icons to the new assets.
- Unit Tests Required:
  - Existing UI logic tests + metadata/icon mapping tests.
- Acceptance Criteria:
  - Formation cards and skillbar slots show correct icons.
  - Active skillbar slot is visibly highlighted.

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
  2. Enemy death drop events now spawn packs with delay (`0.9s`) before pickup/homing.
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
  1. Added `Paused` overlay with buttons: `Resume`, `Restart`, `Main Menu`.
  2. Added pause-menu action handler:
     - `Resume` -> `InRun`
     - `Restart` -> resets run + sends `StartRunEvent`
     - `Main Menu` -> `MainMenu`
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
  2. Added active-formation bounds helper and inside-bounds check.
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

## CRU-046 - Minimap XP Pack Markers
- Status: `DONE`
- Type: `UI/HUD`
- Priority: `P1`
- Depends on: none
- Goal: Surface XP pack locations on minimap for clearer pickup routing.
- Context:
  - XP packs are now an active on-map collection loop and need minimap visibility.
  - Existing minimap already draws commander, friendlies, enemies, rescuables, and banner.
- Implementation:
  1. Added `ExpPack` query path to minimap refresh system.
  2. Added yellow minimap dot color constant and max blip cap for XP packs.
  3. Rendered XP pack dots during periodic minimap redraw.
- Unit Tests Required:
  - Existing minimap world-to-panel mapping tests.
- Acceptance Criteria:
  - Active XP packs appear on minimap as yellow dots.
  - XP minimap rendering obeys per-type blip cap.

## CRU-047 - Formation-Pressure Movement Slowdown
- Status: `DONE`
- Type: `Gameplay/Movement`
- Priority: `P1`
- Depends on: none
- Goal: Slow commander movement based on enemy units inside the formation footprint.
- Context:
  - Requested tradeoff: aggressive formation charges should carry mobility risk.
  - Slowdown must be capped so movement never fully locks.
- Implementation:
  1. Added enemy-inside-active-formation-footprint detection helper in `src/squad.rs`.
  2. Added per-enemy slowdown multiplier with clamp floor.
  3. Applied multiplier in commander movement pipeline alongside existing penalties.
- Unit Tests Required:
  - Slowdown multiplier floor/cap behavior test.
  - Inside-formation bounds helper behavior test.
- Acceptance Criteria:
  - Commander speed decreases as more enemies are inside formation bounds.
  - Movement speed stays above configured minimum multiplier.

## CRU-048 - Pause Menu Label Cleanup
- Status: `DONE`
- Type: `UI/Flow`
- Priority: `P2`
- Depends on: none
- Goal: Rename in-run pause button from `Main Menu / Quit` to `Main Menu`.
- Context:
  - Current pause flow returns to main menu and no longer directly quits.
- Implementation:
  1. Updated pause menu button label text.
- Unit Tests Required:
  - Existing UI state/flow tests.
- Acceptance Criteria:
  - Pause menu third button displays `Main Menu`.

## CRU-049 - Mandatory Level-Up Draft Screen (3 Cards)
- Status: `DONE`
- Type: `Gameplay/UI`
- Priority: `P0`
- Depends on: none
- Goal: Pause run on level-up until player picks one of three upgrade cards.
- Context:
  - Prior flow auto-resolved upgrades in-run.
  - New flow requires explicit player selection with no skip path.
- Implementation:
  1. Added `GameState::LevelUp`.
  2. Reworked upgrade flow to open draft state on level threshold and resume only after selection event.
  3. Added 3-option draft roll, keyboard `1..3` support, and card-click selection.
  4. Added full-screen level-up overlay with tall cards (title + icon + description).
  5. Kept pause toggle constrained to `InRun`, so Escape is inactive in `LevelUp`.
- Unit Tests Required:
  - Upgrade option count test (`3` options).
  - Upgrade metadata mapping test for UI display text/icon routing.
- Acceptance Criteria:
  - On level-up, gameplay pauses and level-up overlay appears.
  - Player must select one card to continue.
  - Escape does not open pause menu while level-up overlay is active.

## CRU-050 - Weighted 8-Upgrade Draft Overhaul (3 Choices)
- Status: `DONE`
- Type: `Gameplay/Progression`
- Priority: `P0`
- Depends on: none
- Goal: Replace legacy upgrade pool with the agreed 8-upgrade set and random 3-choice drafts.
- Context:
  - Upgrade pool changed from mixed placeholder upgrades to a fixed tactical set.
  - Value strength needs min/max weighted randomness (higher rolls rarer).
- Implementation:
  1. Extended upgrade schema with `min_value`, `max_value`, `value_step`, `weight_exponent`.
  2. Replaced `assets/data/upgrades.json` with the new 8 upgrades.
  3. Added deterministic run-seeded RNG for upgrade option/value rolling.
  4. Updated level-up UI/input to 3 selections (`1..3`) and 3 cards on screen.
- Unit Tests Required:
  - Unique 3-option draft test.
  - Weighted roll min/max bounds test.
- Acceptance Criteria:
  - Each level-up offers random 3 upgrades from the 8-upgrade pool.
  - Rolled values stay within min/max and are weighted toward lower values.
- Superseded Note:
  - Later expanded by `CRU-054` with one-time skillbar-bound formation unlock entries layered on top of the repeatable pool.

## CRU-051 - Commander Aura Effects (Authority + Hospitalier)
- Status: `DONE`
- Type: `Gameplay/Systems`
- Priority: `P0`
- Depends on: `CRU-050`
- Goal: Activate commander aura upgrades as real runtime mechanics with in-range-only effects.
- Context:
  - Aura hooks existed but were mostly placeholder.
  - Requested effects are range-gated and additive across level-ups.
- Implementation:
  1. Expanded `GlobalBuffs` for aura radius, authority mitigation/drain, and hospitalier regens.
  2. Authority aura:
     - reduces friendly morale/cohesion loss for damage/death events while in aura.
     - applies passive morale drain to enemies in aura.
  3. Hospitalier aura:
     - applies passive HP/morale regen to friendlies in aura.
     - applies cohesion regen scaled by friendly aura coverage.
  4. Added aura-radius helper using commander base aura + upgrade bonus.
- Unit Tests Required:
  - Authority mitigation multiplier test.
  - Commander aura radius bonus test.
- Acceptance Criteria:
  - Aura effects apply only to entities inside commander aura radius.
  - Stacked upgrades increase aura strength additively.

## CRU-052 - Commander Ranged Arrow Attack
- Status: `DONE`
- Type: `Gameplay/Combat`
- Priority: `P0`
- Depends on: none
- Goal: Add commander ranged projectile attack used only when enemies are outside melee range.
- Context:
  - Commander needed non-instant ranged capability with physical arrows.
  - Arrow must despawn on hit or max travel distance.
- Implementation:
  1. Added commander ranged config fields in `units.json` and data schema.
  2. Added commander ranged attack profile + cooldown components.
  3. Added ranged attack system that fires arrows only when target is outside melee range and inside ranged range.
  4. Updated projectile runtime to despawn by remaining travel distance or collision.
- Unit Tests Required:
  - Projectile travel-distance depletion test.
  - Existing combat/projectile tests.
- Acceptance Criteria:
  - Commander shoots arrows at valid ranged targets.
  - Arrows disappear on hit or when max travel distance is consumed.

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
