use bevy::prelude::*;

use crate::data::GameData;
use crate::map::MapBounds;
use crate::model::{FriendlyUnit, GainXpEvent, GameState, StartRunEvent};
use crate::visuals::ArtAssets;

#[derive(Component, Clone, Copy, Debug)]
pub struct ExpPack {
    pub xp_value: f32,
}

#[derive(Resource, Clone, Debug)]
struct DropSpawnRuntime {
    timer: Timer,
    sequence: u32,
}

impl Default for DropSpawnRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.5, TimerMode::Repeating),
            sequence: 0,
        }
    }
}

pub struct DropsPlugin;

impl Plugin for DropsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DropSpawnRuntime>()
            .add_systems(Update, spawn_exp_packs_on_run_start)
            .add_systems(
                Update,
                (spawn_exp_packs_over_time, pickup_exp_packs).run_if(in_state(GameState::InRun)),
            );
    }
}

fn spawn_exp_packs_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    existing_packs: Query<Entity, With<ExpPack>>,
    mut runtime: ResMut<DropSpawnRuntime>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in &existing_packs {
        commands.entity(entity).despawn_recursive();
    }

    let initial_count = data.drops.initial_spawn_count.max(1);
    for sequence in 0..initial_count {
        let position = drop_spawn_position(sequence, bounds.as_deref().copied());
        spawn_exp_pack(&mut commands, position, data.drops.xp_per_pack, &art);
    }

    runtime.sequence = initial_count;
    runtime.timer = Timer::from_seconds(data.drops.spawn_interval_secs, TimerMode::Repeating);
}

fn spawn_exp_packs_over_time(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    packs: Query<Entity, With<ExpPack>>,
    mut runtime: ResMut<DropSpawnRuntime>,
) {
    let max_active = data.drops.max_active_packs as usize;
    if packs.iter().count() >= max_active {
        return;
    }

    runtime.timer.tick(time.delta());
    if !runtime.timer.just_finished() {
        return;
    }

    let position = drop_spawn_position(runtime.sequence, bounds.as_deref().copied());
    spawn_exp_pack(&mut commands, position, data.drops.xp_per_pack, &art);
    runtime.sequence = runtime.sequence.saturating_add(1);
}

fn pickup_exp_packs(
    mut commands: Commands,
    data: Res<GameData>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    packs: Query<(Entity, &Transform, &ExpPack)>,
    mut xp_events: EventWriter<GainXpEvent>,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }

    let pickup_radius = data.drops.pickup_radius;
    for (entity, transform, pack) in &packs {
        let pack_position = transform.translation.truncate();
        if any_friendly_in_pickup_radius(pack_position, &friendly_positions, pickup_radius) {
            xp_events.send(GainXpEvent(pack.xp_value));
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_exp_pack(commands: &mut Commands, position: Vec2, xp_value: f32, art: &ArtAssets) {
    commands.spawn((
        ExpPack { xp_value },
        SpriteBundle {
            texture: art.exp_pack_coin_stack.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(18.0, 18.0)),
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, 4.0),
            ..default()
        },
    ));
}

fn drop_spawn_position(sequence: u32, bounds: Option<MapBounds>) -> Vec2 {
    let max_radius = bounds
        .map(|b| b.half_width.min(b.half_height) * 0.86)
        .unwrap_or(820.0);
    let min_radius = max_radius * 0.12;
    let ring_fraction = 0.2 + (sequence % 9) as f32 * 0.08;
    let radius = min_radius + (max_radius - min_radius) * ring_fraction.clamp(0.2, 0.92);
    let angle = sequence as f32 * 2.399_963_1 + 0.75;
    Vec2::new(radius * angle.cos(), radius * angle.sin())
}

pub fn any_friendly_in_pickup_radius(
    drop_position: Vec2,
    friendly_positions: &[Vec2],
    pickup_radius: f32,
) -> bool {
    let pickup_radius_sq = pickup_radius * pickup_radius;
    friendly_positions
        .iter()
        .any(|position| position.distance_squared(drop_position) <= pickup_radius_sq)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::drops::{any_friendly_in_pickup_radius, drop_spawn_position};
    use crate::map::MapBounds;

    #[test]
    fn drop_spawn_points_stay_inside_expected_radius() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 900.0,
        };
        for sequence in 0..48 {
            let point = drop_spawn_position(sequence, Some(bounds));
            assert!(point.length() <= 900.0 * 0.86 + 0.01);
        }
    }

    #[test]
    fn pickup_radius_detects_nearby_friendly() {
        let friendly_positions = [Vec2::new(10.0, 10.0), Vec2::new(120.0, 40.0)];
        assert!(any_friendly_in_pickup_radius(
            Vec2::new(12.0, 9.0),
            &friendly_positions,
            5.0
        ));
        assert!(!any_friendly_in_pickup_radius(
            Vec2::new(40.0, 40.0),
            &friendly_positions,
            10.0
        ));
    }
}
