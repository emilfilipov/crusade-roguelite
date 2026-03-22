use bevy::prelude::*;

use crate::data::{GameData, WavesConfigFile};
use crate::formation::{ActiveFormation, active_formation_config, formation_contains_position};
use crate::map::{MapBounds, playable_bounds};
use crate::model::{
    Armor, AttackCooldown, AttackProfile, ColliderRadius, CommanderUnit, EnemyUnit, FriendlyUnit,
    GameState, Health, Morale, MoveSpeed, StartRunEvent, Team, Unit, UnitCohesion, UnitKind,
};
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
    pub spawn_sequence: u32,
}

#[derive(Clone, Debug)]
pub struct PendingEnemyBatch {
    pub remaining: u32,
    pub wave_number: u32,
    pub stat_scale: f32,
    pub next_spawn_time: f32,
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
const WAVE_STAT_GROWTH_PER_WAVE: f32 = 0.092;
const WAVE_BATCH_SIZE: u32 = 7;
const WAVE_BATCH_INTERVAL_SECS: f32 = 0.7;
const ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER: f32 = 200.0;
const ENEMY_SPAWN_ATTEMPTS: u32 = 8;
const DEFAULT_SPAWN_HALF_WIDTH: f32 = 900.0;
const DEFAULT_SPAWN_HALF_HEIGHT: f32 = 700.0;

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

fn spawn_pending_enemy_batches(
    mut commands: Commands,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut wave_runtime: ResMut<WaveRuntime>,
) {
    if wave_runtime.pending_batches.is_empty() {
        return;
    }

    let spawn_bounds = bounds
        .as_deref()
        .copied()
        .map(playable_bounds)
        .unwrap_or_else(default_spawn_bounds);
    let commander_position = commanders
        .get_single()
        .map(|transform| transform.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    let current_time = wave_runtime.elapsed;
    let mut spawn_sequence = wave_runtime.spawn_sequence;
    let mut remaining = Vec::with_capacity(wave_runtime.pending_batches.len());
    for mut batch in wave_runtime.pending_batches.drain(..) {
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
            spawn_bounds,
            commander_position,
            batch.wave_number,
            batch.stat_scale,
            &mut spawn_sequence,
        );
        batch.remaining = batch.remaining.saturating_sub(spawn_now);
        if batch.remaining > 0 {
            batch.next_spawn_time = current_time + batch_interval_secs(batch.wave_number);
            remaining.push(batch);
        }
    }
    wave_runtime.spawn_sequence = spawn_sequence;
    wave_runtime.pending_batches = remaining;
}

#[allow(clippy::too_many_arguments)]
fn spawn_enemy_batch(
    commands: &mut Commands,
    count: u32,
    data: &GameData,
    art: &ArtAssets,
    bounds: MapBounds,
    commander_position: Vec2,
    wave_number: u32,
    stat_scale: f32,
    spawn_sequence: &mut u32,
) {
    let cfg = &data.enemies.bandit_raider;
    let hp = cfg.max_hp * stat_scale;
    let armor = cfg.armor + (stat_scale - 1.0) * 2.0;
    let damage = cfg.damage * stat_scale;
    let attack_cooldown_secs = (cfg.attack_cooldown_secs / (1.0 + (stat_scale - 1.0) * 0.15))
        .clamp(0.2, cfg.attack_cooldown_secs);
    let move_speed = enemy_move_speed(cfg.move_speed);
    for _ in 0..count {
        let seq = *spawn_sequence;
        *spawn_sequence = spawn_sequence.saturating_add(1);
        let pos = random_spawn_position(bounds, commander_position, wave_number, seq);
        commands.spawn((
            Unit {
                team: Team::Enemy,
                kind: UnitKind::EnemyBanditRaider,
                level: 1,
            },
            EnemyUnit,
            BanditVisualRuntime {
                last_position: pos,
                state: BanditVisualState::Idle,
            },
            EnemyMovementState { moving: true },
            Health::new(hp),
            Morale::new(cfg.morale),
            UnitCohesion::new(cfg.cohesion),
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
                texture: art.enemy_bandit_raider_idle.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(32.0)),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 5.0),
                ..default()
            },
        ));
    }
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
    mut enemies: Query<
        (
            &MoveSpeed,
            &AttackProfile,
            &mut EnemyMovementState,
            &mut Transform,
        ),
        (With<EnemyUnit>, Without<FriendlyUnit>),
    >,
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

    for (move_speed, attack_profile, mut movement_state, mut enemy_transform) in &mut enemies {
        let enemy_position = enemy_transform.translation.truncate();
        if let Some(target) = choose_nearest(enemy_position, &targets) {
            let delta = target - enemy_position;
            let distance = delta.length();
            let stop_distance = (attack_profile.range * STOP_FACTOR).max(10.0);
            let resume_distance = (attack_profile.range * RESUME_FACTOR).max(stop_distance + 3.0);

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

pub fn should_move_towards_target(
    was_moving: bool,
    distance_to_target: f32,
    stop_distance: f32,
    resume_distance: f32,
) -> bool {
    if was_moving {
        return distance_to_target > stop_distance;
    }
    distance_to_target > resume_distance
}

pub fn chase_step_distance(distance_to_target: f32, stop_distance: f32, max_step: f32) -> f32 {
    if max_step <= 0.0 {
        return 0.0;
    }
    (distance_to_target - stop_distance).max(0.0).min(max_step)
}

pub fn chase_target_positions(all_friendlies: &[(Vec2, bool)]) -> Vec<Vec2> {
    if all_friendlies.is_empty() {
        return Vec::new();
    }
    let has_retinue = all_friendlies.iter().any(|(_, is_commander)| !is_commander);
    all_friendlies
        .iter()
        .filter_map(|(position, is_commander)| {
            if has_retinue && *is_commander {
                None
            } else {
                Some(*position)
            }
        })
        .collect()
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
        if unit.kind != UnitKind::EnemyBanditRaider {
            continue;
        }

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
            *texture = bandit_texture_for_state(&art, next_state);
            runtime.state = next_state;
        }
        runtime.last_position = position;
    }
}

fn bandit_texture_for_state(art: &ArtAssets, state: BanditVisualState) -> Handle<Image> {
    match state {
        BanditVisualState::Idle => art.enemy_bandit_raider_idle.clone(),
        BanditVisualState::Move => art.enemy_bandit_raider_move.clone(),
        BanditVisualState::Attack => art.enemy_bandit_raider_attack.clone(),
        BanditVisualState::Hit => art.enemy_bandit_raider_hit.clone(),
        BanditVisualState::Dead => art.enemy_bandit_raider_dead.clone(),
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

pub fn choose_nearest(origin: Vec2, candidates: &[Vec2]) -> Option<Vec2> {
    candidates.iter().copied().min_by(|a, b| {
        let da = origin.distance_squared(*a);
        let db = origin.distance_squared(*b);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::data::{WaveConfig, WavesConfigFile};
    use crate::enemies::{
        BanditVisualState, batch_interval_secs, batch_size_for_wave, chase_step_distance,
        chase_target_positions, choose_nearest, decide_bandit_visual_state, enemy_move_speed,
        formation_overflow_repel_target, formation_perimeter_target,
        max_inside_enemy_count_for_formation, overflow_indices_by_distance, random_spawn_position,
        should_move_towards_target, units_per_second_for_wave, wave_duration_secs,
        wave_stat_multiplier,
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
        assert!((wave_stat_multiplier(2) - 1.092).abs() < 0.001);
        assert!(wave_stat_multiplier(8) > wave_stat_multiplier(3));
        assert!(enemy_move_speed(100.0) < 100.0);
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
