# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for current MVP runtime behavior.
Use this for entity/component/system lookup without scanning all source files.

## Latest Update (2026-03-21)
- Added `RunModalState` state machine for in-run utility screens (`Inventory`, `Stats`, `Skill Book`, `Archive`, `Unit Upgrade`).
- Added shared modal request event path (`RunModalRequestEvent`) so keyboard and UI button actions use the same reducer logic.
- Added modal hotkeys in-run: `I`, `O`, `K`, `B`, `U`; `Escape` closes modal first, otherwise opens pause menu.
- Added responsive UI scaling based on current primary-window resolution (reference `1280x720`, clamped range `0.7..3.0`) to keep HUD/modals usable across windowed/fullscreen/borderless modes.
- Added pause-state `Escape` behavior: pressing `Escape` while paused resumes the run.
- Added modal overlay scaffold renderer that pauses in-run simulation while open.
- Added direct close-button modal state clear path to avoid stale open overlays from UI interaction edge cases.
- Stabilized Unit Upgrade modal close/interaction behavior by removing per-frame refresh churn from promotion feedback updates.
- Added top-right in-run utility bar with five icon buttons mapped to the same modal requests as hotkeys.
- Added in-run commander aura footprint gizmo for clearer aura coverage.
- Formation slot assignment now prioritizes melee units on outer slots and ranged/support units on inner slots.
- Added `ArchivePlugin` + `ArchiveDataset` with generated codex entries (units/enemies/skills/stats/bonuses/drops).
- Added shared archive renderer used by both in-run `B` modal and main-menu `Bestiary` screen.
- Added mouse-wheel scrollable sections for `Archive`/`Bestiary` and `Skill Book` to prevent clipping.
- Stats table content in the in-run stats modal is now scrollable to prevent overflow.
- Main menu flow now exposes:
  - `Play Offline` (opens match setup)
  - `Play Online` (disabled placeholder)
  - `Settings`
  - `Bestiary`
  - `Exit`
- Added `MatchSetup` screen:
  - faction selection (`Christian` enabled, `Muslim` disabled),
  - map selection from data-driven map list,
  - start gate that only allows valid faction/map combinations.
- Added roster level-budget economy:
  - tier-0 units cost `0` locked levels,
  - each tier-step promotion adds `+1` locked level,
  - unit death refunds the unit's locked level cost,
  - allowed max commander level is derived from locked budget (`200 - locked_levels`).
- Added progression/upgrade lock feedback surfaces:
  - `ProgressionLockFeedback` emits reason text when XP leveling is blocked by roster costs,
  - `RosterEconomyFeedback` emits reason text when promotions are rejected by budget/path constraints,
  - Unit Upgrade modal now displays live budget and latest block reason strings.
- Added wave-gated tier upgrade scaffolding:
  - tier 1 unlocks at wave 11,
  - tier 2 at wave 21,
  - tier 3 at wave 31,
  - tier 4 at wave 41,
  - tier 5 at wave 51.
- Added `Hero` equipment-tier scaffold for inventory/UI only (no promotion or spawn path enabled yet).
- Replaced peasant-priest potion placeholder sprite with a character sprite (`tile_0109`) for clearer class readability.
- Implemented Unit Upgrade modal runtime:
  - left roster list with selectable unit source rows,
  - right promotion-grid table with promotion options and affordability columns,
  - bulk action buttons (`+1`, `+5`, `MAX`) with affordability clamping.
- Promotion validation now rejects non-upgrade paths (same-tier or invalid-tier conversions).
- Enemy wave runtime now uses continuous units-per-second scheduling with staggered batch emission.
- Wave progression is finite at `100` waves; victory triggers only when wave-100 spawning is finished and all enemies are cleared.
- HUD commander level text now renders `current/allowed` and appends a lock marker when progression is budget-locked.
- Rescue config now includes `recruit_pool` and validator rejects non-tier0 rescue entries.
- Added inventory scaffold module/resource (`InventoryState`) with serializable bag/equipment setup model.
- Inventory modal now renders:
  - bag drops as 1-item-per-slot grid,
  - separate equipment rows for commander and each unit tier (`Tier 0..5`) plus scaffolded `Hero` row.
- Commander slots: `Banner`, `Instrument`, `Chant`, `Squire`, `Symbol of Power`; unit-tier slots: `Melee`, `Ranged`, `Armor`, `Banner`, `Squire`.
- Backpack viewport is now `5x6` slots.
- Stats modal now renders a table (`Stat | Base | Bonus | Final`) with color-coded bonuses (green positive, red negative).
- Skill Book modal now uses structured upgrade records with:
  - category grouping (formations/auras/combat/utility)
  - icon + description rows
  - stack-aware entries
  - formation active/inactive indicators
- Skill Book now displays cumulative effect totals per owned upgrade entry.
- Renamed the old recruit `Infantry/Knight` to `Christian Peasant Infantry`.
- Added `Christian Peasant Archer` as a second recruitable retinue unit.
- Rescue spawns now use a data-driven recruit pool and are currently constrained to tier-0 entries only.
- Active tier-0 rescue pool entries: `christian_peasant_infantry`, `christian_peasant_archer`, `christian_peasant_priest`.
- Added rescuable-priest variant mapping so priest rescues flow through the same recruit pipeline.
- Recruit events now preserve rescued unit type so formation/combat/collision pipelines auto-handle both variants.
- Wired equipment bonuses into combat runtime:
  - melee-weapon slot `base_bonus` applies to melee damage only by default,
  - ranged-weapon slot `base_bonus` applies to ranged damage only by default,
  - armor slot `base_bonus` applies to armor only by default,
  - explicit per-item fields (`melee_damage_bonus`, `ranged_damage_bonus`, `armor_bonus`) add cross-slot effects when desired.
- Gear effects are tier-targeted:
  - commander setup affects commander only,
  - tier setups affect only units in that exact tier.
- Upgraded ranged combat to a shared unit system (no longer commander-only).
- Christian Peasant Archer now uses hybrid combat: weak melee profile + stronger projectile ranged profile.
- Added formation skillbar (bottom-center, 10 slots, keys `1..0`) with exclusive active formation switching.
- Square formation now uses neutral multipliers (`x1` baseline).
- Added one-time `Diamond` formation unlock card in level-up draft:
  - unlock card is skillbar-bound,
  - appears once per run,
  - auto-adds to next free skillbar slot.
- Added simple generated formation icons for skillbar/cards:
  - `assets/sprites/skills/formation_square.png`
  - `assets/sprites/skills/formation_diamond.png`
- Added Diamond gameplay tuning:
  - offense bonus while commander is moving,
  - slight movement speed bonus,
  - slight defense penalty.
- Diamond slot assignment now uses explicit ring + clockwise ordering around commander for clearer unit arrangement.
- Draft filtering now removes skillbar-bound cards when skillbar is full.
- Replaced the level-up pool with weighted random 3-option drafts from repeatable upgrades plus one-time skill unlocks.
- Upgrade values now roll via weighted min/max sampling (higher values are rarer).
- Activated commander aura mechanics:
  - Authority aura: in-range friendly morale/cohesion-loss resistance + enemy morale drain.
  - Hospitalier aura: in-range friendly HP/morale/cohesion regen.
- Added shared ranged projectile attacks (outside-melee targeting, projectile travel, despawn on hit/max distance).
- Added XP pack minimap markers (yellow blips).
- Added commander movement slowdown from enemy pressure inside formation bounds (capped at 50% minimum speed multiplier).
- Pause menu button label now reads `Main Menu`.
- Added mandatory `LevelUp` state with 3-card draft overlay (image + description) and no skip path.
- Level-up weighted upgrades now roll into fixed 5-tier value buckets:
  - `Common`, `Uncommon`, `Rare`, `Epic`, `Mythical`
  - one-time upgrades (`formations`, `mob_*`) are classified as `Unique`
- Level-up card visuals now use tier-based border + glow colors:
  - `Common` = white/gray, `Uncommon` = blue, `Rare` = green,
    `Epic` = purple, `Mythical` = orange, `Unique` = brown.
- Raised banner follow offset so it renders visibly behind/above the commander during movement.
- Dropped banner now uses the standard upright banner sprite for stronger in-world readability.
- Minimap now shows dropped-banner position and rescuable-retinue positions.
- Added one-time level-up upgrade `Encirclement Doctrine` (`formation_breach`): once acquired, enemies inside the friendly formation footprint take `+20%` damage.
- Added critical-hit combat stats for friendlies:
  - `crit_chance_bonus` (additive chance, clamped to 95%)
  - `crit_damage_multiplier` (base `x1.5`, increased by upgrades)
- Added repeatable level-up cards:
  - `Killer Instinct` (`crit_chance`)
  - `Deadly Precision` (`crit_damage`)
- Stats modal now shows `Crit Chance` and `Crit Damage` bonus rows.
- Skill Book cumulative descriptions now include crit chance and crit damage totals.
- Removed decorative floor foliage overlay; battlefield floor now renders as pure sand tiles only.
- Switched foliage overlay to transparent detail tile to remove opaque square artifacts on the floor.
- Enemy waves now spawn as staggered batches at pseudo-random positions across the playable map (not border ring-only).
- `Escape` now only triggers while in `InRun`, opening a centered pause overlay with `Resume`, `Restart`, and `Main Menu`.
- Added enemy chase hysteresis and removed unit position snapping to reduce movement jitter.
- Added delayed enemy XP drops (`0.9s` pickup lock) before homing can start.
- Ambient XP packs now spawn around commander position for better visibility.
- XP homing speed now scales from commander base speed and stays slightly faster.
- Increased base drop pickup radius from `30` to `45`.
- Fixed Windows installer asset coverage for runtime-loaded art (`assets/sprites` + `oga_ishtar` pack).
- Switched battlefield floor to cleaner sand tile set.
- Added visible perimeter wall ring and hard playfield confinement for units.
- Added first minimap prototype (now top-right HUD panel) with commander/friendly/enemy blips.
- Enabled `CollisionPlugin` in app wiring (enemy collision now active).
- Added `GameOver` overlay flow with `Restart` and `Main Menu` actions.
- Rebuilt map floor rendering into tiled desert ground.
- Increased Christian Peasant Infantry attack range from `32` to `36`.
- Added drop transit-to-commander flow: friendly pickup starts homing, drop effect triggers only on commander contact.
- Added floating combat damage text:
  - `DamageTextEvent` emitted from finalized damage application,
  - world-space numeric text with rise/fade animation,
  - per-frame and active-entity caps to prevent text spikes under high hit density.
- Reduced dense enemy crowd jitter/stacking:
  - enemy collision radius is now data-driven (`enemies.bandit_raider.collision_radius`),
  - collision correction now uses frame-time-aware damping + max push clamp,
  - collision solver now runs 2 iterative passes per frame with per-pass push cap,
  - enemy-enemy pairs use slightly larger separation distance (`x1.14`) to reduce mass overlap,
  - chase movement step is clamped to avoid overshooting into stop distance.
- Added per-upgrade requirement framework:
  - data schema now supports typed requirement discriminators (`tier0_share`, `formation_active`, `map_tag`),
  - conditional upgrade ownership is tracked generically (not hardwired per-mob-flag),
  - conditional effects are re-evaluated continuously and cleanly revoke when requirements are unmet,
  - Skill Book now surfaces owned conditional upgrades as active/inactive with unmet-requirement messaging.
- Completed `Mob's Fury` + `Mob's Justice` runtime feedback loop:
  - `Mob's Fury` active/inactive state now appears in-run in the top-center HUD status line,
  - `Mob's Justice` execute hits now emit explicit floating `EXECUTE` combat text,
  - execute resolution uses a shared threshold helper (`<=10%` HP) across melee and projectile hit paths.
- Completed `Mob's Mercy` conditional rescue-speed effect:
  - rescue channel duration is computed via shared `effective_rescue_duration` and multiplied by conditional effects,
  - active Mercy state now appears alongside other mob upgrade statuses in the in-run HUD line,
  - added explicit tests for Mercy activation/deactivation and non-cross-wired interaction with Fury/Justice.
- Completed `Christian Peasant Priest` support runtime:
  - priest promotion now initializes full `20s` support cooldown and does not attach direct attack profiles,
  - priests auto-cast a `10s` attack-speed blessing on friendlies in range and overlapping casts refresh duration,
  - in-run HUD status line now surfaces active priest blessing remaining time.
- Replaced placeholder `morale_weight` usage with active per-unit `Morale` (friendlies and enemies).
- Added morale-based combat debuff below 50% morale.
- Refactored cohesion to event-driven behavior (damage/death/kill events + low-morale pressure).
- Reworked banner loop:
  - auto-drop at low cohesion tier
  - 10s pickup unlock delay
  - 5s pickup channel
  - pickup restores cohesion to recovery tier
  - dropped-banner effect is friendly move-speed penalty
- Added HUD bottom-left vertical meters for average army morale and cohesion.
- Added banner pickup progress bar under XP bar.
- Removed oasis from active runtime schema/config usage.

## Runtime Architecture

### App Builders
- Runtime app: `build_runtime_app()` in `src/lib.rs`
  - `DefaultPlugins`
  - Window: `1280x720`
- Headless app: `build_headless_app()`
  - `MinimalPlugins`

### Plugin Order (`configure_game_app`)
1. `DataPlugin`
2. `ArchivePlugin`
3. `CorePlugin`
4. `SettingsPlugin`
5. `PerformancePlugin`
6. `VisualPlugin`
7. `MapPlugin`
8. `InventoryPlugin`
9. `SquadPlugin`
10. `FormationPlugin`
11. `CollisionPlugin`
12. `RescuePlugin`
13. `DropsPlugin`
14. `EnemyPlugin`
15. `CombatPlugin`
16. `ProjectilePlugin`
17. `MoralePlugin`
18. `BannerPlugin`
19. `UpgradePlugin`
20. `UiPlugin`
21. `PlatformPlugin`

### Runtime Note
- `src/collision.rs` is now registered in app setup.
- `src/archive.rs` generates and validates bestiary/archive entries from loaded game data.

### Game States
- `Boot`
- `MainMenu`
- `MatchSetup`
- `Archive`
- `Settings`
- `InRun`
- `LevelUp` (run is paused until an upgrade card is selected)
- `Paused`
- `GameOver` (defeat pauses run and shows overlay actions)
- `Victory`

## Data Files and Live Values
Loaded from `assets/data` by `GameData::load_from_dir`.

### `units.json`
- Commander (`baldiun`): `hp=120`, `armor=6`, `damage=12`, `cd=0.9`, `range=34`, `move=170`, `morale=120`, `aura_radius=180`
  - Ranged profile: `damage=9`, `cd=1.2`, `range=250`, `projectile_speed=420`, `max_distance=260`
- Recruit `christian_peasant_infantry`: `hp=95`, `armor=4`, `damage=9`, `cd=1.1`, `range=36`, `move=150`, `morale=100`
- Recruit `christian_peasant_archer`: `hp=72`, `armor=2`, `move=154`, `morale=92`
  - Melee profile: `damage=4`, `cd=1.45`, `range=26`
  - Ranged profile: `damage=9`, `cd=1.15`, `range=220`, `projectile_speed=460`, `max_distance=235`

### `enemies.json`
- `bandit_raider`: `hp=34`, `armor=1`, `damage=6`, `cd=1.3`, `range=30`, `move=118`, `morale=90`, `collision_radius=16`

### `formations.json`
- `square`: `slot_spacing=30`, `offense=1.0`, `offense_while_moving=1.0`, `defense=1.0`, `anti_cavalry=1.0`, `move_speed=1.0`
- `diamond`: `slot_spacing=30`, `offense=1.0`, `offense_while_moving=1.2`, `defense=0.9`, `anti_cavalry=0.95`, `move_speed=1.08`

### `waves.json`
Scripted waves:
1. `t=0`, `count=8`
2. `t=30`, `count=12`
3. `t=60`, `count=16`
4. `t=90`, `count=20`
5. `t=120`, `count=24`

Runtime scripted count scaling:
- Effective count per scripted wave: `round(configured_count * 1.18^wave_index)`

Procedural continuation:
- Interval: 30s
- Count: `round(base * 1.22^(index+1))`
- Stat scale: `1.0 + (wave-1)*0.092`

### `drops.json`
- `initial_spawn_count=8`
- `spawn_interval_secs=2.5`
- `pickup_radius=45`
- `xp_per_pack=6`
- `max_active_packs=5000`

### `rescue.json`
- `spawn_count=6`
- `rescue_radius=60`
- `rescue_duration_secs=2.2`
- `recruit_pool=["christian_peasant_infantry","christian_peasant_archer","christian_peasant_priest"]` (tier-0 validation enforced)

### `upgrades.json`
- `unlock_formation_diamond` (`one_time`, `adds_to_skillbar`, `formation_id=diamond`)
- `encirclement_doctrine` (`kind=formation_breach`, `one_time`, grants inside-formation damage bonus)
- `damage`
- `attack_speed`
- `armor`
- `pickup_radius`
- `aura_radius`
- `authority_aura`
- `move_speed`
- `hospitalier_aura`
- `mob_fury` (`one_time`, `requirement_type=tier0_share`, `requirement_min_tier0_share=1.0`)
- `mob_justice` (`one_time`, `requirement_type=tier0_share`, `requirement_min_tier0_share=1.0`)
- `mob_mercy` (`one_time`, `requirement_type=tier0_share`, `requirement_min_tier0_share=1.0`)

Roll fields:
- `min_value`
- `max_value`
- `value_step`
- `weight_exponent`
- `one_time`
- `adds_to_skillbar`
- `formation_id`
- requirement fields:
  - `requirement_type`
  - `requirement_min_tier0_share`
  - `requirement_active_formation`
  - `requirement_map_tag`

### `map.json`
- Data-driven map list:
  - `id`, `name`, `description`
  - `width`, `height`
  - `allowed_factions`
  - optional `spawn_profile_id`
- Current runtime entry:
  - `desert_battlefield` (`2400x2400`, `allowed_factions=["christian"]`)

## ECS Inventory

### Core Components (`src/model.rs`)
- `Unit { team, kind, level }`
- `Health { current, max }`
- `BaseMaxHealth`
- `Morale { current, max }`
- `Armor`
- `AttackProfile`
- `AttackCooldown`
- `MoveSpeed`
- `ColliderRadius`
- Markers/data components: `PlayerControlled`, `FriendlyUnit`, `EnemyUnit`, `RescuableUnit { recruit_kind }`, `CommanderUnit`

### Module Components
- `RangedAttackProfile`, `RangedAttackCooldown` (`src/combat.rs`)
- `BanditVisualRuntime`, `BanditVisualState` (`src/enemies.rs`)
- `RescueProgress` (`src/rescue.rs`)
- `ExpPack`, `DropInTransitToCommander`, `MagnetPickup` (`src/drops.rs`)
- `Projectile` (`src/projectiles.rs`)
- `BannerMarker` (`src/banner.rs`)

### Resources
- `RunSession`
- `MatchSetupSelection`
- `RunModalState`
- `FrameRateCap`
- `GameData`
- `MapBounds`
- `InventoryState`
- `SquadRoster`
- `ActiveFormation`, `FormationModifiers`
- `FormationSkillBar`
- `WaveRuntime`
- `Cohesion`, `CohesionCombatModifiers`
- `BannerState`, `BannerMovementPenalty`
- `Progression`, `UpgradeDraft`, `GlobalBuffs`
- `ProgressionLockFeedback`
- `OneTimeUpgradeTracker`
- `ConditionalUpgradeOwnership`
- `ConditionalUpgradeStatus`
- `RosterEconomy`, `RosterEconomyFeedback`
- `UnitUpgradeUiState`
- `CommanderMotionState`
- `HudSnapshot`
- `PlatformRuntime`
- `ArtAssets`

### Events
- `StartRunEvent`
- `RunModalRequestEvent`
- `RecruitEvent`
- `PromoteUnitsEvent`
- `DamageEvent`
- `UnitDamagedEvent`
- `DamageTextEvent`
- `UnitDiedEvent`
- `GainXpEvent`
- `SpawnExpPackEvent`

## Key Gameplay Formulas

### Morale Debuff (`src/combat.rs`)
`morale_effect_multiplier(ratio)`:
- `ratio >= 0.5`: `1.0`
- `< 0.5`: linearly scales down to `0.75` at `0.0`

Applied to:
- outgoing damage
- attack cooldown progression (effective attack speed)

### Friendly Outgoing Multiplier Floor
Friendly combined outgoing multiplier has lower clamp:
- minimum `0.55`

### Enemy-In-Formation Vulnerability Bonus (`src/combat.rs`)
- Base state: no inside-formation vulnerability bonus is active.
- After the one-time upgrade `Encirclement Doctrine` is acquired, enemies inside the friendly formation footprint take multiplier `1.2` from friendly outgoing damage.
- Formation footprint is approximated from:
  - commander position
  - current recruit count
  - active formation slot spacing
- If commander has no recruits, bonus does not apply.

### Movement Slowdown From Enemies Inside Formation (`src/squad.rs`)
- Commander movement applies additional multiplier based on enemy count inside square formation footprint.
- Per-enemy slowdown: `0.04` (4%).
- Minimum multiplier clamp: `0.5` (commander cannot be fully stopped by this effect).
- Formula:
  - `multiplier = clamp(1.0 - enemy_count * 0.04, 0.5, 1.0)`

### Formation Footprint Occupancy Cap + Repel (`src/enemies.rs`)
- Enemies allowed inside the active formation footprint are capped dynamically by retinue size.
- Cap model:
  - derives from estimated formation perimeter slots + small recruit-count bonus
  - clamped to `[4, 96]`
- Overflow enemies are sorted deterministically by distance-to-commander and redirected toward the formation perimeter projection (square/diamond-specific boundary math).
- Repel movement is step-limited each frame for stability (`280 units/sec`).

### Diamond Formation Combat/Movement Effects
- Formation offense multiplier now has a moving-state modifier:
  - `effective_offense = offense_multiplier * offense_while_moving_multiplier` when commander is moving.
- Commander movement speed is multiplied by active formation move-speed multiplier.
- Friendly effective armor is multiplied by active formation defense multiplier.

### Ranged Projectile Attacks (`src/combat.rs`, `src/projectiles.rs`)
- Units with `RangedAttackProfile` fire projectiles only when targets are outside melee range and inside ranged range.
- Current ranged units: commander + Christian Peasant Archer.
- Projectile is non-instant and travels via velocity each frame.
- Projectile despawns on hit or when max travel distance is consumed.

### Commander XP Requirement (`src/upgrades.rs`)
- Bracketed exponential scaling:
  - `base = 30`
  - bracket size: `10 levels`
  - bracket multiplier: `5.5^bracket_index`
  - intra-bracket multiplier: `1.18^within_bracket_index`
- Formula:
  - `xp_required(level) = 30 * 5.5^bracket * 1.18^within_bracket`

### Commander Allowed Max Level from Roster Budget (`src/squad.rs`)
- Hard commander cap: `200`.
- Roster lock rule:
  - `allowed_max_level = max(1, 200 - locked_levels)`
- Promotion guard:
  - a promotion is rejected if it would reduce `allowed_max_level` below current commander level.

### Unit Upgrade Bulk Affordability (`src/ui.rs`)
- For each promotion row, UI computes:
  - `step_cost = promotion_step_cost(from_kind, to_kind)` (allowed specialization paths can cost `1` even when tiers match)
  - iterate requested count from `1..=source_count`
  - stop when `level_cap_from_locked_budget(locked_levels + step_cost * requested) < commander_level`
- `MAX` button uses the computed affordable count.
- `+5` clamps to affordable count when fewer than 5 promotions are currently valid.

### Conditional Upgrade Requirement Evaluation (`src/upgrades.rs`)
- Owned conditional upgrades are evaluated each frame from live runtime context:
  - `tier0_share` compares roster tier-0 ratio against configured minimum.
  - `formation_active` checks currently active formation id.
  - `map_tag` is schema-supported; currently reports unmet in runtime until map tags are introduced.
- Effects are rebuilt from scratch each refresh and deduplicated by upgrade id, preventing duplicate-frame stacking bugs.

### Wave Spawn Rate + Victory Gate (`src/enemies.rs`, `src/core.rs`)
- Wave duration: `30s`.
- Spawn pacing:
  - `units_per_second_for_wave = clamp(wave_base_count * 2.0, 1.0, 1000.0) / 30.0`
  - spawned units are queued into timed batches (`batch_size` scales by wave, interval shrinks with floor clamp).
- Enemy stat progression:
  - `wave_stat_multiplier = 1.0 + (wave - 1) * 0.092` (15% steeper than previous `0.08` slope).
- Wave progression:
  - `current_wave` increases until `MAX_WAVES = 100`.
  - spawning stops after wave 100 finishes its duration window.
- Victory condition:
  - `finished_spawning == true`
  - `current_wave >= 100`
  - `pending_batches` empty
  - alive enemy count is `0`

### Upgrade Roll Formula (`src/upgrades.rs`)
- Draft picks `3` unique upgrades from the configured pool.
- Rolled value uses:
  - `roll = random(0..1)^weight_exponent`
  - `value = min + (max - min) * roll`
  - optional quantization by `value_step`.

### Cohesion Tier Table (`src/morale.rs`)
- `>=80`: damage `1.08`, attack speed `1.08`, defense `1.05`
- `60-79`: neutral `1.0`
- `40-59`: damage/attack speed `0.9`, defense `0.93`
- `20-39`: damage/attack speed `0.8`, defense `0.86`
- `<20`: damage/attack speed `0.7`, defense `0.8`, `collapse_risk=true`

### Cohesion Event Tuning
- Friendly damage taken: cohesion and army morale loss scale with post-mitigation damage.
- Enemy kill rewards (friendly morale/cohesion gains) trigger on every 3rd enemy death only.
- Friendly death: larger cohesion/morale loss scaled by fallen unit max HP (commander death penalty multiplier).
- Authority aura mitigates in-range friendly morale/cohesion losses from damage and death events.
- Hospitalier aura provides in-range passive regen:
  - HP regen (highest)
  - cohesion regen (medium)
  - morale regen (lowest)
- Low-morale retinue pressure:
  - if `>=50%` of retinue below 50% morale: cohesion drains at `3.0/s`
  - else cohesion recovers at `0.25/s`

## Banner Loop (`src/banner.rs`)
- Auto-drop trigger: cohesion `<20` (with anti-redrop grace check)
- Dropped effect: `BannerMovementPenalty.friendly_speed_multiplier = 0.72`
- Banner follow render offset: banner is rendered with positive Y offset behind commander for visibility.
- Pickup unlock delay: 10s after drop
- Pickup channel: 5s while friendly unit is within recovery radius
- Successful recovery:
  - banner returns to up state
  - cohesion restored to `65`
  - redrop grace timer starts

### Banner Progress UI
- Banner channel progress is surfaced under XP bar through same progress-strip region used by rescue bars.

## Drop Flow (`src/drops.rs`)
1. Spawn ambient packs + event packs (enemy death events).
2. Enemy-death drops spawn with `0.9s` pickup delay before any homing can start.
3. Any friendly within pickup radius marks pack as `DropInTransitToCommander` (after delay).
   - Effective pickup radius = `base pickup radius + stacked pickup-radius upgrades`.
4. Transit pack homes to commander each frame at speed slightly above commander base speed.
5. On commander contact radius, pack is consumed and effect is applied (`GainXpEvent`).
6. Magnet pickup lifecycle:
   - spawns at wave start on waves divisible by `3` (map center),
   - despawns automatically when the next wave starts.
7. Magnet pickup effect:
   - on friendly pickup, all active XP packs are immediately forced into transit-to-commander mode.

## System Summary (By Module)

### `core.rs`
- Boot -> menu transition
- Main menu cleanup
- menu clear color handling for `MainMenu`, `MatchSetup`, `Settings`, `Archive`
- in-run modal hotkeys (`I/O/K/B/U`) through reducer-based modal request flow
- `Escape` behavior priority:
  - close open run modal
  - otherwise open pause menu
- while paused, `Escape` resumes run (same as pause menu `Resume`)
- virtual time pause/unpause sync while run modal is open
- survival timer
- commander-loss transition to `GameOver`

### `map.rs`
- camera spawn
- map bounds init from selected match-setup map (fallback to first configured map)
- tiled desert floor spawn (respawned on run start to match selected map bounds)
- perimeter wall visuals
- camera follow + camera-only pixel snap + map-edge clamp
- unit confinement to playable area inside wall inset

### `inventory.rs`
- runtime inventory scaffold resource initialization
- unit-tier + commander equipment setup defaults
- serializable bag/equipment model
- slot-aware gear bonus resolution (`gear_bonuses_for_unit`)
  - default slot behavior applies `base_bonus` to matching stat only
  - explicit item bonus fields can add cross-stat effects

### `archive.rs`
- builds `ArchiveDataset` entries from live data files
- validates archive entries for required fields (title + description)
- exposes category groupings reused by main-menu and in-run archive UIs

### `collision.rs`
- resolves eligible overlap pairs (enemy-enemy and enemy-inner-retinue)
- applies iterative damped/clamped correction vectors for frame-rate-stable separation
- inflates enemy-enemy minimum spacing to reduce dense crowd overlap
- keeps post-separation positions inside map bounds

### `squad.rs`
- run start commander spawn
- commander movement (includes enemy-inside-formation slowdown multiplier)
- recruit spawn from rescue/upgrade events
- roster sync/casualties

### `formation.rs`
- square offsets and smoothing
- depth sorting
- formation movement now scaled by `BannerMovementPenalty`

### `rescue.rs`
- start spawn + timed respawn of rescuables (`20s` cadence, max `6` active at once)
- typed rescuable metadata driven by `rescue.recruit_pool` (tier-0-only entries accepted by config validator)
- any-friendly rescue channel logic

### `drops.rs`
- ambient and event XP pack spawning with wave/level XP scaling
- pickup-delay-aware pack pickup detection (any friendly can trigger)
- transit-to-commander homing consume flow
- wave magnet pickup spawn/despawn cadence (every 3 waves)
- magnet pickup force-homes all active XP packs

### `enemies.rs`
- finite 100-wave runtime with units-per-second spawning
- queued enemy batch spawning with wave-scaled batch sizes/intervals
- pseudo-random spawn points within playable map bounds
- chase AI (retinue-prioritized targeting)
- active-formation inside-footprint cap with perimeter repel for overflow enemies
- visual state texture mapping

### `combat.rs`
- attack cooldown tick
- shared unit ranged projectile emission (commander + archer hybrid behavior)
- in-range targeting + damage emit
- melee outgoing base damage includes resolved equipment melee bonus
- ranged outgoing base damage includes resolved equipment ranged bonus
- friendly armor mitigation includes resolved equipment armor bonus
- enemy-in-formation vulnerability check (`+20%` friendly damage when inside formation bounds)
- friendly crit roll on melee and ranged outgoing hits (before armor mitigation)
- damage apply + `UnitDamagedEvent` + `DamageTextEvent` (uses final applied damage, not requested pre-clamp amount)
- death resolve + drop spawn events

### `morale.rs`
- run-start cohesion reset
- morale/cohesion updates from damage/death events
- authority aura in-range mitigation + enemy morale drain
- hospitalier aura in-range HP/morale/cohesion regen
- low-morale retinue pressure on cohesion
- cohesion modifier recalculation

### `banner.rs`
- run-start banner reset
- low-cohesion drop
- delayed pickup channel
- movement penalty state updates

### `ui.rs`
- main menu buttons (`Play Offline`, `Play Online` disabled, `Settings`, `Bestiary`, `Exit`)
- main-menu `Bestiary` screen (same dataset/content source as in-run archive modal)
- `MatchSetup` screen with faction + map selectors and `Start`/`Back` actions
- settings screen with FPS selector
- global UI scale sync from live window resolution (`UiScale`) for resolution-mode resilience
- pause overlay buttons (`Resume`, `Restart`, `Main Menu`)
- level-up overlay (3 mandatory upgrade cards, icon + description, no skip)
- game-over overlay buttons (`Restart`, `Main Menu`)
- top HUD (left column: wave/time, center: level/xp/rescue bars)
- progress strips (rescue + banner pickup)
- bottom-left vertical bars (average morale + cohesion)
- commander aura footprint indicator (subtle world-space circle around commander)
- world-space health bars
- world-space floating damage text with timed rise/fade cleanup
- top-right minimap prototype with periodic blip refresh (`204px`, +20% from previous size)
  - commander/friendlies/enemies
  - XP packs (yellow)
  - wave magnet pickup symbol (cross for Christian, crescent for Muslim)
  - rescuable retinue markers
  - dropped-banner marker
- utility action bar moved to bottom-right (swapped with minimap position)
- bottom-center skillbar (10 slots)
  - slot `1` default Square formation (active)
  - key labels `1..0`
  - active slot border highlight
- in-run modal overlay scaffolds for:
  - `Inventory`
  - `Stats`
  - `Skill Book`
  - `Bestiary`
  - `Unit Upgrade`
- inventory modal content:
  - bag drops grid (1 item = 1 slot, with empty placeholders)
  - fixed 5x6 backpack viewport (first 30 slots shown in-grid)
  - equipment panel with commander + unit tier rows using short labels (`C`, `T0..T5`, `H`)
  - commander slots: `Banner`, `Instrument`, `Chant`, `Squire`, `Symbol of Power`
  - unit-tier slots: `Melee`, `Ranged`, `Armor`, `Banner`, `Squire`
- stats modal content:
  - table layout (`Stat | Base | Bonus | Final`)
  - table rows are rendered in a scrollable viewport
  - `Unit HP` row is bonus-only (`Base` and `Final` show `-`)
  - bonus color coding (green positive, red negative)
  - separate `Active Buffs` column for formation/auras/conditional effects/priest blessing
- skill book modal content:
  - grouped sections (`Formations`, `Auras`, `Combat`, `Utility`)
  - icon-backed entries with stacked counts
  - cumulative effect descriptions per owned upgrade
  - active/inactive markers for mutually exclusive formation skills
  - active/inactive markers + unmet-requirement detail for owned conditional upgrades
- archive/bestiary modal content:
  - mouse-wheel scrolling with clipped viewport to prevent content overflow
- top-right utility icon bar:
  - `Inventory` (`I`)
  - `Stats` (`O`)
  - `Skill Book` (`K`)
  - `Bestiary` (`B`)
  - `Unit Upgrade` (`U`)

### `upgrades.rs`
- XP thresholds and explicit level-up draft flow (`InRun -> LevelUp -> InRun`)
- 3-option upgrade draft cards (keyboard `1..3` and mouse click selection)
- weighted random min/max upgrade value rolls
- additive stacked upgrade effects
- repeatable crit upgrades (`crit_chance`, `crit_damage`) wired into `GlobalBuffs`
- passive commander level scaling
- level-up full-heal sync for friendlies
- generic conditional-upgrade ownership + typed requirement parsing/evaluation
- runtime conditional status snapshot used by Skill Book UI

### `steam.rs`
- feature-gated platform runtime (`standalone`/`steam`)

## Current Hooks / Known Gaps
- `FormationModifiers.defense_multiplier` and anti-cavalry values are still not fully wired into incoming damage resolution.
