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
                (follow_camera_commander, snap_camera_to_pixel_grid)
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

const FLOOR_TILE_WORLD_SIZE: f32 = 96.0;
const FOLIAGE_TILE_WORLD_SIZE: f32 = 72.0;

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
    commands
        .spawn((BackgroundVisual, SpatialBundle::default()))
        .with_children(|parent| {
            spawn_floor_tiles(parent, &bounds, &art);
            spawn_sparse_foliage(parent, &bounds, &art);
        });
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

fn snap_camera_to_pixel_grid(mut cameras: Query<&mut Transform, (With<Camera2d>, Without<Unit>)>) {
    for mut transform in &mut cameras {
        transform.translation.x = transform.translation.x.round();
        transform.translation.y = transform.translation.y.round();
    }
}

fn spawn_floor_tiles(parent: &mut ChildBuilder, bounds: &MapBounds, art: &ArtAssets) {
    let tiles_x = ((bounds.half_width * 2.0) / FLOOR_TILE_WORLD_SIZE).ceil() as i32;
    let tiles_y = ((bounds.half_height * 2.0) / FLOOR_TILE_WORLD_SIZE).ceil() as i32;
    let start_x = -bounds.half_width + FLOOR_TILE_WORLD_SIZE * 0.5;
    let start_y = -bounds.half_height + FLOOR_TILE_WORLD_SIZE * 0.5;

    for y in 0..tiles_y {
        for x in 0..tiles_x {
            let world_x = start_x + x as f32 * FLOOR_TILE_WORLD_SIZE;
            let world_y = start_y + y as f32 * FLOOR_TILE_WORLD_SIZE;
            parent.spawn(SpriteBundle {
                texture: art.terrain_desert_base_tile_a.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(FLOOR_TILE_WORLD_SIZE)),
                    ..default()
                },
                transform: Transform::from_xyz(world_x, world_y, -20.0),
                ..default()
            });
        }
    }
}

fn spawn_sparse_foliage(parent: &mut ChildBuilder, bounds: &MapBounds, art: &ArtAssets) {
    let tiles_x = ((bounds.half_width * 2.0) / FLOOR_TILE_WORLD_SIZE).ceil() as i32;
    let tiles_y = ((bounds.half_height * 2.0) / FLOOR_TILE_WORLD_SIZE).ceil() as i32;
    let start_x = -bounds.half_width + FLOOR_TILE_WORLD_SIZE * 0.5;
    let start_y = -bounds.half_height + FLOOR_TILE_WORLD_SIZE * 0.5;

    for y in 0..tiles_y {
        for x in 0..tiles_x {
            if !should_place_foliage_tile(x, y) {
                continue;
            }
            let jitter = tile_jitter(x, y, 10.0);
            let world_x = start_x + x as f32 * FLOOR_TILE_WORLD_SIZE + jitter.x;
            let world_y = start_y + y as f32 * FLOOR_TILE_WORLD_SIZE + jitter.y;
            parent.spawn(SpriteBundle {
                texture: art.terrain_desert_foliage_tile_a.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(FOLIAGE_TILE_WORLD_SIZE)),
                    color: Color::srgba(1.0, 1.0, 1.0, 0.75),
                    ..default()
                },
                transform: Transform::from_xyz(world_x, world_y, -19.0),
                ..default()
            });
        }
    }
}

pub fn should_place_foliage_tile(grid_x: i32, grid_y: i32) -> bool {
    pseudo_hash(grid_x, grid_y).is_multiple_of(17)
}

fn tile_jitter(grid_x: i32, grid_y: i32, max_offset: f32) -> Vec2 {
    let hash = pseudo_hash(grid_x, grid_y);
    let x_bits = (hash & 0xFF) as f32 / 255.0;
    let y_bits = ((hash >> 8) & 0xFF) as f32 / 255.0;
    Vec2::new(
        (x_bits - 0.5) * max_offset * 2.0,
        (y_bits - 0.5) * max_offset * 2.0,
    )
}

fn pseudo_hash(grid_x: i32, grid_y: i32) -> u32 {
    let x = grid_x as u32;
    let y = grid_y as u32;
    x.wrapping_mul(1_103_515_245)
        .wrapping_add(y.wrapping_mul(97_867_311))
        .rotate_left(11)
}

#[cfg(test)]
mod tests {
    use crate::map::should_place_foliage_tile;

    #[test]
    fn foliage_placement_is_stable_and_sparse() {
        assert_eq!(
            should_place_foliage_tile(10, 22),
            should_place_foliage_tile(10, 22)
        );
        let mut placed = 0;
        for y in 0..20 {
            for x in 0..20 {
                if should_place_foliage_tile(x, y) {
                    placed += 1;
                }
            }
        }
        assert!(placed > 0);
        assert!(placed < 40);
    }
}
