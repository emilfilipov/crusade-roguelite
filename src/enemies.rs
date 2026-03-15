use bevy::prelude::*;

use crate::data::GameData;
use crate::map::MapBounds;
use crate::model::{
    Armor, AttackCooldown, AttackProfile, EnemyUnit, FriendlyUnit, GameState, Health, MoveSpeed,
    StartRunEvent, Team, Unit, UnitKind,
};

#[derive(Resource, Clone, Debug, Default)]
pub struct WaveRuntime {
    pub elapsed: f32,
    pub next_wave_index: usize,
}

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveRuntime>()
            .add_systems(Update, reset_waves_on_run_start)
            .add_systems(
                Update,
                (spawn_waves, enemy_chase_targets).run_if(in_state(GameState::InRun)),
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
            bounds.as_deref().copied(),
            wave_runtime.next_wave_index,
        );
        wave_runtime.next_wave_index += 1;
    }
}

fn spawn_enemy_wave(
    commands: &mut Commands,
    count: u32,
    data: &GameData,
    bounds: Option<MapBounds>,
    wave_idx: usize,
) {
    let cfg = &data.enemies.infantry_melee;
    let radius = bounds
        .map(|b| b.half_width.max(b.half_height) * 0.9)
        .unwrap_or(900.0);
    for idx in 0..count {
        let angle = (idx as f32 / count as f32) * std::f32::consts::TAU + wave_idx as f32 * 0.21;
        let pos = Vec2::new(radius * angle.cos(), radius * angle.sin());
        commands.spawn((
            Unit {
                team: Team::Enemy,
                kind: UnitKind::EnemyInfantry,
                level: 1,
                morale_weight: 1.0,
            },
            EnemyUnit,
            Health::new(cfg.max_hp),
            Armor(cfg.armor),
            AttackProfile {
                damage: cfg.damage,
                range: cfg.attack_range,
                cooldown_secs: cfg.attack_cooldown_secs,
            },
            AttackCooldown(Timer::from_seconds(
                cfg.attack_cooldown_secs,
                TimerMode::Repeating,
            )),
            MoveSpeed(cfg.move_speed),
            Transform::from_xyz(pos.x, pos.y, 5.0),
            GlobalTransform::default(),
        ));
    }
}

#[allow(clippy::type_complexity)]
fn enemy_chase_targets(
    time: Res<Time>,
    mut enemies: Query<(&MoveSpeed, &mut Transform), (With<EnemyUnit>, Without<FriendlyUnit>)>,
    friendlies: Query<&Transform, (With<FriendlyUnit>, Without<EnemyUnit>)>,
) {
    let targets: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if targets.is_empty() {
        return;
    }

    for (move_speed, mut enemy_transform) in &mut enemies {
        let enemy_position = enemy_transform.translation.truncate();
        if let Some(target) = choose_nearest(enemy_position, &targets) {
            let delta = target - enemy_position;
            if delta.length_squared() > 0.0001 {
                let step = delta.normalize() * move_speed.0 * time.delta_seconds();
                enemy_transform.translation.x += step.x;
                enemy_transform.translation.y += step.y;
            }
        }
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

    use crate::enemies::choose_nearest;

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
}
