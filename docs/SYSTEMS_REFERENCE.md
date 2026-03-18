# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for the current MVP runtime.
Use this document to inspect entities, components, resources, events, data values, and system behavior without re-reading all source modules.

## Latest Update (2026-03-19)
- Documentation synchronized with current runtime behavior and data files.
- Waves run on 30-second cadence with infinite procedural continuation after scripted waves.
- Enemy deaths now spawn XP packs; XP is awarded only by pack pickup (no direct kill XP grant).
- XP pack value scales by wave and commander level.
- Commander level grants passive +1% damage and +1% attack speed per level, plus +1 max HP per level to commander and retinue.
- Level-up triggers full heal to current max HP for all friendlies.
- Main menu uses `Start`/`Exit` buttons and bottom-right FPS cap selector (60/90/120).
- In-run HUD shows wave (top-left), commander level + XP bar + rescue progress bars (top-center), elapsed time (top-right).

## Runtime Architecture

### App Builders
- Runtime app: `build_runtime_app()` in `src/lib.rs`
  - Uses `DefaultPlugins`
  - Window title: `Crusade Roguelite`
  - Resolution: `1280x720`
  - Uses Windows subsystem mode in release (`src/main.rs`) to hide terminal window.
- Headless app: `build_headless_app()` in `src/lib.rs`
  - Uses `MinimalPlugins`

### Plugin Order (configured in `src/lib.rs`)
1. `DataPlugin`
2. `CorePlugin`
3. `PerformancePlugin`
4. `VisualPlugin`
5. `MapPlugin`
6. `SquadPlugin`
7. `FormationPlugin`
8. `RescuePlugin`
9. `DropsPlugin`
10. `EnemyPlugin`
11. `CombatPlugin`
12. `ProjectilePlugin`
13. `MoralePlugin`
14. `BannerPlugin`
15. `UpgradePlugin`
16. `UiPlugin`
17. `PlatformPlugin`

### Plugin Note
- `src/collision.rs` contains collision resolution logic and tests, but `CollisionPlugin` is currently not registered in `configure_game_app()`.

### Global Game States (`src/model.rs`)
1. `Boot`
2. `MainMenu`
3. `InRun`
4. `Paused`
5. `GameOver` (defined but not actively used in current transition flow)

## Data Files and Live Values
Loaded by `GameData::load_from_dir("assets/data")` (`src/data.rs`).

### `assets/data/units.json`
- Commander (`baldiun`):
  - `max_hp: 120`
  - `armor: 6`
  - `damage: 12`
  - `attack_cooldown_secs: 0.9`
  - `attack_range: 34`
  - `move_speed: 170`
  - `morale_weight: 2`
  - `aura_radius: 180`
- Recruit (`infantry_knight`):
  - `max_hp: 95`
  - `armor: 4`
  - `damage: 9`
  - `attack_cooldown_secs: 1.1`
  - `attack_range: 32`
  - `move_speed: 150`
  - `morale_weight: 1`

### `assets/data/enemies.json`
- Enemy `bandit_raider`:
  - `max_hp: 34`
  - `armor: 1`
  - `damage: 6`
  - `attack_cooldown_secs: 1.3`
  - `attack_range: 30`
  - `move_speed: 118`

### `assets/data/formations.json`
- Square:
  - `slot_spacing: 30`
  - `offense_multiplier: 0.95`
  - `defense_multiplier: 1.1`
  - `anti_cavalry_multiplier: 1.2` (loaded, not yet consumed)

### `assets/data/waves.json`
Scripted waves:
1. `t=0s`, `count=8`
2. `t=30s`, `count=12`
3. `t=60s`, `count=16`
4. `t=90s`, `count=20`
5. `t=120s`, `count=24`

After scripted waves:
- Infinite waves every 30s.
- Enemy count: `base_count + (procedural_index + 1) * 4`.
- Enemy stat multiplier: `1.0 + (procedural_index + 1) * 0.08`.

### `assets/data/drops.json`
- `initial_spawn_count: 8`
- `spawn_interval_secs: 2.5`
- `pickup_radius: 30`
- `xp_per_pack: 6`
- `max_active_packs: 5000`

### `assets/data/rescue.json`
- `spawn_count: 14`
- `rescue_radius: 60`
- `rescue_duration_secs: 2.2`

### `assets/data/upgrades.json`
- `add_units` (`add_units`, `1.0`)
- `armor_up` (`armor`, `1.0`)
- `damage_up` (`damage`, `1.5`)
- `attack_speed_up` (`attack_speed`, `0.06`)
- `cohesion_up` (`cohesion`, `5.0`)
- `commander_aura_up` (`commander_aura`, `8.0`)

### `assets/data/map.json`
- `width: 2400`
- `height: 2400`
- `oasis_center: [180, -140]`
- `oasis_radius: 120`
- `oasis_heal_per_second: 4`
- Oasis values are currently data-only; no active oasis gameplay system is wired.

## ECS Inventory

### Core Components (`src/model.rs`)
- `Unit { team, kind, level, morale_weight }`
- `Health { current, max }`
- `BaseMaxHealth(f32)`
- `Armor(f32)`
- `AttackProfile { damage, range, cooldown_secs }`
- `AttackCooldown(Timer)`
- `MoveSpeed(f32)`
- `ColliderRadius(f32)`
- Markers:
  - `PlayerControlled`
  - `FriendlyUnit`
  - `EnemyUnit`
  - `RescuableUnit`
  - `CommanderUnit`

### Module Components
- `BanditVisualRuntime { last_position, state }` (`src/enemies.rs`)
- `BanditVisualState` (`Idle`, `Move`, `Attack`, `Hit`, `Dead`) (`src/enemies.rs`)
- `RescueProgress { elapsed }` (`src/rescue.rs`)
- `ExpPack { xp_value }` (`src/drops.rs`)
- `Projectile { velocity, damage, lifetime_secs, radius, source_team }` (`src/projectiles.rs`)
- `BannerMarker` (`src/banner.rs`)

### Resources
- `RunSession { survived_seconds }`
- `FrameRateCap` (`Fps60`, `Fps90`, `Fps120`)
- `GameData` (all config files)
- `MapBounds { half_width, half_height }`
- `SquadRoster { commander, friendly_count, casualties }`
- `ActiveFormation` (`Square`)
- `FormationModifiers { offense_multiplier, defense_multiplier }`
- `WaveRuntime { elapsed, next_wave_index, infinite_wave_index, next_infinite_spawn_time }`
- `Cohesion { value }`
- `CohesionCombatModifiers { damage_multiplier, defense_multiplier, attack_speed_multiplier, collapse_risk }`
- `BannerState { is_dropped, world_position }`
- `BannerCombatModifiers { attack_multiplier, defense_multiplier }`
- `Progression { xp, level, next_level_xp }`
- `UpgradeDraft { active, options, autopick_timer }`
- `GlobalBuffs { damage_multiplier, armor_bonus, attack_speed_multiplier, cohesion_bonus, commander_aura_bonus }`
- `HudSnapshot { cohesion, banner_dropped, squad_size, level, xp, next_level_xp, wave_index, current_wave, elapsed_seconds }`
- `PlatformRuntime { service }`
- `ArtAssets` (runtime sprite handles)

### Events
- `StartRunEvent`
- `RecruitEvent { world_position }`
- `DamageEvent { target, source_team, amount }`
- `UnitDiedEvent { team, kind, world_position }`
- `GainXpEvent(f32)`
- `SpawnExpPackEvent { world_position, xp_value_override }`

## Entity Archetypes

### Commander (run start)
Spawned in `spawn_commander()` (`src/squad.rs`):
- `Unit { team: Friendly, kind: Commander, level: 1 }`
- `CommanderUnit`, `FriendlyUnit`, `PlayerControlled`
- `Health`, `BaseMaxHealth`, `Armor`, `ColliderRadius(14.0)`
- `AttackProfile`, `AttackCooldown`, `MoveSpeed`
- `SpriteBundle`

### Friendly Recruit (Infantry/Knight)
Spawned in `spawn_recruit()` (`src/squad.rs`):
- `Unit { team: Friendly, kind: InfantryKnight, level: 1 }`
- `FriendlyUnit`
- `Health`, `BaseMaxHealth`, `Armor`, `ColliderRadius(12.0)`
- `AttackProfile`, `AttackCooldown`, `MoveSpeed`
- `SpriteBundle`

### Enemy (`bandit_raider`)
Spawned in `spawn_enemy_wave()` (`src/enemies.rs`):
- `Unit { team: Enemy, kind: EnemyBanditRaider }`
- `EnemyUnit`
- `BanditVisualRuntime`
- `Health`, `Armor`, `ColliderRadius(12.0)`
- `AttackProfile`, `AttackCooldown`, `MoveSpeed`
- `SpriteBundle`

### Rescuable Neutral
Spawned in `spawn_rescuable()` (`src/rescue.rs`):
- `Unit { team: Neutral, kind: RescuableInfantry }`
- `RescuableUnit`
- `RescueProgress`
- `SpriteBundle`

### XP Pack
Spawned in `spawn_exp_pack()` (`src/drops.rs`):
- `ExpPack { xp_value }`
- `SpriteBundle` (`art.exp_pack_coin_stack`)

### Banner
Spawned in `reset_banner_on_run_start()` (`src/banner.rs`):
- `BannerMarker`
- `SpriteBundle`

## System Reference (By Module)

### `src/core.rs`
- `boot_to_menu` (`OnEnter(Boot)`): sets state to `MainMenu`.
- `cleanup_run_entities_on_menu_enter` (`OnEnter(MainMenu)`): clears units, drops, banner, projectiles; resets `RunSession`.
- `set_main_menu_clear_color` / `set_in_run_clear_color`: switches background color by state.
- `pause_toggle` / `resume_from_pause` (`Escape`): toggles `InRun` <-> `Paused`.
- `tick_survival_time`: increments run timer during `InRun`.
- `detect_game_over` (`PostUpdate`, `InRun`): if commander missing, returns to `MainMenu`.

### `src/performance.rs`
- `limit_frame_rate` (`Last`): software frame limiter based on `FrameRateCap`.
- UI can switch cap via `FrameRateCap` resource (60/90/120).

### `src/data.rs`
- `load_data_on_boot`: loads/validates all `assets/data/*.json` and inserts `GameData`.
- Validation includes positive ranges and strict increasing `waves[].time_secs`.

### `src/visuals.rs`
- `load_art_assets` (`Startup`): loads sprite handles used by runtime entities/HUD visuals.

### `src/map.rs`
- `spawn_camera_once` (`Startup`)
- `initialize_map_resources` (`OnEnter(MainMenu)`)
- `spawn_background_visual` (`OnEnter(MainMenu)`)
- `follow_camera_commander` (`InRun`): camera tracks commander.
- `snap_world_to_pixel_grid` (`InRun`): rounds world/camera coordinates to reduce jitter.

### `src/squad.rs`
- `handle_start_run`: clears old units, spawns commander, resets `SquadRoster`.
- `commander_movement`: WASD/arrow movement with map clamping.
- `apply_recruit_events`: converts recruit events into knight spawns.
- `sync_roster`: keeps friendly count and commander entity current.
- `on_unit_died`: increments friendly casualty count.

### `src/formation.rs`
- `load_square_modifiers`: loads square offense/defense modifiers.
- `apply_square_formation`: lerps recruit positions into square slots around commander.
- `sync_friendly_depth_sorting`: sets z-order from world Y (`depth_z_for_world_y`).

### `src/rescue.rs`
- `spawn_rescuables_on_run_start`: clears/reseeds neutral rescuables.
- `spawn_rescuables_over_time`: continuous respawn up to active cap (`12`).
- `tick_rescue_progress`: rescue channels when any `FriendlyUnit` is in radius; complete rescue emits `RecruitEvent`.

### `src/drops.rs`
- `spawn_exp_packs_on_run_start`: clears and reseeds pack field.
- `spawn_exp_packs_over_time`: ambient pack spawning over time.
- `spawn_exp_packs_from_events`: spawns packs from gameplay events (enemy deaths).
- `pickup_exp_packs`: any friendly touching pack grants XP and despawns pack.
- XP pack scaling: `base * (1 + 0.06*(wave-1)) * (1 + 0.04*(commander_level-1))`.

### `src/enemies.rs`
- `reset_waves_on_run_start`
- `spawn_waves`: scripted schedule then infinite procedural waves.
- `enemy_chase_targets`: melee chase behavior.
  - If retinue exists, enemies prioritize non-commander friendly targets.
- `update_bandit_visual_states`: swaps idle/move/attack/hit/dead textures by runtime state.
- Enemy speed runtime scaling: `base_speed * 0.72`.

### `src/combat.rs`
- `tick_attack_timers`: advances cooldowns; friendly cadence affected by cohesion, upgrades, commander-level passive.
- `emit_damage_events`: nearest in-range auto-targeting and damage event creation.
- `apply_damage_events`: subtracts HP.
- `resolve_deaths`: emits `UnitDiedEvent`, spawns XP pack event for enemy deaths, despawns dead units.

Damage formula (`compute_damage`):
1. Start with base damage.
2. If source is friendly, multiply by formation offense, cohesion damage multiplier, banner attack multiplier, and global damage multiplier.
3. Subtract armor.
4. Clamp to minimum `1.0`.

### `src/projectiles.rs`
- Projectile travel/lifetime and collision pipeline exists and is test-covered.
- Current MVP roster does not spawn projectile attackers yet.

### `src/morale.rs`
- `update_cohesion_and_modifiers`: recomputes cohesion from casualties, banner state, and cohesion buffs.
- Threshold table drives damage/defense/attack speed penalties and collapse risk flag.

### `src/banner.rs`
- `reset_banner_on_run_start`
- `follow_commander_when_banner_up`
- `drop_banner_condition` (`cohesion < 25 && casualties > 0`)
- `recover_banner_nearby` (commander within 40 units)
- `sync_banner_texture` and `refresh_banner_modifiers`

### `src/upgrades.rs`
- `reset_progress_on_run_start`
- `gain_xp`
- `open_draft_on_level_up`: opens pause-state draft when threshold reached.
- `resolve_upgrade_draft`: key select (`1/2/3`) with autopick fallback.
- `sync_friendly_level_health_caps`: applies commander-level HP scaling and full-heal on level-up.

Progression helpers:
- `xp_required_for_level(level)`: starts at `30`, multiplies by `1.25` per level.
- `commander_level_hp_bonus(level)`: `level - 1`.
- `commander_level_combat_multiplier(level)` (from `combat.rs`): `1 + 0.01*(level-1)`.

### `src/ui.rs`
- Main menu lifecycle:
  - `spawn_main_menu` / `despawn_main_menu`
  - `Start` button: enters `InRun`, sends `StartRunEvent`
  - `Exit` button: sends `AppExit`
  - FPS cap selector at bottom-right (`60/90/120`)
- In-run HUD lifecycle:
  - `spawn_in_run_hud` / `despawn_in_run_hud`
  - `refresh_hud_snapshot` + `update_in_run_hud` (wave, level, XP bar, timer)
  - `update_rescue_progress_hud` (active rescue bars only)
- World-space health bars:
  - `attach_health_bars_to_units`
  - `update_health_bar_fills`

### `src/steam.rs`
- Feature-gated platform runtime (`standalone` by default, `steam` behind feature flag).

### `src/collision.rs`
- Contains pairwise collision resolution utilities:
  - enemy-enemy separation
  - enemy-vs-inner-ring-retinue separation rules
- Test-covered but currently inactive at runtime until plugin registration.

## Progression and XP Economy
- XP sources:
  - Ambient XP pack spawns.
  - Enemy death-triggered XP pack spawns.
- XP is only granted when a friendly unit enters pickup radius of an XP pack.
- XP packs are capped by `drops.max_active_packs`.
- Level-up effects:
  - Upgrade draft pause flow.
  - Full heal to all friendlies.
  - Passive scaling to combat cadence/damage and max HP.

## Current Gaps / Hooks
- `GlobalBuffs.armor_bonus` is stored but not yet applied to armor calculations.
- `FormationModifiers.defense_multiplier` and `anti_cavalry_multiplier` are loaded but not yet applied in incoming-damage logic.
- Oasis config is present but no oasis gameplay system currently active.
- `GameState::GameOver` exists but current defeat flow returns directly to `MainMenu`.
- Collision module exists but `CollisionPlugin` is not currently registered.
