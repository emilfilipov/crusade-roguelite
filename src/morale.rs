use bevy::prelude::*;

use crate::banner::BannerState;
use crate::data::GameData;
use crate::formation::{ActiveFormation, active_formation_config, formation_contains_position};
use crate::inventory::{
    EquipmentArmyEffects, InventoryState, UnitCombatRole,
    commander_armywide_bonuses_with_banner_state,
};
use crate::model::{
    CommanderUnit, EnemyUnit, FriendlyUnit, GameState, GlobalBuffs, Health, MatchSetupSelection,
    Morale, PlayerFaction, StartRunEvent,
};
use crate::rescue::spawn_rescuable_entity;
use crate::upgrades::ConditionalUpgradeEffects;
use crate::visuals::ArtAssets;

// --- Core single-morale tuning ---------------------------------------------------------------

const MORALE_NEUTRAL_THRESHOLD_RATIO: f32 = 0.51;
const MORALE_LOW_THRESHOLD_RATIO: f32 = 0.50;
const MORALE_FULL_THRESHOLD_RATIO: f32 = 1.0;

const FULL_MORALE_DAMAGE_BONUS_MAX: f32 = 0.08; // +8% at 100 morale
const FULL_MORALE_ARMOR_BONUS_MAX: f32 = 0.08; // +8% at 100 morale
const FULL_MORALE_HP_REGEN_MAX_HP_RATIO_PER_SEC: f32 = 0.004; // 0.4% max HP/s at 100 morale

const LOW_MORALE_ARMOR_DEBUFF_MAX: f32 = 0.12; // -12% at 0 morale
const LOW_MORALE_ESCAPE_SPEED_BONUS_MAX: f32 = 0.16; // +16% at 0 morale

const PRESSURE_DELAY_SECS: f32 = 3.0;
const PRESSURE_MORALE_DRAIN_PER_SEC_MAX: f32 = 1.1;
const SAFE_MORALE_RECOVERY_PER_SEC: f32 = 0.30;
const ENCIRCLEMENT_FORMATION_PADDING_SLOTS: f32 = 0.35;

const COLLAPSE_CASUALTY_RATIO: f32 = 0.10;
const COLLAPSE_RESET_DELAY_SECS: f32 = 3.0;
const COLLAPSE_RESET_RATIO: f32 = 0.70;
const COLLAPSE_GRACE_SECS: f32 = 6.0;

const MAX_AUTHORITY_LOSS_RESISTANCE: f32 = 0.75;

// Threshold edges for UX notifications.
// We treat these as "crossing points" in [0, 1] morale ratio space.
const MORALE_THRESHOLD_EDGES: [f32; 4] = [0.25, 0.50, 0.80, 1.00];

// --- Events ----------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MoraleThresholdDirection {
    Rising,
    Falling,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct MoraleThresholdCrossedEvent {
    pub threshold_ratio: f32,
    pub direction: MoraleThresholdDirection,
    pub current_average_ratio: f32,
}

// --- Runtime state ---------------------------------------------------------------------------

#[derive(Resource, Clone, Copy, Debug, Default)]
struct MoralePressureState {
    pressure_secs: f32,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct MoraleCollapseState {
    grace_remaining: f32,
    reset_delay_remaining: f32,
    reset_pending: bool,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct MoraleThresholdTracker {
    initialized: bool,
    bucket: u8,
}

// --- Plugin ----------------------------------------------------------------------------------

pub struct MoralePlugin;

impl Plugin for MoralePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MoralePressureState>()
            .init_resource::<MoraleCollapseState>()
            .init_resource::<MoraleThresholdTracker>()
            .add_event::<MoraleThresholdCrossedEvent>()
            .add_systems(Update, reset_morale_state_on_run_start)
            .add_systems(
                Update,
                (
                    apply_encirclement_morale_pressure,
                    apply_full_morale_regen,
                    apply_morale_collapse,
                    emit_player_morale_threshold_events,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_morale_state_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut pressure: ResMut<MoralePressureState>,
    mut collapse: ResMut<MoraleCollapseState>,
    mut threshold_tracker: ResMut<MoraleThresholdTracker>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    pressure.pressure_secs = 0.0;
    collapse.grace_remaining = 0.0;
    collapse.reset_delay_remaining = 0.0;
    collapse.reset_pending = false;

    threshold_tracker.initialized = false;
    threshold_tracker.bucket = 0;
}

// --- Main systems ----------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn apply_encirclement_morale_pressure(
    time: Res<Time>,
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    inventory: Option<Res<InventoryState>>,
    buffs: Option<Res<GlobalBuffs>>,
    equipment_effects: Option<Res<EquipmentArmyEffects>>,
    banner_state: Option<Res<BannerState>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    enemies_for_pressure: Query<&Transform, (With<EnemyUnit>, Without<FriendlyUnit>)>,
    mut enemies: Query<(&Transform, &mut Morale), (With<EnemyUnit>, Without<FriendlyUnit>)>,
    retinue: Query<&Transform, (With<FriendlyUnit>, Without<CommanderUnit>)>,
    mut pressure: ResMut<MoralePressureState>,
    mut friendlies: Query<(&Transform, &mut Morale), (With<FriendlyUnit>, Without<EnemyUnit>)>,
) {
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };

    let dt = time.delta_seconds().max(0.0);
    if dt <= f32::EPSILON {
        return;
    }

    let commander_pos = commander_transform.translation.truncate();
    let retinue_count = retinue.iter().count();
    let slot_spacing = active_formation_config(&data, *active_formation).slot_spacing;
    let inside_enemy_count = enemies_for_pressure
        .iter()
        .filter(|enemy| {
            formation_contains_position(
                *active_formation,
                commander_pos,
                enemy.translation.truncate(),
                retinue_count,
                slot_spacing,
                ENCIRCLEMENT_FORMATION_PADDING_SLOTS,
            )
        })
        .count();

    let pressure_ratio = if retinue_count == 0 {
        0.0
    } else {
        (inside_enemy_count as f32 / retinue_count as f32).clamp(0.0, 1.0)
    };

    if pressure_ratio > 0.0 {
        pressure.pressure_secs += dt;
    } else {
        pressure.pressure_secs = 0.0;
    }

    let player_faction = selected_player_faction(setup_selection.as_deref());
    let faction_mods = data.factions.for_faction(player_faction);
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    let commander_bonuses = inventory
        .as_deref()
        .map(|inv| {
            commander_armywide_bonuses_with_banner_state(
                inv,
                UnitCombatRole::Commander,
                banner_item_active,
            )
        })
        .unwrap_or_default();
    let aura_radius = commander_aura_radius(
        &data,
        buffs.as_deref(),
        inventory.as_deref(),
        player_faction,
        banner_item_active,
    );
    let aura_context = (aura_radius > 0.0).then_some((commander_pos, aura_radius));
    let conditional_loss = conditional_effects
        .as_deref()
        .map(|v| v.friendly_morale_loss_multiplier)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let has_pressure = pressure_ratio > 0.0;
    let pressure_drain_active = has_pressure && pressure.pressure_secs >= PRESSURE_DELAY_SECS;

    let enemy_aura_drain_per_sec = authority_enemy_morale_drain_per_sec(
        buffs.as_deref(),
        faction_mods.authority_enemy_morale_drain_multiplier,
        commander_bonuses.aura_enemy_effect_bonus_multiplier,
    );
    if enemy_aura_drain_per_sec > f32::EPSILON {
        let enemy_delta = enemy_aura_drain_per_sec * dt;
        for (enemy_transform, mut enemy_morale) in &mut enemies {
            if !in_commander_aura(enemy_transform.translation.truncate(), aura_context) {
                continue;
            }
            enemy_morale.current =
                (enemy_morale.current - enemy_delta).clamp(0.0, enemy_morale.max);
        }
    }

    for (friendly_transform, mut morale) in &mut friendlies {
        let in_aura = in_commander_aura(friendly_transform.translation.truncate(), aura_context);
        let authority_loss_multiplier =
            friendly_loss_multiplier_from_authority(in_aura, buffs.as_deref());
        let hospitalier_morale_regen_per_sec = if in_aura {
            buffs
                .as_deref()
                .map(|value| value.hospitalier_morale_regen_per_sec)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let effective_loss_multiplier = effective_friendly_morale_loss_multiplier(
            conditional_loss,
            faction_mods.friendly_morale_loss_multiplier,
            authority_loss_multiplier,
            commander_bonuses.morale_loss_resistance,
            equipment_effects
                .as_deref()
                .map(|effects| effects.morale_loss_immunity)
                .unwrap_or(false),
        );
        let passive_regen_per_sec =
            commander_bonuses.morale_regen_per_sec + hospitalier_morale_regen_per_sec;
        let morale_delta_per_sec = encirclement_morale_delta_per_sec(
            pressure_ratio,
            has_pressure,
            pressure_drain_active,
            faction_mods.friendly_morale_gain_multiplier,
            passive_regen_per_sec,
            effective_loss_multiplier,
        );
        if morale_delta_per_sec.abs() <= f32::EPSILON {
            continue;
        }
        morale.current = (morale.current + morale_delta_per_sec * dt).clamp(0.0, morale.max);
    }
}

fn apply_full_morale_regen(time: Res<Time>, mut units: Query<(&Morale, &mut Health)>) {
    let dt = time.delta_seconds().max(0.0);
    if dt <= f32::EPSILON {
        return;
    }

    for (morale, mut health) in &mut units {
        let ratio = morale.ratio();
        let regen_per_sec = morale_hp_regen_per_sec(health.max, ratio);
        if regen_per_sec <= 0.0 {
            continue;
        }
        health.current = (health.current + regen_per_sec * dt).clamp(0.0, health.max);
    }
}

#[allow(clippy::type_complexity)]
fn apply_morale_collapse(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<ArtAssets>,
    mut collapse: ResMut<MoraleCollapseState>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut all_friendlies_morale: Query<&mut Morale, With<FriendlyUnit>>,
    retinue: Query<
        (Entity, &Transform, &crate::model::Unit),
        (With<FriendlyUnit>, Without<CommanderUnit>),
    >,
) {
    let dt = time.delta_seconds().max(0.0);
    collapse.grace_remaining = (collapse.grace_remaining - dt).max(0.0);

    if collapse.reset_pending {
        collapse.reset_delay_remaining = (collapse.reset_delay_remaining - dt).max(0.0);
        if collapse.reset_delay_remaining <= 0.0 {
            for mut morale in &mut all_friendlies_morale {
                morale.current = (morale.max * COLLAPSE_RESET_RATIO).clamp(0.0, morale.max);
            }
            collapse.reset_pending = false;
            collapse.grace_remaining = COLLAPSE_GRACE_SECS;
        }
        return;
    }

    let morale_ratios: Vec<f32> = all_friendlies_morale
        .iter()
        .map(|morale| morale.ratio())
        .collect();
    let avg_ratio = average_morale_ratio(&morale_ratios).clamp(0.0, 1.0);

    if avg_ratio > 0.0 || collapse.grace_remaining > 0.0 {
        return;
    }

    let commander_pos = commanders
        .get_single()
        .map(|t| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);

    let mut candidates: Vec<(Entity, Vec2, Option<crate::model::RecruitUnitKind>, f32)> = retinue
        .iter()
        .map(|(entity, transform, unit)| {
            let position = transform.translation.truncate();
            (
                entity,
                position,
                unit.kind.as_recruit_unit_kind(),
                position.distance_squared(commander_pos),
            )
        })
        .collect();

    candidates.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
    let casualties = collapse_casualty_count(candidates.len());

    for (entity, position, recruit_kind, _) in candidates.into_iter().take(casualties) {
        if let Some(kind) = recruit_kind {
            spawn_rescuable_entity(&mut commands, position, kind, &art);
        }
        commands.entity(entity).despawn_recursive();
    }

    collapse.reset_pending = true;
    collapse.reset_delay_remaining = COLLAPSE_RESET_DELAY_SECS;
}

fn emit_player_morale_threshold_events(
    friendlies: Query<&Morale, With<FriendlyUnit>>,
    mut tracker: ResMut<MoraleThresholdTracker>,
    mut writer: EventWriter<MoraleThresholdCrossedEvent>,
) {
    let ratios: Vec<f32> = friendlies.iter().map(|m| m.ratio()).collect();
    let avg_ratio = average_morale_ratio(&ratios).clamp(0.0, 1.0);
    let bucket = threshold_bucket(avg_ratio);

    if !tracker.initialized {
        tracker.initialized = true;
        tracker.bucket = bucket;
        return;
    }

    if bucket == tracker.bucket {
        return;
    }

    let crossed = crossed_thresholds(tracker.bucket, bucket);
    for (threshold_ratio, direction) in crossed {
        writer.send(MoraleThresholdCrossedEvent {
            threshold_ratio,
            direction,
            current_average_ratio: avg_ratio,
        });
    }

    tracker.bucket = bucket;
}

// --- Public helper API -----------------------------------------------------------------------

pub fn morale_bonus_scale(morale_ratio: f32) -> f32 {
    let r = morale_ratio.clamp(0.0, 1.0);
    if r < MORALE_NEUTRAL_THRESHOLD_RATIO {
        return 0.0;
    }
    ((r - MORALE_NEUTRAL_THRESHOLD_RATIO)
        / (MORALE_FULL_THRESHOLD_RATIO - MORALE_NEUTRAL_THRESHOLD_RATIO))
        .clamp(0.0, 1.0)
}

pub fn morale_penalty_scale(morale_ratio: f32) -> f32 {
    let r = morale_ratio.clamp(0.0, 1.0);
    if r >= MORALE_LOW_THRESHOLD_RATIO {
        return 0.0;
    }
    ((MORALE_LOW_THRESHOLD_RATIO - r) / MORALE_LOW_THRESHOLD_RATIO).clamp(0.0, 1.0)
}

pub fn morale_damage_multiplier(morale_ratio: f32) -> f32 {
    1.0 + FULL_MORALE_DAMAGE_BONUS_MAX * morale_bonus_scale(morale_ratio)
}

pub fn morale_armor_multiplier(morale_ratio: f32) -> f32 {
    let bonus = FULL_MORALE_ARMOR_BONUS_MAX * morale_bonus_scale(morale_ratio);
    let penalty = LOW_MORALE_ARMOR_DEBUFF_MAX * morale_penalty_scale(morale_ratio);
    (1.0 + bonus - penalty).max(0.1)
}

pub fn morale_movement_multiplier(morale_ratio: f32) -> f32 {
    1.0 + LOW_MORALE_ESCAPE_SPEED_BONUS_MAX * morale_penalty_scale(morale_ratio)
}

pub fn morale_hp_regen_per_sec(max_hp: f32, morale_ratio: f32) -> f32 {
    max_hp.max(0.0) * FULL_MORALE_HP_REGEN_MAX_HP_RATIO_PER_SEC * morale_bonus_scale(morale_ratio)
}

pub fn average_morale_ratio(morale_ratios: &[f32]) -> f32 {
    if morale_ratios.is_empty() {
        return 1.0;
    }
    morale_ratios.iter().sum::<f32>() / morale_ratios.len() as f32
}

pub fn low_morale_ratio(morale_ratios: &[f32], threshold: f32) -> f32 {
    if morale_ratios.is_empty() {
        return 0.0;
    }
    let low_count = morale_ratios.iter().filter(|v| **v < threshold).count();
    low_count as f32 / morale_ratios.len() as f32
}

pub fn morale_threshold_message(
    threshold_ratio: f32,
    direction: MoraleThresholdDirection,
) -> String {
    let pct = (threshold_ratio * 100.0).round() as i32;
    match direction {
        MoraleThresholdDirection::Falling => match pct {
            100 => "Morale no longer inspired".to_string(),
            80 => "Morale wavering".to_string(),
            50 => "Morale faltering".to_string(),
            25 => "Morale collapsing".to_string(),
            _ => format!("Morale dropped below {}%", pct),
        },
        MoraleThresholdDirection::Rising => match pct {
            25 => "Morale recovering".to_string(),
            50 => "Morale steadying".to_string(),
            80 => "Morale restored".to_string(),
            100 => "Morale inspired".to_string(),
            _ => format!("Morale rose above {}%", pct),
        },
    }
}

// --- Existing utility API retained ------------------------------------------------------------

fn selected_player_faction(setup_selection: Option<&MatchSetupSelection>) -> PlayerFaction {
    setup_selection
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian)
}

pub fn commander_aura_radius(
    data: &GameData,
    buffs: Option<&GlobalBuffs>,
    inventory: Option<&InventoryState>,
    player_faction: PlayerFaction,
    banner_item_active: bool,
) -> f32 {
    let base_radius = data
        .units
        .commander_for_faction(player_faction)
        .aura_radius
        .max(0.0);
    let faction_bonus = data
        .factions
        .for_faction(player_faction)
        .commander_aura_radius_bonus;
    let upgrade_bonus = buffs
        .map(|value| value.commander_aura_radius_bonus)
        .unwrap_or(0.0);
    let gear_bonus = inventory
        .map(|inv| {
            commander_armywide_bonuses_with_banner_state(
                inv,
                UnitCombatRole::Commander,
                banner_item_active,
            )
            .aura_radius_bonus
        })
        .unwrap_or(0.0);
    (base_radius + faction_bonus + upgrade_bonus + gear_bonus).max(0.0)
}

pub fn in_commander_aura(position: Vec2, aura_context: Option<(Vec2, f32)>) -> bool {
    let Some((commander_position, aura_radius)) = aura_context else {
        return false;
    };
    position.distance_squared(commander_position) <= aura_radius * aura_radius
}

pub fn friendly_loss_multiplier_from_authority(in_aura: bool, buffs: Option<&GlobalBuffs>) -> f32 {
    if !in_aura {
        return 1.0;
    }
    let resistance = buffs
        .map(|v| v.authority_friendly_loss_resistance)
        .unwrap_or(0.0)
        .clamp(0.0, MAX_AUTHORITY_LOSS_RESISTANCE);
    (1.0 - resistance).clamp(0.1, 1.0)
}

pub fn effective_friendly_morale_loss_multiplier(
    conditional_loss_multiplier: f32,
    faction_loss_multiplier: f32,
    authority_loss_multiplier: f32,
    gear_morale_loss_resistance: f32,
    morale_loss_immunity: bool,
) -> f32 {
    if morale_loss_immunity {
        return 0.0;
    }
    let gear_loss_multiplier =
        (1.0 - gear_morale_loss_resistance.clamp(0.0, 0.95)).clamp(0.05, 1.0);
    conditional_loss_multiplier.clamp(0.0, 1.0)
        * faction_loss_multiplier.max(0.0)
        * authority_loss_multiplier.max(0.0)
        * gear_loss_multiplier
}

pub fn encirclement_morale_delta_per_sec(
    pressure_ratio: f32,
    has_pressure: bool,
    pressure_drain_active: bool,
    faction_gain_multiplier: f32,
    passive_regen_per_sec: f32,
    effective_loss_multiplier: f32,
) -> f32 {
    let pressure = pressure_ratio.clamp(0.0, 1.0);
    let safe_recovery_per_sec = if !has_pressure {
        SAFE_MORALE_RECOVERY_PER_SEC * faction_gain_multiplier.max(0.0)
    } else {
        0.0
    };
    let pressure_loss_per_sec = if pressure_drain_active && pressure > 0.0 {
        PRESSURE_MORALE_DRAIN_PER_SEC_MAX * pressure * effective_loss_multiplier.max(0.0)
    } else {
        0.0
    };
    safe_recovery_per_sec + passive_regen_per_sec.max(0.0) - pressure_loss_per_sec
}

pub fn authority_enemy_morale_drain_per_sec(
    buffs: Option<&GlobalBuffs>,
    faction_authority_multiplier: f32,
    aura_enemy_effect_bonus_multiplier: f32,
) -> f32 {
    let base_drain_per_sec = buffs
        .map(|value| value.authority_enemy_morale_drain_per_sec)
        .unwrap_or(0.0)
        .max(0.0);
    let aura_multiplier = (1.0 + aura_enemy_effect_bonus_multiplier).max(0.0);
    base_drain_per_sec * faction_authority_multiplier.max(0.0) * aura_multiplier
}

// --- Threshold helpers -----------------------------------------------------------------------

fn threshold_bucket(ratio: f32) -> u8 {
    let r = ratio.clamp(0.0, 1.0);
    if r < 0.25 {
        0
    } else if r < 0.50 {
        1
    } else if r < 0.80 {
        2
    } else if r < 1.0 {
        3
    } else {
        4
    }
}

fn crossed_thresholds(from_bucket: u8, to_bucket: u8) -> Vec<(f32, MoraleThresholdDirection)> {
    if from_bucket == to_bucket {
        return Vec::new();
    }

    let mut out = Vec::new();
    if to_bucket > from_bucket {
        // Rising: emit in ascending order.
        for edge_index in from_bucket as usize..to_bucket as usize {
            if let Some(edge) = MORALE_THRESHOLD_EDGES.get(edge_index) {
                out.push((*edge, MoraleThresholdDirection::Rising));
            }
        }
    } else {
        // Falling: emit in descending order.
        for edge_index in (to_bucket as usize..from_bucket as usize).rev() {
            if let Some(edge) = MORALE_THRESHOLD_EDGES.get(edge_index) {
                out.push((*edge, MoraleThresholdDirection::Falling));
            }
        }
    }
    out
}

fn collapse_casualty_count(retinue_count: usize) -> usize {
    if retinue_count == 0 {
        return 0;
    }
    ((retinue_count as f32 * COLLAPSE_CASUALTY_RATIO).ceil() as usize).clamp(1, retinue_count)
}

// --- Tests -----------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bonus_scale_is_zero_below_neutral_zone_and_one_at_full() {
        assert!((morale_bonus_scale(0.50) - 0.0).abs() < 0.001);
        assert!((morale_bonus_scale(0.51) - 0.0).abs() < 0.001);
        assert!((morale_bonus_scale(1.00) - 1.0).abs() < 0.001);
    }

    #[test]
    fn penalty_scale_is_zero_at_or_above_half_and_one_at_zero() {
        assert!((morale_penalty_scale(0.50) - 0.0).abs() < 0.001);
        assert!((morale_penalty_scale(0.25) - 0.5).abs() < 0.001);
        assert!((morale_penalty_scale(0.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn movement_multiplier_is_escape_bonus_under_half() {
        assert!((morale_movement_multiplier(0.50) - 1.0).abs() < 0.001);
        assert!((morale_movement_multiplier(0.25) - 1.08).abs() < 0.001);
        assert!((morale_movement_multiplier(0.0) - 1.16).abs() < 0.001);
    }

    #[test]
    fn thresholds_emit_all_crossed_edges_in_direction_order() {
        let rising = crossed_thresholds(1, 4);
        assert_eq!(
            rising,
            vec![
                (0.50, MoraleThresholdDirection::Rising),
                (0.80, MoraleThresholdDirection::Rising),
                (1.00, MoraleThresholdDirection::Rising),
            ]
        );

        let falling = crossed_thresholds(4, 1);
        assert_eq!(
            falling,
            vec![
                (1.00, MoraleThresholdDirection::Falling),
                (0.80, MoraleThresholdDirection::Falling),
                (0.50, MoraleThresholdDirection::Falling),
            ]
        );
    }

    #[test]
    fn collapse_casualty_is_ten_percent_with_min_one() {
        assert_eq!(collapse_casualty_count(0), 0);
        assert_eq!(collapse_casualty_count(1), 1);
        assert_eq!(collapse_casualty_count(10), 1);
        assert_eq!(collapse_casualty_count(40), 4);
    }

    #[test]
    fn average_ratio_can_reach_zero() {
        assert!((average_morale_ratio(&[0.0, 0.0, 0.0]) - 0.0).abs() < 0.001);
    }

    #[test]
    fn friendly_loss_multiplier_respects_resistance_and_immunity() {
        let reduced = effective_friendly_morale_loss_multiplier(0.8, 1.1, 0.9, 0.2, false);
        assert!((reduced - 0.6336).abs() < 0.001);

        let immune = effective_friendly_morale_loss_multiplier(1.0, 1.0, 1.0, 0.0, true);
        assert_eq!(immune, 0.0);
    }

    #[test]
    fn encirclement_delta_combines_recovery_regen_and_drain() {
        let safe = encirclement_morale_delta_per_sec(0.0, false, false, 1.1, 0.2, 1.0);
        assert!((safe - 0.53).abs() < 0.001);

        let draining = encirclement_morale_delta_per_sec(1.0, true, true, 1.1, 0.2, 0.6);
        assert!((draining + 0.46).abs() < 0.001);
    }

    #[test]
    fn authority_enemy_drain_scales_with_faction_and_aura_bonus() {
        let buffs = GlobalBuffs {
            authority_enemy_morale_drain_per_sec: 2.0,
            ..GlobalBuffs::default()
        };
        let drain = authority_enemy_morale_drain_per_sec(Some(&buffs), 1.2, 0.25);
        assert!((drain - 3.0).abs() < 0.001);
    }
}
