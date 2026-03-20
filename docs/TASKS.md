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

## Requested Update Summary (Grouped)
1. In-match UX and pause behavior:
   - Add five in-run screens with hotkeys and top-right quick-access buttons:
     - `Inventory` (`I`)
     - `Stats` (`O`)
     - `Skill Book` (`P`)
     - `Archive/Bestiary` (`K`)
     - `Unit Upgrade` (`U`)
   - Opening any of these screens must pause gameplay until closed.
   - Move elapsed time below wave number on HUD.
   - Show commander level as `current level / max allowed level` where max is reduced by roster level-cost locks.
2. Main menu and run entry flow:
   - Replace `Start` with `Play Offline`.
   - Add disabled `Play Online` entry (not developed).
   - Add `Bestiary` entry in main menu between `Settings` and `Exit`.
   - `Play Offline` opens a match setup screen (faction and map selection scaffold).
3. Progression and match pacing:
   - Rework enemy spawning to units-per-second (not fixed total per wave).
   - Increase XP requirement scaling (steeper).
   - Cap levels at `200`.
   - Cap waves at `100` while preserving scaling style.
   - Player wins by clearing wave `100`.
4. Roster economy and upgrades:
   - Introduce roster level-cost system:
     - Tier 0 peasants cost `0`.
     - Each upgrade step to a higher tier adds exactly `+1` locked level budget (`0->1` costs 1, `1->2` costs 1, and so on).
     - No demotion; death frees the unit's level cost.
   - Unit upgrades must fail when level budget disallows them.
   - If player level reaches allowed max due to locked budget, level progression is blocked until cost is freed by deaths.
   - Only tier 0 units spawn as rescues; higher tiers come from upgrades.
5. Combat readability and stability:
   - Add floating damage text.
   - Reduce enemy stacking/jitter in large crowds (collision footprint and separation tuning).
6. New level-up upgrades:
   - These three upgrades share tier-0 composition requirement controls.
   - Other upgrades may use different requirement families.
   - `Mob's Fury`: tier-0 heavy/all-peasant composition grants morale/cohesion immunity + offensive/mobility bonuses.
   - `Mob's Justice`: execute enemies below 10% HP on hit.
   - `Mob's Mercy`: reduces rescue time by `50%`.
7. Unit content:
   - Formal tier scaffolding and map rescue lock to tier 0.
   - Add `Peasant Priest` support unit with auto-cast attack-speed buff aura (10s duration, 20s cooldown, refresh on overlap).

## Active Backlog

### CRU-060 - In-Run Modal Pause State Machine
- Status: `DONE`
- Type: `Core`
- Priority: `P0`
- Depends on: `none`
- Goal: Add a single modal framework so inventory/stats/skillbook/archive/unit-upgrade screens pause simulation while open.
- Context:
  - Current in-run UI has multiple overlays; we need one authoritative modal owner to avoid input conflicts.
  - Must coexist with existing pause and level-up states.
  - Expected files: `src/core.rs`, `src/ui.rs`, `src/settings.rs`.
- Implementation:
  1. Add explicit UI modal state enum and transitions for all requested in-run screens.
  2. Route keyboard and button-open events through this state machine.
  3. Ensure simulation schedules are paused while modal is active and resume on close.
  4. Define conflict priority: level-up/defeat menus suppress other modal opens.
- Unit Tests Required:
  - Opening each modal transitions state and pauses simulation tick.
  - Closing modal restores prior run state without side effects.
  - Modal open requests are ignored during level-up/defeat overlays.
- Acceptance Criteria:
  - Pressing modal hotkeys in-run opens the correct screen and freezes gameplay.
  - Closing the screen unpauses gameplay exactly once.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-061 - Top-Right Utility Bar + Icon Asset Mapping
- Status: `DONE`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-060`
- Goal: Provide a compact top-right menu bar with icon buttons for all in-run screens.
- Context:
  - User requested non-invasive quick-access buttons plus keyboard shortcuts.
  - Icons should use existing free asset packs where possible.
  - Expected files: `src/ui.rs`, `src/visuals.rs`, `assets/sprites/ui/*`, `docs/ASSET_SOURCES.md`.
- Implementation:
  1. Create utility bar container anchored top-right with five icon buttons.
  2. Map buttons to the same events as `I/O/P/K/U`.
  3. Select and wire icon assets from existing third-party packs; generate placeholders only if no fit exists.
  4. Add hover/active visual states without obstructing combat view.
- Unit Tests Required:
  - Button-to-modal dispatch mapping is correct for all five entries.
  - Keyboard shortcut and button click dispatch identical modal events.
- Acceptance Criteria:
  - Top-right bar appears in-run with five clickable icons.
  - Each icon opens its matching screen and pauses game via modal system.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/ASSET_SOURCES.md`

### CRU-062 - Inventory Screen Scaffold (Gear + Equipment Layout)
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-060`, `CRU-061`
- Goal: Add inventory UI that can host drop items and equipment setups per unit type.
- Context:
  - Gear gameplay loop may be partial initially; UI/data structure must be future-proof.
  - Must show at least: bag list, equipped slots by unit archetype, and item tooltip scaffold.
  - Expected files: `src/ui.rs`, `src/model.rs`, `src/drops.rs`, `assets/data/drops.json`.
- Implementation:
  1. Add inventory panel layout and placeholder slot groups by unit type.
  2. Add runtime data structs for item entries and equipped assignments (scaffold if effects are not yet active).
  3. Add close interactions (`Esc`/button) returning to run state.
  4. Show empty-state messaging when no gear exists.
- Unit Tests Required:
  - Inventory data model serializes/deserializes without loss.
  - Open/close interactions do not mutate unrelated gameplay state.
- Acceptance Criteria:
  - Pressing `I` opens inventory with unit-type equipment sections and item list region.
  - UI works even when zero gear items exist.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md` (if gear scope is expanded beyond scaffold)

### CRU-063 - Stats Screen (Base + Level-Up Bonus Breakdown)
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-060`, `CRU-061`
- Goal: Provide a stats panel showing base values and additive/multiplicative bonuses from level-ups.
- Context:
  - Player requested transparent stat accounting.
  - Must include commander, global friendly modifiers, and derived totals.
  - Expected files: `src/ui.rs`, `src/upgrades.rs`, `src/model.rs`.
- Implementation:
  1. Add stats view model with base, bonus, and final columns.
  2. Populate from existing buff/upgrade systems.
  3. Expose key stats: HP, damage, attack speed, armor, move speed, pickup radius, aura radii.
  4. Add formatting and ordering for quick readability.
- Unit Tests Required:
  - Derived stat calculations match runtime formulas for representative upgrade stacks.
  - Screen rendering model remains stable when no upgrades are selected.
- Acceptance Criteria:
  - Pressing `O` opens a readable stat breakdown with base + bonus totals.
  - Values update correctly after level-ups.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-064 - Skill Book Screen (Chosen Skills Registry)
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-060`, `CRU-061`
- Goal: Show all selected skills/upgrades/formations/auras in a dedicated in-run panel.
- Context:
  - Player needs visibility into active build composition.
  - Should reflect one-time unlocks and repeatable level-up stacks.
  - Expected files: `src/ui.rs`, `src/upgrades.rs`, `assets/data/upgrades.json`.
- Implementation:
  1. Add skill-book panel listing acquired effects with current values/stacks.
  2. Group entries by category (formation, aura, passive combat, utility).
  3. Add icon + short description rendering from upgrade data.
  4. Include active/inactive indicator for mutually exclusive formation skills.
- Unit Tests Required:
  - Skill list builder includes all selected upgrades exactly once per stack semantics.
  - One-time upgrades do not duplicate in registry.
- Acceptance Criteria:
  - Pressing `P` opens a panel listing currently owned skill effects with descriptions.
  - Data matches actual runtime bonuses.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-065 - Archive/Bestiary Data Browser (In-Run + Main Menu)
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `CRU-060`, `CRU-061`
- Goal: Add a bestiary/archive browser for skills, units, stats, bonuses, and drop types.
- Context:
  - Must be reachable both in-run (`K`) and from main menu.
  - Intended as reference codex, not progression-gated content for now.
  - Expected files: `src/ui.rs`, `src/data.rs`, `assets/data/*.json`.
- Implementation:
  1. Define archive entry schema (category, title, icon, description, source references).
  2. Build sections for units, enemies, skills, stats, upgrades, and drops.
  3. Add filtering/search tabs if lightweight; otherwise category tabs for MVP.
  4. Reuse same panel renderer for in-run and main-menu entry points.
- Unit Tests Required:
  - Archive data loader validates required fields for all entries.
  - In-run and main-menu open paths load identical archive dataset.
- Acceptance Criteria:
  - Pressing `K` in match opens archive; main menu includes `Bestiary` entry opening same content.
  - Archive includes at least all currently implemented units, upgrades, and drops.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/ASSET_SOURCES.md` (if new icons/assets are added)

### CRU-066 - Main Menu Flow Overhaul
- Status: `TODO`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-065`
- Goal: Replace start flow with explicit offline/online choices and add main-menu bestiary entry.
- Context:
  - Required button order: `Play Offline`, `Play Online` (disabled), `Settings`, `Bestiary`, `Exit`.
  - `Play Online` must be visibly disabled with explanatory tooltip/state text.
  - Expected files: `src/ui.rs`, `src/core.rs`.
- Implementation:
  1. Replace main-menu button set and wire navigation actions.
  2. Add disabled state handling and styling for `Play Online`.
  3. Route `Bestiary` button to archive panel from CRU-065.
  4. Preserve current settings and exit behavior.
- Unit Tests Required:
  - Main-menu button action map resolves correct state transitions.
  - Disabled online action cannot start a run.
- Acceptance Criteria:
  - Main menu shows requested entries and order.
  - `Play Online` is present but non-interactive.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-067 - Offline Match Setup Screen (Faction + Map Config Scaffold)
- Status: `TODO`
- Type: `Core`
- Priority: `P0`
- Depends on: `CRU-066`
- Goal: Insert a setup screen before run start for faction and map selection.
- Context:
  - Christian faction selectable; Muslim shown but disabled.
  - Map list must be data-driven to support future map-specific events and enemy tables.
  - Expected files: `src/ui.rs`, `src/map.rs`, `src/data.rs`, `assets/data/map.json`.
- Implementation:
  1. Add match-setup UI with selectable faction and map list.
  2. Add config schema for map entries (id, name, description, allowed factions, spawn profile hooks).
  3. Disable Muslim faction selection with clear "not implemented" message.
  4. Start run only after valid setup selection and persist chosen map/faction in run context.
- Unit Tests Required:
  - Map config parsing validates required fields and rejects invalid entries.
  - Disabled faction cannot be selected through keyboard/mouse path.
- Acceptance Criteria:
  - `Play Offline` opens setup screen first.
  - User can start run only with Christian faction and a valid map selection.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-068 - Roster Cost Economy + Level Budget Locks
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-067`
- Goal: Implement locked-level budget driven by roster tier costs and enforce upgrade/progression restrictions.
- Context:
  - Tier-0 peasant cost = `0`.
  - Any single upgrade step into the next tier increases locked level budget by exactly `+1`.
  - Example: `0->1` costs `+1`, `1->2` costs `+1`, `2->3` costs `+1`.
  - Units cannot be demoted; only death frees their locked budget.
  - Expected files: `src/squad.rs`, `src/upgrades.rs`, `src/model.rs`, `src/ui.rs`.
- Implementation:
  1. Add per-unit tier cost and aggregate locked-level budget state.
  2. On upgrade/recruit/death, update locked budget deterministically.
  3. Enforce "cannot upgrade if resulting locked budget exceeds available max level headroom."
  4. Enforce progression lock when commander reaches current allowed cap due to locked budget.
  5. Emit UI-facing reason strings for blocked upgrades/blocked level progression.
- Unit Tests Required:
  - Locked budget increases/decreases correctly on upgrade and death.
  - Upgrade validation rejects purchases that exceed allowed budget.
  - Progression lock engages/disengages based on budget changes.
- Acceptance Criteria:
  - Roster tiering visibly reduces allowed max level.
  - Unit deaths free locked budget and unblock progression/upgrade opportunities.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-069 - Unit Upgrade Screen (Tree Layout + Bulk Upgrade Actions)
- Status: `TODO`
- Type: `UI`
- Priority: `P0`
- Depends on: `CRU-060`, `CRU-061`, `CRU-068`
- Goal: Build `U` screen where clicking a unit shows its upgrade tree and allows bulk upgrading with budget validation.
- Context:
  - Must follow provided sketch concept: tier columns with branch paths and selectable nodes.
  - User can choose quantity to upgrade for eligible transitions.
  - Expected files: `src/ui.rs`, `src/squad.rs`, `src/model.rs`, `assets/data/units.json`.
- Implementation:
  1. Create unit-upgrade panel with roster list + detail tree pane.
  2. Render tier columns/branches and allowed promotion paths.
  3. Add quantity controls (`+1`, `+5`, `max possible`) with affordability checks from CRU-068.
  4. Apply upgrades atomically and refresh roster counts/cost locks immediately.
- Unit Tests Required:
  - Bulk upgrade operation produces expected upgraded counts and budget deltas.
  - Invalid promotion paths are rejected.
  - Affordability guard prevents over-budget upgrades for any quantity option.
- Acceptance Criteria:
  - Pressing `U` opens upgrade screen with selectable unit trees.
  - User can perform valid bulk upgrades; invalid attempts display why they fail.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-070 - Enemy Spawn Pacing Rework (Units/Second + 100-Wave Cap + Victory)
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `CRU-067`
- Goal: Convert spawn logic to units-per-second pacing and end run with victory after clearing wave 100.
- Context:
  - Current pacing feels empty at times; should keep pressure continuous.
  - Need finite run completion while preserving escalating wave scaling style.
  - Expected files: `src/enemies.rs`, `src/core.rs`, `src/ui.rs`, `assets/data/waves.json`.
- Implementation:
  1. Replace per-wave total-count logic with per-wave spawn-rate profile (units/second).
  2. Keep existing scaling identity while applying rate-based scheduling.
  3. Cap wave index at 100 and trigger win state when wave 100 is fully cleared.
  4. Add win-state overlay/flow compatible with restart/main menu paths.
- Unit Tests Required:
  - Spawn scheduler emits expected counts for rate/time inputs.
  - Wave completion and wave-100 victory transition trigger deterministically.
- Acceptance Criteria:
  - Enemy flow is continuous within waves via rate spawning.
  - Clearing all enemies after wave 100 triggers player victory.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-071 - XP Curve Steepen + Level Cap 200 + HUD Level Budget Readout
- Status: `TODO`
- Type: `Balance`
- Priority: `P0`
- Depends on: `CRU-068`, `CRU-070`
- Goal: Rebalance XP progression, enforce level cap 200, and display `level / allowed max` in HUD.
- Context:
  - User requested steeper XP scaling and explicit maximum level handling.
  - Allowed max level is dynamic due to roster cost locks.
  - Expected files: `src/upgrades.rs`, `src/ui.rs`, `assets/data/upgrades.json`.
- Implementation:
  1. Update XP requirement formula/table to a steeper curve and clamp at level 200.
  2. Integrate dynamic allowed-max-level from roster budget.
  3. Adjust HUD: wave top-left, elapsed time directly under wave, level shown as `X / Y`.
  4. Ensure progression lock behavior is visible in UI when `X == Y` due to cost locks.
- Unit Tests Required:
  - XP requirement curve is monotonic and steeper than current baseline.
  - Commander level never exceeds 200.
  - HUD formatter returns correct `current/allowed` text for representative states.
- Acceptance Criteria:
  - Leveling stops at 200 hard cap.
  - HUD clearly shows wave, elapsed time below wave, and level `current / allowed`.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-072 - Tiered Rescue Rules (Tier-0-Only Rescue Spawns)
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-067`, `CRU-068`
- Goal: Enforce that only tier-0 peasant units spawn as rescuable recruits; higher tiers come from upgrade paths.
- Context:
  - User wants manual progression through unit-upgrade system.
  - Needs explicit tier metadata on units and spawn filters.
  - Expected files: `src/rescue.rs`, `src/data.rs`, `assets/data/units.json`, `assets/data/rescue.json`.
- Implementation:
  1. Ensure unit definitions include tier metadata and promotion mapping.
  2. Filter rescue spawn pool to tier-0 units only.
  3. Add validation preventing non-tier0 entries in rescue config for this map set.
  4. Surface tier in UI where relevant (unit upgrade/inventory/archive).
- Unit Tests Required:
  - Rescue spawn selection excludes non-tier0 units.
  - Config validator fails when rescue pools include disallowed tiers.
- Acceptance Criteria:
  - All rescue recruits are tier-0 peasants only.
  - Higher-tier units appear only via explicit upgrades.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-073 - Floating Damage Text
- Status: `TODO`
- Type: `UI`
- Priority: `P1`
- Depends on: `none`
- Goal: Add readable floating damage numbers for combat feedback.
- Context:
  - Must be performant under heavy combat.
  - Should avoid visual clutter via lifetime, stacking offsets, and optional batching.
  - Expected files: `src/combat.rs`, `src/ui.rs`, `src/visuals.rs`.
- Implementation:
  1. Emit damage-text events from finalized damage resolution.
  2. Spawn text entities with world-to-screen anchoring and timed fade/float animation.
  3. Add optional cap/throttle to avoid entity spikes in high-density scenarios.
- Unit Tests Required:
  - Damage event payload maps to text spawn data correctly.
  - Lifetime cleanup removes expired text entities/events.
- Acceptance Criteria:
  - Visible damage numbers appear on hits and disappear smoothly.
  - No runaway text buildup in stress scenarios.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-074 - Enemy Crowd Stability (Collision Footprint + Jitter Reduction)
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P0`
- Depends on: `none`
- Goal: Reduce enemy overlap stacking and jitter in large mobs by tuning collision/separation behavior.
- Context:
  - Current crowds can interpenetrate and vibrate due to tight radii and correction loops.
  - Changes should preserve CPU budget and pathing responsiveness.
  - Expected files: `src/collision.rs`, `src/enemies.rs`, `src/squad.rs`, `assets/data/enemies.json`.
- Implementation:
  1. Increase or decouple enemy collision radii used for separation.
  2. Add damping/clamping in separation resolution to avoid oscillation.
  3. Validate behavior in dense wave scenarios with commander/retinue interaction.
  4. Expose key tuning constants in data where practical.
- Unit Tests Required:
  - Separation solver maintains minimum distance invariants in deterministic sample setups.
  - Jitter damping step remains numerically stable across varying frame deltas.
- Acceptance Criteria:
  - Large enemy groups show less stacking and noticeably less jitter.
  - Collision tuning does not cause enemies to freeze or tunnel.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-075 - Per-Upgrade Requirement Framework (Mob Trio Uses Tier-0 Share)
- Status: `TODO`
- Type: `Core`
- Priority: `P1`
- Depends on: `CRU-068`, `CRU-072`
- Goal: Add a requirement framework where each upgrade can define its own condition type and parameters.
- Context:
  - The mob trio uses tier-0 composition requirements.
  - Other upgrades can use different requirement types later; requirement logic must not be hardwired to tier-0 only.
  - All requirements must be tunable in data, not hardcoded.
  - Expected files: `src/upgrades.rs`, `src/squad.rs`, `assets/data/upgrades.json`.
- Implementation:
  1. Add upgrade requirement schema with a typed discriminator (for example `tier0_share`, `formation_active`, `map_tag`, etc.).
  2. Evaluate conditions continuously and apply/remove conditional buffs at runtime.
  3. Add UI messaging for inactive conditional upgrades (owned but currently unmet).
  4. Validate condition parsing with clear error output.
- Unit Tests Required:
  - Predicate evaluator returns expected active/inactive states for varied roster mixes.
  - Conditional buffs apply and revoke without duplicate stacking bugs.
- Acceptance Criteria:
  - Requirement behavior is configured per-upgrade and not globally coupled.
  - Mob upgrades support tunable tier-0 share thresholds.
  - Owned conditional upgrades visibly deactivate when requirements are not met.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`

### CRU-076 - Mob's Fury and Mob's Justice Upgrades
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-075`
- Goal: Implement two new mob-themed upgrades with requested gameplay effects.
- Context:
  - `Mob's Fury`: morale/cohesion-loss immunity + damage/attack-speed/move-speed bonuses when tier requirement is met.
  - `Mob's Justice`: execute target when hit and target HP fraction is below 10%.
  - Expected files: `src/upgrades.rs`, `src/combat.rs`, `src/morale.rs`, `assets/data/upgrades.json`.
- Implementation:
  1. Add both upgrades to level-up pool with icons, descriptions, and tunable value ranges.
  2. Wire Fury immunity and stat bonuses into morale/cohesion and combat pipelines.
  3. Wire Justice execute check into post-hit damage resolution ordering.
  4. Add UI indicators for active Fury state and Justice trigger feedback.
- Unit Tests Required:
  - Fury immunity blocks morale/cohesion loss events while condition is active.
  - Justice executes only when target HP ratio < 10% at hit resolution.
  - Fury bonuses deactivate when roster no longer satisfies threshold.
- Acceptance Criteria:
  - Both upgrades can appear in level-up drafts and function as specified.
  - Effects are visible and test-covered.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-077 - Mob's Mercy Effect Definition and Implementation
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-075`
- Goal: Implement Mob's Mercy as a rescue-speed upgrade that reduces rescue channel time by 50%.
- Context:
  - Mob's Mercy belongs to the mob trio and should use the same requirement framework from CRU-075.
  - Rescue-time multiplier must be data-driven for tuning.
  - Expected files: `src/upgrades.rs`, `assets/data/upgrades.json`, `docs/SYSTEMS_REFERENCE.md`.
- Implementation:
  1. Add Mob's Mercy entry in upgrade pool with icon, description, and requirement wiring.
  2. Implement effect with conditional framework from CRU-075.
  3. Apply rescue-time multiplier (`0.5x`) while active.
  4. Add icon/description text and activation feedback.
  5. Add tests for trigger and interaction with rescue progression behavior.
- Unit Tests Required:
  - Rescue completion-time calculation respects Mob's Mercy multiplier when active.
  - Upgrade activation/deactivation correctly toggles rescue-time multiplier.
  - Interaction tests with Fury/Justice requirement logic do not cross-wire effects.
- Acceptance Criteria:
  - Picking Mob's Mercy reduces rescue channel duration by 50% under its requirement rules.
  - Behavior is fully test-covered and visible in UI descriptions.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`

### CRU-078 - Peasant Priest Unit (Auto-Cast Attack-Speed Blessing)
- Status: `TODO`
- Type: `Gameplay`
- Priority: `P1`
- Depends on: `CRU-072`
- Goal: Add non-damaging support unit that auto-casts a 10s attack-speed buff every 20s and refreshes duration on overlap.
- Context:
  - Priest should have no direct attack action.
  - Multiple priests can refresh the same buff timer if casts overlap before expiration.
  - Expected files: `src/squad.rs`, `src/combat.rs`, `src/model.rs`, `assets/data/units.json`, `src/ui.rs`.
- Implementation:
  1. Add peasant priest unit definition and role flags (support/non-attacking).
  2. Implement autonomous periodic cast targeting friendlies in range.
  3. Apply buff with duration refresh semantics (set remaining duration back to 10s on reapply).
  4. Add lightweight VFX/icon feedback for active priest buff state.
- Unit Tests Required:
  - Priest cast timer triggers on 20s cadence.
  - Overlapping casts refresh buff duration rather than stack multiplicatively (unless explicitly configured).
  - Priest unit never enters direct-damage attack pipeline.
- Acceptance Criteria:
  - Priests automatically maintain attack-speed buff loops in combat.
  - Multiple priests produce refresh behavior exactly as described.
- Documentation Updates Required:
  - `docs/SYSTEMS_REFERENCE.md`
  - `docs/SYSTEM_SCOPE_MAP.md`
  - `docs/ASSET_SOURCES.md` (if new priest assets are added)

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
