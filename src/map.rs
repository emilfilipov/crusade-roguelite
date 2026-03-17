use bevy::prelude::*;

use crate::data::GameData;
use crate::model::{CommanderUnit, FriendlyUnit, GameState, Health, StartRunEvent};
use crate::visuals::ArtAssets;

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
            .add_systems(OnEnter(GameState::MainMenu), spawn_background_visual)
            .add_systems(Update, handle_start_run_oasis)
            .add_systems(
                Update,
                (heal_units_inside_oasis, follow_camera_commander)
                    .run_if(in_state(GameState::InRun)),
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

#[derive(Component)]
struct BackgroundVisual;

fn spawn_background_visual(
    mut commands: Commands,
    bounds: Option<Res<MapBounds>>,
    art: Res<ArtAssets>,
    existing: Query<Entity, With<BackgroundVisual>>,
) {
    for entity in existing.iter() {
        commands.entity(entity).despawn_recursive();
    }
    let Some(bounds) = bounds else {
        return;
    };
    commands.spawn((
        BackgroundVisual,
        SpriteBundle {
            texture: art.terrain_desert_base_tile_a.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::new(bounds.half_width * 2.0, bounds.half_height * 2.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, -10.0),
            ..default()
        },
    ));
}

fn handle_start_run_oasis(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
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
    commands.spawn((
        OasisZone {
            center,
            radius: data.map.oasis_radius,
            heal_per_second: data.map.oasis_heal_per_second,
        },
        SpriteBundle {
            texture: art.oasis_water_core.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(data.map.oasis_radius * 2.0)),
                ..default()
            },
            transform: Transform::from_xyz(center.x, center.y, 1.0),
            ..default()
        },
    ));
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

fn follow_camera_commander(
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<CommanderUnit>)>,
) {
    let Ok(commander) = commanders.get_single() else {
        return;
    };
    let commander_position = commander.translation;
    for mut camera in &mut cameras {
        camera.translation.x = commander_position.x;
        camera.translation.y = commander_position.y;
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
