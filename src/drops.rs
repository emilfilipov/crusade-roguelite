use crate::random::runtime_entropy_seed_u32;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::data::GameData;
use crate::enemies::{MajorArmyDefeatedEvent, WaveCompletedEvent, WaveRuntime};
use crate::inventory::{
    EquipmentChestState, InventoryRngState, ItemRarityRollBonus, ItemTemplateCatalog,
    roll_chest_items, roll_chest_items_with_catalog,
};
use crate::map::MapBounds;
use crate::model::{
    CommanderUnit, EnemySpawnLane, FriendlyUnit, GainGoldEvent, GainHearTheCallTokenEvent,
    GameState, GlobalBuffs, MatchSetupSelection, MoveSpeed, PlayerFaction, RunModalAction,
    RunModalRequestEvent, RunModalScreen, SpawnGoldPackEvent, StartRunEvent,
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
const MAJOR_ARMY_CHEST_SPREAD_DISTANCE: f32 = 42.0;
const MAJOR_ARMY_CHEST_MIN_SEPARATION: f32 = 24.0;
const HEAR_THE_CALL_CHEST_PICKUP_SIZE: f32 = 32.0;
const HEAR_THE_CALL_CHEST_PICKUP_Z: f32 = 44.0;
const HEAR_THE_CALL_PICKUP_DELAY_SECS: f32 = 0.9;
const HEAR_THE_CALL_PICKUP_CHANNEL_SECS: f32 = 1.5;
const HEAR_THE_CALL_SMALL_BASE_CHANCE_PERCENT: f32 = 0.8;
const HEAR_THE_CALL_SMALL_CAP_CHANCE_PERCENT: f32 = 1.8;
const HEAR_THE_CALL_MINOR_BASE_CHANCE_PERCENT: f32 = 1.6;
const HEAR_THE_CALL_MINOR_CAP_CHANCE_PERCENT: f32 = 4.0;
const HEAR_THE_CALL_MAJOR_BASE_CHANCE_PERCENT: f32 = 6.0;
const HEAR_THE_CALL_MAJOR_CAP_CHANCE_PERCENT: f32 = 14.0;
const EQUIPMENT_SMALL_BASE_CHANCE_PERCENT: f32 = 2.5;
const EQUIPMENT_SMALL_CAP_CHANCE_PERCENT: f32 = 6.0;
const EQUIPMENT_MINOR_BASE_CHANCE_PERCENT: f32 = 6.0;
const EQUIPMENT_MINOR_CAP_CHANCE_PERCENT: f32 = 14.0;
const EQUIPMENT_MAJOR_BASE_CHANCE_PERCENT: f32 = 25.0;
const EQUIPMENT_MAJOR_CAP_CHANCE_PERCENT: f32 = 55.0;
const WAVE_RARITY_BONUS_PER_WAVE: f32 = 0.0035;
const WAVE_RARITY_BONUS_CAP: f32 = 0.35;

#[derive(Component, Clone, Copy, Debug)]
pub struct GoldPack {
    pub gold_value: f32,
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

#[derive(Component, Clone, Copy, Debug)]
pub struct HearTheCallDrop {
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

#[derive(Resource, Clone, Debug)]
struct MajorArmyChestRuntime {
    granted_waves: HashSet<u32>,
    rng_state: u64,
}

impl Default for MajorArmyChestRuntime {
    fn default() -> Self {
        Self {
            granted_waves: HashSet::new(),
            rng_state: 0xA6D4_39F1_C211_5B0E,
        }
    }
}

#[derive(Resource, Clone, Debug)]
struct HearTheCallDropRuntime {
    rng_state: u64,
}

impl Default for HearTheCallDropRuntime {
    fn default() -> Self {
        Self {
            rng_state: 0xC2A5_71DD_09B4_4F13,
        }
    }
}

pub struct DropsPlugin;

impl Plugin for DropsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DropSpawnRuntime>()
            .init_resource::<MagnetWaveRuntime>()
            .init_resource::<ChestWaveRuntime>()
            .init_resource::<MajorArmyChestRuntime>()
            .init_resource::<HearTheCallDropRuntime>()
            .add_systems(Update, reset_hear_the_call_drops_on_run_start)
            .add_systems(Update, spawn_gold_packs_on_run_start)
            .add_systems(
                Update,
                (
                    spawn_gold_packs_over_time,
                    spawn_gold_packs_from_events,
                    spawn_army_reward_chests_on_wave_completed,
                    update_wave_magnet_pickup,
                    update_wave_chest_drop,
                    spawn_major_army_reward_chests_on_defeat,
                    pickup_wave_magnet,
                    pickup_equipment_chest,
                    pickup_hear_the_call_tokens,
                    pickup_gold_packs,
                    transit_drops_to_commander,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_hear_the_call_drops_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    existing_drops: Query<Entity, With<HearTheCallDrop>>,
    mut runtime: ResMut<HearTheCallDropRuntime>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    for entity in &existing_drops {
        commands.entity(entity).despawn_recursive();
    }
    runtime.rng_state = rng_state_from_seed(runtime_seed_from_time() ^ 0x72A1_B9D0);
}

#[allow(clippy::too_many_arguments)]
fn spawn_gold_packs_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    progression: Option<Res<Progression>>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    existing_packs: Query<Entity, With<GoldPack>>,
    existing_magnets: Query<Entity, With<MagnetPickup>>,
    existing_chests: Query<Entity, With<EquipmentChestDrop>>,
    mut runtime: ResMut<DropSpawnRuntime>,
    mut magnet_runtime: ResMut<MagnetWaveRuntime>,
    mut chest_runtime: ResMut<ChestWaveRuntime>,
    mut major_chest_runtime: ResMut<MajorArmyChestRuntime>,
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
    major_chest_runtime.rng_state = rng_state_from_seed(runtime_seed ^ 0x5AC6_9D12);
    major_chest_runtime.granted_waves.clear();

    let initial_count = data.drops.initial_spawn_count.max(1);
    let wave_number = current_wave_number(waves.as_deref());
    let commander_level = current_commander_level(progression.as_deref());
    let gold_value = scaled_pack_gold(data.drops.gold_per_pack, wave_number, commander_level);
    let center = commander_spawn_center(&commanders);
    for _ in 0..initial_count {
        let position =
            drop_spawn_position(&mut runtime.rng_state, bounds.as_deref().copied(), center);
        spawn_gold_pack(
            &mut commands,
            position,
            gold_value,
            AMBIENT_PICKUP_DELAY_SECS,
            &art,
        );
    }

    runtime.timer = Timer::from_seconds(data.drops.spawn_interval_secs, TimerMode::Repeating);
    magnet_runtime.last_seen_wave = 0;
    chest_runtime.last_seen_wave = 0;
}

#[allow(clippy::too_many_arguments)]
fn spawn_gold_packs_over_time(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    progression: Option<Res<Progression>>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    packs: Query<Entity, With<GoldPack>>,
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
    let gold_value = scaled_pack_gold(data.drops.gold_per_pack, wave_number, commander_level);
    spawn_gold_pack(
        &mut commands,
        position,
        gold_value,
        AMBIENT_PICKUP_DELAY_SECS,
        &art,
    );
}

fn spawn_gold_packs_from_events(
    mut commands: Commands,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    waves: Option<Res<WaveRuntime>>,
    progression: Option<Res<Progression>>,
    packs: Query<Entity, With<GoldPack>>,
    mut spawn_events: EventReader<SpawnGoldPackEvent>,
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
        let base_gold = event
            .gold_value_override
            .unwrap_or(data.drops.gold_per_pack);
        let gold_value = scaled_pack_gold(base_gold, wave_number, commander_level);
        let pickup_delay = event
            .pickup_delay_secs
            .unwrap_or(AMBIENT_PICKUP_DELAY_SECS)
            .max(0.0);
        spawn_gold_pack(
            &mut commands,
            event.world_position,
            gold_value,
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
fn spawn_army_reward_chests_on_wave_completed(
    mut commands: Commands,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    progression: Option<Res<Progression>>,
    mut equipment_runtime: ResMut<MajorArmyChestRuntime>,
    mut hear_runtime: ResMut<HearTheCallDropRuntime>,
    mut wave_completed_events: EventReader<WaveCompletedEvent>,
) {
    if wave_completed_events.is_empty() {
        return;
    }
    let stash_tokens = progression
        .as_deref()
        .map(|value| value.hear_the_call_tokens)
        .unwrap_or(0);
    let map_bounds = bounds.as_deref().copied();
    let commander_center = commander_spawn_center(&commanders);

    for event in wave_completed_events.read() {
        let wave = event.wave_number.max(1);
        maybe_roll_lane_rewards(
            &mut commands,
            &art,
            map_bounds,
            EnemySpawnLane::Small,
            wave,
            commander_center,
            &mut equipment_runtime.rng_state,
            &mut hear_runtime.rng_state,
            stash_tokens,
        );
        if wave.is_multiple_of(2) {
            maybe_roll_lane_rewards(
                &mut commands,
                &art,
                map_bounds,
                EnemySpawnLane::Minor,
                wave,
                commander_center,
                &mut equipment_runtime.rng_state,
                &mut hear_runtime.rng_state,
                stash_tokens,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_major_army_reward_chests_on_defeat(
    mut commands: Commands,
    art: Res<ArtAssets>,
    progression: Option<Res<Progression>>,
    bounds: Option<Res<MapBounds>>,
    mut equipment_runtime: ResMut<MajorArmyChestRuntime>,
    mut hear_runtime: ResMut<HearTheCallDropRuntime>,
    mut defeated_events: EventReader<MajorArmyDefeatedEvent>,
) {
    if defeated_events.is_empty() {
        return;
    }
    let stash_tokens = progression
        .as_deref()
        .map(|value| value.hear_the_call_tokens)
        .unwrap_or(0);
    let map_bounds = bounds.as_deref().copied();
    for event in defeated_events.read() {
        if !should_grant_major_boss_chests(&mut equipment_runtime.granted_waves, event.wave_number)
        {
            continue;
        }
        let wave = event.wave_number.max(1);
        let equipment_chance = equipment_drop_chance_percent(EnemySpawnLane::Major, wave);
        if roll_chance_percent(&mut equipment_runtime.rng_state, equipment_chance) {
            let positions = major_army_chest_positions(
                event.position,
                map_bounds,
                &mut equipment_runtime.rng_state,
            );
            spawn_equipment_chest(&mut commands, &art, wave, positions[0]);
            spawn_equipment_chest(&mut commands, &art, wave, positions[1]);
        }

        let token_chance =
            hear_the_call_drop_chance_percent(EnemySpawnLane::Major, wave, stash_tokens);
        if roll_chance_percent(&mut hear_runtime.rng_state, token_chance) {
            let token_position =
                chest_spawn_position(&mut hear_runtime.rng_state, map_bounds, event.position);
            spawn_hear_the_call_drop(&mut commands, &art, wave, token_position);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn pickup_equipment_chest(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    item_catalog: Option<Res<ItemTemplateCatalog>>,
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

        let effective_rarity_bonus_percent =
            rarity_bonus_percent + wave_rarity_roll_bonus_percent(chest.wave);
        let item_count = (rng.next_u32_roll() % 3 + 1) as usize;
        let items = if let Some(catalog) = item_catalog.as_deref() {
            roll_chest_items_with_catalog(
                &mut rng,
                catalog,
                faction,
                item_count,
                effective_rarity_bonus_percent,
            )
        } else {
            roll_chest_items(
                &mut rng,
                faction,
                item_count,
                effective_rarity_bonus_percent,
            )
        };
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

fn pickup_hear_the_call_tokens(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    mut tokens: Query<(Entity, &mut HearTheCallDrop, &Transform)>,
    mut gain_token_events: EventWriter<GainHearTheCallTokenEvent>,
) {
    let friendly_positions: Vec<Vec2> = friendlies
        .iter()
        .map(|transform| transform.translation.truncate())
        .collect();
    if friendly_positions.is_empty() {
        return;
    }
    let pickup_radius = data.drops.pickup_radius.max(1.0);
    for (entity, mut token, transform) in &mut tokens {
        token.pickup_delay_remaining =
            tick_pickup_delay(token.pickup_delay_remaining, time.delta_seconds());
        if token.pickup_delay_remaining > 0.0 {
            continue;
        }
        let in_range = any_friendly_in_pickup_radius(
            transform.translation.truncate(),
            &friendly_positions,
            pickup_radius,
        );
        if in_range {
            token.pickup_progress = (token.pickup_progress + time.delta_seconds())
                .min(HEAR_THE_CALL_PICKUP_CHANNEL_SECS);
        } else {
            token.pickup_progress = (token.pickup_progress - time.delta_seconds() * 0.75).max(0.0);
        }

        if token.pickup_progress + f32::EPSILON < HEAR_THE_CALL_PICKUP_CHANNEL_SECS {
            continue;
        }
        gain_token_events.send(GainHearTheCallTokenEvent(1));
        commands.entity(entity).despawn_recursive();
    }
}

#[allow(clippy::type_complexity)]
fn pickup_wave_magnet(
    mut commands: Commands,
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    magnets: Query<(Entity, &Transform), With<MagnetPickup>>,
    mut packs: Query<(Entity, Option<&DropInTransitToCommander>, &mut GoldPack)>,
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
fn pickup_gold_packs(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
    mut packs: Query<
        (Entity, &mut GoldPack, &Transform),
        (Without<DropInTransitToCommander>, With<GoldPack>),
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
        (Entity, &GoldPack, &mut Transform),
        (With<DropInTransitToCommander>, Without<CommanderUnit>),
    >,
    mut gold_events: EventWriter<GainGoldEvent>,
) {
    let Ok((commander_transform, commander_speed)) = commanders.get_single() else {
        return;
    };
    let player_faction = setup_selection
        .as_deref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let faction_gold_multiplier = data
        .factions
        .for_faction(player_faction)
        .gold_gain_multiplier;
    let target = commander_transform.translation.truncate();
    let homing_speed = homing_speed_from_commander_base(commander_speed.0);
    let max_step = homing_speed * time.delta_seconds();

    for (entity, pack, mut transform) in &mut packs {
        let current = transform.translation.truncate();
        let next = step_towards_target(current, target, max_step);
        transform.translation.x = next.x;
        transform.translation.y = next.y;

        if reached_target(next, target, DROP_CONSUME_RADIUS) {
            let gold_gain = apply_gold_gain_multiplier(
                pack.gold_value,
                buffs.as_deref(),
                faction_gold_multiplier,
            );
            gold_events.send(GainGoldEvent(gold_gain));
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn spawn_gold_pack(
    commands: &mut Commands,
    position: Vec2,
    gold_value: f32,
    pickup_delay_secs: f32,
    art: &ArtAssets,
) {
    commands.spawn((
        GoldPack {
            gold_value,
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

fn should_grant_major_boss_chests(granted_waves: &mut HashSet<u32>, wave_number: u32) -> bool {
    granted_waves.insert(wave_number)
}

fn hear_the_call_stash_damping_multiplier(tokens: u32) -> f32 {
    match tokens {
        0 | 1 => 1.0,
        2 | 3 => 0.6,
        4 | 5 => 0.3,
        _ => 0.15,
    }
}

fn hear_the_call_base_and_cap_chance_percent(lane: EnemySpawnLane) -> (f32, f32) {
    match lane {
        EnemySpawnLane::Small => (
            HEAR_THE_CALL_SMALL_BASE_CHANCE_PERCENT,
            HEAR_THE_CALL_SMALL_CAP_CHANCE_PERCENT,
        ),
        EnemySpawnLane::Minor => (
            HEAR_THE_CALL_MINOR_BASE_CHANCE_PERCENT,
            HEAR_THE_CALL_MINOR_CAP_CHANCE_PERCENT,
        ),
        EnemySpawnLane::Major => (
            HEAR_THE_CALL_MAJOR_BASE_CHANCE_PERCENT,
            HEAR_THE_CALL_MAJOR_CAP_CHANCE_PERCENT,
        ),
    }
}

fn hear_the_call_drop_chance_percent(
    lane: EnemySpawnLane,
    wave_number: u32,
    stash_tokens: u32,
) -> f32 {
    let (base_percent, cap_percent) = hear_the_call_base_and_cap_chance_percent(lane);
    let wave_progress = wave_number.saturating_sub(1).min(99) as f32 / 99.0;
    let scaled = base_percent + (cap_percent - base_percent) * wave_progress;
    let damped = scaled * hear_the_call_stash_damping_multiplier(stash_tokens);
    damped.clamp(0.0, cap_percent)
}

fn equipment_base_and_cap_chance_percent(lane: EnemySpawnLane) -> (f32, f32) {
    match lane {
        EnemySpawnLane::Small => (
            EQUIPMENT_SMALL_BASE_CHANCE_PERCENT,
            EQUIPMENT_SMALL_CAP_CHANCE_PERCENT,
        ),
        EnemySpawnLane::Minor => (
            EQUIPMENT_MINOR_BASE_CHANCE_PERCENT,
            EQUIPMENT_MINOR_CAP_CHANCE_PERCENT,
        ),
        EnemySpawnLane::Major => (
            EQUIPMENT_MAJOR_BASE_CHANCE_PERCENT,
            EQUIPMENT_MAJOR_CAP_CHANCE_PERCENT,
        ),
    }
}

fn equipment_drop_chance_percent(lane: EnemySpawnLane, wave_number: u32) -> f32 {
    let (base_percent, cap_percent) = equipment_base_and_cap_chance_percent(lane);
    let wave_progress = wave_number.saturating_sub(1).min(99) as f32 / 99.0;
    let scaled = base_percent + (cap_percent - base_percent) * wave_progress;
    scaled.clamp(0.0, cap_percent)
}

fn roll_chance_percent(rng_state: &mut u64, chance_percent: f32) -> bool {
    let clamped = chance_percent.clamp(0.0, 100.0);
    let roll = next_random_f32(rng_state) * 100.0;
    roll <= clamped
}

#[allow(clippy::too_many_arguments)]
fn maybe_roll_lane_rewards(
    commands: &mut Commands,
    art: &ArtAssets,
    map_bounds: Option<MapBounds>,
    lane: EnemySpawnLane,
    wave: u32,
    center: Vec2,
    equipment_rng_state: &mut u64,
    hear_rng_state: &mut u64,
    stash_tokens: u32,
) {
    let equipment_chance = equipment_drop_chance_percent(lane, wave);
    if roll_chance_percent(equipment_rng_state, equipment_chance) {
        let equipment_position = chest_spawn_position(equipment_rng_state, map_bounds, center);
        spawn_equipment_chest(commands, art, wave, equipment_position);
    }
    let hear_chance = hear_the_call_drop_chance_percent(lane, wave, stash_tokens);
    if roll_chance_percent(hear_rng_state, hear_chance) {
        let hear_position = chest_spawn_position(hear_rng_state, map_bounds, center);
        spawn_hear_the_call_drop(commands, art, wave, hear_position);
    }
}

fn wave_rarity_roll_bonus_percent(wave_number: u32) -> f32 {
    (wave_number.saturating_sub(1) as f32 * WAVE_RARITY_BONUS_PER_WAVE)
        .clamp(0.0, WAVE_RARITY_BONUS_CAP)
}

fn major_army_chest_positions(
    center: Vec2,
    bounds: Option<MapBounds>,
    rng_state: &mut u64,
) -> [Vec2; 2] {
    let angle = next_random_f32(rng_state) * std::f32::consts::TAU;
    let direction = Vec2::from_angle(angle);
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let spread = MAJOR_ARMY_CHEST_SPREAD_DISTANCE + next_random_f32(rng_state) * 8.0;
    let jitter = (next_random_f32(rng_state) - 0.5) * 18.0;

    let mut first = center + direction * spread + perpendicular * jitter;
    let mut second =
        center - direction * (spread + MAJOR_ARMY_CHEST_MIN_SEPARATION) - perpendicular * jitter;
    if let Some(active_bounds) = bounds {
        first = clamp_position_to_bounds(first, active_bounds);
        second = clamp_position_to_bounds(second, active_bounds);
        let separation = first.distance(second);
        if separation + f32::EPSILON < MAJOR_ARMY_CHEST_MIN_SEPARATION {
            let direction = (second - first).normalize_or_zero();
            let fallback_direction = if direction == Vec2::ZERO {
                Vec2::X
            } else {
                direction
            };
            second = clamp_position_to_bounds(
                first + fallback_direction * MAJOR_ARMY_CHEST_MIN_SEPARATION,
                active_bounds,
            );
        }
    }

    [first, second]
}

fn clamp_position_to_bounds(position: Vec2, bounds: MapBounds) -> Vec2 {
    Vec2::new(
        position.x.clamp(-bounds.half_width, bounds.half_width),
        position.y.clamp(-bounds.half_height, bounds.half_height),
    )
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

fn spawn_hear_the_call_drop(commands: &mut Commands, art: &ArtAssets, wave: u32, position: Vec2) {
    commands.spawn((
        HearTheCallDrop {
            wave,
            pickup_delay_remaining: HEAR_THE_CALL_PICKUP_DELAY_SECS,
            pickup_progress: 0.0,
        },
        SpriteBundle {
            texture: art.chest_drop_closed.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(HEAR_THE_CALL_CHEST_PICKUP_SIZE)),
                color: Color::srgb(0.96, 0.82, 0.26),
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, HEAR_THE_CALL_CHEST_PICKUP_Z),
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

pub fn scaled_pack_gold(base_gold: f32, wave_number: u32, commander_level: u32) -> f32 {
    let wave_scale = 1.0 + wave_number.saturating_sub(1) as f32 * 0.06;
    let level_scale = 1.0 + commander_level.saturating_sub(1) as f32 * 0.03;
    (base_gold * wave_scale * level_scale).max(1.0)
}

pub fn apply_gold_gain_multiplier(
    base_gold: f32,
    buffs: Option<&GlobalBuffs>,
    faction_multiplier: f32,
) -> f32 {
    let multiplier = buffs
        .map(|value| value.gold_gain_multiplier)
        .unwrap_or(1.0)
        .max(0.0)
        * faction_multiplier.max(0.0);
    (base_gold * multiplier).max(0.0)
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

pub fn hear_the_call_pickup_progress_ratio(drop: &HearTheCallDrop) -> Option<f32> {
    if drop.pickup_delay_remaining > 0.0 || drop.pickup_progress <= 0.0 {
        return None;
    }
    let ratio = (drop.pickup_progress / HEAR_THE_CALL_PICKUP_CHANNEL_SECS).clamp(0.0, 1.0);
    if ratio >= 1.0 { None } else { Some(ratio) }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::drops::{
        EquipmentChestDrop, HearTheCallDrop, any_friendly_in_pickup_radius,
        apply_gold_gain_multiplier, chest_pickup_progress_ratio, chest_wave_lifecycle,
        drop_spawn_position, force_home_pack_state, hear_the_call_drop_chance_percent,
        hear_the_call_pickup_progress_ratio, hear_the_call_stash_damping_multiplier,
        homing_speed_from_commander_base, magnet_wave_lifecycle, major_army_chest_positions,
        reached_target, scaled_pack_gold, should_grant_major_boss_chests,
        should_spawn_chest_for_wave, should_spawn_magnet_for_wave, step_towards_target,
        tick_pickup_delay,
    };
    use crate::map::MapBounds;
    use crate::model::{EnemySpawnLane, GlobalBuffs};

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
    fn gold_pack_scaling_increases_with_wave_and_level() {
        let base = 6.0;
        let early = scaled_pack_gold(base, 1, 1);
        let later_wave = scaled_pack_gold(base, 5, 1);
        let later_level = scaled_pack_gold(base, 1, 6);
        let both = scaled_pack_gold(base, 5, 6);

        assert!(later_wave > early);
        assert!(later_level > early);
        assert!(both > later_wave);
        assert!(both > later_level);
    }

    #[test]
    fn gold_gain_multiplier_scales_consumed_pack_gold() {
        let base = 12.0;
        let default_gain = apply_gold_gain_multiplier(base, None, 1.0);
        assert!((default_gain - 12.0).abs() < 0.001);

        let buffs = GlobalBuffs {
            gold_gain_multiplier: 1.35,
            ..GlobalBuffs::default()
        };
        let boosted = apply_gold_gain_multiplier(base, Some(&buffs), 1.0);
        assert!((boosted - 16.2).abs() < 0.001);

        let faction_boosted = apply_gold_gain_multiplier(base, Some(&buffs), 1.1);
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

    #[test]
    fn hear_the_call_pickup_progress_ratio_visible_only_after_delay_and_before_completion() {
        let mut drop = HearTheCallDrop {
            wave: 18,
            pickup_delay_remaining: 0.3,
            pickup_progress: 0.8,
        };
        assert_eq!(hear_the_call_pickup_progress_ratio(&drop), None);

        drop.pickup_delay_remaining = 0.0;
        assert!(hear_the_call_pickup_progress_ratio(&drop).is_some());

        drop.pickup_progress = super::HEAR_THE_CALL_PICKUP_CHANNEL_SECS;
        assert_eq!(hear_the_call_pickup_progress_ratio(&drop), None);
    }

    #[test]
    fn major_army_chest_positions_are_non_overlapping_and_in_bounds() {
        let mut rng_state = 0x9182_6A77_11CD_44E0u64;
        let bounds = MapBounds {
            half_width: 300.0,
            half_height: 260.0,
        };
        let [first, second] =
            major_army_chest_positions(Vec2::new(280.0, -245.0), Some(bounds), &mut rng_state);

        assert!(first.x >= -bounds.half_width && first.x <= bounds.half_width);
        assert!(first.y >= -bounds.half_height && first.y <= bounds.half_height);
        assert!(second.x >= -bounds.half_width && second.x <= bounds.half_width);
        assert!(second.y >= -bounds.half_height && second.y <= bounds.half_height);
        assert!(first.distance(second) + f32::EPSILON >= super::MAJOR_ARMY_CHEST_MIN_SEPARATION);
    }

    #[test]
    fn major_army_chest_reward_is_only_granted_once_per_wave() {
        let mut granted = std::collections::HashSet::new();
        assert!(should_grant_major_boss_chests(&mut granted, 10));
        assert!(!should_grant_major_boss_chests(&mut granted, 10));
        assert!(should_grant_major_boss_chests(&mut granted, 20));
    }

    #[test]
    fn hear_the_call_drop_chance_scales_by_lane_wave_and_stash() {
        let small_early = hear_the_call_drop_chance_percent(EnemySpawnLane::Small, 1, 0);
        let small_late = hear_the_call_drop_chance_percent(EnemySpawnLane::Small, 100, 0);
        let minor_late = hear_the_call_drop_chance_percent(EnemySpawnLane::Minor, 100, 0);
        let major_late = hear_the_call_drop_chance_percent(EnemySpawnLane::Major, 100, 0);
        let major_late_damped = hear_the_call_drop_chance_percent(EnemySpawnLane::Major, 100, 6);

        assert!(small_late > small_early);
        assert!(minor_late > small_late);
        assert!(major_late > minor_late);
        assert!(major_late_damped < major_late);
    }

    #[test]
    fn equipment_drop_chance_scales_by_lane_and_wave() {
        let small_early = super::equipment_drop_chance_percent(EnemySpawnLane::Small, 1);
        let small_late = super::equipment_drop_chance_percent(EnemySpawnLane::Small, 100);
        let minor_late = super::equipment_drop_chance_percent(EnemySpawnLane::Minor, 100);
        let major_late = super::equipment_drop_chance_percent(EnemySpawnLane::Major, 100);

        assert!(small_late > small_early);
        assert!(minor_late > small_late);
        assert!(major_late > minor_late);
    }

    #[test]
    fn hear_the_call_stash_damping_has_expected_breakpoints() {
        assert!((hear_the_call_stash_damping_multiplier(0) - 1.0).abs() < 0.0001);
        assert!((hear_the_call_stash_damping_multiplier(2) - 0.6).abs() < 0.0001);
        assert!((hear_the_call_stash_damping_multiplier(4) - 0.3).abs() < 0.0001);
        assert!((hear_the_call_stash_damping_multiplier(8) - 0.15).abs() < 0.0001);
    }

    #[test]
    fn wave_rarity_bonus_scales_gradually_and_caps() {
        assert!((super::wave_rarity_roll_bonus_percent(1) - 0.0).abs() < 0.0001);
        let mid = super::wave_rarity_roll_bonus_percent(50);
        let late = super::wave_rarity_roll_bonus_percent(100);
        assert!(mid > 0.0);
        assert!(late > mid);
        assert!(late <= super::WAVE_RARITY_BONUS_CAP + 0.0001);
    }
}
