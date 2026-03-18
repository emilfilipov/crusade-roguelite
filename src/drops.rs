use bevy::prelude::*;

use crate::data::GameData;
use crate::enemies::WaveRuntime;
use crate::map::MapBounds;
use crate::model::{
    CommanderUnit, FriendlyUnit, GainXpEvent, GameState, SpawnExpPackEvent, StartRunEvent,
};
use crate::upgrades::Progression;
use crate::visuals::ArtAssets;

const DROP_HOMING_SPEED: f32 = 340.0;
const DROP_CONSUME_RADIUS: f32 = 16.0;

#[derive(Component, Clone, Copy, Debug)]
pub struct ExpPack {
    pub xp_value: f32,
}

#[derive(Component, Clone, Copy, Debug)]
struct DropInTransitToCommander;

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
                (
                    spawn_exp_packs_over_time,
                    spawn_exp_packs_from_events,
                    pickup_exp_packs,
                    transit_drops_to_commander,
                )
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_exp_packs_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    progression: Option<Res<Progression>>,
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
    let wave_number = current_wave_number(waves.as_deref());
    let commander_level = current_commander_level(progression.as_deref());
    let xp_value = scaled_pack_xp(data.drops.xp_per_pack, wave_number, commander_level);
    for sequence in 0..initial_count {
        let position = drop_spawn_position(sequence, bounds.as_deref().copied());
        spawn_exp_pack(&mut commands, position, xp_value, &art);
    }

    runtime.sequence = initial_count;
    runtime.timer = Timer::from_seconds(data.drops.spawn_interval_secs, TimerMode::Repeating);
}

#[allow(clippy::too_many_arguments)]
fn spawn_exp_packs_over_time(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    progression: Option<Res<Progression>>,
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
    let wave_number = current_wave_number(waves.as_deref());
    let commander_level = current_commander_level(progression.as_deref());
    let xp_value = scaled_pack_xp(data.drops.xp_per_pack, wave_number, commander_level);
    spawn_exp_pack(&mut commands, position, xp_value, &art);
    runtime.sequence = runtime.sequence.saturating_add(1);
}

fn spawn_exp_packs_from_events(
    mut commands: Commands,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    progression: Option<Res<Progression>>,
    packs: Query<Entity, With<ExpPack>>,
    mut spawn_events: EventReader<SpawnExpPackEvent>,
) {
    if spawn_events.is_empty() {
        return;
    }
    let wave_number = current_wave_number(waves.as_deref());
    let commander_level = current_commander_level(progression.as_deref());
    let max_active = data.drops.max_active_packs as usize;
    let mut active_count = packs.iter().count();
    for event in spawn_events.read() {
        if active_count >= max_active {
            break;
        }
        let base_xp = event.xp_value_override.unwrap_or(data.drops.xp_per_pack);
        let xp_value = scaled_pack_xp(base_xp, wave_number, commander_level);
        spawn_exp_pack(&mut commands, event.world_position, xp_value, &art);
        active_count = active_count.saturating_add(1);
    }
}

#[allow(clippy::type_complexity)]
fn pickup_exp_packs(
    mut commands: Commands,
    data: Res<GameData>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    packs: Query<(Entity, &Transform), (With<ExpPack>, Without<DropInTransitToCommander>)>,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }

    let pickup_radius = data.drops.pickup_radius;
    for (entity, transform) in &packs {
        let pack_position = transform.translation.truncate();
        if any_friendly_in_pickup_radius(pack_position, &friendly_positions, pickup_radius) {
            commands.entity(entity).insert(DropInTransitToCommander);
        }
    }
}

#[allow(clippy::type_complexity)]
fn transit_drops_to_commander(
    mut commands: Commands,
    time: Res<Time>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut packs: Query<
        (Entity, &ExpPack, &mut Transform),
        (With<DropInTransitToCommander>, Without<CommanderUnit>),
    >,
    mut xp_events: EventWriter<GainXpEvent>,
) {
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let target = commander_transform.translation.truncate();
    let max_step = DROP_HOMING_SPEED * time.delta_seconds();

    for (entity, pack, mut transform) in &mut packs {
        let current = transform.translation.truncate();
        let next = step_towards_target(current, target, max_step);
        transform.translation.x = next.x;
        transform.translation.y = next.y;

        if reached_target(next, target, DROP_CONSUME_RADIUS) {
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

fn current_wave_number(waves: Option<&WaveRuntime>) -> u32 {
    let Some(runtime) = waves else {
        return 1;
    };
    (runtime.next_wave_index as u32 + runtime.infinite_wave_index).max(1)
}

fn current_commander_level(progression: Option<&Progression>) -> u32 {
    progression.map(|value| value.level.max(1)).unwrap_or(1)
}

pub fn scaled_pack_xp(base_xp: f32, wave_number: u32, commander_level: u32) -> f32 {
    let wave_scale = 1.0 + wave_number.saturating_sub(1) as f32 * 0.06;
    let level_scale = 1.0 + commander_level.saturating_sub(1) as f32 * 0.04;
    (base_xp * wave_scale * level_scale).max(1.0)
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

pub fn step_towards_target(current: Vec2, target: Vec2, max_step: f32) -> Vec2 {
    let delta = target - current;
    let distance = delta.length();
    if distance <= max_step || distance <= 0.0001 {
        target
    } else {
        current + delta / distance * max_step
    }
}

pub fn reached_target(position: Vec2, target: Vec2, radius: f32) -> bool {
    position.distance_squared(target) <= radius * radius
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::drops::{
        any_friendly_in_pickup_radius, drop_spawn_position, reached_target, scaled_pack_xp,
        step_towards_target,
    };
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

    #[test]
    fn xp_pack_scaling_increases_with_wave_and_level() {
        let base = 6.0;
        let early = scaled_pack_xp(base, 1, 1);
        let later_wave = scaled_pack_xp(base, 5, 1);
        let later_level = scaled_pack_xp(base, 1, 6);
        let both = scaled_pack_xp(base, 5, 6);

        assert!(later_wave > early);
        assert!(later_level > early);
        assert!(both > later_wave);
        assert!(both > later_level);
    }

    #[test]
    fn step_towards_target_never_overshoots() {
        let next = step_towards_target(Vec2::ZERO, Vec2::new(10.0, 0.0), 3.0);
        assert!((next.x - 3.0).abs() < 0.001);

        let arrive = step_towards_target(Vec2::new(9.0, 0.0), Vec2::new(10.0, 0.0), 3.0);
        assert_eq!(arrive, Vec2::new(10.0, 0.0));
    }

    #[test]
    fn reached_target_uses_radius() {
        assert!(reached_target(Vec2::new(1.0, 1.0), Vec2::ZERO, 2.0));
        assert!(!reached_target(Vec2::new(3.0, 0.0), Vec2::ZERO, 2.0));
    }
}
