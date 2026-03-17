use bevy::prelude::*;

use crate::data::GameData;
use crate::map::MapBounds;
use crate::model::{
    Armor, AttackCooldown, AttackProfile, CommanderUnit, EnemyUnit, FriendlyUnit, GameState,
    Health, MoveSpeed, StartRunEvent, Team, Unit, UnitKind,
};
use crate::visuals::ArtAssets;

#[derive(Resource, Clone, Debug, Default)]
pub struct WaveRuntime {
    pub elapsed: f32,
    pub next_wave_index: usize,
    pub infinite_wave_index: u32,
    pub next_infinite_spawn_time: f32,
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

const ENEMY_BASE_SPEED_MULTIPLIER: f32 = 0.72;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveRuntime>()
            .add_systems(Update, reset_waves_on_run_start)
            .add_systems(
                Update,
                (
                    spawn_waves,
                    enemy_chase_targets,
                    update_bandit_visual_states,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
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
    *wave_runtime = WaveRuntime::default();
}

fn spawn_waves(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    mut wave_runtime: ResMut<WaveRuntime>,
) {
    wave_runtime.elapsed += time.delta_seconds();
    while let Some(next_wave) = data.waves.waves.get(wave_runtime.next_wave_index) {
        if wave_runtime.elapsed < next_wave.time_secs {
            break;
        }
        spawn_enemy_wave(
            &mut commands,
            next_wave.count,
            &data,
            &art,
            bounds.as_deref().copied(),
            wave_runtime.next_wave_index,
            1.0,
        );
        wave_runtime.next_wave_index += 1;
    }

    if wave_runtime.next_wave_index < data.waves.waves.len() {
        return;
    }

    if wave_runtime.next_infinite_spawn_time <= 0.0 {
        wave_runtime.next_infinite_spawn_time = data
            .waves
            .waves
            .last()
            .map(|wave| wave.time_secs)
            .unwrap_or(0.0)
            + infinite_wave_interval_secs();
    }

    let base_count = data.waves.waves.last().map(|wave| wave.count).unwrap_or(3);
    while wave_runtime.elapsed >= wave_runtime.next_infinite_spawn_time {
        let procedural_index = wave_runtime.infinite_wave_index;
        let count = infinite_wave_enemy_count(base_count, procedural_index);
        let stat_scale = infinite_wave_stat_multiplier(procedural_index);
        spawn_enemy_wave(
            &mut commands,
            count,
            &data,
            &art,
            bounds.as_deref().copied(),
            data.waves.waves.len() + procedural_index as usize,
            stat_scale,
        );
        wave_runtime.infinite_wave_index = wave_runtime.infinite_wave_index.saturating_add(1);
        wave_runtime.next_infinite_spawn_time += infinite_wave_interval_secs();
    }
}

fn spawn_enemy_wave(
    commands: &mut Commands,
    count: u32,
    data: &GameData,
    art: &ArtAssets,
    bounds: Option<MapBounds>,
    wave_idx: usize,
    stat_scale: f32,
) {
    let cfg = &data.enemies.bandit_raider;
    let radius = bounds
        .map(|b| b.half_width.max(b.half_height) * 0.9)
        .unwrap_or(900.0);
    let hp = cfg.max_hp * stat_scale;
    let armor = cfg.armor + (stat_scale - 1.0) * 2.0;
    let damage = cfg.damage * stat_scale;
    let attack_cooldown_secs = (cfg.attack_cooldown_secs / (1.0 + (stat_scale - 1.0) * 0.15))
        .clamp(0.2, cfg.attack_cooldown_secs);
    let move_speed = enemy_move_speed(cfg.move_speed);
    for idx in 0..count {
        let angle = (idx as f32 / count as f32) * std::f32::consts::TAU + wave_idx as f32 * 0.21;
        let pos = Vec2::new(radius * angle.cos(), radius * angle.sin());
        commands.spawn((
            Unit {
                team: Team::Enemy,
                kind: UnitKind::EnemyBanditRaider,
                level: 1,
                morale_weight: 1.0,
            },
            EnemyUnit,
            BanditVisualRuntime {
                last_position: pos,
                state: BanditVisualState::Idle,
            },
            Health::new(hp),
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

fn infinite_wave_interval_secs() -> f32 {
    10.0
}

pub fn enemy_move_speed(base_speed: f32) -> f32 {
    base_speed * ENEMY_BASE_SPEED_MULTIPLIER
}

pub fn infinite_wave_enemy_count(base_count: u32, procedural_wave_index: u32) -> u32 {
    base_count.saturating_add((procedural_wave_index.saturating_add(1)) * 2)
}

pub fn infinite_wave_stat_multiplier(procedural_wave_index: u32) -> f32 {
    1.0 + (procedural_wave_index.saturating_add(1)) as f32 * 0.08
}

#[allow(clippy::type_complexity)]
fn enemy_chase_targets(
    time: Res<Time>,
    mut enemies: Query<
        (&MoveSpeed, &AttackProfile, &mut Transform),
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

    for (move_speed, attack_profile, mut enemy_transform) in &mut enemies {
        let enemy_position = enemy_transform.translation.truncate();
        if let Some(target) = choose_nearest(enemy_position, &targets) {
            let delta = target - enemy_position;
            let dist_sq = delta.length_squared();
            let stop_distance = (attack_profile.range * 0.85).max(10.0);
            if dist_sq > stop_distance * stop_distance {
                let step = delta.normalize() * move_speed.0 * time.delta_seconds();
                enemy_transform.translation.x += step.x;
                enemy_transform.translation.y += step.y;
            }
        }
    }
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

    use crate::enemies::{
        BanditVisualState, chase_target_positions, choose_nearest, decide_bandit_visual_state,
        enemy_move_speed, infinite_wave_enemy_count, infinite_wave_stat_multiplier,
    };

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
    fn infinite_wave_progression_scales_count_and_stats() {
        assert_eq!(infinite_wave_enemy_count(6, 0), 8);
        assert_eq!(infinite_wave_enemy_count(6, 4), 16);
        assert!((infinite_wave_stat_multiplier(0) - 1.08).abs() < 0.001);
        assert!(infinite_wave_stat_multiplier(5) > infinite_wave_stat_multiplier(2));
        assert!(enemy_move_speed(100.0) < 100.0);
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
}
