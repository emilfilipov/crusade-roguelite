# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for current MVP runtime behavior.
Use this for entity/component/system lookup without scanning all source files.

## Latest Update (2026-03-19)
- Added enemy chase hysteresis and removed unit position snapping to reduce movement jitter.
- Added delayed enemy XP drops (`0.9s` pickup lock) before homing can start.
- Ambient XP packs now spawn around commander position for better visibility.
- XP homing speed now scales from commander base speed and stays slightly faster.
- Increased base drop pickup radius from `30` to `45`.
- Removed level-up pause hitch by resolving upgrade picks in-run instead of transitioning to `Paused`.
- Fixed Windows installer asset coverage for runtime-loaded art (`assets/sprites` + `oga_ishtar` pack).
- Switched battlefield floor to cleaner sand tile set and reduced noisy foliage density.
- Added visible perimeter wall ring and hard playfield confinement for units.
- Added first minimap prototype (bottom-right HUD panel) with commander/friendly/enemy blips.
- Enabled `CollisionPlugin` in app wiring (enemy collision now active).
- Added `GameOver` overlay flow with `Restart` and `Main Menu` actions.
- Rebuilt map floor rendering into tiled desert ground + sparse foliage overlay.
- Increased knight attack range from `32` to `36`.
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
7. `SquadPlugin`
8. `FormationPlugin`
9. `CollisionPlugin`
10. `RescuePlugin`
11. `DropsPlugin`
12. `EnemyPlugin`
13. `CombatPlugin`
14. `ProjectilePlugin`
15. `MoralePlugin`
16. `BannerPlugin`
17. `UpgradePlugin`
18. `UiPlugin`
19. `PlatformPlugin`

### Runtime Note
- `src/collision.rs` is now registered in app setup.

### Game States
- `Boot`
- `MainMenu`
- `Settings`
- `InRun`
- `Paused`
- `GameOver` (defeat pauses run and shows overlay actions)

## Data Files and Live Values
Loaded from `assets/data` by `GameData::load_from_dir`.

### `units.json`
- Commander (`baldiun`): `hp=120`, `armor=6`, `damage=12`, `cd=0.9`, `range=34`, `move=170`, `morale=120`, `aura_radius=180`
- Recruit knight (`infantry_knight`): `hp=95`, `armor=4`, `damage=9`, `cd=1.1`, `range=36`, `move=150`, `morale=100`

### `enemies.json`
- `bandit_raider`: `hp=34`, `armor=1`, `damage=6`, `cd=1.3`, `range=30`, `move=118`, `morale=90`

### `formations.json`
- `square`: `slot_spacing=30`, `offense=0.95`, `defense=1.1`, `anti_cavalry=1.2`

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
- `add_units`
- `armor_up`
- `damage_up`
- `attack_speed_up`
- `cohesion_up`
- `commander_aura_up`

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
- Markers: `PlayerControlled`, `FriendlyUnit`, `EnemyUnit`, `RescuableUnit`, `CommanderUnit`

### Module Components
- `BanditVisualRuntime`, `BanditVisualState` (`src/enemies.rs`)
- `RescueProgress` (`src/rescue.rs`)
- `ExpPack`, `DropInTransitToCommander` (`src/drops.rs`)
- `Projectile` (`src/projectiles.rs`)
- `BannerMarker` (`src/banner.rs`)

### Resources
- `RunSession`
- `FrameRateCap`
- `GameData`
- `MapBounds`
- `SquadRoster`
- `ActiveFormation`, `FormationModifiers`
- `WaveRuntime`
- `Cohesion`, `CohesionCombatModifiers`
- `BannerState`, `BannerMovementPenalty`
- `Progression`, `UpgradeDraft`, `GlobalBuffs`
- `HudSnapshot`
- `PlatformRuntime`
- `ArtAssets`

### Events
- `StartRunEvent`
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

### Commander XP Requirement (`src/upgrades.rs`)
- Bracketed exponential scaling:
  - `base = 30`
  - bracket size: `10 levels`
  - bracket multiplier: `5.5^bracket_index`
  - intra-bracket multiplier: `1.18^within_bracket_index`
- Formula:
  - `xp_required(level) = 30 * 5.5^bracket * 1.18^within_bracket`

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
- Low-morale retinue pressure:
  - if `>=50%` of retinue below 50% morale: cohesion drains at `3.0/s`
  - else cohesion recovers at `0.25/s`

## Banner Loop (`src/banner.rs`)
- Auto-drop trigger: cohesion `<20` (with anti-redrop grace check)
- Dropped effect: `BannerMovementPenalty.friendly_speed_multiplier = 0.72`
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
4. Transit pack homes to commander each frame at speed slightly above commander base speed.
5. On commander contact radius, pack is consumed and effect is applied (`GainXpEvent`).

## System Summary (By Module)

### `core.rs`
- Boot -> menu transition
- Main menu cleanup
- pause/resume
- survival timer
- commander-loss transition to `GameOver`

### `map.rs`
- camera spawn
- map bounds init
- tiled desert floor + sparse foliage spawn
- perimeter wall visuals
- camera follow + camera-only pixel snap + map-edge clamp
- unit confinement to playable area inside wall inset

### `squad.rs`
- run start commander spawn
- commander movement
- recruit spawn from rescue/upgrade events
- roster sync/casualties

### `formation.rs`
- square offsets and smoothing
- depth sorting
- formation movement now scaled by `BannerMovementPenalty`

### `rescue.rs`
- start spawn + timed respawn of rescuables
- any-friendly rescue channel logic

### `enemies.rs`
- scripted + infinite waves
- chase AI (retinue-prioritized targeting)
- visual state texture mapping

### `combat.rs`
- attack cooldown tick
- in-range targeting + damage emit
- damage apply + `UnitDamagedEvent`
- death resolve + drop spawn events

### `morale.rs`
- run-start cohesion reset
- morale/cohesion updates from damage/death events
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
- game-over overlay buttons (`Restart`, `Main Menu`)
- top HUD (wave/level/xp/time)
- progress strips (rescue + banner pickup)
- bottom-left vertical bars (average morale + cohesion)
- world-space health bars
- bottom-right minimap prototype with periodic blip refresh (commander/friendlies/enemies)

### `upgrades.rs`
- XP thresholds and in-run auto-pick upgrade flow (no state pause on level-up)
- passive commander level scaling
- level-up full-heal sync for friendlies

### `steam.rs`
- feature-gated platform runtime (`standalone`/`steam`)

## Current Hooks / Known Gaps
- Commander aura fields/upgrades are still hooks only (`aura_radius`, `commander_aura_bonus` not yet driving active aura effects).
- `FormationModifiers.defense_multiplier` and anti-cavalry values are still not fully wired into incoming damage resolution.
