use bevy::prelude::*;
use bevy::window::PrimaryWindow;

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
                (
                    confine_units_to_playable_bounds,
                    follow_camera_commander,
                    snap_camera_to_pixel_grid,
                )
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
const WALL_TILE_WORLD_SIZE: f32 = 56.0;
pub const MAP_WALL_INSET: f32 = 56.0;

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
            spawn_perimeter_walls(parent, &bounds, &art);
        });
}

#[allow(clippy::type_complexity)]
fn follow_camera_commander(
    windows: Query<&Window, With<PrimaryWindow>>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut cameras: Query<
        (&mut Transform, &OrthographicProjection),
        (With<Camera2d>, Without<CommanderUnit>),
    >,
) {
    let Ok(commander) = commanders.get_single() else {
        return;
    };
    let commander_position = commander.translation.truncate();
    for (mut camera, projection) in &mut cameras {
        let mut target = commander_position;
        if let (Some(map_bounds), Ok(window)) = (&bounds, windows.get_single()) {
            let half_view = camera_half_view_world(window, projection);
            target = clamped_camera_target(commander_position, **map_bounds, half_view);
        }
        camera.translation.x = target.x;
        camera.translation.y = target.y;
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
                    color: Color::srgba(1.0, 1.0, 1.0, 0.45),
                    ..default()
                },
                transform: Transform::from_xyz(world_x, world_y, -19.0),
                ..default()
            });
        }
    }
}

fn spawn_perimeter_walls(parent: &mut ChildBuilder, bounds: &MapBounds, art: &ArtAssets) {
    let horizontal_tiles = ((bounds.half_width * 2.0) / WALL_TILE_WORLD_SIZE).ceil() as i32 + 2;
    let vertical_tiles = ((bounds.half_height * 2.0) / WALL_TILE_WORLD_SIZE).ceil() as i32 + 2;
    let start_x = -bounds.half_width - WALL_TILE_WORLD_SIZE * 0.5;
    let start_y = -bounds.half_height - WALL_TILE_WORLD_SIZE * 0.5;
    let top_y = bounds.half_height - MAP_WALL_INSET * 0.5;
    let bottom_y = -bounds.half_height + MAP_WALL_INSET * 0.5;
    let left_x = -bounds.half_width + MAP_WALL_INSET * 0.5;
    let right_x = bounds.half_width - MAP_WALL_INSET * 0.5;

    for x in 0..horizontal_tiles {
        let world_x = start_x + x as f32 * WALL_TILE_WORLD_SIZE;
        spawn_wall_tile(
            parent,
            &art.terrain_boundary_wall_tile_a,
            Vec2::new(world_x, top_y),
        );
        spawn_wall_tile(
            parent,
            &art.terrain_boundary_wall_tile_a,
            Vec2::new(world_x, bottom_y),
        );
    }

    for y in 0..vertical_tiles {
        let world_y = start_y + y as f32 * WALL_TILE_WORLD_SIZE;
        spawn_wall_tile(
            parent,
            &art.terrain_boundary_wall_tile_a,
            Vec2::new(left_x, world_y),
        );
        spawn_wall_tile(
            parent,
            &art.terrain_boundary_wall_tile_a,
            Vec2::new(right_x, world_y),
        );
    }
}

fn spawn_wall_tile(parent: &mut ChildBuilder, texture: &Handle<Image>, position: Vec2) {
    parent.spawn(SpriteBundle {
        texture: texture.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::splat(WALL_TILE_WORLD_SIZE)),
            color: Color::srgba(0.95, 0.95, 0.95, 0.95),
            ..default()
        },
        transform: Transform::from_xyz(position.x, position.y, -18.0),
        ..default()
    });
}

fn confine_units_to_playable_bounds(
    bounds: Option<Res<MapBounds>>,
    mut units: Query<&mut Transform, With<Unit>>,
) {
    let Some(bounds) = bounds else {
        return;
    };
    let playable = playable_bounds(*bounds);
    for mut transform in &mut units {
        transform.translation.x = transform
            .translation
            .x
            .clamp(-playable.half_width, playable.half_width);
        transform.translation.y = transform
            .translation
            .y
            .clamp(-playable.half_height, playable.half_height);
    }
}

pub fn playable_bounds(bounds: MapBounds) -> MapBounds {
    MapBounds {
        half_width: (bounds.half_width - MAP_WALL_INSET).max(0.0),
        half_height: (bounds.half_height - MAP_WALL_INSET).max(0.0),
    }
}

fn camera_half_view_world(window: &Window, projection: &OrthographicProjection) -> Vec2 {
    let scale = projection.scale.max(0.01);
    Vec2::new(
        window.resolution.width() * 0.5 * scale,
        window.resolution.height() * 0.5 * scale,
    )
}

pub fn clamped_camera_target(command_pos: Vec2, bounds: MapBounds, half_view: Vec2) -> Vec2 {
    let playable = playable_bounds(bounds);
    let max_x = (playable.half_width - half_view.x).max(0.0);
    let max_y = (playable.half_height - half_view.y).max(0.0);
    Vec2::new(
        command_pos.x.clamp(-max_x, max_x),
        command_pos.y.clamp(-max_y, max_y),
    )
}

pub fn should_place_foliage_tile(grid_x: i32, grid_y: i32) -> bool {
    pseudo_hash(grid_x, grid_y).is_multiple_of(29)
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
    use bevy::prelude::Vec2;

    use crate::map::{
        MapBounds, clamped_camera_target, playable_bounds, should_place_foliage_tile,
    };

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

    #[test]
    fn playable_bounds_apply_wall_inset() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 900.0,
        };
        let playable = playable_bounds(bounds);
        assert!(playable.half_width < bounds.half_width);
        assert!(playable.half_height < bounds.half_height);
    }

    #[test]
    fn camera_target_is_clamped_to_visible_world() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 900.0,
        };
        let half_view = Vec2::new(640.0, 360.0);
        let target = clamped_camera_target(Vec2::new(9999.0, -9999.0), bounds, half_view);
        assert!(target.x <= 1200.0);
        assert!(target.y >= -900.0);
    }
}
