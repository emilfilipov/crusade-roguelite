use bevy::prelude::*;

use crate::data::GameData;
use crate::map::MapBounds;
use crate::model::{
    FriendlyUnit, GameState, RecruitEvent, RecruitUnitKind, RescuableUnit, StartRunEvent, Team,
    Unit,
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
        let recruit_kind = recruit_kind_for_sequence(idx);
        spawn_rescuable(
            &mut commands,
            rescue_spawn_position(idx, bounds.as_deref().copied()),
            recruit_kind,
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
    let recruit_kind = recruit_kind_for_sequence(runtime.sequence);
    spawn_rescuable(&mut commands, spawn_position, recruit_kind, &art);
    runtime.sequence = runtime.sequence.saturating_add(1);
}

fn tick_rescue_progress(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    mut rescuables: Query<
        (Entity, &Transform, &RescuableUnit, &mut RescueProgress),
        With<RescuableUnit>,
    >,
    mut recruit_events: EventWriter<RecruitEvent>,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }
    let rescue_radius = data.rescue.rescue_radius;
    let rescue_duration = data.rescue.rescue_duration_secs;

    for (entity, transform, rescuable_unit, mut rescue_progress) in &mut rescuables {
        let in_range = any_friendly_in_rescue_radius(
            transform.translation.truncate(),
            &friendly_positions,
            rescue_radius,
        );
        rescue_progress.elapsed = advance_rescue_progress(
            rescue_progress.elapsed,
            in_range,
            time.delta_seconds(),
            rescue_duration,
        );
        if rescue_progress.elapsed >= rescue_duration {
            recruit_events.send(RecruitEvent {
                world_position: transform.translation.truncate(),
                recruit_kind: rescuable_unit.recruit_kind,
            });
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub fn any_friendly_in_rescue_radius(
    rescuable_position: Vec2,
    friendly_positions: &[Vec2],
    rescue_radius: f32,
) -> bool {
    let rescue_radius_sq = rescue_radius * rescue_radius;
    friendly_positions
        .iter()
        .any(|position| position.distance_squared(rescuable_position) <= rescue_radius_sq)
}

fn spawn_rescuable(
    commands: &mut Commands,
    position: Vec2,
    recruit_kind: RecruitUnitKind,
    art: &ArtAssets,
) {
    let (rescuable_unit_kind, texture, tint) = match recruit_kind {
        RecruitUnitKind::ChristianPeasantInfantry => (
            crate::model::UnitKind::RescuableChristianPeasantInfantry,
            art.friendly_peasant_infantry_rescuable_variant.clone(),
            Color::srgb(0.88, 0.92, 1.0),
        ),
        RecruitUnitKind::ChristianPeasantArcher => (
            crate::model::UnitKind::RescuableChristianPeasantArcher,
            art.friendly_peasant_archer_rescuable_variant.clone(),
            Color::srgb(0.86, 0.95, 0.86),
        ),
    };

    commands.spawn((
        Unit {
            team: Team::Neutral,
            kind: rescuable_unit_kind,
            level: 1,
        },
        RescuableUnit { recruit_kind },
        RescueProgress { elapsed: 0.0 },
        SpriteBundle {
            texture,
            sprite: Sprite {
                color: tint,
                custom_size: Some(Vec2::splat(32.0)),
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, 2.0),
            ..default()
        },
    ));
}

fn recruit_kind_for_sequence(sequence: u32) -> RecruitUnitKind {
    if sequence.is_multiple_of(2) {
        RecruitUnitKind::ChristianPeasantInfantry
    } else {
        RecruitUnitKind::ChristianPeasantArcher
    }
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
    use bevy::prelude::Vec2;

    use crate::map::MapBounds;
    use crate::model::RecruitUnitKind;
    use crate::rescue::{
        advance_rescue_progress, any_friendly_in_rescue_radius, recruit_kind_for_sequence,
        rescue_spawn_position,
    };

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

    #[test]
    fn any_friendly_in_range_allows_non_commander_rescue() {
        let friendlies = [Vec2::new(100.0, 40.0), Vec2::new(-35.0, 12.0)];
        assert!(any_friendly_in_rescue_radius(
            Vec2::new(102.0, 42.0),
            &friendlies,
            4.0
        ));
        assert!(!any_friendly_in_rescue_radius(
            Vec2::new(180.0, 180.0),
            &friendlies,
            12.0
        ));
    }

    #[test]
    fn rescue_spawn_sequence_alternates_recruit_kinds() {
        assert_eq!(
            recruit_kind_for_sequence(0),
            RecruitUnitKind::ChristianPeasantInfantry
        );
        assert_eq!(
            recruit_kind_for_sequence(1),
            RecruitUnitKind::ChristianPeasantArcher
        );
        assert_eq!(
            recruit_kind_for_sequence(2),
            RecruitUnitKind::ChristianPeasantInfantry
        );
    }
}
