use bevy::prelude::*;

use crate::data::{GameData, RescueConfig};
use crate::map::MapBounds;
use crate::model::{
    FriendlyUnit, GameState, MatchSetupSelection, PlayerFaction, RecruitArchetype, RecruitEvent,
    RecruitUnitKind, RescuableUnit, StartRunEvent, Team, Unit,
};
use crate::random::runtime_entropy_seed_u32;
use crate::upgrades::ConditionalUpgradeEffects;
use crate::visuals::ArtAssets;

const RESCUE_RESPAWN_INTERVAL_SECS: f32 = 12.0;
const MAX_ACTIVE_RESCUABLES: usize = 6;
const RESCUE_PITY_WEIGHT_STEP: u32 = 1;

#[derive(Component, Clone, Copy, Debug)]
pub struct RescueProgress {
    pub elapsed: f32,
}

#[derive(Resource, Clone, Debug)]
struct RescueSpawnRuntime {
    timer: Timer,
    sequence: u32,
    seed: u32,
    rng_state: u64,
    pity: RescueSpawnPity,
}

impl Default for RescueSpawnRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(RESCUE_RESPAWN_INTERVAL_SECS, TimerMode::Repeating),
            sequence: 0,
            seed: 0xA1B2_C3D4,
            rng_state: 0x71F0_9D52_CAF3_BA17,
            pity: RescueSpawnPity::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct RescueSpawnPity {
    infantry_drought: u32,
    archer_drought: u32,
    priest_drought: u32,
}

impl RescueSpawnPity {
    fn drought_for(self, kind: RecruitUnitKind) -> u32 {
        match kind.archetype() {
            RecruitArchetype::Infantry => self.infantry_drought,
            RecruitArchetype::Archer => self.archer_drought,
            RecruitArchetype::Priest => self.priest_drought,
        }
    }

    fn set_drought_for(&mut self, kind: RecruitUnitKind, value: u32) {
        match kind.archetype() {
            RecruitArchetype::Infantry => self.infantry_drought = value,
            RecruitArchetype::Archer => self.archer_drought = value,
            RecruitArchetype::Priest => self.priest_drought = value,
        }
    }

    fn note_spawn(
        &mut self,
        spawned: RecruitUnitKind,
        config: &RescueConfig,
        player_faction: PlayerFaction,
    ) {
        for kind in RecruitUnitKind::all_for_faction(player_faction) {
            if !rescue_pool_contains_kind(config, kind) {
                continue;
            }
            if kind == spawned {
                self.set_drought_for(kind, 0);
            } else {
                let next = self.drought_for(kind).saturating_add(1);
                self.set_drought_for(kind, next);
            }
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

#[allow(clippy::too_many_arguments)]
fn spawn_rescuables_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    existing_rescuables: Query<Entity, With<RescuableUnit>>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    mut spawn_runtime: ResMut<RescueSpawnRuntime>,
    setup_selection: Option<Res<MatchSetupSelection>>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in existing_rescuables.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let faction = setup_selection
        .as_ref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    spawn_runtime.seed = runtime_seed_from_time();
    spawn_runtime.rng_state = rng_state_from_seed(spawn_runtime.seed);
    let count = data.rescue.spawn_count.max(1);
    for idx in 0..count {
        let seeded_sequence = idx.wrapping_add(spawn_runtime.seed);
        let recruit_kind =
            recruit_kind_for_sequence(seeded_sequence, &data.rescue, spawn_runtime.pity, faction);
        spawn_runtime
            .pity
            .note_spawn(recruit_kind, &data.rescue, faction);
        spawn_rescuable(
            &mut commands,
            rescue_spawn_position(&mut spawn_runtime.rng_state, bounds.as_deref().copied()),
            recruit_kind,
            &art,
        );
    }

    spawn_runtime.sequence = count;
    spawn_runtime.timer = Timer::from_seconds(RESCUE_RESPAWN_INTERVAL_SECS, TimerMode::Repeating);
}

#[allow(clippy::too_many_arguments)]
fn spawn_rescuables_over_time(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    bounds: Option<Res<MapBounds>>,
    rescuables: Query<Entity, With<RescuableUnit>>,
    mut runtime: ResMut<RescueSpawnRuntime>,
    setup_selection: Option<Res<MatchSetupSelection>>,
) {
    if rescuables.iter().count() >= MAX_ACTIVE_RESCUABLES {
        return;
    }

    runtime.timer.tick(time.delta());
    if !runtime.timer.just_finished() {
        return;
    }

    let faction = setup_selection
        .as_ref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let seeded_sequence = runtime.sequence.wrapping_add(runtime.seed);
    let spawn_position = rescue_spawn_position(&mut runtime.rng_state, bounds.as_deref().copied());
    let recruit_kind =
        recruit_kind_for_sequence(seeded_sequence, &data.rescue, runtime.pity, faction);
    runtime.pity.note_spawn(recruit_kind, &data.rescue, faction);
    spawn_rescuable(&mut commands, spawn_position, recruit_kind, &art);
    runtime.sequence = runtime.sequence.saturating_add(1);
}

fn runtime_seed_from_time() -> u32 {
    runtime_entropy_seed_u32()
}

#[allow(clippy::too_many_arguments)]
fn tick_rescue_progress(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
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
    let player_faction = setup_selection
        .as_deref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let rescue_duration = effective_rescue_duration(
        data.rescue.rescue_duration_secs,
        conditional_effects.as_deref(),
        data.factions
            .for_faction(player_faction)
            .rescue_time_multiplier,
    );

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

pub(crate) fn spawn_rescuable_entity(
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
        RecruitUnitKind::ChristianPeasantPriest => (
            crate::model::UnitKind::RescuableChristianPeasantPriest,
            art.friendly_peasant_priest_idle.clone(),
            Color::srgb(0.94, 0.92, 0.98),
        ),
        RecruitUnitKind::MuslimPeasantInfantry => (
            crate::model::UnitKind::RescuableMuslimPeasantInfantry,
            art.muslim_peasant_infantry_rescuable_variant.clone(),
            Color::srgb(0.86, 0.9, 1.0),
        ),
        RecruitUnitKind::MuslimPeasantArcher => (
            crate::model::UnitKind::RescuableMuslimPeasantArcher,
            art.muslim_peasant_archer_rescuable_variant.clone(),
            Color::srgb(0.84, 0.95, 0.86),
        ),
        RecruitUnitKind::MuslimPeasantPriest => (
            crate::model::UnitKind::RescuableMuslimPeasantPriest,
            art.muslim_peasant_priest_idle.clone(),
            Color::srgb(0.9, 0.9, 0.98),
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

fn spawn_rescuable(
    commands: &mut Commands,
    position: Vec2,
    recruit_kind: RecruitUnitKind,
    art: &ArtAssets,
) {
    spawn_rescuable_entity(commands, position, recruit_kind, art);
}

fn recruit_kind_for_sequence(
    sequence: u32,
    config: &RescueConfig,
    pity: RescueSpawnPity,
    player_faction: PlayerFaction,
) -> RecruitUnitKind {
    let fallback =
        RecruitUnitKind::from_faction_and_archetype(player_faction, RecruitArchetype::Infantry);
    let mut total_weight = 0u64;
    let mut weighted_entries = Vec::with_capacity(config.recruit_pool.len());
    for entry in &config.recruit_pool {
        let kind = entry.as_recruit_unit_kind();
        if kind.faction() != player_faction {
            continue;
        }
        let drought = pity.drought_for(kind) as u64;
        let weight = 1u64 + drought * RESCUE_PITY_WEIGHT_STEP as u64;
        total_weight = total_weight.saturating_add(weight);
        weighted_entries.push((kind, weight));
    }

    if total_weight == 0 {
        return fallback;
    }
    let roll = (rescue_hash_seed(sequence, 0xA5A5_5A5A) as u64) % total_weight;
    let mut cursor = 0u64;
    for (kind, weight) in weighted_entries {
        cursor = cursor.saturating_add(weight);
        if roll < cursor {
            return kind;
        }
    }
    config
        .recruit_pool
        .iter()
        .rev()
        .map(|value| value.as_recruit_unit_kind())
        .find(|kind| kind.faction() == player_faction)
        .unwrap_or(fallback)
}

fn rescue_pool_contains_kind(config: &RescueConfig, kind: RecruitUnitKind) -> bool {
    config
        .recruit_pool
        .iter()
        .any(|entry| entry.as_recruit_unit_kind() == kind)
}

fn rescue_spawn_position(rng_state: &mut u64, bounds: Option<MapBounds>) -> Vec2 {
    let (map_half_width, map_half_height) = bounds
        .map(|b| (b.half_width.max(1.0), b.half_height.max(1.0)))
        .unwrap_or((1000.0, 800.0));
    let central_half_width = (map_half_width * 0.45).max(140.0);
    let central_half_height = (map_half_height * 0.45).max(140.0);

    for attempt in 0..8 {
        let x = lerp(
            -central_half_width,
            central_half_width,
            next_random_f32(rng_state),
        );
        let y = lerp(
            -central_half_height,
            central_half_height,
            next_random_f32(rng_state),
        );
        let candidate = Vec2::new(x, y);
        // Keep candidates generally central while avoiding exact center pileups.
        if candidate.length_squared() >= 40.0 * 40.0 || attempt == 7 {
            return candidate;
        }
    }

    Vec2::ZERO
}

fn rescue_hash_seed(sequence: u32, attempt: u32) -> u32 {
    let mut value = sequence
        .wrapping_mul(1_103_515_245)
        .wrapping_add(attempt.wrapping_mul(747_796_405))
        .wrapping_add(0x9E37_79B9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^ (value >> 16)
}

fn rng_state_from_seed(seed: u32) -> u64 {
    let mixed = (seed as u64) ^ 0x9E37_79B9_7F4A_7C15;
    if mixed == 0 {
        0x71F0_9D52_CAF3_BA17
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

pub fn effective_rescue_duration(
    base_duration_secs: f32,
    conditional_effects: Option<&ConditionalUpgradeEffects>,
    faction_multiplier: f32,
) -> f32 {
    let multiplier = conditional_effects
        .map(|effects| effects.rescue_time_multiplier)
        .unwrap_or(1.0)
        .max(0.0);
    base_duration_secs.max(0.0) * multiplier * faction_multiplier.max(0.0)
}

pub const fn rescue_respawn_interval_secs() -> f32 {
    RESCUE_RESPAWN_INTERVAL_SECS
}

pub const fn rescue_max_active() -> usize {
    MAX_ACTIVE_RESCUABLES
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::data::{RescueConfig, RescueRecruitKindConfig};
    use crate::map::MapBounds;
    use crate::model::{PlayerFaction, RecruitUnitKind};
    use crate::rescue::{
        RescueSpawnPity, advance_rescue_progress, any_friendly_in_rescue_radius,
        effective_rescue_duration, recruit_kind_for_sequence, rescue_max_active,
        rescue_respawn_interval_secs, rescue_spawn_position,
    };
    use crate::upgrades::ConditionalUpgradeEffects;

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
    fn rescue_spawn_points_stay_within_central_zone() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 1000.0,
        };
        let mut rng_state = 0x1234_5678_9ABC_DEF0;
        for _ in 0..32 {
            let point = rescue_spawn_position(&mut rng_state, Some(bounds));
            assert!(point.x.abs() <= bounds.half_width * 0.45 + 0.01);
            assert!(point.y.abs() <= bounds.half_height * 0.45 + 0.01);
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
    fn rescue_spawn_selector_returns_only_pool_entries() {
        let config = RescueConfig {
            spawn_count: 3,
            rescue_radius: 10.0,
            rescue_duration_secs: 1.0,
            recruit_pool: vec![
                RescueRecruitKindConfig::ChristianPeasantInfantry,
                RescueRecruitKindConfig::ChristianPeasantArcher,
                RescueRecruitKindConfig::ChristianPeasantPriest,
            ],
        };
        let pity = RescueSpawnPity::default();
        for sequence in 0..64 {
            let kind = recruit_kind_for_sequence(sequence, &config, pity, PlayerFaction::Christian);
            assert!(
                matches!(
                    kind,
                    RecruitUnitKind::ChristianPeasantInfantry
                        | RecruitUnitKind::ChristianPeasantArcher
                        | RecruitUnitKind::ChristianPeasantPriest
                ),
                "kind {kind:?} should be in rescue pool"
            );
        }
    }

    #[test]
    fn rescue_spawn_selector_filters_mixed_pool_by_player_faction() {
        let config = RescueConfig {
            spawn_count: 3,
            rescue_radius: 10.0,
            rescue_duration_secs: 1.0,
            recruit_pool: vec![
                RescueRecruitKindConfig::ChristianPeasantInfantry,
                RescueRecruitKindConfig::ChristianPeasantArcher,
                RescueRecruitKindConfig::ChristianPeasantPriest,
                RescueRecruitKindConfig::MuslimPeasantInfantry,
                RescueRecruitKindConfig::MuslimPeasantArcher,
                RescueRecruitKindConfig::MuslimPeasantPriest,
            ],
        };
        let pity = RescueSpawnPity::default();
        for sequence in 0..96 {
            let muslim_kind =
                recruit_kind_for_sequence(sequence, &config, pity, PlayerFaction::Muslim);
            assert_eq!(muslim_kind.faction(), PlayerFaction::Muslim);
        }
    }

    #[test]
    fn pity_counters_reset_spawned_kind_and_increase_others() {
        let config = RescueConfig {
            spawn_count: 3,
            rescue_radius: 10.0,
            rescue_duration_secs: 1.0,
            recruit_pool: vec![
                RescueRecruitKindConfig::ChristianPeasantInfantry,
                RescueRecruitKindConfig::ChristianPeasantArcher,
                RescueRecruitKindConfig::ChristianPeasantPriest,
            ],
        };
        let mut pity = RescueSpawnPity::default();
        pity.note_spawn(
            RecruitUnitKind::ChristianPeasantInfantry,
            &config,
            PlayerFaction::Christian,
        );
        assert_eq!(pity.infantry_drought, 0);
        assert_eq!(pity.archer_drought, 1);
        assert_eq!(pity.priest_drought, 1);

        pity.note_spawn(
            RecruitUnitKind::ChristianPeasantArcher,
            &config,
            PlayerFaction::Christian,
        );
        assert_eq!(pity.infantry_drought, 1);
        assert_eq!(pity.archer_drought, 0);
        assert_eq!(pity.priest_drought, 2);
    }

    #[test]
    fn pity_weighting_increases_spawn_rate_for_starved_kind() {
        let config = RescueConfig {
            spawn_count: 3,
            rescue_radius: 10.0,
            rescue_duration_secs: 1.0,
            recruit_pool: vec![
                RescueRecruitKindConfig::ChristianPeasantInfantry,
                RescueRecruitKindConfig::ChristianPeasantArcher,
                RescueRecruitKindConfig::ChristianPeasantPriest,
            ],
        };
        let baseline = RescueSpawnPity::default();
        let starved_archer = RescueSpawnPity {
            infantry_drought: 0,
            archer_drought: 10,
            priest_drought: 0,
        };
        let mut baseline_archer = 0u32;
        let mut starved_archer_count = 0u32;
        for sequence in 0..240 {
            if recruit_kind_for_sequence(sequence, &config, baseline, PlayerFaction::Christian)
                == RecruitUnitKind::ChristianPeasantArcher
            {
                baseline_archer = baseline_archer.saturating_add(1);
            }
            if recruit_kind_for_sequence(
                sequence,
                &config,
                starved_archer,
                PlayerFaction::Christian,
            ) == RecruitUnitKind::ChristianPeasantArcher
            {
                starved_archer_count = starved_archer_count.saturating_add(1);
            }
        }
        assert!(starved_archer_count > baseline_archer);
    }

    #[test]
    fn rescue_duration_uses_mob_mercy_multiplier_when_active() {
        let base = 4.0;
        let default_duration = effective_rescue_duration(base, None, 1.0);
        let mercy_effects = ConditionalUpgradeEffects {
            rescue_time_multiplier: 0.5,
            ..ConditionalUpgradeEffects::default()
        };
        let mercy_duration = effective_rescue_duration(base, Some(&mercy_effects), 1.0);
        let faction_adjusted_duration = effective_rescue_duration(base, Some(&mercy_effects), 0.9);
        assert!((default_duration - 4.0).abs() < 0.001);
        assert!((mercy_duration - 2.0).abs() < 0.001);
        assert!((faction_adjusted_duration - 1.8).abs() < 0.001);
    }

    #[test]
    fn rescue_spawn_pacing_defaults_are_faster_and_limited() {
        assert!((rescue_respawn_interval_secs() - 12.0).abs() < 0.001);
        assert_eq!(rescue_max_active(), 6);
    }
}
