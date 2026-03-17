use bevy::prelude::*;

use crate::data::GameData;
use crate::map::MapBounds;
use crate::model::{
    CommanderUnit, GameState, RecruitEvent, RescuableUnit, StartRunEvent, Team, Unit, UnitKind,
};
use crate::visuals::ArtAssets;

#[derive(Component, Clone, Copy, Debug)]
pub struct RescueProgress {
    pub elapsed: f32,
}

#[derive(Resource, Clone, Debug)]
struct RescueSpawnRuntime {
    timer: Timer,
    sequence: u32,
}

impl Default for RescueSpawnRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(12.0, TimerMode::Repeating),
            sequence: 0,
        }
    }
}

pub struct RescuePlugin;

impl Plugin for RescuePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RescueSpawnRuntime>()
            .add_systems(Update, spawn_rescuables_on_run_start)
            .add_systems(
                Update,
                (spawn_rescuables_over_time, tick_rescue_progress)
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn spawn_rescuables_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    existing_rescuables: Query<Entity, With<RescuableUnit>>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    mut spawn_runtime: ResMut<RescueSpawnRuntime>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in existing_rescuables.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let count = data.rescue.spawn_count.max(1);
    for idx in 0..count {
        spawn_rescuable(
            &mut commands,
            rescue_spawn_position(idx, bounds.as_deref().copied()),
            &art,
        );
    }

    spawn_runtime.sequence = count;
    spawn_runtime.timer = Timer::from_seconds(12.0, TimerMode::Repeating);
}

fn spawn_rescuables_over_time(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    rescuables: Query<Entity, With<RescuableUnit>>,
    mut runtime: ResMut<RescueSpawnRuntime>,
) {
    const MAX_ACTIVE_RESCUABLES: usize = 12;
    if rescuables.iter().count() >= MAX_ACTIVE_RESCUABLES {
        return;
    }

    runtime.timer.tick(time.delta());
    if !runtime.timer.just_finished() {
        return;
    }

    let spawn_position = rescue_spawn_position(runtime.sequence, bounds.as_deref().copied());
    spawn_rescuable(&mut commands, spawn_position, &art);
    runtime.sequence = runtime.sequence.saturating_add(1);
}

fn tick_rescue_progress(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut rescuables: Query<(Entity, &Transform, &mut RescueProgress), With<RescuableUnit>>,
    mut recruit_events: EventWriter<RecruitEvent>,
) {
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let commander_pos = commander_transform.translation.truncate();
    let rescue_radius = data.rescue.rescue_radius;
    let rescue_duration = data.rescue.rescue_duration_secs;

    for (entity, transform, mut rescue_progress) in &mut rescuables {
        let in_range = transform.translation.truncate().distance(commander_pos) <= rescue_radius;
        rescue_progress.elapsed = advance_rescue_progress(
            rescue_progress.elapsed,
            in_range,
            time.delta_seconds(),
            rescue_duration,
        );
        if rescue_progress.elapsed >= rescue_duration {
            recruit_events.send(RecruitEvent {
                world_position: transform.translation.truncate(),
            });
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_rescuable(commands: &mut Commands, position: Vec2, art: &ArtAssets) {
    commands.spawn((
        Unit {
            team: Team::Neutral,
            kind: UnitKind::RescuableInfantry,
            level: 1,
            morale_weight: 1.0,
        },
        RescuableUnit,
        RescueProgress { elapsed: 0.0 },
        SpriteBundle {
            texture: art.friendly_knight_rescuable_variant.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(32.0)),
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, 2.0),
            ..default()
        },
    ));
}

fn rescue_spawn_position(sequence: u32, bounds: Option<MapBounds>) -> Vec2 {
    let max_radius = bounds
        .map(|b| b.half_width.min(b.half_height) * 0.82)
        .unwrap_or(800.0);
    let ring_fraction = 0.35 + (sequence % 6) as f32 * 0.08;
    let radius = (max_radius * ring_fraction).min(max_radius);
    let angle = sequence as f32 * 2.399_963_1;
    Vec2::new(radius * angle.cos(), radius * angle.sin())
}

pub fn advance_rescue_progress(
    current: f32,
    in_range: bool,
    delta_seconds: f32,
    duration: f32,
) -> f32 {
    if in_range {
        (current + delta_seconds).min(duration)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use crate::map::MapBounds;
    use crate::rescue::{advance_rescue_progress, rescue_spawn_position};

    #[test]
    fn rescue_progress_advances_when_in_range() {
        let progress = advance_rescue_progress(1.0, true, 0.5, 2.5);
        assert!((progress - 1.5).abs() < 0.001);
    }

    #[test]
    fn rescue_progress_resets_out_of_range() {
        let progress = advance_rescue_progress(1.8, false, 0.1, 2.5);
        assert_eq!(progress, 0.0);
    }

    #[test]
    fn rescue_spawn_points_stay_within_map_radius() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 1000.0,
        };
        for sequence in 0..32 {
            let point = rescue_spawn_position(sequence, Some(bounds));
            assert!(point.length() <= 1000.0 * 0.82 + 0.01);
        }
    }
}
