# SYSTEMS_REFERENCE.md

## Purpose
Single-file technical reference for the current MVP implementation.
Use this to inspect any entity, component, resource, event, or gameplay system without scanning source files.

## Runtime Architecture

### App Builders
- Runtime app: `build_runtime_app()` in `src/lib.rs`
  - Uses `DefaultPlugins`
  - Window title: `Crusade Roguelite`
  - Resolution: `1280x720`
- Headless app: `build_headless_app()` in `src/lib.rs`
  - Uses `MinimalPlugins`

### Plugin Order
Configured in `configure_game_app()` (`src/lib.rs`) in this order:
1. `DataPlugin`
2. `CorePlugin`
3. `VisualPlugin`
4. `MapPlugin`
5. `SquadPlugin`
6. `FormationPlugin`
7. `RescuePlugin`
8. `EnemyPlugin`
9. `CombatPlugin`
10. `ProjectilePlugin`
11. `MoralePlugin`
12. `BannerPlugin`
13. `UpgradePlugin`
14. `UiPlugin`
15. `PlatformPlugin`

### Global Game States
Defined in `src/model.rs` (`GameState`):
1. `Boot`
2. `MainMenu`
3. `InRun`
4. `Paused`
5. `GameOver`

## Data Files and Current Live Values
Loaded by `GameData::load_from_dir("assets/data")` (`src/data.rs`).

### `assets/data/units.json`
- Commander (`baldiun`)
  - `max_hp: 120`
  - `armor: 6`
  - `damage: 12`
  - `attack_cooldown_secs: 0.9`
  - `attack_range: 26`
  - `move_speed: 170`
  - `morale_weight: 2`
  - `aura_radius: 180`
- Recruit (`infantry_knight`)
  - `max_hp: 95`
  - `armor: 4`
  - `damage: 9`
  - `attack_cooldown_secs: 1.1`
  - `attack_range: 24`
  - `move_speed: 150`
  - `morale_weight: 1`

### `assets/data/enemies.json`
- Enemy `bandit_raider` (melee)
  - `max_hp: 34`
  - `armor: 1`
  - `damage: 6`
  - `attack_cooldown_secs: 1.3`
  - `attack_range: 22`
  - `move_speed: 118`

### `assets/data/formations.json`
- Square
  - `slot_spacing: 30`
  - `offense_multiplier: 0.95`
  - `defense_multiplier: 1.1`
  - `anti_cavalry_multiplier: 1.2` (stored, not yet consumed by combat systems)

### `assets/data/waves.json`
- Wave schedule:
  1. `t=6s`, `count=4`
  2. `t=19s`, `count=6`
  3. `t=35s`, `count=8`
  4. `t=52s`, `count=10`
  5. `t=72s`, `count=12`

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

### `assets/data/rescue.json`
- `spawn_count: 14`
- `rescue_radius: 60`
- `rescue_duration_secs: 2.2`

## ECS Inventory

### Core Components (`src/model.rs`)
- `Unit { team, kind, level, morale_weight }`
- `Health { current, max }`
- `Armor(f32)`
- `AttackProfile { damage, range, cooldown_secs }`
- `AttackCooldown(Timer)`
- `MoveSpeed(f32)`
- Marker components:
  - `PlayerControlled`
  - `FriendlyUnit`
  - `EnemyUnit`
  - `RescuableUnit`
  - `CommanderUnit`

### Module-Specific Components
- `OasisZone { center, radius, heal_per_second }` (`src/map.rs`)
- `RescueProgress { elapsed }` (`src/rescue.rs`)
- `BanditVisualState` (`Idle`, `Move`, `Attack`, `Hit`, `Dead`) (`src/enemies.rs`)
- `BanditVisualRuntime { last_position, state }` (`src/enemies.rs`)
- `Projectile { velocity, damage, lifetime_secs, radius, source_team }` (`src/projectiles.rs`)
- `BannerMarker` (`src/banner.rs`)

### Resources
- `RunSession { survived_seconds }`
- `GlobalBuffs`
  - `damage_multiplier`
  - `armor_bonus`
  - `attack_speed_multiplier`
  - `cohesion_bonus`
  - `commander_aura_bonus`
- `GameData` (all loaded config blobs)
- `MapBounds { half_width, half_height }`
- `SquadRoster { commander, friendly_count, casualties }`
- `ActiveFormation` (currently `Square` only)
- `FormationModifiers { offense_multiplier, defense_multiplier }`
- `WaveRuntime { elapsed, next_wave_index }`
- `Cohesion { value }`
- `CohesionCombatModifiers { damage_multiplier, defense_multiplier, attack_speed_multiplier, collapse_risk }`
- `BannerState { is_dropped, world_position }`
- `BannerCombatModifiers { attack_multiplier, defense_multiplier }`
- `Progression { xp, level, next_level_xp }`
- `UpgradeDraft { active, options, autopick_timer }`
- `HudSnapshot { cohesion, banner_dropped, squad_size, xp, wave_index }`
- `PlatformRuntime { service }`
- `ArtAssets` (texture handles for commander/recruit/bandit-state set/banner/oasis/background)

### Events
- `StartRunEvent`
- `RecruitEvent { world_position }`
- `DamageEvent { target, source_team, amount }`
- `UnitDiedEvent { team, kind, world_position }`
- `GainXpEvent(f32)`

## Entity Archetypes

### Commander (spawned at run start)
Spawned by `spawn_commander()` in `src/squad.rs`.
- Components:
  - `Unit { team: Friendly, kind: Commander, level: 1, morale_weight: from config }`
  - `CommanderUnit`
  - `FriendlyUnit`
  - `PlayerControlled`
  - `Health`
  - `Armor`
  - `AttackProfile`
  - `AttackCooldown` (repeating timer)
  - `MoveSpeed`
  - `Transform`, `GlobalTransform`

### Friendly Recruit (Knight)
Spawned by `spawn_recruit()` in `src/squad.rs` on `RecruitEvent`.
- Components:
  - `Unit { team: Friendly, kind: InfantryKnight, level: 1, morale_weight: from config }`
  - `FriendlyUnit`
  - `Health`
  - `Armor`
  - `AttackProfile`
  - `AttackCooldown`
  - `MoveSpeed`
  - `Transform`, `GlobalTransform`

### Enemy Bandit Raider
Spawned by `spawn_enemy_wave()` in `src/enemies.rs`.
- Components:
  - `Unit { team: Enemy, kind: EnemyBanditRaider, level: 1 }`
  - `EnemyUnit`
  - `BanditVisualRuntime`
  - `Health`
  - `Armor`
  - `AttackProfile`
  - `AttackCooldown`
  - `MoveSpeed`
  - `Transform`, `GlobalTransform`

### Rescuable Neutral
Spawned by `spawn_rescuables_on_run_start()` in `src/rescue.rs`.
- Components:
  - `Unit { team: Neutral, kind: RescuableInfantry, level: 1 }`
  - `RescuableUnit`
  - `RescueProgress`
  - `Transform`, `GlobalTransform`

### Banner Entity
Spawned by `reset_banner_on_run_start()` in `src/banner.rs`.
- Components:
  - `BannerMarker`
  - `Transform`, `GlobalTransform`

### Oasis Entity
Spawned by `handle_start_run_oasis()` in `src/map.rs`.
- Components:
  - `OasisZone`

### Projectile Entity
Expected when ranged attacks are introduced.
- Components:
  - `Projectile`
  - `Transform`

## System Reference (By Module)

### `src/core.rs`
- `boot_to_menu` (`OnEnter(Boot)`)
  - Sets next state to `MainMenu`.
- `start_run_from_main_menu` (`Update`)
  - On `Enter` key in `MainMenu`:
    - resets `RunSession`
    - transitions to `InRun`
    - emits `StartRunEvent`
- `pause_toggle` (`Update`)
  - In `InRun`, `Escape` -> `Paused`.
- `resume_from_pause` (`Update`)
  - In `Paused`, `Escape` -> `InRun`.
- `tick_survival_time` (`Update`, `InRun`)
  - Increments `RunSession.survived_seconds`.
- `detect_game_over` (`Update`)
  - In `InRun`, if no `CommanderUnit` remains -> `GameOver`.
- `restart_from_game_over` (`Update`)
  - In `GameOver`, `Enter` -> reset `RunSession`, move to `MainMenu`.

### `src/data.rs`
- `load_data_on_boot` (`OnEnter(Boot)`)
  - Loads all JSON config and validates them.
  - Inserts `GameData` resource.
- Validation includes:
  - positive stat/range checks for units/enemies
  - non-empty upgrades/waves
  - strictly increasing `waves[].time_secs`

### `src/visuals.rs`
- `load_art_assets` (`Startup`)
  - Loads texture handles from `assets/third_party/kenney_desert-shooter-pack_1.0/**` into `ArtAssets`.
  - Falls back to default handles when `AssetServer` is unavailable (headless tests).

### `src/map.rs`
- `spawn_camera_once` (`Startup`)
  - Spawns a `Camera2dBundle`.
- `initialize_map_resources` (`OnEnter(MainMenu)`)
  - Inserts `MapBounds` from map config.
- `spawn_background_visual` (`OnEnter(MainMenu)`)
  - Spawns one background sprite covering map extents.
- `handle_start_run_oasis` (`Update`)
  - On run start:
    - clears old oasis entities
    - spawns one `OasisZone`
- `heal_units_inside_oasis` (`Update`, `InRun`)
  - Heals `FriendlyUnit` by `oasis_heal_per_second * dt` while inside radius.

### `src/squad.rs`
- `handle_start_run` (`Update`)
  - On `StartRunEvent`:
    - despawns all `Unit` entities
    - spawns commander
    - resets `SquadRoster`
- `commander_movement` (`Update`, `InRun`)
  - `WASD` movement for commander.
  - Clamped to `MapBounds`.
- `apply_recruit_events` (`Update`, `InRun`)
  - Spawns knight recruit at event position.
- `sync_roster` (`Update`, `InRun`)
  - Recomputes friendly count and commander entity handle.
- `on_unit_died` (`Update`)
  - Increments casualties on friendly deaths.

### `src/formation.rs`
- `load_square_modifiers` (`OnEnter(MainMenu)`)
  - Loads square offense/defense multipliers into resource.
- `apply_square_formation` (`Update`, `InRun`)
  - Keeps all non-commander friendlies in square slots around commander.
  - Smoothly interpolates movement each frame.
- `square_offsets(count, spacing)`
  - Produces slot coordinates in square grid.

### `src/rescue.rs`
- `spawn_rescuables_on_run_start` (`Update`)
  - On run start:
    - clears old rescuables
    - spawns `spawn_count` neutral rescuables in ring distribution
- `tick_rescue_progress` (`Update`, `InRun`)
  - For each rescuable:
    - if commander within `rescue_radius`, increment timer
    - else reset timer to 0
    - when timer reaches `rescue_duration_secs`, emit `RecruitEvent` and despawn rescuable
- `advance_rescue_progress(current, in_range, dt, duration)`
  - In-range: `min(current + dt, duration)`
  - Out-of-range: `0`

### `src/enemies.rs`
- `reset_waves_on_run_start` (`Update`)
  - Resets elapsed time and wave index.
- `spawn_waves` (`Update`, `InRun`)
  - Increments wave timer.
  - Spawns all waves whose scheduled time has passed.
- `spawn_enemy_wave`
  - Spawns `bandit_raider` enemies on perimeter ring.
  - Radius uses `max(map half width/height) * 0.9`.
- `enemy_chase_targets` (`Update`, `InRun`)
  - Enemy AI: nearest-friendly chase.
  - Movement step: `normalize(target - enemy) * move_speed * dt`.
- `update_bandit_visual_states` (`Update`, `InRun`)
  - Computes per-bandit visual state from movement, attack cooldown progress, and HP ratio.
  - Swaps sprite handle to matching state texture.
- `decide_bandit_visual_state(...)`
  - Priority: `Dead` -> `Hit` -> `Attack` -> `Move` -> `Idle`.
- `choose_nearest(origin, candidates)`
  - Returns nearest point by squared distance.

### `src/combat.rs`
- `tick_attack_timers` (`Update`, `InRun`)
  - Advances all attack cooldown timers.
  - Friendly attack speed affected by:
    - cohesion attack speed multiplier
    - global attack speed multiplier
- `emit_damage_events` (`Update`, `InRun`)
  - For attackers with finished cooldown:
    - find nearest in-range target on opposite team
    - compute damage
    - emit `DamageEvent`
    - reset cooldown timer
- `apply_damage_events` (`Update`, `InRun`)
  - Subtracts event damage from `Health.current`.
- `resolve_deaths` (`Update`, `InRun`)
  - Despawns units with `Health.current <= 0`.
  - Emits `UnitDiedEvent`.
  - Emits `GainXpEvent(5.0)` for enemy deaths.

Damage formula (`compute_damage`):
1. Start with `base_damage`.
2. If source is friendly, multiply by:
   - `formation_offense`
   - `cohesion_damage_multiplier`
   - `banner_attack_multiplier`
   - `global_damage_multiplier`
3. Subtract target armor.
4. Clamp minimum to `1.0`.

### `src/projectiles.rs`
- `tick_projectiles` (`Update`, `InRun`)
  - Moves projectiles by velocity.
  - Reduces lifetime and despawns expired projectiles.
- `projectile_collisions` (`Update`, `InRun`)
  - Checks distance collision to opposing-team targets.
  - Emits `DamageEvent` on hit and despawns projectile.

### `src/morale.rs`
- `update_cohesion_and_modifiers` (`Update`, `InRun`)
  - Computes cohesion value:
    - `100 - casualties * 6`
    - `-20` if banner dropped
    - `+ GlobalBuffs.cohesion_bonus`
    - clamped `[0, 100]`
  - Writes current cohesion modifiers via threshold table.

Cohesion thresholds (`cohesion_modifiers`):
- `>= 70`: no penalties
- `>= 40`: damage `0.9`, defense `0.95`, attack speed `0.95`
- `>= 20`: damage `0.8`, defense `0.9`, attack speed `0.9`
- `< 20`: damage `0.7`, defense `0.8`, attack speed `0.85`, `collapse_risk = true`

### `src/banner.rs`
- `reset_banner_on_run_start` (`Update`)
  - On run start:
    - clear old banner entities
    - set banner position to commander position
    - set `is_dropped = false`
    - spawn banner marker entity
- `follow_commander_when_banner_up` (`Update`, `InRun`)
  - While not dropped, banner follows commander position.
- `drop_banner_condition` (`Update`, `InRun`)
  - Drops banner when `should_drop_banner` returns true.
- `recover_banner_nearby` (`Update`, `InRun`)
  - If dropped and commander within distance `<= 40`, recover banner.
- `refresh_banner_modifiers` (`Update`, `InRun`)
  - Not dropped: attack/defense multipliers `1.0`
  - Dropped: attack `0.8`, defense `0.85`

Banner drop rule:
- `should_drop_banner(cohesion, casualties) = cohesion < 25 && casualties > 0`

### `src/upgrades.rs`
- `reset_progress_on_run_start` (`Update`)
  - Resets `Progression`, `UpgradeDraft`, and `GlobalBuffs`.
- `gain_xp` (`Update`, `InRun`)
  - Adds incoming `GainXpEvent` values.
- `open_draft_on_level_up` (`Update`, `InRun`)
  - When `xp >= next_level_xp`:
    - level up
    - consume xp by threshold
    - multiply `next_level_xp` by `1.25`
    - roll 3 draft options
    - set `draft.active = true`
    - transition to `Paused`
- `resolve_upgrade_draft` (`Update`, `Paused`)
  - Select option via keys `1/2/3`.
  - Auto-picks option 1 after `0.2s`.
  - Applies upgrade effect.
  - Clears draft and returns to `InRun`.

Upgrade option roll (`roll_upgrade_options`):
- Deterministic selection using offset `level % pool_len`.
- Picks 3 consecutive entries, wrapping around.

Applied upgrade effects:
- `add_units`: emits `RecruitEvent` near commander.
- `armor`: increments `GlobalBuffs.armor_bonus`.
- `damage`: increments `GlobalBuffs.damage_multiplier` by `value * 0.01`.
- `attack_speed`: increments `GlobalBuffs.attack_speed_multiplier` by `value`.
- `cohesion`: increments `GlobalBuffs.cohesion_bonus`.
- `commander_aura`: increments `GlobalBuffs.commander_aura_bonus`.

### `src/ui.rs`
- `refresh_hud_snapshot` (`Update`, `InRun`)
  - Populates `HudSnapshot` from live resources:
    - cohesion
    - banner dropped flag
    - squad size
    - xp
    - next wave index
- `attach_health_bars_to_units` (`Update`, `InRun`)
  - Adds world-space health bar children to friendly/enemy units.
- `update_health_bar_fills` (`Update`, `InRun`)
  - Updates bar width and color based on owner HP and team.
- `health_bar_fill_width(current, max, full_width)`
  - Clamps ratio into `[0, 1]` for deterministic width math.

### `src/steam.rs`
- `PlatformPlugin`
  - If `steam` feature enabled: inserts `SteamService`.
  - Otherwise inserts `StandaloneService`.
- `PlatformRuntime` resource exposes `platform_name()`.

## Progression and Leveling Notes

### Unit Levels
- Every spawned unit currently starts with `level = 1`.
- There is no per-unit XP or per-unit leveling pipeline yet.
- `Unit.level` exists as future expansion hook.

### Player/Run Progression
- Run-level progression uses `Progression` resource.
- XP source:
  - `+5` per enemy death (`resolve_deaths` in `combat`).
- Level threshold:
  - starts at `30`
  - multiplies by `1.25` each level-up.

## AI and Wave Behavior Summary
- AI type currently implemented:
  - `bandit_raider` melee chase AI with nearest-target selection.
- No pathfinding graph, no avoidance fields, no ranged kiting yet.
- Waves are strictly time-driven and deterministic based on config.

## Current Gaps / Hooks (Intentional)
- `GlobalBuffs.armor_bonus` is stored but not yet applied in combat resolution.
- `FormationModifiers.defense_multiplier` and `anti_cavalry_multiplier` are loaded; defense value is not yet wired into incoming damage mitigation.
- Commander aura and battle cry are represented as upgrade hooks and config fields; active aura-system logic is placeholder-level.
- Projectile system exists but no ranged attacker currently spawns projectiles in MVP.
