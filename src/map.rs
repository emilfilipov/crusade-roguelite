use bevy::prelude::*;

use crate::data::GameData;
use crate::model::{CommanderUnit, GameState, Unit};
use crate::visuals::ArtAssets;

#[derive(Resource, Clone, Copy, Debug)]
pub struct MapBounds {
    pub half_width: f32,
    pub half_height: f32,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera_once)
            .add_systems(OnEnter(GameState::MainMenu), initialize_map_resources)
            .add_systems(OnEnter(GameState::MainMenu), spawn_background_visual)
            .add_systems(
                Update,
                (follow_camera_commander, snap_world_to_pixel_grid)
                    .chain()
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

fn snap_world_to_pixel_grid(
    mut units: Query<&mut Transform, With<Unit>>,
    mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Unit>)>,
) {
    for mut transform in &mut units {
        transform.translation.x = transform.translation.x.round();
        transform.translation.y = transform.translation.y.round();
    }
    for mut transform in &mut cameras {
        transform.translation.x = transform.translation.x.round();
        transform.translation.y = transform.translation.y.round();
    }
}
