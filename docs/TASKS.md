# TASKS.md

## Planning Notes
- Board style: Jira-like backlog with task keys, dependencies, implementation steps, and acceptance criteria.
- Primary stack: Rust + Bevy.
- Primary target: Windows (`x86_64-pc-windows-msvc`).
- Distribution: Steam-ready, but local Windows installer is required from early milestones.
- Initial gameplay scaffold: square formation only, commander-only start, rescue-based recruitment, and one recruitable unit archetype.
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

## CRU-001 - Bootstrap Rust + Bevy Project Skeleton
- Status: `DONE`
- Type: `Setup`
- Priority: `P0`
- Depends on: none
- Goal: Create the initial Bevy app with plugin-oriented structure and runnable desktop window.
- Implementation:
  1. Initialize cargo project and add Bevy dependency.
  2. Add module/plugin folders: `core`, `squad`, `formation`, `combat`, `enemies`, `upgrades`, `morale`, `banner`, `ui`, `map`, `packaging`.
  3. Add root plugin registration order in `main.rs`.
  4. Add app state enum (`Boot`, `MainMenu`, `InRun`, `Paused`, `GameOver`).
  5. Add placeholder startup scene (camera + simple background color/quad).
- Unit Tests Required:
  - Plugin registration test that app initializes all required plugins.
  - State transition smoke test (`Boot` -> `MainMenu`).
- Acceptance Criteria:
  - App launches from `cargo run`.
  - Project compiles cleanly with no clippy warnings.
  - Tests pass.

## CRU-002 - Toolchain, Linting, and CI Quality Gates
- Status: `DONE`
- Type: `DevEx`
- Priority: `P0`
- Depends on: `CRU-001`
- Goal: Enforce stable, repeatable quality checks locally and in CI.
- Implementation:
  1. Add `rust-toolchain.toml` pinned to stable toolchain.
  2. Add `.cargo/config.toml` for Windows target defaults where appropriate.
  3. Add CI workflow for fmt, clippy, tests, and Windows release build.
  4. Add helper scripts (`scripts/check.ps1`, `scripts/build_windows.ps1`).
  5. Document required commands in `README.md`.
- Unit Tests Required:
  - Script argument parsing tests if script logic includes conditionals.
- Acceptance Criteria:
  - CI runs all gates on pull request.
  - Local script executes same checks as CI.
  - Build succeeds for `x86_64-pc-windows-msvc`.

## CRU-003 - Data-Driven Gameplay Config Pipeline
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-001`
- Goal: Load unit, enemy, formation, and upgrade values from data files.
- Implementation:
  1. Create `assets/data` schema files (`units`, `enemies`, `formations`, `upgrades`, `waves`).
  2. Implement serde-backed config structs and loader service.
  3. Add validation pass for required fields and value ranges.
  4. Add startup failure messages that identify invalid file and key.
- Unit Tests Required:
  - Deserialize valid sample files.
  - Reject invalid/missing fields.
  - Validate numeric bounds (for example no negative cooldown).
- Acceptance Criteria:
  - Game boots with data from `assets/data`.
  - Validation errors are actionable and deterministic.

## CRU-004 - Core Run Loop and Session State
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-001`, `CRU-003`
- Goal: Establish one playable run lifecycle and reset flow.
- Implementation:
  1. Add run session resource (time survived, squad roster, morale, xp).
  2. Implement transitions: `MainMenu` -> `InRun` -> `GameOver` -> `MainMenu`.
  3. Implement deterministic reset of run resources on new run.
  4. Add pause toggle behavior for `InRun`.
- Unit Tests Required:
  - Session reset test restores default values.
  - Pause toggle test freezes run-tick systems.
- Acceptance Criteria:
  - Player can start, lose, and restart a run without stale state.

## CRU-005 - Squad Movement Controller
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-004`
- Goal: Player controls one squad blob via movement input.
- Implementation:
  1. Create squad anchor entity with velocity and acceleration tuning.
  2. Read keyboard/mouse input and move anchor in world space.
  3. Clamp movement to map bounds.
  4. Add basic damping/friction for readable movement.
- Unit Tests Required:
  - Movement integration math tests (accel, max speed, damping).
  - Boundary clamp tests.
- Acceptance Criteria:
  - Squad anchor movement is responsive and deterministic.
  - No unit-level manual control exists.

## CRU-006 - Formation System (Square First)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-005`, `CRU-003`
- Goal: Implement `Square` formation as the only playable formation in the first scaffold.
- Implementation:
  1. Add formation resource and lock initial playable option to `Square`.
  2. Compute slot offsets for square relative to squad anchor.
  3. Apply square formation modifiers from data.
  4. Add transition smoothing to avoid teleport snapping.
  5. Keep data/model structures extensible for later line/wedge additions without enabling them yet.
- Unit Tests Required:
  - Slot generation tests for square counts.
  - Modifier application tests.
  - Transition interpolation tests.
- Acceptance Criteria:
  - Square formation works at runtime.
  - No other formation is exposed in MVP scaffold UI/input.
  - Visible unit placement reflects square slot layout.

## CRU-007 - Commander-First Start and Initial Unit Archetype
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-003`, `CRU-006`
- Goal: Start each run with commander (`Baldiun`) only, and support one recruitable soldier archetype (`Infantry` with first subtype `Knight`).
- Implementation:
  1. Define commander component set (support stats, aura radius placeholders, battle-cry placeholders, basic auto-attack profile).
  2. Define recruit archetype/subtype components for initial soldier (`Infantry/Knight`).
  3. Build run-start spawn flow that creates commander only.
  4. Bind commander and future recruits to square formation slot assignment.
  5. Add runtime roster change hooks for recruitment and upgrades.
- Unit Tests Required:
  - Commander run-start spawn test (exactly one commander, zero recruits).
  - Unit stat derivation tests from config values.
  - Slot assignment validity tests (no duplicate slot occupancy).
- Acceptance Criteria:
  - New run starts with commander only.
  - Commander can auto-attack basic enemies.
  - Initial recruit archetype is the only recruitable soldier type in scaffold.
  - Roster and formation stay synchronized.

## CRU-021 - Rescue Recruitment Loop (Stand-and-Rescue)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-005`, `CRU-007`
- Goal: Recruit soldiers by standing near them for a fixed rescue duration, with no hard retinue cap in MVP.
- Implementation:
  1. Add neutral rescuable soldier entities with recruit payload (`Infantry/Knight`).
  2. Add rescue channel system (proximity check, timer progress, cancel on leaving radius).
  3. On rescue completion, convert neutral soldier into squad recruit and assign formation slot.
  4. Add unlimited-retinue handling in data model and guardrails for performance telemetry.
  5. Add UI event hooks for rescue started/progress/completed.
- Unit Tests Required:
  - Rescue timer progress and cancellation tests.
  - Recruitment conversion tests (neutral -> squad member).
  - Retinue growth tests for repeated rescues.
- Acceptance Criteria:
  - Player can rescue soldiers by standing nearby for defined duration.
  - Rescued soldiers immediately join active retinue.
  - No hard cap blocks additional rescues in MVP scaffold.

## CRU-008 - Enemy Waves and Spawner
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-004`, `CRU-003`, `CRU-021`
- Goal: Spawn escalating waves, starting with basic infantry enemies for early commander-only progression.
- Implementation:
  1. Implement wave timeline resource using data-defined schedule.
  2. Add spawn points around map perimeter.
  3. Spawn enemies by archetype and count (start with infantry only, extend later).
  4. Track alive enemy budget for pacing safety.
- Unit Tests Required:
  - Wave scheduler timing tests.
  - Spawn composition tests by wave id.
- Acceptance Criteria:
  - Waves progress over time without manual triggers.
  - Enemy mix matches data definitions.

## CRU-009 - Enemy Behavior (Chase, Kite, Charge)
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-008`, `CRU-007`
- Goal: Give each enemy archetype readable behavior.
- Implementation:
  1. Infantry: direct chase nearest valid squad target.
  2. Archers: maintain preferred range and fire cadence.
  3. Cavalry: charge windows with cooldown and bonus impact.
  4. Add simple local avoidance so enemies do not fully stack.
- Unit Tests Required:
  - Behavior decision tests per archetype.
  - Charge cooldown/state machine tests.
- Acceptance Criteria:
  - Enemy types feel behaviorally distinct.
  - No permanent AI deadlock states.

## CRU-010 - Auto-Combat and Damage Resolution
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-007`, `CRU-009`
- Goal: Implement automatic attacking for commander, recruited squad members, and enemies.
- Implementation:
  1. Add targeting system (closest threat with role filters).
  2. Add attack timers and windup/recovery cycle.
  3. Resolve hit formula with armor, formation, and morale modifiers.
  4. Emit combat events for UI/FX hooks.
- Unit Tests Required:
  - Damage formula tests (armor floor, modifiers, min/max bounds).
  - Target selection tests with mixed candidates.
  - Attack cadence timer tests.
- Acceptance Criteria:
  - Commander can clear early enemies solo via auto-attacks.
  - Recruited units fight automatically once rescued.
  - Combat results are deterministic under fixed timestep.

## CRU-011 - Projectile System for Ranged Units
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-010`
- Goal: Support archer projectile attacks for squad and enemies.
- Implementation:
  1. Add projectile spawn event and projectile component.
  2. Integrate travel, lifetime, and collision checks.
  3. Apply hit damage via shared combat pipeline.
  4. Add simple pooling or despawn control for performance.
- Unit Tests Required:
  - Projectile travel math tests.
  - Collision/hit registration tests.
  - Lifetime expiry tests.
- Acceptance Criteria:
  - Ranged attacks are visible and mechanically functional.

## CRU-012 - Unit Death, Casualties, and Squad Integrity
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-010`
- Goal: Units can die, be removed from formation, and affect squad performance.
- Implementation:
  1. Handle hp <= 0 transitions and despawn/death markers.
  2. Repack formation slots when casualties occur.
  3. Update squad aggregate stats after each casualty.
  4. Add death events for morale and UI.
- Unit Tests Required:
  - Casualty handling tests.
  - Formation repack tests after multiple deaths.
  - Aggregate stat recalculation tests.
- Acceptance Criteria:
  - Casualties are reflected immediately in formation and power.

## CRU-013 - Morale/Cohesion System
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-012`
- Goal: Implement cohesion meter that modifies combat effectiveness.
- Implementation:
  1. Add cohesion resource with decay/recovery rules.
  2. Apply penalties at thresholds (attack speed, defense, formation looseness).
  3. Trigger collapse risk state below critical cohesion.
  4. Integrate cohesion changes from casualties and banner state.
- Unit Tests Required:
  - Threshold modifier tests.
  - Decay/recovery curve tests.
  - Collapse trigger condition tests.
- Acceptance Criteria:
  - Low cohesion creates clearly felt penalties.
  - Cohesion changes are explainable and predictable.

## CRU-014 - Banner Failure and Recovery
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-013`
- Goal: Banner acts as squad anchor with failure/recovery loop.
- Implementation:
  1. Add banner entity tied to squad state.
  2. Define drop conditions and dropped-banner world state.
  3. Apply morale/cohesion penalties while banner is down.
  4. Add recovery action when squad returns to banner position.
- Unit Tests Required:
  - Banner drop condition tests.
  - Penalty application/removal tests.
  - Recovery flow tests.
- Acceptance Criteria:
  - Banner loss and recovery materially affect survival decisions.

## CRU-015 - Upgrade Draft and Level-Up Choices
- Status: `DONE`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-007`, `CRU-013`, `CRU-021`, `CRU-003`
- Goal: Offer upgrade choices that build army identity during a run.
- Implementation:
  1. Add XP gain events and level thresholds.
  2. Pause run at level-up and show choice set.
  3. Implement starter categories: add units, armor, damage, attack speed, morale/cohesion, commander support bonuses (auras/battle cries).
  4. Ensure upgrades are data-driven and stack safely.
- Unit Tests Required:
  - Upgrade eligibility and roll logic tests.
  - Stat stacking tests.
  - XP threshold progression tests.
- Acceptance Criteria:
  - Player repeatedly receives meaningful upgrade choices in-run.

## CRU-016 - First Map: Desert Battlefield + Oasis Zone
- Status: `DONE`
- Type: `Content`
- Priority: `P1`
- Depends on: `CRU-004`
- Goal: Deliver one readable map with optional healing oasis.
- Implementation:
  1. Build map bounds, terrain markers, and spawn ring.
  2. Add oasis zone trigger with limited healing behavior.
  3. Keep visuals placeholder-friendly but stylistically dusty/grounded.
  4. Validate readability at expected camera distance.
- Unit Tests Required:
  - Zone trigger and healing rate tests.
  - Map bounds enforcement tests.
- Acceptance Criteria:
  - Single playable map supports full MVP loop.

## CRU-017 - HUD and Combat Readability
- Status: `DONE`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-013`, `CRU-014`, `CRU-015`
- Goal: Display key tactical state clearly.
- Implementation:
  1. Add HUD elements for cohesion, banner status, squad size, XP, wave index.
  2. Add rescue progress indicator and rescue completion feedback.
  3. Add formation indicator (square) and future-formation placeholder state.
  4. Add game over summary with cause and time survived.
  5. Keep UI minimal and legible under combat load.
- Unit Tests Required:
  - UI state mapping tests (resource values -> displayed values).
- Acceptance Criteria:
  - Player can read critical state without pausing.

## CRU-018 - Windows Installer Pipeline (Local QA)
- Status: `DONE`
- Type: `Release`
- Priority: `P0`
- Depends on: `CRU-002`
- Goal: Produce installable Windows `.exe` for non-Steam local testing.
- Implementation:
  1. Add installer toolchain (Inno Setup script recommended).
  2. Package release binary + assets + required runtime files.
  3. Add versioned installer output naming convention.
  4. Add script command (`scripts/package_windows_installer.ps1`).
  5. Document install/uninstall QA checklist.
- Unit Tests Required:
  - Script path/version parsing tests if script contains logic.
- Acceptance Criteria:
  - One command produces a working installer.
  - Fresh Windows machine can install and launch game.

## CRU-019 - Steam Readiness via Feature-Gated Integration
- Status: `DONE`
- Type: `Release`
- Priority: `P2`
- Depends on: `CRU-018`
- Goal: Keep Steam integration optional while preserving local non-Steam builds.
- Implementation:
  1. Add `steam` cargo feature and conditional compile gates.
  2. Isolate platform API wrappers behind interface trait.
  3. Implement no-op fallback for non-Steam builds.
  4. Add build commands for both feature sets.
- Unit Tests Required:
  - Feature-gated interface tests for steam/no-steam variants.
- Acceptance Criteria:
  - Build succeeds with and without `--features steam`.
  - Local installer works without Steam client/account.

## CRU-020 - MVP Stabilization and Regression Suite
- Status: `DONE`
- Type: `QA`
- Priority: `P0`
- Depends on: `CRU-005` through `CRU-018`, `CRU-021`
- Goal: Lock MVP behavior and prevent regressions.
- Implementation:
  1. Add deterministic simulation tests for combat, morale, and wave pacing.
  2. Add smoke test that runs headless tick progression.
  3. Add bug triage template and known-issues file.
  4. Define release checklist for each build candidate.
- Unit Tests Required:
  - Regression cases for previously fixed gameplay bugs.
  - End-to-end smoke test for run lifecycle.
- Acceptance Criteria:
  - Core MVP loop remains stable across consecutive changes.
  - No critical regression in movement, rescue recruitment, combat, formation, morale, banner, or upgrades.

## CRU-022 - Enemy Identity Pass (`bandit_raider`)
- Status: `DONE`
- Type: `Gameplay/Content`
- Priority: `P1`
- Depends on: `CRU-008`, `CRU-009`, `CRU-010`
- Goal: Rename and align the MVP enemy implementation to a concrete identity (`bandit_raider`) across data, runtime enums, and docs.
- Implementation:
  1. Rename enemy data key from `infantry_melee` to `bandit_raider`.
  2. Rename runtime unit kind from `EnemyInfantry` to `EnemyBanditRaider`.
  3. Update enemy spawn pipeline and config validation to use the new identity key.
  4. Update all impacted tests and documentation references.
- Unit Tests Required:
  - Data loading tests cover renamed enemy schema.
  - Existing enemy selection/chase tests continue to pass.
- Acceptance Criteria:
  - Game loads/spawns enemy via `bandit_raider` config key only.
  - No stale `infantry_melee` or `EnemyInfantry` references remain in runtime code.

## CRU-023 - Bandit Visual State Mapping (Idle/Move/Attack/Hit/Dead)
- Status: `DONE`
- Type: `Gameplay/Visual`
- Priority: `P1`
- Depends on: `CRU-022`
- Goal: Improve enemy readability by swapping bandit sprite variants based on combat/movement state.
- Implementation:
  1. Add bandit visual state components (`BanditVisualState`, `BanditVisualRuntime`).
  2. Add tiny-dungeon sprite handles for all state variants in `ArtAssets`.
  3. Add enemy visual update system that maps health/cooldown/movement to state-specific textures.
  4. Keep logic deterministic and testable via pure state-decision function.
- Unit Tests Required:
  - State priority tests (`Dead` -> `Hit` -> `Attack` -> `Move` -> `Idle`).
- Acceptance Criteria:
  - Bandit sprites visibly change state during movement/combat.
  - State decisions are deterministic and test-covered.

## CRU-024 - Windows Installer Slimming (Runtime Assets Only)
- Status: `DONE`
- Type: `Release`
- Priority: `P1`
- Depends on: `CRU-018`, `CRU-023`
- Goal: Reduce installer payload by packaging only runtime-required assets rather than entire `assets` tree.
- Implementation:
  1. Replace broad `assets\*` include rule in Inno Setup script with explicit runtime subsets.
  2. Keep `assets/data` plus active art pack subsets only.
  3. Include third-party `License.txt` files for bundled packs.
  4. Validate packaging still resolves all runtime asset paths.
- Unit Tests Required:
  - N/A (packaging script change; validated via packaging build step).
- Acceptance Criteria:
  - Installer generation succeeds with reduced asset scope.
  - Installed game launches with no missing-asset errors for current MVP content.

## CRU-025 - Combat Readability Pass v1 (World-Space Health Bars)
- Status: `DONE`
- Type: `Gameplay/UI`
- Priority: `P1`
- Depends on: `CRU-017`, `CRU-023`
- Goal: Improve in-combat readability for unit survivability and target priority.
- Implementation:
  1. Add lightweight world-space health bars for friendly/enemy units.
  2. Attach bars automatically to units at runtime and update fill width from HP ratio.
  3. Apply team-color coding to improve friend/enemy scan speed.
  4. Keep implementation data-light and cheap for MVP entity counts.
- Unit Tests Required:
  - Health-bar fill width clamp tests.
- Acceptance Criteria:
  - Health state is readable during active fights without opening menus.
  - New UI logic remains deterministic and clippy-clean.

## CRU-026 - Balance Pass v1 (Enemy/Wave/Rescue Cadence)
- Status: `DONE`
- Type: `Balance`
- Priority: `P1`
- Depends on: `CRU-022`, `CRU-025`
- Goal: Smooth early-run pacing for commander-only starts and rescue onboarding.
- Implementation:
  1. Tune `bandit_raider` stat line in `assets/data/enemies.json`.
  2. Adjust wave schedule/counts for gentler early pressure and clearer ramp.
  3. Adjust rescue cadence values for slightly faster onboarding into squad growth.
  4. Add wave validation guard ensuring strictly increasing wave times.
- Unit Tests Required:
  - Config validation test for unsorted wave times.
- Acceptance Criteria:
  - Early run pacing is more recoverable while still escalating.
  - Balance data remains fully data-driven and validation-hardened.

---

## Recommended Implementation Order
1. `CRU-001`
2. `CRU-002`
3. `CRU-003`
4. `CRU-004`
5. `CRU-005`
6. `CRU-006`
7. `CRU-007`
8. `CRU-021`
9. `CRU-008`
10. `CRU-009`
11. `CRU-010`
12. `CRU-011`
13. `CRU-012`
14. `CRU-013`
15. `CRU-014`
16. `CRU-015`
17. `CRU-016`
18. `CRU-017`
19. `CRU-018`
20. `CRU-019`
21. `CRU-020`
22. `CRU-022`
23. `CRU-023`
24. `CRU-024`
25. `CRU-025`
26. `CRU-026`
