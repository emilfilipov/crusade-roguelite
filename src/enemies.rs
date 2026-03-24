use bevy::prelude::*;
use std::collections::HashMap;

use crate::ai::{
    chase_step_distance, chase_target_positions, choose_nearest, choose_support_follow_target,
    should_move_towards_target,
};
use crate::combat::{RangedAttackCooldown, RangedAttackProfile};
use crate::data::{EnemyStatsConfig, GameData, WavesConfigFile};
use crate::formation::{ActiveFormation, active_formation_config, formation_contains_position};
use crate::map::{MapBounds, playable_bounds};
use crate::model::{
    Armor, AttackCooldown, AttackProfile, ColliderRadius, CommanderUnit, EnemyUnit, FriendlyUnit,
    GameState, Health, MatchSetupSelection, Morale, MoveSpeed, PlayerFaction, StartRunEvent, Team,
    Unit, UnitCohesion, UnitKind,
};
use crate::squad::PriestSupportCaster;
use crate::squad::RosterEconomy;
use crate::upgrades::Progression;
use crate::visuals::ArtAssets;

#[derive(Resource, Clone, Debug, Default)]
pub struct WaveRuntime {
    pub elapsed: f32,
    pub current_wave: u32,
    pub wave_elapsed: f32,
    pub spawn_accumulator: f32,
    pub finished_spawning: bool,
    pub victory_announced: bool,
    pub pending_batches: Vec<PendingEnemyBatch>,
    role_mix: HashMap<u32, EnemyRoleCounts>,
    pub spawn_sequence: u32,
}

#[derive(Clone, Debug)]
pub struct PendingEnemyBatch {
    pub remaining: u32,
    pub wave_number: u32,
    pub stat_scale: f32,
    pub next_spawn_time: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnemySpawnRole {
    Melee,
    Ranged,
    Support,
}

#[derive(Clone, Copy, Debug, Default)]
struct EnemyRoleCounts {
    total: u32,
    melee: u32,
    ranged: u32,
    support: u32,
}

impl EnemyRoleCounts {
    fn count_for(self, role: EnemySpawnRole) -> u32 {
        match role {
            EnemySpawnRole::Melee => self.melee,
            EnemySpawnRole::Ranged => self.ranged,
            EnemySpawnRole::Support => self.support,
        }
    }

    fn register(&mut self, role: EnemySpawnRole) {
        self.total = self.total.saturating_add(1);
        match role {
            EnemySpawnRole::Melee => {
                self.melee = self.melee.saturating_add(1);
            }
            EnemySpawnRole::Ranged => {
                self.ranged = self.ranged.saturating_add(1);
            }
            EnemySpawnRole::Support => {
                self.support = self.support.saturating_add(1);
            }
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum BanditVisualState {
    Idle,
    Move,
    Attack,
    Hit,
    Dead,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BanditVisualRuntime {
    pub last_position: Vec2,
    pub state: BanditVisualState,
}

#[derive(Component, Clone, Copy, Debug)]
struct EnemyMovementState {
    moving: bool,
}

const ENEMY_BASE_SPEED_MULTIPLIER: f32 = 0.72;
const WAVE_DURATION_SECS: f32 = 30.0;
pub const MAX_WAVES: u32 = 100;
const STOP_FACTOR: f32 = 0.82;
const RESUME_FACTOR: f32 = 0.98;
const ENEMY_INSIDE_FORMATION_PADDING_SLOTS: f32 = 0.35;
const ENEMY_FORMATION_REPEL_MARGIN_SLOTS: f32 = 0.12;
const WAVE_UNITS_MULTIPLIER: f32 = 2.0;
const MAX_ENEMIES_PER_WAVE: f32 = 1000.0;
const POST_SCRIPTED_WAVE_COUNT_GROWTH: f32 = 1.18;
const WAVE_STAT_GROWTH_PER_WAVE: f32 = 0.102;
const WAVE_BATCH_SIZE: u32 = 7;
const WAVE_BATCH_INTERVAL_SECS: f32 = 0.7;
const ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER: f32 = 200.0;
const ENEMY_SPAWN_ATTEMPTS: u32 = 8;
const DEFAULT_SPAWN_HALF_WIDTH: f32 = 900.0;
const DEFAULT_SPAWN_HALF_HEIGHT: f32 = 700.0;
const ENEMY_LEVEL_PRESSURE_PER_LEVEL: f32 = 0.011;
const ENEMY_RETINUE_PRESSURE_PER_UNIT: f32 = 0.008;
const ENEMY_RETINUE_PRESSURE_EXPONENT: f32 = 0.72;
const ENEMY_PLAYER_PRESSURE_STAT_CAP: f32 = 3.5;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveRuntime>()
            .add_systems(Update, reset_waves_on_run_start)
            .add_systems(
                Update,
                (
                    spawn_waves,
                    spawn_pending_enemy_batches,
                    enemy_chase_targets,
                    update_bandit_visual_states,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Last,
                repel_enemy_overflow_from_formation.run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_waves_on_run_start(
    mut wave_runtime: ResMut<WaveRuntime>,
    mut start_events: EventReader<StartRunEvent>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *wave_runtime = WaveRuntime {
        current_wave: 1,
        ..WaveRuntime::default()
    };
}

fn spawn_waves(time: Res<Time>, data: Res<GameData>, mut wave_runtime: ResMut<WaveRuntime>) {
    let dt = time.delta_seconds();
    if dt <= 0.0 {
        return;
    }
    if wave_runtime.current_wave == 0 {
        wave_runtime.current_wave = 1;
    }

    wave_runtime.elapsed += dt;
    if wave_runtime.finished_spawning {
        return;
    }

    wave_runtime.wave_elapsed += dt;
    while wave_runtime.wave_elapsed >= WAVE_DURATION_SECS && wave_runtime.current_wave < MAX_WAVES {
        wave_runtime.wave_elapsed -= WAVE_DURATION_SECS;
        wave_runtime.current_wave = wave_runtime.current_wave.saturating_add(1);
    }

    if wave_runtime.current_wave >= MAX_WAVES && wave_runtime.wave_elapsed >= WAVE_DURATION_SECS {
        wave_runtime.finished_spawning = true;
        wave_runtime.wave_elapsed = WAVE_DURATION_SECS;
        return;
    }

    let spawn_rate = units_per_second_for_wave(&data.waves, wave_runtime.current_wave);
    wave_runtime.spawn_accumulator += spawn_rate * dt;
    let spawn_count = wave_runtime.spawn_accumulator.floor().max(0.0) as u32;
    if spawn_count == 0 {
        return;
    }
    wave_runtime.spawn_accumulator -= spawn_count as f32;
    let wave_number = wave_runtime.current_wave;
    let stat_scale = wave_stat_multiplier(wave_number);
    enqueue_wave_batch(&mut wave_runtime, spawn_count, wave_number, stat_scale);
}

#[allow(clippy::too_many_arguments)]
fn spawn_pending_enemy_batches(
    mut commands: Commands,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    progression: Option<Res<Progression>>,
    roster_economy: Option<Res<RosterEconomy>>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut wave_runtime: ResMut<WaveRuntime>,
) {
    if wave_runtime.pending_batches.is_empty() {
        return;
    }

    let player_faction = setup_selection
        .as_ref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let spawn_bounds = bounds
        .as_deref()
        .copied()
        .map(playable_bounds)
        .unwrap_or_else(default_spawn_bounds);
    let commander_position = commanders
        .get_single()
        .map(|transform| transform.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    let commander_level = progression.as_ref().map(|value| value.level).unwrap_or(1);
    let retinue_count = roster_economy
        .as_ref()
        .map(|value| value.total_retinue_count)
        .unwrap_or(0);
    let player_pressure_multiplier =
        enemy_player_pressure_multiplier(commander_level, retinue_count);
    let current_time = wave_runtime.elapsed;
    let mut spawn_sequence = wave_runtime.spawn_sequence;
    let mut pending_batches = std::mem::take(&mut wave_runtime.pending_batches);
    let mut role_mix = std::mem::take(&mut wave_runtime.role_mix);
    let mut remaining = Vec::with_capacity(pending_batches.len());
    for mut batch in pending_batches.drain(..) {
        if current_time + f32::EPSILON < batch.next_spawn_time {
            remaining.push(batch);
            continue;
        }

        let spawn_now = batch_size_for_wave(batch.wave_number).min(batch.remaining);
        spawn_enemy_batch(
            &mut commands,
            spawn_now,
            &data,
            &art,
            player_faction,
            spawn_bounds,
            commander_position,
            batch.wave_number,
            batch.stat_scale * player_pressure_multiplier,
            &mut spawn_sequence,
            &mut role_mix,
        );
        batch.remaining = batch.remaining.saturating_sub(spawn_now);
        if batch.remaining > 0 {
            batch.next_spawn_time = current_time + batch_interval_secs(batch.wave_number);
            remaining.push(batch);
        }
    }
    wave_runtime.spawn_sequence = spawn_sequence;
    wave_runtime.pending_batches = remaining;
    wave_runtime.role_mix = role_mix;
}

#[allow(clippy::too_many_arguments)]
fn spawn_enemy_batch(
    commands: &mut Commands,
    count: u32,
    data: &GameData,
    art: &ArtAssets,
    player_faction: PlayerFaction,
    bounds: MapBounds,
    commander_position: Vec2,
    wave_number: u32,
    stat_scale: f32,
    spawn_sequence: &mut u32,
    role_mix: &mut HashMap<u32, EnemyRoleCounts>,
) {
    let enemy_pool = data.enemies.opposing_enemy_pool(player_faction);
    let enemy_pool_roles: Vec<(UnitKind, EnemySpawnRole)> = enemy_pool
        .iter()
        .copied()
        .filter_map(|kind| {
            let cfg = data.enemies.enemy_profile_for_kind(kind)?;
            Some((kind, enemy_spawn_role(kind, cfg)))
        })
        .collect();
    if enemy_pool_roles.is_empty() {
        return;
    }
    let has_melee = enemy_pool_roles
        .iter()
        .any(|(_, role)| *role == EnemySpawnRole::Melee);
    let has_ranged = enemy_pool_roles
        .iter()
        .any(|(_, role)| *role == EnemySpawnRole::Ranged);
    let has_support = enemy_pool_roles
        .iter()
        .any(|(_, role)| *role == EnemySpawnRole::Support);

    for _ in 0..count {
        let seq = *spawn_sequence;
        *spawn_sequence = spawn_sequence.saturating_add(1);
        let counters = role_mix.get(&wave_number).copied().unwrap_or_default();
        let spawn_role = pick_next_spawn_role(
            counters,
            has_melee,
            has_ranged,
            has_support,
            seq ^ 0x5F9D_A5C7,
        );
        let enemy_kind = choose_enemy_kind_for_role(
            &enemy_pool_roles,
            spawn_role,
            hash_seed(wave_number, seq, 0xC55A_A5AA),
        );
        let Some(cfg) = data.enemies.enemy_profile_for_kind(enemy_kind) else {
            continue;
        };
        role_mix
            .entry(wave_number)
            .or_default()
            .register(spawn_role);
        let enemy_faction = enemy_kind.faction().unwrap_or(player_faction.opposing());
        let faction_mods = data.factions.for_faction(enemy_faction);
        let hp = cfg.max_hp * stat_scale * faction_mods.enemy_health_multiplier;
        let armor = cfg.armor + (stat_scale - 1.0) * 2.0;
        let damage = cfg.damage * stat_scale * faction_mods.enemy_damage_multiplier;
        let base_cooldown = (cfg.attack_cooldown_secs / (1.0 + (stat_scale - 1.0) * 0.15))
            .clamp(0.2, cfg.attack_cooldown_secs);
        let attack_cooldown_secs =
            scale_enemy_attack_cooldown(base_cooldown, faction_mods.enemy_attack_speed_multiplier);
        let ranged_cooldown_secs = if cfg.ranged_attack_damage > 0.0 {
            let base_ranged_cooldown = (cfg.ranged_attack_cooldown_secs
                / (1.0 + (stat_scale - 1.0) * 0.15))
                .clamp(0.15, cfg.ranged_attack_cooldown_secs);
            Some(scale_enemy_attack_cooldown(
                base_ranged_cooldown,
                faction_mods.enemy_attack_speed_multiplier,
            ))
        } else {
            None
        };
        let move_speed =
            enemy_move_speed(cfg.move_speed * faction_mods.enemy_move_speed_multiplier);
        let morale = (cfg.morale * faction_mods.enemy_morale_multiplier).max(1.0);
        let cohesion = (cfg.cohesion * faction_mods.enemy_cohesion_multiplier).max(1.0);
        let texture = enemy_texture_for_kind(art, enemy_kind);
        let pos = random_spawn_position(bounds, commander_position, wave_number, seq);
        let mut entity = commands.spawn((
            Unit {
                team: Team::Enemy,
                kind: enemy_kind,
                level: 1,
            },
            EnemyUnit,
            BanditVisualRuntime {
                last_position: pos,
                state: BanditVisualState::Idle,
            },
            EnemyMovementState { moving: true },
            Health::new(hp),
            Morale::new(morale),
            UnitCohesion::new(cohesion),
            Armor(armor),
            AttackProfile {
                damage,
                range: cfg.attack_range,
                cooldown_secs: attack_cooldown_secs,
            },
            AttackCooldown(Timer::from_seconds(
                attack_cooldown_secs,
                TimerMode::Repeating,
            )),
            MoveSpeed(move_speed),
            ColliderRadius(cfg.collision_radius),
            SpriteBundle {
                texture,
                sprite: Sprite {
                    color: Color::srgb(1.0, 0.85, 0.85),
                    custom_size: Some(Vec2::splat(32.0)),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 5.0),
                ..default()
            },
        ));
        if let Some(cooldown_secs) = ranged_cooldown_secs {
            entity.insert((
                RangedAttackProfile {
                    damage: cfg.ranged_attack_damage,
                    range: cfg.ranged_attack_range,
                    projectile_speed: cfg.ranged_projectile_speed,
                    projectile_max_distance: cfg.ranged_projectile_max_distance,
                },
                RangedAttackCooldown(Timer::from_seconds(cooldown_secs, TimerMode::Repeating)),
            ));
        }
        if enemy_kind.is_priest() {
            entity.insert(PriestSupportCaster { cooldown: 20.0 });
        }
    }
}

fn enemy_spawn_role(kind: UnitKind, cfg: &EnemyStatsConfig) -> EnemySpawnRole {
    if kind.is_priest() || cfg.damage <= 0.0 {
        EnemySpawnRole::Support
    } else if cfg.ranged_attack_damage > 0.0 {
        EnemySpawnRole::Ranged
    } else {
        EnemySpawnRole::Melee
    }
}

fn choose_enemy_kind_for_role(
    enemy_pool_roles: &[(UnitKind, EnemySpawnRole)],
    preferred_role: EnemySpawnRole,
    seed: u32,
) -> UnitKind {
    if enemy_pool_roles.is_empty() {
        return UnitKind::ChristianPeasantInfantry;
    }
    let matching_count = enemy_pool_roles
        .iter()
        .filter(|(_, role)| *role == preferred_role)
        .count();
    if matching_count == 0 {
        return enemy_pool_roles[(seed as usize) % enemy_pool_roles.len()].0;
    }
    let target_match_index = (seed as usize) % matching_count;
    let mut seen = 0usize;
    for (kind, role) in enemy_pool_roles {
        if *role != preferred_role {
            continue;
        }
        if seen == target_match_index {
            return *kind;
        }
        seen = seen.saturating_add(1);
    }
    enemy_pool_roles[0].0
}

fn pick_next_spawn_role(
    counts: EnemyRoleCounts,
    has_melee: bool,
    has_ranged: bool,
    has_support: bool,
    tie_seed: u32,
) -> EnemySpawnRole {
    let next_total = counts.total.saturating_add(1);
    let support_cap = next_total / 4;
    let support_allowed = has_support && counts.support < support_cap;

    let mut candidates = [EnemySpawnRole::Melee; 3];
    let mut candidate_count = 0usize;
    if has_melee {
        candidates[candidate_count] = EnemySpawnRole::Melee;
        candidate_count += 1;
    }
    if has_ranged {
        candidates[candidate_count] = EnemySpawnRole::Ranged;
        candidate_count += 1;
    }
    if support_allowed {
        candidates[candidate_count] = EnemySpawnRole::Support;
        candidate_count += 1;
    }

    if candidate_count == 0 {
        if has_melee {
            candidates[candidate_count] = EnemySpawnRole::Melee;
            candidate_count += 1;
        }
        if has_ranged {
            candidates[candidate_count] = EnemySpawnRole::Ranged;
            candidate_count += 1;
        }
        if has_support {
            candidates[candidate_count] = EnemySpawnRole::Support;
            candidate_count += 1;
        }
    }
    if candidate_count == 0 {
        return EnemySpawnRole::Melee;
    }

    let mut best_deficit = f32::MIN;
    let mut best = [EnemySpawnRole::Melee; 3];
    let mut best_count = 0usize;
    let non_support_role_count = has_melee as u32 + has_ranged as u32;
    for role in candidates.iter().copied().take(candidate_count) {
        let deficit = target_role_count(role, next_total, has_support, non_support_role_count)
            - counts.count_for(role) as f32;
        if deficit > best_deficit + f32::EPSILON {
            best_deficit = deficit;
            best[0] = role;
            best_count = 1;
            continue;
        }
        if (deficit - best_deficit).abs() <= f32::EPSILON {
            best[best_count] = role;
            best_count += 1;
        }
    }

    best[(tie_seed as usize) % best_count]
}

fn target_role_count(
    role: EnemySpawnRole,
    next_total: u32,
    has_support: bool,
    non_support_role_count: u32,
) -> f32 {
    let ratio = match role {
        EnemySpawnRole::Support => {
            if !has_support {
                0.0
            } else if non_support_role_count == 0 {
                1.0
            } else {
                0.25
            }
        }
        EnemySpawnRole::Melee | EnemySpawnRole::Ranged => {
            if non_support_role_count == 0 {
                0.0
            } else {
                let support_ratio = if has_support { 0.25 } else { 0.0 };
                (1.0 - support_ratio) / non_support_role_count as f32
            }
        }
    };
    next_total as f32 * ratio
}

fn scale_enemy_attack_cooldown(base_cooldown: f32, speed_multiplier: f32) -> f32 {
    (base_cooldown / speed_multiplier.max(0.01)).max(0.08)
}

fn enqueue_wave_batch(
    wave_runtime: &mut WaveRuntime,
    count: u32,
    wave_number: u32,
    stat_scale: f32,
) {
    if count == 0 {
        return;
    }
    wave_runtime.pending_batches.push(PendingEnemyBatch {
        remaining: count,
        wave_number,
        stat_scale,
        next_spawn_time: wave_runtime.elapsed,
    });
}

pub fn wave_duration_secs() -> f32 {
    WAVE_DURATION_SECS
}

pub fn enemy_move_speed(base_speed: f32) -> f32 {
    base_speed * ENEMY_BASE_SPEED_MULTIPLIER
}

fn wave_base_count(config: &WavesConfigFile, wave_number: u32) -> f32 {
    if config.waves.is_empty() {
        return 1.0;
    }
    let index = wave_number.saturating_sub(1) as usize;
    if let Some(wave) = config.waves.get(index) {
        return wave.count as f32;
    }
    let last_count = config.waves.last().map(|wave| wave.count).unwrap_or(1) as f32;
    let extra_waves = index.saturating_sub(config.waves.len().saturating_sub(1)) as f32;
    last_count * POST_SCRIPTED_WAVE_COUNT_GROWTH.powf(extra_waves)
}

pub fn units_per_second_for_wave(config: &WavesConfigFile, wave_number: u32) -> f32 {
    let base_count = wave_base_count(config, wave_number);
    let wave_total = (base_count * WAVE_UNITS_MULTIPLIER).clamp(1.0, MAX_ENEMIES_PER_WAVE);
    wave_total / WAVE_DURATION_SECS
}

pub fn wave_stat_multiplier(wave_number: u32) -> f32 {
    1.0 + wave_number.saturating_sub(1) as f32 * WAVE_STAT_GROWTH_PER_WAVE
}

pub fn enemy_player_pressure_multiplier(commander_level: u32, retinue_count: u32) -> f32 {
    let level_multiplier =
        1.0 + commander_level.saturating_sub(1) as f32 * ENEMY_LEVEL_PRESSURE_PER_LEVEL;
    let retinue_multiplier = 1.0
        + (retinue_count as f32).powf(ENEMY_RETINUE_PRESSURE_EXPONENT)
            * ENEMY_RETINUE_PRESSURE_PER_UNIT;
    (level_multiplier * retinue_multiplier).clamp(1.0, ENEMY_PLAYER_PRESSURE_STAT_CAP)
}

fn default_spawn_bounds() -> MapBounds {
    MapBounds {
        half_width: DEFAULT_SPAWN_HALF_WIDTH,
        half_height: DEFAULT_SPAWN_HALF_HEIGHT,
    }
}

fn batch_size_for_wave(wave_number: u32) -> u32 {
    (WAVE_BATCH_SIZE + wave_number / 4).clamp(WAVE_BATCH_SIZE, 22)
}

fn batch_interval_secs(wave_number: u32) -> f32 {
    (WAVE_BATCH_INTERVAL_SECS - wave_number as f32 * 0.01).clamp(0.24, WAVE_BATCH_INTERVAL_SECS)
}

pub fn should_trigger_victory(runtime: &WaveRuntime, alive_enemy_count: usize) -> bool {
    runtime.finished_spawning
        && runtime.current_wave >= MAX_WAVES
        && runtime.pending_batches.is_empty()
        && alive_enemy_count == 0
}

pub fn random_spawn_position(
    bounds: MapBounds,
    commander_position: Vec2,
    wave_seed: u32,
    spawn_sequence: u32,
) -> Vec2 {
    let min_distance_sq =
        ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER * ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER;
    let mut fallback = commander_position;
    for attempt in 0..ENEMY_SPAWN_ATTEMPTS {
        let seed = hash_seed(wave_seed, spawn_sequence, attempt);
        let x = normalized_seed(seed);
        let y = normalized_seed(seed ^ 0x9E37_79B9);
        let candidate = Vec2::new(
            lerp(-bounds.half_width, bounds.half_width, x),
            lerp(-bounds.half_height, bounds.half_height, y),
        );
        fallback = candidate;
        if candidate.distance_squared(commander_position) >= min_distance_sq {
            return candidate;
        }
    }
    fallback
}

fn hash_seed(wave_seed: u32, spawn_sequence: u32, attempt: u32) -> u32 {
    let mut value = wave_seed
        .wrapping_mul(1_103_515_245)
        .wrapping_add(spawn_sequence.wrapping_mul(747_796_405))
        .wrapping_add(attempt.wrapping_mul(2_891_336_453))
        .wrapping_add(0x9E37_79B9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^ (value >> 16)
}

fn normalized_seed(seed: u32) -> f32 {
    seed as f32 / u32::MAX as f32
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
    min + (max - min) * t
}

#[allow(clippy::type_complexity)]
fn enemy_chase_targets(
    time: Res<Time>,
    mut enemy_sets: ParamSet<(
        Query<
            (
                Entity,
                &Unit,
                &MoveSpeed,
                &AttackProfile,
                Option<&RangedAttackProfile>,
                &mut EnemyMovementState,
                &mut Transform,
            ),
            (With<EnemyUnit>, Without<FriendlyUnit>),
        >,
        Query<(Entity, &Transform, &Unit), (With<EnemyUnit>, Without<FriendlyUnit>)>,
    )>,
    friendlies: Query<
        (&Transform, Option<&CommanderUnit>),
        (With<FriendlyUnit>, Without<EnemyUnit>),
    >,
) {
    let all_friendlies: Vec<(Vec2, bool)> = friendlies
        .iter()
        .map(|(transform, commander)| (transform.translation.truncate(), commander.is_some()))
        .collect();
    let targets = chase_target_positions(&all_friendlies);
    if targets.is_empty() {
        return;
    }

    let all_enemy_positions: Vec<(Entity, Vec2, bool)> = enemy_sets
        .p1()
        .iter()
        .map(|(entity, transform, unit)| {
            (
                entity,
                transform.translation.truncate(),
                unit.kind.is_priest(),
            )
        })
        .collect();
    let non_support_follow_targets: Vec<(Entity, Vec2)> = all_enemy_positions
        .iter()
        .filter(|(_, _, is_priest)| !*is_priest)
        .map(|(entity, position, _)| (*entity, *position))
        .collect();
    let fallback_follow_targets: Vec<(Entity, Vec2)> = all_enemy_positions
        .iter()
        .map(|(entity, position, _)| (*entity, *position))
        .collect();

    for (
        enemy_entity,
        unit,
        move_speed,
        attack_profile,
        ranged_profile,
        mut movement_state,
        mut enemy_transform,
    ) in &mut enemy_sets.p0()
    {
        let enemy_position = enemy_transform.translation.truncate();
        let target = if unit.kind.is_priest() {
            choose_support_follow_target(
                enemy_entity,
                enemy_position,
                &non_support_follow_targets,
                &fallback_follow_targets,
            )
        } else {
            choose_nearest(enemy_position, &targets)
        };
        if let Some(target) = target {
            let delta = target - enemy_position;
            let distance = delta.length();
            let desired_range = enemy_engagement_range(attack_profile.range, ranged_profile);
            let stop_distance = (desired_range * STOP_FACTOR).max(10.0);
            let resume_distance = (desired_range * RESUME_FACTOR).max(stop_distance + 3.0);

            movement_state.moving = should_move_towards_target(
                movement_state.moving,
                distance,
                stop_distance,
                resume_distance,
            );
            if movement_state.moving && distance > 0.001 {
                let step_distance = chase_step_distance(
                    distance,
                    stop_distance,
                    move_speed.0 * time.delta_seconds(),
                );
                if step_distance <= 0.0 {
                    continue;
                }
                let step = delta.normalize() * step_distance;
                enemy_transform.translation.x += step.x;
                enemy_transform.translation.y += step.y;
            }
        }
    }
}

pub fn enemy_engagement_range(
    melee_range: f32,
    ranged_profile: Option<&RangedAttackProfile>,
) -> f32 {
    match ranged_profile {
        Some(profile) if profile.range > melee_range && profile.damage > 0.0 => profile.range,
        _ => melee_range,
    }
}

#[derive(Clone, Copy, Debug)]
struct InsideEnemySample {
    entity: Entity,
    position: Vec2,
    distance_sq: f32,
}

#[allow(clippy::type_complexity)]
fn repel_enemy_overflow_from_formation(
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    recruits: Query<Entity, (With<FriendlyUnit>, Without<CommanderUnit>)>,
    mut enemies: Query<(Entity, &mut Transform), (With<EnemyUnit>, Without<CommanderUnit>)>,
) {
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let recruit_count = recruits.iter().count();
    if recruit_count == 0 {
        return;
    }
    let commander_position = commander_transform.translation.truncate();
    let formation_cfg = active_formation_config(&data, *active_formation);
    let slot_spacing = formation_cfg.slot_spacing;
    if slot_spacing <= 0.0 {
        return;
    }

    let inside_cap = max_inside_enemy_count_for_formation(recruit_count);
    let mut inside_samples = Vec::new();
    for (entity, transform) in &mut enemies {
        let enemy_position = transform.translation.truncate();
        if formation_contains_position(
            *active_formation,
            commander_position,
            enemy_position,
            recruit_count,
            slot_spacing,
            ENEMY_INSIDE_FORMATION_PADDING_SLOTS,
        ) {
            inside_samples.push(InsideEnemySample {
                entity,
                position: enemy_position,
                distance_sq: commander_position.distance_squared(enemy_position),
            });
        }
    }
    if inside_samples.len() <= inside_cap {
        return;
    }

    let distances: Vec<f32> = inside_samples
        .iter()
        .map(|sample| sample.distance_sq)
        .collect();
    let overflow_indices = overflow_indices_by_distance(&distances, inside_cap);
    for overflow_index in overflow_indices {
        let sample = inside_samples[overflow_index];
        let target = formation_overflow_repel_target(
            *active_formation,
            commander_position,
            sample.position,
            recruit_count,
            slot_spacing,
        );
        if let Ok((_, mut transform)) = enemies.get_mut(sample.entity) {
            transform.translation.x = target.x;
            transform.translation.y = target.y;
        }
    }
}

pub fn max_inside_enemy_count_for_formation(recruit_count: usize) -> usize {
    if recruit_count == 0 {
        return 0;
    }
    recruit_count / 4
}

pub fn overflow_indices_by_distance(distances_sq: &[f32], cap: usize) -> Vec<usize> {
    if distances_sq.len() <= cap {
        return Vec::new();
    }
    let mut sorted: Vec<(usize, f32)> = distances_sq.iter().copied().enumerate().collect();
    sorted.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let overflow_count = sorted.len().saturating_sub(cap);
    sorted
        .into_iter()
        .take(overflow_count)
        .map(|(index, _)| index)
        .collect()
}

pub fn formation_perimeter_target(
    active_formation: ActiveFormation,
    commander_position: Vec2,
    enemy_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
) -> Vec2 {
    if recruit_count == 0 || slot_spacing <= 0.0 {
        return enemy_position;
    }
    let half_extent = formation_half_extent(
        recruit_count,
        slot_spacing,
        ENEMY_INSIDE_FORMATION_PADDING_SLOTS,
    );
    let delta = enemy_position - commander_position;
    let local = match active_formation {
        ActiveFormation::Square => project_to_square_perimeter(delta, half_extent),
        ActiveFormation::Diamond => project_to_diamond_perimeter(delta, half_extent),
    };
    commander_position + local
}

fn formation_overflow_repel_target(
    active_formation: ActiveFormation,
    commander_position: Vec2,
    enemy_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
) -> Vec2 {
    if recruit_count == 0 || slot_spacing <= 0.0 {
        return enemy_position;
    }
    let half_extent = formation_half_extent(
        recruit_count,
        slot_spacing,
        ENEMY_INSIDE_FORMATION_PADDING_SLOTS + ENEMY_FORMATION_REPEL_MARGIN_SLOTS,
    );
    let delta = enemy_position - commander_position;
    let local = match active_formation {
        ActiveFormation::Square => project_to_square_perimeter(delta, half_extent),
        ActiveFormation::Diamond => project_to_diamond_perimeter(delta, half_extent),
    };
    commander_position + local
}

fn formation_half_extent(recruit_count: usize, slot_spacing: f32, padding_slots: f32) -> f32 {
    let side = ((recruit_count + 1) as f32).sqrt().ceil();
    ((side - 1.0) * 0.5 + padding_slots) * slot_spacing
}

fn project_to_square_perimeter(delta: Vec2, half_extent: f32) -> Vec2 {
    if half_extent <= 0.0 {
        return delta;
    }
    let dominant = delta.x.abs().max(delta.y.abs());
    if dominant <= f32::EPSILON {
        return Vec2::new(half_extent, 0.0);
    }
    delta * (half_extent / dominant)
}

fn project_to_diamond_perimeter(delta: Vec2, half_extent: f32) -> Vec2 {
    let diamond_radius = half_extent * std::f32::consts::SQRT_2;
    if diamond_radius <= 0.0 {
        return delta;
    }
    let l1 = delta.x.abs() + delta.y.abs();
    if l1 <= f32::EPSILON {
        return Vec2::new(diamond_radius, 0.0);
    }
    delta * (diamond_radius / l1)
}

#[allow(clippy::type_complexity)]
fn update_bandit_visual_states(
    art: Res<ArtAssets>,
    mut enemies: Query<
        (
            &Unit,
            &Health,
            &AttackProfile,
            &AttackCooldown,
            &Transform,
            &mut BanditVisualRuntime,
            &mut Handle<Image>,
        ),
        With<EnemyUnit>,
    >,
) {
    for (unit, health, attack, attack_cd, transform, mut runtime, mut texture) in &mut enemies {
        let position = transform.translation.truncate();
        let moved_distance_sq = runtime.last_position.distance_squared(position);
        let next_state = decide_bandit_visual_state(
            moved_distance_sq,
            attack_cd.0.elapsed_secs(),
            attack.cooldown_secs,
            health.current,
            health.max,
        );

        if runtime.state != next_state {
            *texture = enemy_texture_for_state(&art, unit.kind, next_state);
            runtime.state = next_state;
        }
        runtime.last_position = position;
    }
}

fn enemy_texture_for_state(
    art: &ArtAssets,
    kind: UnitKind,
    _state: BanditVisualState,
) -> Handle<Image> {
    enemy_texture_for_kind(art, kind)
}

fn enemy_texture_for_kind(art: &ArtAssets, kind: UnitKind) -> Handle<Image> {
    match kind {
        UnitKind::ChristianPeasantInfantry => art.friendly_peasant_infantry_idle.clone(),
        UnitKind::ChristianPeasantArcher => art.friendly_peasant_archer_idle.clone(),
        UnitKind::ChristianPeasantPriest => art.friendly_peasant_priest_idle.clone(),
        UnitKind::MuslimPeasantInfantry => art.muslim_peasant_infantry_idle.clone(),
        UnitKind::MuslimPeasantArcher => art.muslim_peasant_archer_idle.clone(),
        UnitKind::MuslimPeasantPriest => art.muslim_peasant_priest_idle.clone(),
        UnitKind::Commander
        | UnitKind::RescuableChristianPeasantInfantry
        | UnitKind::RescuableChristianPeasantArcher
        | UnitKind::RescuableChristianPeasantPriest
        | UnitKind::RescuableMuslimPeasantInfantry
        | UnitKind::RescuableMuslimPeasantArcher
        | UnitKind::RescuableMuslimPeasantPriest => art.enemy_bandit_raider_idle.clone(),
    }
}

pub fn decide_bandit_visual_state(
    moved_distance_sq: f32,
    cooldown_elapsed_secs: f32,
    attack_cooldown_secs: f32,
    current_hp: f32,
    max_hp: f32,
) -> BanditVisualState {
    if current_hp <= 0.0 {
        return BanditVisualState::Dead;
    }
    let hp_ratio = (current_hp / max_hp).clamp(0.0, 1.0);
    if hp_ratio <= 0.35 {
        return BanditVisualState::Hit;
    }
    let attack_window = (attack_cooldown_secs * 0.2).clamp(0.06, 0.2);
    if cooldown_elapsed_secs <= attack_window {
        return BanditVisualState::Attack;
    }
    if moved_distance_sq > 1.0 {
        BanditVisualState::Move
    } else {
        BanditVisualState::Idle
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::ai::{
        chase_step_distance, chase_target_positions, choose_nearest, should_move_towards_target,
    };
    use crate::combat::RangedAttackProfile;
    use crate::data::{WaveConfig, WavesConfigFile};
    use crate::enemies::{
        BanditVisualState, EnemyRoleCounts, EnemySpawnRole, batch_interval_secs,
        batch_size_for_wave, decide_bandit_visual_state, enemy_engagement_range, enemy_move_speed,
        enemy_player_pressure_multiplier, formation_overflow_repel_target,
        formation_perimeter_target, max_inside_enemy_count_for_formation,
        overflow_indices_by_distance, pick_next_spawn_role, random_spawn_position,
        units_per_second_for_wave, wave_duration_secs, wave_stat_multiplier,
    };
    use crate::formation::{ActiveFormation, formation_contains_position};
    use crate::map::MapBounds;

    #[test]
    fn chooses_nearest_target() {
        let origin = Vec2::new(0.0, 0.0);
        let targets = [
            Vec2::new(5.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(10.0, 0.0),
        ];
        let nearest = choose_nearest(origin, &targets).expect("target");
        assert_eq!(nearest, Vec2::new(2.0, 0.0));
    }

    #[test]
    fn no_targets_returns_none() {
        assert_eq!(choose_nearest(Vec2::ZERO, &[]), None);
    }

    #[test]
    fn bandit_visual_state_priority_is_dead_then_hit_then_attack_then_move_idle() {
        assert_eq!(
            decide_bandit_visual_state(0.0, 0.1, 1.0, 0.0, 10.0),
            BanditVisualState::Dead
        );
        assert_eq!(
            decide_bandit_visual_state(10.0, 0.1, 1.0, 3.0, 10.0),
            BanditVisualState::Hit
        );
        assert_eq!(
            decide_bandit_visual_state(10.0, 0.05, 1.0, 9.0, 10.0),
            BanditVisualState::Attack
        );
        assert_eq!(
            decide_bandit_visual_state(2.0, 0.8, 1.0, 9.0, 10.0),
            BanditVisualState::Move
        );
        assert_eq!(
            decide_bandit_visual_state(0.1, 0.8, 1.0, 9.0, 10.0),
            BanditVisualState::Idle
        );
    }

    #[test]
    fn wave_progression_scales_spawn_rate_and_stats() {
        let config = WavesConfigFile {
            waves: vec![
                WaveConfig {
                    time_secs: 0.0,
                    count: 8,
                },
                WaveConfig {
                    time_secs: 30.0,
                    count: 12,
                },
            ],
        };
        let first = units_per_second_for_wave(&config, 1);
        let second = units_per_second_for_wave(&config, 2);
        let late = units_per_second_for_wave(&config, 10);
        assert!(second >= first);
        assert!(late > second);
        assert_eq!(wave_duration_secs(), 30.0);
        assert!((wave_stat_multiplier(1) - 1.0).abs() < 0.001);
        assert!((wave_stat_multiplier(2) - 1.102).abs() < 0.001);
        assert!(wave_stat_multiplier(8) > wave_stat_multiplier(3));
        assert!(enemy_move_speed(100.0) < 100.0);
    }

    #[test]
    fn player_pressure_multiplier_increases_with_level_and_retinue() {
        let baseline = enemy_player_pressure_multiplier(1, 0);
        let mid = enemy_player_pressure_multiplier(30, 20);
        let late = enemy_player_pressure_multiplier(100, 120);
        assert!((baseline - 1.0).abs() < 0.001);
        assert!(mid > baseline);
        assert!(late > mid);
    }

    #[test]
    fn player_pressure_multiplier_outpaces_passive_level_scaling_with_retinue() {
        let commander_level: u32 = 60;
        let retinue_count: u32 = 40;
        let passive_friendly = 1.0 + commander_level.saturating_sub(1) as f32 * 0.01;
        let enemy_pressure = enemy_player_pressure_multiplier(commander_level, retinue_count);
        assert!(enemy_pressure > passive_friendly);
    }

    #[test]
    fn units_per_second_is_capped_to_thousand_enemies_per_wave() {
        let config = WavesConfigFile {
            waves: vec![WaveConfig {
                time_secs: 0.0,
                count: 700,
            }],
        };
        let rate = units_per_second_for_wave(&config, 1);
        assert!((rate - (1000.0 / 30.0)).abs() < 0.001);
    }

    #[test]
    fn wave_role_picker_enforces_support_cap_and_target_mix() {
        let mut counts = EnemyRoleCounts::default();
        for seed in 0..40 {
            let role = pick_next_spawn_role(counts, true, true, true, seed);
            counts.register(role);
            assert!(counts.support <= counts.total / 4);
        }
        assert_eq!(counts.total, 40);
        assert_eq!(counts.support, 10);
        assert_eq!(counts.melee + counts.ranged, 30);
        assert!((counts.melee as i32 - counts.ranged as i32).abs() <= 1);
    }

    #[test]
    fn wave_role_picker_falls_back_when_support_is_only_available_role() {
        let mut counts = EnemyRoleCounts::default();
        for seed in 0..7 {
            let role = pick_next_spawn_role(counts, false, false, true, seed);
            assert_eq!(role, EnemySpawnRole::Support);
            counts.register(role);
        }
        assert_eq!(counts.total, 7);
        assert_eq!(counts.support, 7);
        assert_eq!(counts.melee, 0);
        assert_eq!(counts.ranged, 0);
    }

    #[test]
    fn inside_formation_cap_scales_with_roster_size() {
        assert_eq!(max_inside_enemy_count_for_formation(0), 0);
        assert_eq!(max_inside_enemy_count_for_formation(3), 0);
        assert_eq!(max_inside_enemy_count_for_formation(4), 1);
        assert_eq!(max_inside_enemy_count_for_formation(40), 10);
        assert_eq!(max_inside_enemy_count_for_formation(180), 45);
    }

    #[test]
    fn overflow_selection_keeps_outermost_inside_formation_cap() {
        let overflow = overflow_indices_by_distance(&[16.0, 4.0, 9.0, 1.0, 25.0], 2);
        assert_eq!(overflow.len(), 3);
        assert!(overflow.contains(&1));
        assert!(overflow.contains(&2));
        assert!(overflow.contains(&3));
    }

    #[test]
    fn square_perimeter_target_projects_to_boundary() {
        let target = formation_perimeter_target(
            ActiveFormation::Square,
            Vec2::ZERO,
            Vec2::new(10.0, 5.0),
            16,
            30.0,
        );
        assert!(target.x.abs() > target.y.abs());
        assert!(target.x.abs() > 20.0);
    }

    #[test]
    fn diamond_perimeter_target_projects_to_boundary() {
        let target = formation_perimeter_target(
            ActiveFormation::Diamond,
            Vec2::ZERO,
            Vec2::new(7.0, 4.0),
            16,
            30.0,
        );
        let diamond_radius = ((16.0f32 + 1.0).sqrt().ceil() - 1.0) * 0.5 + 0.35;
        let expected = diamond_radius * 30.0 * std::f32::consts::SQRT_2;
        assert!(((target.x.abs() + target.y.abs()) - expected).abs() < 0.2);
    }

    #[test]
    fn overflow_repel_target_is_outside_formation_footprint() {
        let commander = Vec2::ZERO;
        let recruit_count = 40;
        let spacing = 30.0;
        let sample = Vec2::new(10.0, 10.0);

        let square_target = formation_overflow_repel_target(
            ActiveFormation::Square,
            commander,
            sample,
            recruit_count,
            spacing,
        );
        assert!(!formation_contains_position(
            ActiveFormation::Square,
            commander,
            square_target,
            recruit_count,
            spacing,
            0.35,
        ));

        let diamond_target = formation_overflow_repel_target(
            ActiveFormation::Diamond,
            commander,
            sample,
            recruit_count,
            spacing,
        );
        assert!(!formation_contains_position(
            ActiveFormation::Diamond,
            commander,
            diamond_target,
            recruit_count,
            spacing,
            0.35,
        ));
    }

    #[test]
    fn chase_targets_exclude_commander_when_retinue_exists() {
        let commander = (Vec2::new(0.0, 0.0), true);
        let retinue = (Vec2::new(10.0, 0.0), false);
        let targets = chase_target_positions(&[commander, retinue]);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], retinue.0);

        let commander_only = chase_target_positions(&[commander]);
        assert_eq!(commander_only, vec![commander.0]);
    }

    #[test]
    fn movement_hysteresis_prevents_stop_resume_jitter() {
        assert!(!should_move_towards_target(true, 20.0, 22.0, 26.0));
        assert!(!should_move_towards_target(false, 24.0, 22.0, 26.0));
        assert!(should_move_towards_target(false, 30.0, 22.0, 26.0));
    }

    #[test]
    fn chase_step_distance_prevents_overshoot_into_stop_range() {
        let step = chase_step_distance(24.0, 20.0, 10.0);
        assert!((step - 4.0).abs() < 0.001);
        assert_eq!(chase_step_distance(19.5, 20.0, 10.0), 0.0);
        assert_eq!(chase_step_distance(30.0, 20.0, 0.0), 0.0);
    }

    #[test]
    fn engagement_range_prefers_valid_ranged_profile() {
        let ranged = RangedAttackProfile {
            damage: 8.0,
            range: 260.0,
            projectile_speed: 200.0,
            projectile_max_distance: 300.0,
        };
        assert!((enemy_engagement_range(30.0, Some(&ranged)) - 260.0).abs() < 0.001);
        assert!((enemy_engagement_range(30.0, None) - 30.0).abs() < 0.001);
    }

    #[test]
    fn wave_batches_scale_size_and_reduce_interval_over_time() {
        assert!(batch_size_for_wave(0) >= 7);
        assert!(batch_size_for_wave(18) > batch_size_for_wave(2));
        assert!(batch_interval_secs(10) < batch_interval_secs(0));
    }

    #[test]
    fn random_spawn_positions_stay_inside_bounds_and_off_commander() {
        let bounds = MapBounds {
            half_width: 500.0,
            half_height: 350.0,
        };
        let commander = Vec2::ZERO;
        for sequence in 0..80 {
            let point = random_spawn_position(bounds, commander, 4, sequence);
            assert!(point.x >= -500.0 && point.x <= 500.0);
            assert!(point.y >= -350.0 && point.y <= 350.0);
            assert!(point.length() >= 170.0);
        }
    }
}
