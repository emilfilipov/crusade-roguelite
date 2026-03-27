use bevy::prelude::*;
use crate::random::runtime_entropy_seed_u32;

use crate::data::GameData;
use crate::enemies::WaveRuntime;
use crate::inventory::{
    EquipmentChestState, InventoryRngState, ItemRarityRollBonus, roll_chest_items,
};
use crate::map::MapBounds;
use crate::model::{
    CommanderUnit, FriendlyUnit, GainXpEvent, GameState, GlobalBuffs, MatchSetupSelection,
    MoveSpeed, PlayerFaction, RunModalAction, RunModalRequestEvent, RunModalScreen,
    SpawnExpPackEvent, StartRunEvent,
};
use crate::upgrades::Progression;
use crate::visuals::ArtAssets;

const DROP_HOMING_SPEED_MULTIPLIER: f32 = 1.2;
const DROP_CONSUME_RADIUS: f32 = 16.0;
const AMBIENT_PICKUP_DELAY_SECS: f32 = 0.0;
const DROP_RENDER_SIZE: f32 = 16.0;
const DROP_RENDER_Z: f32 = 40.0;
const MAGNET_PICKUP_SIZE: f32 = 30.0;
const MAGNET_PICKUP_Z: f32 = 42.0;
const MAGNET_PICKUP_WAVE_INTERVAL: u32 = 3;
const CHEST_PICKUP_SIZE: f32 = 34.0;
const CHEST_PICKUP_Z: f32 = 43.0;
const CHEST_PICKUP_DELAY_SECS: f32 = 0.9;
const CHEST_PICKUP_CHANNEL_SECS: f32 = 2.0;
const CHEST_PICKUP_WAVE_INTERVAL: u32 = 3;

#[derive(Component, Clone, Copy, Debug)]
pub struct ExpPack {
    pub xp_value: f32,
    pub pickup_delay_remaining: f32,
}

#[derive(Component, Clone, Copy, Debug)]
struct DropInTransitToCommander;

#[derive(Component, Clone, Copy, Debug)]
pub struct MagnetPickup {
    pub faction: PlayerFaction,
    pub wave: u32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct EquipmentChestDrop {
    pub wave: u32,
    pub pickup_delay_remaining: f32,
    pub pickup_progress: f32,
}

#[derive(Resource, Clone, Debug)]
struct DropSpawnRuntime {
    timer: Timer,
    rng_state: u64,
}

impl Default for DropSpawnRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.5, TimerMode::Repeating),
            rng_state: 0xD0C5_AA11_BA77_5EED,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct MagnetWaveRuntime {
    last_seen_wave: u32,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct ChestWaveRuntime {
    last_seen_wave: u32,
    rng_state: u64,
}

pub struct DropsPlugin;

impl Plugin for DropsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DropSpawnRuntime>()
            .init_resource::<MagnetWaveRuntime>()
            .init_resource::<ChestWaveRuntime>()
            .add_systems(Update, spawn_exp_packs_on_run_start)
            .add_systems(
                Update,
                (
                    spawn_exp_packs_over_time,
                    spawn_exp_packs_from_events,
                    update_wave_magnet_pickup,
                    update_wave_chest_drop,
                    pickup_wave_magnet,
                    pickup_equipment_chest,
                    pickup_exp_packs,
                    transit_drops_to_commander,
                )
                    .chain()
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
    commanders: Query<&Transform, With<CommanderUnit>>,
    existing_packs: Query<Entity, With<ExpPack>>,
    existing_magnets: Query<Entity, With<MagnetPickup>>,
    existing_chests: Query<Entity, With<EquipmentChestDrop>>,
    mut runtime: ResMut<DropSpawnRuntime>,
    mut magnet_runtime: ResMut<MagnetWaveRuntime>,
    mut chest_runtime: ResMut<ChestWaveRuntime>,
    mut chest_state: ResMut<EquipmentChestState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in &existing_packs {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &existing_magnets {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &existing_chests {
        commands.entity(entity).despawn_recursive();
    }
    chest_state.clear();
    let runtime_seed = runtime_seed_from_time();
    runtime.rng_state = rng_state_from_seed(runtime_seed);
    chest_runtime.rng_state = rng_state_from_seed(runtime_seed ^ 0xA5A5_3C3C);

    let initial_count = data.drops.initial_spawn_count.max(1);
    let wave_number = current_wave_number(waves.as_deref());
    let commander_level = current_commander_level(progression.as_deref());
    let xp_value = scaled_pack_xp(data.drops.xp_per_pack, wave_number, commander_level);
    let center = commander_spawn_center(&commanders);
    for _ in 0..initial_count {
        let position =
            drop_spawn_position(&mut runtime.rng_state, bounds.as_deref().copied(), center);
        spawn_exp_pack(
            &mut commands,
            position,
            xp_value,
            AMBIENT_PICKUP_DELAY_SECS,
            &art,
        );
    }

    runtime.timer = Timer::from_seconds(data.drops.spawn_interval_secs, TimerMode::Repeating);
    magnet_runtime.last_seen_wave = 0;
    chest_runtime.last_seen_wave = 0;
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
    commanders: Query<&Transform, With<CommanderUnit>>,
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

    let center = commander_spawn_center(&commanders);
    let position = drop_spawn_position(&mut runtime.rng_state, bounds.as_deref().copied(), center);
    let wave_number = current_wave_number(waves.as_deref());
    let commander_level = current_commander_level(progression.as_deref());
    let xp_value = scaled_pack_xp(data.drops.xp_per_pack, wave_number, commander_level);
    spawn_exp_pack(
        &mut commands,
        position,
        xp_value,
        AMBIENT_PICKUP_DELAY_SECS,
        &art,
    );
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
        let pickup_delay = event
            .pickup_delay_secs
            .unwrap_or(AMBIENT_PICKUP_DELAY_SECS)
            .max(0.0);
        spawn_exp_pack(
            &mut commands,
            event.world_position,
            xp_value,
            pickup_delay,
            &art,
        );
        active_count = active_count.saturating_add(1);
    }
}

#[allow(clippy::type_complexity)]
fn update_wave_magnet_pickup(
    mut commands: Commands,
    art: Res<ArtAssets>,
    setup: Res<MatchSetupSelection>,
    waves: Option<Res<WaveRuntime>>,
    mut runtime: ResMut<MagnetWaveRuntime>,
    existing_magnets: Query<Entity, With<MagnetPickup>>,
) {
    let current_wave = current_wave_number(waves.as_deref());
    let (wave_changed, should_spawn) = magnet_wave_lifecycle(runtime.last_seen_wave, current_wave);
    if !wave_changed {
        return;
    }

    for entity in &existing_magnets {
        commands.entity(entity).despawn_recursive();
    }

    if should_spawn {
        spawn_wave_magnet(&mut commands, &art, setup.faction, current_wave);
    }
    runtime.last_seen_wave = current_wave;
}

#[allow(clippy::type_complexity)]
fn update_wave_chest_drop(
    mut commands: Commands,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    existing_chests: Query<Entity, With<EquipmentChestDrop>>,
    mut runtime: ResMut<ChestWaveRuntime>,
) {
    let current_wave = current_wave_number(waves.as_deref());
    let (wave_changed, should_spawn) = chest_wave_lifecycle(runtime.last_seen_wave, current_wave);
    if !wave_changed {
        return;
    }

    runtime.last_seen_wave = current_wave;
    if !should_spawn {
        return;
    }
    if !existing_chests.is_empty() {
        return;
    }

    let center = commander_spawn_center(&commanders);
    let position = chest_spawn_position(&mut runtime.rng_state, bounds.as_deref().copied(), center);
    spawn_equipment_chest(&mut commands, &art, current_wave, position);
}

#[allow(clippy::too_many_arguments)]
fn pickup_equipment_chest(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    mut chest_state: ResMut<EquipmentChestState>,
    mut rng: ResMut<InventoryRngState>,
    rarity_bonus: Option<Res<ItemRarityRollBonus>>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    mut chests: Query<(Entity, &mut EquipmentChestDrop, &Transform)>,
    mut run_modal_requests: EventWriter<RunModalRequestEvent>,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }

    let pickup_radius = data.drops.pickup_radius.max(1.0);
    let faction = setup_selection
        .as_deref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let rarity_bonus_percent = rarity_bonus
        .as_deref()
        .map(|bonus| bonus.percent)
        .unwrap_or(0.0);

    for (entity, mut chest, transform) in &mut chests {
        chest.pickup_delay_remaining =
            tick_pickup_delay(chest.pickup_delay_remaining, time.delta_seconds());
        if chest.pickup_delay_remaining > 0.0 {
            continue;
        }

        let in_range = any_friendly_in_pickup_radius(
            transform.translation.truncate(),
            &friendly_positions,
            pickup_radius,
        );
        if in_range {
            chest.pickup_progress =
                (chest.pickup_progress + time.delta_seconds()).min(CHEST_PICKUP_CHANNEL_SECS);
        } else {
            chest.pickup_progress = (chest.pickup_progress - time.delta_seconds() * 0.75).max(0.0);
        }

        if chest.pickup_progress + f32::EPSILON < CHEST_PICKUP_CHANNEL_SECS {
            continue;
        }

        let item_count = (rng.next_u32_roll() % 3 + 1) as usize;
        let items = roll_chest_items(&mut rng, faction, item_count, rarity_bonus_percent);
        chest_state.clear();
        for (index, item) in items.into_iter().enumerate() {
            if let Some(slot) = chest_state.slots.get_mut(index) {
                *slot = Some(item);
            }
        }
        commands.entity(entity).despawn_recursive();
        run_modal_requests.send(RunModalRequestEvent {
            action: RunModalAction::Open(RunModalScreen::Chest),
        });
    }
}

#[allow(clippy::type_complexity)]
fn pickup_wave_magnet(
    mut commands: Commands,
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    magnets: Query<(Entity, &Transform), With<MagnetPickup>>,
    mut packs: Query<(Entity, Option<&DropInTransitToCommander>, &mut ExpPack)>,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }
    let pickup_radius = (data.drops.pickup_radius
        + buffs
            .as_ref()
            .map(|value| value.pickup_radius_bonus)
            .unwrap_or(0.0))
    .max(1.0);

    for (magnet_entity, magnet_transform) in &magnets {
        let magnet_position = magnet_transform.translation.truncate();
        if !any_friendly_in_pickup_radius(magnet_position, &friendly_positions, pickup_radius) {
            continue;
        }
        for (pack_entity, in_transit, mut pack) in &mut packs {
            let (should_insert_transit, next_delay) =
                force_home_pack_state(in_transit.is_some(), pack.pickup_delay_remaining);
            pack.pickup_delay_remaining = next_delay;
            if should_insert_transit {
                commands
                    .entity(pack_entity)
                    .insert(DropInTransitToCommander);
            }
        }
        commands.entity(magnet_entity).despawn_recursive();
    }
}

fn spawn_wave_magnet(commands: &mut Commands, art: &ArtAssets, faction: PlayerFaction, wave: u32) {
    let texture = magnet_texture_for_faction(art, faction);
    commands.spawn((
        MagnetPickup { faction, wave },
        SpriteBundle {
            texture,
            sprite: Sprite {
                custom_size: Some(Vec2::splat(MAGNET_PICKUP_SIZE)),
                color: Color::WHITE,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, MAGNET_PICKUP_Z),
            ..default()
        },
    ));
}

#[allow(clippy::type_complexity)]
fn pickup_exp_packs(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    mut packs: Query<
        (Entity, &mut ExpPack, &Transform),
        (Without<DropInTransitToCommander>, With<ExpPack>),
    >,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }

    let pickup_radius = (data.drops.pickup_radius
        + buffs
            .as_ref()
            .map(|value| value.pickup_radius_bonus)
            .unwrap_or(0.0))
    .max(1.0);
    for (entity, mut pack, transform) in &mut packs {
        pack.pickup_delay_remaining =
            tick_pickup_delay(pack.pickup_delay_remaining, time.delta_seconds());
        if pack.pickup_delay_remaining > 0.0 {
            continue;
        }
        let pack_position = transform.translation.truncate();
        if any_friendly_in_pickup_radius(pack_position, &friendly_positions, pickup_radius) {
            commands.entity(entity).insert(DropInTransitToCommander);
        }
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn transit_drops_to_commander(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    buffs: Option<Res<GlobalBuffs>>,
    commanders: Query<(&Transform, &MoveSpeed), With<CommanderUnit>>,
    mut packs: Query<
        (Entity, &ExpPack, &mut Transform),
        (With<DropInTransitToCommander>, Without<CommanderUnit>),
    >,
    mut xp_events: EventWriter<GainXpEvent>,
) {
    let Ok((commander_transform, commander_speed)) = commanders.get_single() else {
        return;
    };
    let player_faction = setup_selection
        .as_deref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let faction_xp_multiplier = data.factions.for_faction(player_faction).xp_gain_multiplier;
    let target = commander_transform.translation.truncate();
    let homing_speed = homing_speed_from_commander_base(commander_speed.0);
    let max_step = homing_speed * time.delta_seconds();

    for (entity, pack, mut transform) in &mut packs {
        let current = transform.translation.truncate();
        let next = step_towards_target(current, target, max_step);
        transform.translation.x = next.x;
        transform.translation.y = next.y;

        if reached_target(next, target, DROP_CONSUME_RADIUS) {
            let xp_gain =
                apply_xp_gain_multiplier(pack.xp_value, buffs.as_deref(), faction_xp_multiplier);
            xp_events.send(GainXpEvent(xp_gain));
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_exp_pack(
    commands: &mut Commands,
    position: Vec2,
    xp_value: f32,
    pickup_delay_secs: f32,
    art: &ArtAssets,
) {
    commands.spawn((
        ExpPack {
            xp_value,
            pickup_delay_remaining: pickup_delay_secs.max(0.0),
        },
        SpriteBundle {
            texture: art.exp_pack_coin_stack.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(DROP_RENDER_SIZE)),
                color: Color::srgb(1.0, 0.97, 0.76),
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, DROP_RENDER_Z),
            ..default()
        },
    ));
}

pub fn magnet_wave_lifecycle(last_seen_wave: u32, current_wave: u32) -> (bool, bool) {
    if current_wave == 0 || current_wave == last_seen_wave {
        return (false, false);
    }
    (true, should_spawn_magnet_for_wave(current_wave))
}

pub fn should_spawn_magnet_for_wave(wave_number: u32) -> bool {
    wave_number > 0 && wave_number.is_multiple_of(MAGNET_PICKUP_WAVE_INTERVAL)
}

pub fn chest_wave_lifecycle(last_seen_wave: u32, current_wave: u32) -> (bool, bool) {
    if current_wave == 0 || current_wave == last_seen_wave {
        return (false, false);
    }
    (true, should_spawn_chest_for_wave(current_wave))
}

pub fn should_spawn_chest_for_wave(wave_number: u32) -> bool {
    wave_number > 0 && wave_number.is_multiple_of(CHEST_PICKUP_WAVE_INTERVAL)
}

pub fn force_home_pack_state(
    already_in_transit: bool,
    _pickup_delay_remaining: f32,
) -> (bool, f32) {
    let should_insert_transit = !already_in_transit;
    let next_delay = 0.0;
    (should_insert_transit, next_delay)
}

fn magnet_texture_for_faction(art: &ArtAssets, faction: PlayerFaction) -> Handle<Image> {
    match faction {
        PlayerFaction::Christian => art.magnet_cross_pickup.clone(),
        PlayerFaction::Muslim => art.magnet_crescent_pickup.clone(),
    }
}

fn spawn_equipment_chest(commands: &mut Commands, art: &ArtAssets, wave: u32, position: Vec2) {
    commands.spawn((
        EquipmentChestDrop {
            wave,
            pickup_delay_remaining: CHEST_PICKUP_DELAY_SECS,
            pickup_progress: 0.0,
        },
        SpriteBundle {
            texture: art.chest_drop_closed.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(CHEST_PICKUP_SIZE)),
                color: Color::WHITE,
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, CHEST_PICKUP_Z),
            ..default()
        },
    ));
}

fn current_wave_number(waves: Option<&WaveRuntime>) -> u32 {
    let Some(runtime) = waves else {
        return 1;
    };
    runtime.current_wave.max(1)
}

fn current_commander_level(progression: Option<&Progression>) -> u32 {
    progression.map(|value| value.level.max(1)).unwrap_or(1)
}

fn runtime_seed_from_time() -> u32 {
    runtime_entropy_seed_u32()
}

fn commander_spawn_center(commanders: &Query<&Transform, With<CommanderUnit>>) -> Vec2 {
    commanders
        .get_single()
        .map(|transform| transform.translation.truncate())
        .unwrap_or(Vec2::ZERO)
}

pub fn scaled_pack_xp(base_xp: f32, wave_number: u32, commander_level: u32) -> f32 {
    let wave_scale = 1.0 + wave_number.saturating_sub(1) as f32 * 0.06;
    let level_scale = 1.0 + commander_level.saturating_sub(1) as f32 * 0.03;
    (base_xp * wave_scale * level_scale).max(1.0)
}

pub fn apply_xp_gain_multiplier(
    base_xp: f32,
    buffs: Option<&GlobalBuffs>,
    faction_multiplier: f32,
) -> f32 {
    let multiplier = buffs
        .map(|value| value.xp_gain_multiplier)
        .unwrap_or(1.0)
        .max(0.0)
        * faction_multiplier.max(0.0);
    (base_xp * multiplier).max(0.0)
}

fn drop_spawn_position(rng_state: &mut u64, bounds: Option<MapBounds>, center: Vec2) -> Vec2 {
    if let Some(map_bounds) = bounds {
        let spawn_half_width = map_bounds.half_width * 0.92;
        let spawn_half_height = map_bounds.half_height * 0.92;
        for attempt in 0..8 {
            let candidate = Vec2::new(
                lerp(
                    -spawn_half_width,
                    spawn_half_width,
                    next_random_f32(rng_state),
                ),
                lerp(
                    -spawn_half_height,
                    spawn_half_height,
                    next_random_f32(rng_state),
                ),
            );
            let keep = candidate.distance_squared(center) >= 92.0 * 92.0 || attempt == 7;
            if keep {
                return candidate;
            }
        }
        return Vec2::ZERO;
    }
    let radius = lerp(120.0, 520.0, next_random_f32(rng_state));
    let angle = next_random_f32(rng_state) * std::f32::consts::TAU;
    center + Vec2::new(radius * angle.cos(), radius * angle.sin())
}

fn rng_state_from_seed(seed: u32) -> u64 {
    let mixed = (seed as u64) ^ 0x9E37_79B9_7F4A_7C15;
    if mixed == 0 {
        0xD0C5_AA11_BA77_5EED
    } else {
        mixed
    }
}

fn next_random_u32(rng_state: &mut u64) -> u32 {
    *rng_state = rng_state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*rng_state >> 32) as u32
}

fn next_random_f32(rng_state: &mut u64) -> f32 {
    next_random_u32(rng_state) as f32 / u32::MAX as f32
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
    min + (max - min) * t
}

fn chest_spawn_position(rng_state: &mut u64, bounds: Option<MapBounds>, center: Vec2) -> Vec2 {
    let mut position = drop_spawn_position(rng_state, bounds, center);
    if let Some(map_bounds) = bounds {
        position.x = position
            .x
            .clamp(-map_bounds.half_width * 0.65, map_bounds.half_width * 0.65);
        position.y = position.y.clamp(
            -map_bounds.half_height * 0.65,
            map_bounds.half_height * 0.65,
        );
    }
    position
}

pub fn homing_speed_from_commander_base(commander_base_speed: f32) -> f32 {
    let base = commander_base_speed.max(1.0);
    (base * DROP_HOMING_SPEED_MULTIPLIER).max(base + 8.0)
}

pub fn tick_pickup_delay(remaining: f32, delta_seconds: f32) -> f32 {
    (remaining - delta_seconds.max(0.0)).max(0.0)
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

pub fn chest_pickup_progress_ratio(chest: &EquipmentChestDrop) -> Option<f32> {
    if chest.pickup_delay_remaining > 0.0 || chest.pickup_progress <= 0.0 {
        return None;
    }
    let ratio = (chest.pickup_progress / CHEST_PICKUP_CHANNEL_SECS).clamp(0.0, 1.0);
    if ratio >= 1.0 { None } else { Some(ratio) }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::drops::{
        EquipmentChestDrop, any_friendly_in_pickup_radius, apply_xp_gain_multiplier,
        chest_pickup_progress_ratio, chest_wave_lifecycle, drop_spawn_position,
        force_home_pack_state, homing_speed_from_commander_base, magnet_wave_lifecycle,
        reached_target, scaled_pack_xp, should_spawn_chest_for_wave, should_spawn_magnet_for_wave,
        step_towards_target, tick_pickup_delay,
    };
    use crate::map::MapBounds;
    use crate::model::GlobalBuffs;

    #[test]
    fn drop_spawn_points_stay_inside_bounds_and_are_varied() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 900.0,
        };
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut rng_state = 0x1234_5678_9ABC_DEF0;
        for _ in 0..48 {
            let point = drop_spawn_position(&mut rng_state, Some(bounds), Vec2::ZERO);
            assert!(point.x >= -bounds.half_width * 0.92 - 0.01);
            assert!(point.x <= bounds.half_width * 0.92 + 0.01);
            assert!(point.y >= -bounds.half_height * 0.92 - 0.01);
            assert!(point.y <= bounds.half_height * 0.92 + 0.01);
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }
        assert!(max_x - min_x > 800.0);
        assert!(max_y - min_y > 600.0);
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
    fn xp_gain_multiplier_scales_consumed_pack_xp() {
        let base = 12.0;
        let default_gain = apply_xp_gain_multiplier(base, None, 1.0);
        assert!((default_gain - 12.0).abs() < 0.001);

        let buffs = GlobalBuffs {
            xp_gain_multiplier: 1.35,
            ..GlobalBuffs::default()
        };
        let boosted = apply_xp_gain_multiplier(base, Some(&buffs), 1.0);
        assert!((boosted - 16.2).abs() < 0.001);

        let faction_boosted = apply_xp_gain_multiplier(base, Some(&buffs), 1.1);
        assert!((faction_boosted - 17.82).abs() < 0.001);
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

    #[test]
    fn homing_speed_is_always_above_commander_base() {
        assert!(homing_speed_from_commander_base(170.0) > 170.0);
        assert!(homing_speed_from_commander_base(50.0) > 50.0);
    }

    #[test]
    fn pickup_delay_ticks_down_to_zero() {
        assert!((tick_pickup_delay(0.5, 0.2) - 0.3).abs() < 0.001);
        assert_eq!(tick_pickup_delay(0.1, 1.0), 0.0);
    }

    #[test]
    fn magnet_lifecycle_spawns_every_third_wave_and_expires_next_wave() {
        assert!(!should_spawn_magnet_for_wave(1));
        assert!(!should_spawn_magnet_for_wave(2));
        assert!(should_spawn_magnet_for_wave(3));

        assert_eq!(magnet_wave_lifecycle(2, 2), (false, false));
        assert_eq!(magnet_wave_lifecycle(2, 3), (true, true));
        assert_eq!(magnet_wave_lifecycle(3, 4), (true, false));
    }

    #[test]
    fn magnet_force_homing_marks_non_transit_packs_and_clears_delay() {
        let (insert_transit, delay) = force_home_pack_state(false, 0.8);
        assert!(insert_transit);
        assert_eq!(delay, 0.0);

        let (insert_transit_existing, delay_existing) = force_home_pack_state(true, 1.2);
        assert!(!insert_transit_existing);
        assert_eq!(delay_existing, 0.0);
    }

    #[test]
    fn chest_lifecycle_spawns_every_third_wave() {
        assert!(!should_spawn_chest_for_wave(1));
        assert!(!should_spawn_chest_for_wave(2));
        assert!(should_spawn_chest_for_wave(3));

        assert_eq!(chest_wave_lifecycle(2, 2), (false, false));
        assert_eq!(chest_wave_lifecycle(2, 3), (true, true));
        assert_eq!(chest_wave_lifecycle(3, 4), (true, false));
    }

    #[test]
    fn chest_pickup_progress_ratio_visible_only_after_delay_and_before_completion() {
        let mut chest = EquipmentChestDrop {
            wave: 3,
            pickup_delay_remaining: 0.4,
            pickup_progress: 0.8,
        };
        assert_eq!(chest_pickup_progress_ratio(&chest), None);

        chest.pickup_delay_remaining = 0.0;
        assert!(chest_pickup_progress_ratio(&chest).is_some());

        chest.pickup_progress = 2.0;
        assert_eq!(chest_pickup_progress_ratio(&chest), None);
    }
}
