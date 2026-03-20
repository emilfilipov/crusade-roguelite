# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for current MVP runtime behavior.
Use this for entity/component/system lookup without scanning all source files.

## Latest Update (2026-03-20)
- Added `RunModalState` state machine for in-run utility screens (`Inventory`, `Stats`, `Skill Book`, `Archive`, `Unit Upgrade`).
- Added shared modal request event path (`RunModalRequestEvent`) so keyboard and UI button actions use the same reducer logic.
- Added modal hotkeys in-run: `I`, `O`, `P`, `K`, `U`; `Escape` closes modal first, otherwise opens pause menu.
- Added modal overlay scaffold renderer that pauses in-run simulation while open.
- Added top-right in-run utility bar with five icon buttons mapped to the same modal requests as hotkeys.
- Added inventory scaffold module/resource (`InventoryState`) with serializable bag/equipment setup model.
- Inventory modal now renders dedicated bag list + per-unit equipment setup sections.
- Stats modal now renders base/bonus/final rows for commander and global level-up-driven modifiers.
- Skill Book modal now uses structured upgrade records with:
  - category grouping (formations/auras/combat/utility)
  - icon + description rows
  - stack-aware entries
  - formation active/inactive indicators
- Renamed the old recruit `Infantry/Knight` to `Christian Peasant Infantry`.
- Added `Christian Peasant Archer` as a second recruitable retinue unit.
- Rescue spawns now carry recruit type metadata and alternate infantry/archer by spawn sequence.
- Recruit events now preserve rescued unit type so formation/combat/collision pipelines auto-handle both variants.
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
- Raised banner follow offset so it renders visibly behind/above the commander during movement.
- Dropped banner now uses the standard upright banner sprite for stronger in-world readability.
- Minimap now shows dropped-banner position and rescuable-retinue positions.
- Added melee-composition incentive: enemies inside the friendly formation footprint take `+20%` damage.
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
- Added first minimap prototype (bottom-right HUD panel) with commander/friendly/enemy blips.
- Enabled `CollisionPlugin` in app wiring (enemy collision now active).
- Added `GameOver` overlay flow with `Restart` and `Main Menu` actions.
- Rebuilt map floor rendering into tiled desert ground.
- Increased Christian Peasant Infantry attack range from `32` to `36`.
- Added drop transit-to-commander flow: friendly pickup starts homing, drop effect triggers only on commander contact.
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
2. `CorePlugin`
3. `SettingsPlugin`
4. `PerformancePlugin`
5. `VisualPlugin`
6. `MapPlugin`
7. `InventoryPlugin`
8. `SquadPlugin`
9. `FormationPlugin`
10. `CollisionPlugin`
11. `RescuePlugin`
12. `DropsPlugin`
13. `EnemyPlugin`
14. `CombatPlugin`
15. `ProjectilePlugin`
16. `MoralePlugin`
17. `BannerPlugin`
18. `UpgradePlugin`
19. `UiPlugin`
20. `PlatformPlugin`

### Runtime Note
- `src/collision.rs` is now registered in app setup.

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
- `bandit_raider`: `hp=34`, `armor=1`, `damage=6`, `cd=1.3`, `range=30`, `move=118`, `morale=90`

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
- Stat scale: `1.0 + (index+1)*0.08`

### `drops.json`
- `initial_spawn_count=8`
- `spawn_interval_secs=2.5`
- `pickup_radius=45`
- `xp_per_pack=6`
- `max_active_packs=5000`

### `rescue.json`
- `spawn_count=14`
- `rescue_radius=60`
- `rescue_duration_secs=2.2`

### `upgrades.json`
- `unlock_formation_diamond` (`one_time`, `adds_to_skillbar`, `formation_id=diamond`)
- `damage`
- `attack_speed`
- `armor`
- `pickup_radius`
- `aura_radius`
- `authority_aura`
- `move_speed`
- `hospitalier_aura`

Roll fields:
- `min_value`
- `max_value`
- `value_step`
- `weight_exponent`
- `one_time`
- `adds_to_skillbar`
- `formation_id`

### `map.json`
- `width=2400`
- `height=2400`
- Oasis fields removed from active schema.

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
- `ExpPack`, `DropInTransitToCommander` (`src/drops.rs`)
- `Projectile` (`src/projectiles.rs`)
- `BannerMarker` (`src/banner.rs`)

### Resources
- `RunSession`
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
- `OneTimeUpgradeTracker`
- `CommanderMotionState`
- `HudSnapshot`
- `PlatformRuntime`
- `ArtAssets`

### Events
- `StartRunEvent`
- `RunModalRequestEvent`
- `RecruitEvent`
- `DamageEvent`
- `UnitDamagedEvent`
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
- If an enemy is inside the friendly square-formation footprint, friendly outgoing damage gets multiplier `1.2`.
- Formation footprint is approximated from:
  - commander position
  - current recruit count
  - square slot spacing (`formations.square.slot_spacing`)
- If commander has no recruits, bonus does not apply.

### Movement Slowdown From Enemies Inside Formation (`src/squad.rs`)
- Commander movement applies additional multiplier based on enemy count inside square formation footprint.
- Per-enemy slowdown: `0.04` (4%).
- Minimum multiplier clamp: `0.5` (commander cannot be fully stopped by this effect).
- Formula:
  - `multiplier = clamp(1.0 - enemy_count * 0.04, 0.5, 1.0)`

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

## System Summary (By Module)

### `core.rs`
- Boot -> menu transition
- Main menu cleanup
- in-run modal hotkeys (`I/O/P/K/U`) through reducer-based modal request flow
- `Escape` behavior priority:
  - close open run modal
  - otherwise open pause menu
- virtual time pause/unpause sync while run modal is open
- survival timer
- commander-loss transition to `GameOver`

### `map.rs`
- camera spawn
- map bounds init
- tiled desert floor spawn
- perimeter wall visuals
- camera follow + camera-only pixel snap + map-edge clamp
- unit confinement to playable area inside wall inset

### `inventory.rs`
- runtime inventory scaffold resource initialization
- unit-class equipment setup defaults (weapon/armor/trinket slots)
- serializable bag/equipment model for future gear drops

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
- start spawn + timed respawn of rescuables
- typed rescuable metadata (`Christian Peasant Infantry` / `Christian Peasant Archer`)
- any-friendly rescue channel logic

### `enemies.rs`
- scripted + infinite waves
- queued enemy batch spawning with wave-scaled batch sizes/intervals
- pseudo-random spawn points within playable map bounds
- chase AI (retinue-prioritized targeting)
- visual state texture mapping

### `combat.rs`
- attack cooldown tick
- shared unit ranged projectile emission (commander + archer hybrid behavior)
- in-range targeting + damage emit
- enemy-in-formation vulnerability check (`+20%` friendly damage when inside formation bounds)
- damage apply + `UnitDamagedEvent`
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
- main menu buttons (`Start`, `Settings`, `Exit`)
- settings screen with FPS selector
- pause overlay buttons (`Resume`, `Restart`, `Main Menu`)
- level-up overlay (3 mandatory upgrade cards, icon + description, no skip)
- game-over overlay buttons (`Restart`, `Main Menu`)
- top HUD (wave/level/xp/time)
- progress strips (rescue + banner pickup)
- bottom-left vertical bars (average morale + cohesion)
- world-space health bars
- bottom-right minimap prototype with periodic blip refresh
  - commander/friendlies/enemies
  - XP packs (yellow)
  - rescuable retinue markers
  - dropped-banner marker
- bottom-center skillbar (10 slots)
  - slot `1` default Square formation (active)
  - key labels `1..0`
  - active slot border highlight
- in-run modal overlay scaffolds for:
  - `Inventory`
  - `Stats`
  - `Skill Book`
  - `Archive`
  - `Unit Upgrade`
- inventory modal content:
  - bag drops panel with empty-state fallback
  - equipment setup panel for commander + current friendly unit classes
- stats modal content:
  - active formation label
  - base/bonus/final stat rows (HP, damage, attack speed, armor, move speed, pickup radius, aura, aura effects)
- skill book modal content:
  - grouped sections (`Formations`, `Auras`, `Combat`, `Utility`)
  - icon-backed entries with stacked counts
  - active/inactive markers for mutually exclusive formation skills
- top-right utility icon bar:
  - `Inventory` (`I`)
  - `Stats` (`O`)
  - `Skill Book` (`P`)
  - `Archive` (`K`)
  - `Unit Upgrade` (`U`)

### `upgrades.rs`
- XP thresholds and explicit level-up draft flow (`InRun -> LevelUp -> InRun`)
- 3-option upgrade draft cards (keyboard `1..3` and mouse click selection)
- weighted random min/max upgrade value rolls
- additive stacked upgrade effects
- passive commander level scaling
- level-up full-heal sync for friendlies

### `steam.rs`
- feature-gated platform runtime (`standalone`/`steam`)

## Current Hooks / Known Gaps
- `FormationModifiers.defense_multiplier` and anti-cavalry values are still not fully wired into incoming damage resolution.
