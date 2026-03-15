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

pub struct RescuePlugin;

impl Plugin for RescuePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_rescuables_on_run_start)
            .add_systems(
                Update,
                tick_rescue_progress.run_if(in_state(GameState::InRun)),
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
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in existing_rescuables.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let max_radius = bounds
        .map(|b| b.half_width.min(b.half_height) * 0.8)
        .unwrap_or(800.0);
    let count = data.rescue.spawn_count.max(1);
    for idx in 0..count {
        let angle = idx as f32 / count as f32 * std::f32::consts::TAU;
        let radius = max_radius * 0.45 + (idx as f32 % 5.0) * 20.0;
        let position = Vec2::new(radius * angle.cos(), radius * angle.sin());
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
    use crate::rescue::advance_rescue_progress;

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
}
