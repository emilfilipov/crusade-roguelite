# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for current MVP runtime behavior.
Use this for entity/component/system lookup without scanning all source files.

## Latest Update (2026-03-19)
- Added enemy chase hysteresis and removed unit position snapping to reduce movement jitter.
- Added delayed enemy XP drops (`0.45s` pickup lock) before homing can start.
- Ambient XP packs now spawn around commander position for better visibility.
- XP homing speed now scales from commander base speed and stays slightly faster.
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

Procedural continuation:
- Interval: 30s
- Count: `base + (index+1)*4`
- Stat scale: `1.0 + (index+1)*0.08`

### `drops.json`
- `initial_spawn_count=8`
- `spawn_interval_secs=2.5`
- `pickup_radius=30`
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

### Cohesion Tier Table (`src/morale.rs`)
- `>=80`: damage `1.08`, attack speed `1.08`, defense `1.05`
- `60-79`: neutral `1.0`
- `40-59`: damage/attack speed `0.9`, defense `0.93`
- `20-39`: damage/attack speed `0.8`, defense `0.86`
- `<20`: damage/attack speed `0.7`, defense `0.8`, `collapse_risk=true`

### Cohesion Event Tuning
- Friendly hit: small cohesion loss
- Enemy kill: small cohesion gain
- Friendly retinue death: small cohesion loss
- Low-morale retinue pressure:
  - if `>=50%` of retinue below 50% morale: cohesion drains over time
  - else cohesion slowly recovers

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
2. Enemy-death drops spawn with short pickup delay before any homing can start.
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
- camera follow + camera-only pixel snap

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

### `upgrades.rs`
- XP thresholds and draft flow
- passive commander level scaling
- level-up full-heal sync for friendlies

### `steam.rs`
- feature-gated platform runtime (`standalone`/`steam`)

## Current Hooks / Known Gaps
- Commander aura fields/upgrades are still hooks only (`aura_radius`, `commander_aura_bonus` not yet driving active aura effects).
- `FormationModifiers.defense_multiplier` and anti-cavalry values are still not fully wired into incoming damage resolution.
- Collision plugin remains unregistered in app wiring.
