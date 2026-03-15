use bevy::prelude::*;

use crate::data::GameData;
use crate::model::{FriendlyUnit, GameState, Health, StartRunEvent};

#[derive(Resource, Clone, Copy, Debug)]
pub struct MapBounds {
    pub half_width: f32,
    pub half_height: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct OasisZone {
    pub center: Vec2,
    pub radius: f32,
    pub heal_per_second: f32,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_once)
            .add_systems(OnEnter(GameState::MainMenu), initialize_map_resources)
            .add_systems(Update, handle_start_run_oasis)
            .add_systems(
                Update,
                heal_units_inside_oasis.run_if(in_state(GameState::InRun)),
            );
    }
}

fn spawn_camera_once(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn initialize_map_resources(mut commands: Commands, data: Res<GameData>) {
    commands.insert_resource(MapBounds {
        half_width: data.map.width * 0.5,
        half_height: data.map.height * 0.5,
    });
}

fn handle_start_run_oasis(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    data: Res<GameData>,
    existing_oasis: Query<Entity, With<OasisZone>>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in existing_oasis.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let center = Vec2::new(data.map.oasis_center[0], data.map.oasis_center[1]);
    commands.spawn(OasisZone {
        center,
        radius: data.map.oasis_radius,
        heal_per_second: data.map.oasis_heal_per_second,
    });
}

fn heal_units_inside_oasis(
    time: Res<Time>,
    oasis_query: Query<&OasisZone>,
    mut friendlies: Query<(&Transform, &mut Health), With<FriendlyUnit>>,
) {
    for oasis in oasis_query.iter() {
        for (transform, mut health) in &mut friendlies {
            let position = transform.translation.truncate();
            if is_inside_circle(position, oasis.center, oasis.radius) {
                health.current =
                    (health.current + oasis.heal_per_second * time.delta_seconds()).min(health.max);
            }
        }
    }
}

pub fn is_inside_circle(point: Vec2, center: Vec2, radius: f32) -> bool {
    point.distance_squared(center) <= radius * radius
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;

    use crate::map::is_inside_circle;

    #[test]
    fn detects_inside_oasis_zone() {
        assert!(is_inside_circle(Vec2::new(1.0, 1.0), Vec2::ZERO, 2.0));
        assert!(!is_inside_circle(Vec2::new(5.0, 0.0), Vec2::ZERO, 2.0));
    }
}
