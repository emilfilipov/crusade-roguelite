use bevy::prelude::*;

use crate::banner::BannerState;
use crate::data::GameData;
use crate::formation::{ActiveFormation, active_formation_config, formation_contains_position};
use crate::inventory::{
    EquipmentArmyEffects, InventoryState, UnitCombatRole,
    commander_armywide_bonuses_with_banner_state, gear_bonuses_for_unit_with_banner_state,
};
use crate::model::{
    CommanderUnit, EnemyUnit, FriendlyUnit, GameState, GlobalBuffs, Health, MatchSetupSelection,
    Morale, PlayerFaction, StartRunEvent, Team, UnitCohesion, UnitDamagedEvent, UnitDiedEvent,
    UnitKind, UnitTier,
};
use crate::rescue::spawn_rescuable_entity;
use crate::upgrades::ConditionalUpgradeEffects;
use crate::visuals::ArtAssets;

const STARTING_COHESION: f32 = 100.0;
const LOW_MORALE_THRESHOLD: f32 = 0.5;
const LOW_MORALE_MIN_MOVEMENT_MULTIPLIER: f32 = 0.75;
const DAMAGE_TO_UNIT_MORALE_FACTOR: f32 = 0.32;
const DAMAGE_TO_UNIT_MORALE_MIN: f32 = 0.35;
const FRIENDLY_DAMAGE_COHESION_FACTOR: f32 = 0.12;
const FRIENDLY_DEATH_ARMY_MORALE_FACTOR: f32 = 0.05;
const FRIENDLY_DEATH_ARMY_MORALE_MIN: f32 = 3.0;
const COMMANDER_DEATH_PENALTY_MULTIPLIER: f32 = 1.6;
const ENEMY_KILL_REWARD_EVERY_N: u32 = 3;
const ENEMY_KILL_COHESION_GAIN: f32 = 1.0;
const ENEMY_KILL_MORALE_GAIN: f32 = 2.0;
const ENEMY_DEATH_MORALE_LOSS: f32 = 0.8;
const ENEMY_MORALE_GAIN_ON_FRIENDLY_DEATH: f32 = 1.2;
const MAX_AUTHORITY_LOSS_RESISTANCE: f32 = 0.75;
const MAX_GEAR_LOSS_RESISTANCE: f32 = 0.75;
const ENCIRCLEMENT_FORMATION_PADDING_SLOTS: f32 = 0.35;
const ENCIRCLEMENT_MORALE_DRAIN_PER_SEC_MAX: f32 = 3.0;
const ENCIRCLEMENT_MORALE_RECOVERY_PER_SEC: f32 = 0.8;
const BANNER_DROPPED_MORALE_DRAIN_PER_SEC: f32 = 0.7;
const COHESION_COLLAPSE_CASUALTY_RATIO: f32 = 0.10;
const COHESION_COLLAPSE_RESET_VALUE: f32 = 70.0;
const COHESION_COLLAPSE_GRACE_SECS: f32 = 6.0;
const COHESION_DAMAGE_DEBUFF_MIN: f32 = 0.65;
const HOSPITALIER_MAX_HP_REGEN_PER_SEC_RATIO: f32 = 0.03;

#[derive(Resource, Clone, Copy, Debug, Default)]
struct EnemyKillRewardCounter {
    enemy_deaths: u32,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct CohesionCollapseState {
    grace_remaining: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct Cohesion {
    pub value: f32,
}

impl Default for Cohesion {
    fn default() -> Self {
        Self {
            value: STARTING_COHESION,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct CohesionCombatModifiers {
    pub damage_multiplier: f32,
    pub defense_multiplier: f32,
    pub attack_speed_multiplier: f32,
    pub collapse_risk: bool,
}

impl Default for CohesionCombatModifiers {
    fn default() -> Self {
        Self {
            damage_multiplier: 1.0,
            defense_multiplier: 1.0,
            attack_speed_multiplier: 1.0,
            collapse_risk: false,
        }
    }
}

pub struct MoralePlugin;

impl Plugin for MoralePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cohesion>()
            .init_resource::<CohesionCombatModifiers>()
            .init_resource::<EnemyKillRewardCounter>()
            .init_resource::<CohesionCollapseState>()
            .add_systems(Update, reset_morale_state_on_run_start)
            .add_systems(
                Update,
                (
                    apply_morale_and_cohesion_events,
                    apply_authority_enemy_morale_drain,
                    apply_hospitalier_aura_regen,
                    apply_friendly_gear_regen,
                    apply_encirclement_morale_pressure,
                    apply_cohesion_collapse,
                    refresh_cohesion_modifiers,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_morale_state_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut cohesion: ResMut<Cohesion>,
    mut modifiers: ResMut<CohesionCombatModifiers>,
    mut kill_counter: ResMut<EnemyKillRewardCounter>,
    mut collapse_state: ResMut<CohesionCollapseState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    cohesion.value = STARTING_COHESION;
    *modifiers = cohesion_modifiers(cohesion.value);
    kill_counter.enemy_deaths = 0;
    collapse_state.grace_remaining = 0.0;
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn apply_morale_and_cohesion_events(
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    buffs: Option<Res<GlobalBuffs>>,
    banner_state: Option<Res<BannerState>>,
    inventory: Option<Res<InventoryState>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    equipment_effects: Option<Res<EquipmentArmyEffects>>,
    mut damaged_events: EventReader<UnitDamagedEvent>,
    mut death_events: EventReader<UnitDiedEvent>,
    mut cohesion: ResMut<Cohesion>,
    mut kill_counter: ResMut<EnemyKillRewardCounter>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    transforms: Query<&Transform>,
    units: Query<(&crate::model::Unit, Option<&UnitTier>)>,
    mut morale_sets: ParamSet<(
        Query<&mut Morale>,
        Query<&mut Morale, With<FriendlyUnit>>,
        Query<&mut Morale, With<EnemyUnit>>,
    )>,
) {
    let player_faction = selected_player_faction(setup_selection.as_deref());
    let faction_mods = data.factions.for_faction(player_faction);
    let banner_item_active = banner_item_bonuses_active(banner_state.as_deref());
    let conditional_morale_loss_multiplier = conditional_effects
        .as_deref()
        .map(|effects| effects.friendly_morale_loss_multiplier)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let conditional_cohesion_loss_multiplier = conditional_effects
        .as_deref()
        .map(|effects| effects.friendly_cohesion_loss_multiplier)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let friendly_morale_only_immunity = equipment_effects
        .as_deref()
        .map(|effects| effects.morale_loss_immunity)
        .unwrap_or(false);
    let friendly_morale_immunity = friendly_morale_only_immunity;
    let aura_context = commanders.get_single().ok().map(|transform| {
        (
            transform.translation.truncate(),
            commander_aura_radius(
                &data,
                buffs.as_deref(),
                inventory.as_deref(),
                player_faction,
                banner_item_active,
            ),
        )
    });
    let mut friendly_morale_gain = 0.0;
    let mut friendly_morale_loss = 0.0;
    let mut enemy_morale_gain = 0.0;
    let mut enemy_morale_loss = 0.0;

    for event in damaged_events.read() {
        let friendly_in_aura = if event.team == Team::Friendly {
            transforms
                .get(event.target)
                .ok()
                .map(|transform| in_commander_aura(transform.translation.truncate(), aura_context))
                .unwrap_or(false)
        } else {
            false
        };
        let authority_loss_multiplier =
            friendly_loss_multiplier_from_authority(friendly_in_aura, buffs.as_deref());
        let (gear_morale_loss_multiplier, gear_cohesion_loss_multiplier) =
            if event.team == Team::Friendly {
                units
                    .get(event.target)
                    .ok()
                    .map(|(unit, tier)| {
                        let gear = inventory
                            .as_deref()
                            .map(|inv| {
                                gear_bonuses_for_unit_with_banner_state(
                                    inv,
                                    unit.kind,
                                    tier.map(|value| value.0),
                                    banner_item_active,
                                )
                            })
                            .unwrap_or_default();
                        (
                            loss_multiplier_from_gear_resistance(gear.morale_loss_resistance),
                            loss_multiplier_from_gear_resistance(gear.cohesion_loss_resistance),
                        )
                    })
                    .unwrap_or((1.0, 1.0))
            } else {
                (1.0, 1.0)
            };
        if let Ok(mut morale) = morale_sets.p0().get_mut(event.target) {
            let should_apply_unit_morale_damage_loss = event.team == Team::Enemy;
            if should_apply_unit_morale_damage_loss {
                let morale_loss = unit_morale_loss_from_damage(event.amount)
                    * if event.team == Team::Friendly {
                        authority_loss_multiplier
                            * gear_morale_loss_multiplier
                            * faction_mods.friendly_morale_loss_multiplier
                    } else {
                        1.0
                    };
                morale.current = (morale.current - morale_loss).clamp(0.0, morale.max);
            }
        }
        if event.team == Team::Friendly {
            cohesion.value -= friendly_cohesion_loss_from_damage(event.amount)
                * authority_loss_multiplier
                * gear_cohesion_loss_multiplier
                * conditional_cohesion_loss_multiplier
                * faction_mods.friendly_cohesion_loss_multiplier;
        }
    }

    for event in death_events.read() {
        match event.team {
            Team::Enemy => {
                kill_counter.enemy_deaths = kill_counter.enemy_deaths.saturating_add(1);
                if should_apply_enemy_kill_reward(
                    kill_counter.enemy_deaths,
                    ENEMY_KILL_REWARD_EVERY_N,
                ) {
                    cohesion.value +=
                        ENEMY_KILL_COHESION_GAIN * faction_mods.friendly_cohesion_gain_multiplier;
                    friendly_morale_gain +=
                        ENEMY_KILL_MORALE_GAIN * faction_mods.friendly_morale_gain_multiplier;
                }
                enemy_morale_loss += ENEMY_DEATH_MORALE_LOSS;
            }
            Team::Friendly => {
                let in_aura = in_commander_aura(event.world_position, aura_context);
                let authority_loss_multiplier =
                    friendly_loss_multiplier_from_authority(in_aura, buffs.as_deref());
                let death_role = combat_role_for_unit_kind(event.kind);
                let armywide = inventory
                    .as_deref()
                    .map(|inv| {
                        commander_armywide_bonuses_with_banner_state(
                            inv,
                            death_role,
                            banner_item_active,
                        )
                    })
                    .unwrap_or_default();
                let gear_morale_loss_multiplier =
                    loss_multiplier_from_gear_resistance(armywide.morale_loss_resistance);
                if !friendly_morale_immunity {
                    friendly_morale_loss +=
                        friendly_death_morale_loss(event.max_health, event.kind)
                            * authority_loss_multiplier
                            * gear_morale_loss_multiplier
                            * conditional_morale_loss_multiplier
                            * faction_mods.friendly_morale_loss_multiplier;
                }
                enemy_morale_gain += ENEMY_MORALE_GAIN_ON_FRIENDLY_DEATH;
            }
            Team::Neutral => {}
        }
    }

    if (friendly_morale_gain - friendly_morale_loss).abs() > 0.0001 {
        for mut morale in &mut morale_sets.p1() {
            morale.current = (morale.current + friendly_morale_gain - friendly_morale_loss)
                .clamp(0.0, morale.max);
        }
    }
    if (enemy_morale_gain - enemy_morale_loss).abs() > 0.0001 {
        for mut morale in &mut morale_sets.p2() {
            morale.current =
                (morale.current + enemy_morale_gain - enemy_morale_loss).clamp(0.0, morale.max);
        }
    }

    cohesion.value = cohesion.value.clamp(0.0, 100.0);
}

#[allow(clippy::too_many_arguments)]
fn apply_authority_enemy_morale_drain(
    time: Res<Time>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    buffs: Option<Res<GlobalBuffs>>,
    banner_state: Option<Res<BannerState>>,
    inventory: Option<Res<InventoryState>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut enemies: Query<(&Transform, &mut Morale, Option<&mut UnitCohesion>), With<EnemyUnit>>,
) {
    let Some(buffs) = buffs.as_ref() else {
        return;
    };
    let player_faction = selected_player_faction(setup_selection.as_deref());
    let faction_mods = data.factions.for_faction(player_faction);
    let banner_item_active = banner_item_bonuses_active(banner_state.as_deref());
    if buffs.authority_enemy_morale_drain_per_sec <= 0.0
        && faction_mods.authority_enemy_cohesion_drain_per_sec <= 0.0
    {
        return;
    }
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let aura_radius = commander_aura_radius(
        &data,
        Some(buffs),
        inventory.as_deref(),
        player_faction,
        banner_item_active,
    );
    let aura_enemy_effect_multiplier = (1.0
        + inventory
            .as_deref()
            .map(|inv| {
                commander_armywide_bonuses_with_banner_state(
                    inv,
                    UnitCombatRole::Commander,
                    banner_item_active,
                )
            })
            .unwrap_or_default()
            .aura_enemy_effect_bonus_multiplier)
        .max(0.0);
    let commander_position = commander_transform.translation.truncate();
    let dt = time.delta_seconds();
    let morale_drain = buffs.authority_enemy_morale_drain_per_sec
        * faction_mods.authority_enemy_morale_drain_multiplier
        * aura_enemy_effect_multiplier
        * dt;
    let cohesion_drain =
        faction_mods.authority_enemy_cohesion_drain_per_sec * aura_enemy_effect_multiplier * dt;
    for (transform, mut morale, maybe_cohesion) in &mut enemies {
        let in_aura = in_commander_aura(
            transform.translation.truncate(),
            Some((commander_position, aura_radius)),
        );
        if !in_aura {
            continue;
        }
        if morale_drain > 0.0 {
            morale.current = (morale.current - morale_drain).clamp(0.0, morale.max);
        }
        if cohesion_drain > 0.0
            && let Some(mut cohesion) = maybe_cohesion
        {
            cohesion.current = (cohesion.current - cohesion_drain).clamp(0.0, cohesion.max);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_hospitalier_aura_regen(
    time: Res<Time>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    buffs: Option<Res<GlobalBuffs>>,
    banner_state: Option<Res<BannerState>>,
    inventory: Option<Res<InventoryState>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut cohesion: ResMut<Cohesion>,
    mut friendlies: Query<(&Transform, &mut Health, &mut Morale), With<FriendlyUnit>>,
) {
    let Some(buffs) = buffs.as_ref() else {
        return;
    };
    if buffs.hospitalier_hp_regen_per_sec <= 0.0
        && buffs.hospitalier_morale_regen_per_sec <= 0.0
        && buffs.hospitalier_cohesion_regen_per_sec <= 0.0
    {
        return;
    }
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let player_faction = selected_player_faction(setup_selection.as_deref());
    let banner_item_active = banner_item_bonuses_active(banner_state.as_deref());
    let faction_mods = data.factions.for_faction(player_faction);
    let aura_radius = commander_aura_radius(
        &data,
        Some(buffs),
        inventory.as_deref(),
        player_faction,
        banner_item_active,
    );
    let commander_position = commander_transform.translation.truncate();
    let dt = time.delta_seconds();

    let mut total_count = 0u32;
    let mut in_aura_count = 0u32;
    for (transform, mut health, mut morale) in &mut friendlies {
        total_count = total_count.saturating_add(1);
        if !in_commander_aura(
            transform.translation.truncate(),
            Some((commander_position, aura_radius)),
        ) {
            continue;
        }
        in_aura_count = in_aura_count.saturating_add(1);
        let capped_hp_regen = buffs
            .hospitalier_hp_regen_per_sec
            .min(health.max * HOSPITALIER_MAX_HP_REGEN_PER_SEC_RATIO);
        health.current = (health.current + capped_hp_regen * dt).clamp(0.0, health.max);
        morale.current = (morale.current
            + buffs.hospitalier_morale_regen_per_sec
                * faction_mods.friendly_morale_gain_multiplier
                * dt)
            .clamp(0.0, morale.max);
    }

    if total_count > 0 && in_aura_count > 0 {
        let coverage = in_aura_count as f32 / total_count as f32;
        cohesion.value += buffs.hospitalier_cohesion_regen_per_sec
            * faction_mods.friendly_cohesion_gain_multiplier
            * dt
            * coverage;
        cohesion.value = cohesion.value.clamp(0.0, 100.0);
    }
}

fn apply_friendly_gear_regen(
    time: Res<Time>,
    banner_state: Option<Res<BannerState>>,
    inventory: Option<Res<InventoryState>>,
    mut cohesion: ResMut<Cohesion>,
    mut friendlies: Query<
        (&crate::model::Unit, Option<&UnitTier>, &mut Morale),
        With<FriendlyUnit>,
    >,
) {
    let Some(inventory) = inventory.as_deref() else {
        return;
    };
    let dt = time.delta_seconds().max(0.0);
    if dt <= f32::EPSILON {
        return;
    }
    let banner_item_active = banner_item_bonuses_active(banner_state.as_deref());
    let mut cohesion_regen_sum = 0.0;
    let mut friendly_count = 0u32;
    for (unit, tier, mut morale) in &mut friendlies {
        let gear = gear_bonuses_for_unit_with_banner_state(
            inventory,
            unit.kind,
            tier.map(|value| value.0),
            banner_item_active,
        );
        if gear.morale_regen_per_sec > 0.0 {
            morale.current =
                (morale.current + gear.morale_regen_per_sec * dt).clamp(0.0, morale.max);
        }
        cohesion_regen_sum += gear.cohesion_regen_per_sec.max(0.0);
        friendly_count = friendly_count.saturating_add(1);
    }
    if friendly_count > 0 {
        cohesion.value += cohesion_regen_sum / friendly_count as f32 * dt;
        cohesion.value = cohesion.value.clamp(0.0, 100.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_encirclement_morale_pressure(
    time: Res<Time>,
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    banner_state: Option<Res<BannerState>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    enemies: Query<&Transform, With<EnemyUnit>>,
    retinue: Query<&Transform, (With<FriendlyUnit>, Without<CommanderUnit>)>,
    mut friendly_morale: Query<&mut Morale, With<FriendlyUnit>>,
) {
    let conditional_morale_loss_multiplier = conditional_effects
        .as_deref()
        .map(|effects| effects.friendly_morale_loss_multiplier)
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let commander_position = commander_transform.translation.truncate();
    let retinue_count = retinue.iter().count();
    let slot_spacing = active_formation_config(&data, *active_formation).slot_spacing;
    let inside_enemy_count = enemies
        .iter()
        .filter(|enemy| {
            formation_contains_position(
                *active_formation,
                commander_position,
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
    let player_faction = selected_player_faction(setup_selection.as_deref());
    let faction_mods = data.factions.for_faction(player_faction);
    let dt = time.delta_seconds();
    let mut morale_delta = 0.0;
    if pressure_ratio > 0.0 {
        morale_delta -= ENCIRCLEMENT_MORALE_DRAIN_PER_SEC_MAX
            * pressure_ratio
            * conditional_morale_loss_multiplier
            * faction_mods.friendly_morale_loss_multiplier
            * dt;
    } else {
        morale_delta += ENCIRCLEMENT_MORALE_RECOVERY_PER_SEC
            * faction_mods.friendly_morale_gain_multiplier
            * dt;
    }
    if banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false)
    {
        morale_delta -= BANNER_DROPPED_MORALE_DRAIN_PER_SEC
            * conditional_morale_loss_multiplier
            * faction_mods.friendly_morale_loss_multiplier
            * dt;
    }
    if morale_delta.abs() <= f32::EPSILON {
        return;
    }
    for mut morale in &mut friendly_morale {
        morale.current = (morale.current + morale_delta).clamp(0.0, morale.max);
    }
}

#[allow(clippy::type_complexity)]
fn apply_cohesion_collapse(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<ArtAssets>,
    mut cohesion: ResMut<Cohesion>,
    mut collapse_state: ResMut<CohesionCollapseState>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    retinue: Query<
        (Entity, &Transform, &crate::model::Unit),
        (With<FriendlyUnit>, Without<CommanderUnit>),
    >,
) {
    collapse_state.grace_remaining =
        (collapse_state.grace_remaining - time.delta_seconds()).max(0.0);
    if cohesion.value > 0.0 || collapse_state.grace_remaining > 0.0 {
        return;
    }

    let commander_pos = commanders
        .get_single()
        .map(|transform| transform.translation.truncate())
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
    let casualties = cohesion_collapse_casualty_count(candidates.len());
    for (entity, position, recruit_kind, _) in candidates.into_iter().take(casualties) {
        if let Some(recruit_kind) = recruit_kind {
            spawn_rescuable_entity(&mut commands, position, recruit_kind, &art);
        }
        commands.entity(entity).despawn_recursive();
    }

    cohesion.value = COHESION_COLLAPSE_RESET_VALUE;
    collapse_state.grace_remaining = COHESION_COLLAPSE_GRACE_SECS;
}

fn refresh_cohesion_modifiers(
    cohesion: Res<Cohesion>,
    mut modifiers: ResMut<CohesionCombatModifiers>,
) {
    *modifiers = cohesion_modifiers(cohesion.value);
}

pub fn low_morale_ratio(morale_ratios: &[f32], threshold: f32) -> f32 {
    if morale_ratios.is_empty() {
        return 0.0;
    }
    let low_count = morale_ratios
        .iter()
        .filter(|ratio| **ratio < threshold)
        .count();
    low_count as f32 / morale_ratios.len() as f32
}

pub fn morale_movement_multiplier(morale_ratio: f32) -> f32 {
    if morale_ratio >= LOW_MORALE_THRESHOLD {
        return 1.0;
    }
    let normalized = (morale_ratio / LOW_MORALE_THRESHOLD).clamp(0.0, 1.0);
    LOW_MORALE_MIN_MOVEMENT_MULTIPLIER + normalized * (1.0 - LOW_MORALE_MIN_MOVEMENT_MULTIPLIER)
}

pub fn unit_morale_loss_from_damage(damage: f32) -> f32 {
    (damage.max(0.0) * DAMAGE_TO_UNIT_MORALE_FACTOR).max(DAMAGE_TO_UNIT_MORALE_MIN)
}

pub fn friendly_cohesion_loss_from_damage(damage: f32) -> f32 {
    damage.max(0.0) * FRIENDLY_DAMAGE_COHESION_FACTOR
}

pub fn friendly_army_morale_loss_from_damage(damage: f32) -> f32 {
    (damage.max(0.0) * 0.06).max(0.12)
}

pub fn friendly_death_cohesion_loss(max_health: f32, kind: UnitKind) -> f32 {
    let _ = (max_health, kind);
    0.0
}

pub fn friendly_death_morale_loss(max_health: f32, kind: UnitKind) -> f32 {
    let base = (max_health.max(1.0) * FRIENDLY_DEATH_ARMY_MORALE_FACTOR)
        .max(FRIENDLY_DEATH_ARMY_MORALE_MIN);
    if kind == UnitKind::Commander {
        base * COMMANDER_DEATH_PENALTY_MULTIPLIER
    } else {
        base
    }
}

pub fn should_apply_enemy_kill_reward(enemy_death_count: u32, reward_every_n: u32) -> bool {
    reward_every_n > 0 && enemy_death_count > 0 && enemy_death_count.is_multiple_of(reward_every_n)
}

pub fn average_morale_ratio(morale_ratios: &[f32]) -> f32 {
    if morale_ratios.is_empty() {
        return 1.0;
    }
    morale_ratios.iter().sum::<f32>() / morale_ratios.len() as f32
}

fn selected_player_faction(setup_selection: Option<&MatchSetupSelection>) -> PlayerFaction {
    setup_selection
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian)
}

fn banner_item_bonuses_active(banner_state: Option<&BannerState>) -> bool {
    !banner_state.map(|state| state.is_dropped).unwrap_or(false)
}

fn cohesion_collapse_casualty_count(retinue_count: usize) -> usize {
    if retinue_count == 0 {
        return 0;
    }
    ((retinue_count as f32 * COHESION_COLLAPSE_CASUALTY_RATIO).ceil() as usize)
        .clamp(1, retinue_count)
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
        .map(|value| value.authority_friendly_loss_resistance)
        .unwrap_or(0.0)
        .clamp(0.0, MAX_AUTHORITY_LOSS_RESISTANCE);
    (1.0 - resistance).clamp(0.1, 1.0)
}

fn loss_multiplier_from_gear_resistance(resistance: f32) -> f32 {
    (1.0 - resistance.clamp(0.0, MAX_GEAR_LOSS_RESISTANCE)).clamp(0.1, 1.0)
}

fn combat_role_for_unit_kind(kind: UnitKind) -> UnitCombatRole {
    match kind {
        UnitKind::Commander => UnitCombatRole::Commander,
        UnitKind::ChristianPeasantArcher | UnitKind::MuslimPeasantArcher => UnitCombatRole::Ranged,
        UnitKind::ChristianPeasantPriest | UnitKind::MuslimPeasantPriest => UnitCombatRole::Support,
        _ => UnitCombatRole::Melee,
    }
}

pub fn cohesion_modifiers(value: f32) -> CohesionCombatModifiers {
    let clamped = value.clamp(0.0, 100.0);
    let damage_multiplier = if clamped >= 50.0 {
        1.0
    } else {
        let normalized = (clamped / 50.0).clamp(0.0, 1.0);
        COHESION_DAMAGE_DEBUFF_MIN + normalized * (1.0 - COHESION_DAMAGE_DEBUFF_MIN)
    };
    CohesionCombatModifiers {
        damage_multiplier,
        defense_multiplier: 1.0,
        attack_speed_multiplier: 1.0,
        collapse_risk: clamped <= 0.0,
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::{
        EnemyKillRewardCounter, apply_morale_and_cohesion_events, cohesion_collapse_casualty_count,
        morale_movement_multiplier,
    };
    use crate::model::{
        FriendlyUnit, Morale, Team, Unit, UnitDamagedEvent, UnitDiedEvent, UnitKind,
    };
    use crate::morale::{
        Cohesion, average_morale_ratio, cohesion_modifiers, friendly_army_morale_loss_from_damage,
        friendly_cohesion_loss_from_damage, friendly_death_cohesion_loss,
        friendly_death_morale_loss, friendly_loss_multiplier_from_authority, low_morale_ratio,
        should_apply_enemy_kill_reward, unit_morale_loss_from_damage,
    };
    use crate::{
        data::GameData, inventory::InventoryState, model::GlobalBuffs,
        morale::commander_aura_radius, upgrades::ConditionalUpgradeEffects,
    };

    #[test]
    fn cohesion_starts_full() {
        assert!((Cohesion::default().value - 100.0).abs() < 0.0001);
    }

    #[test]
    fn high_cohesion_has_positive_bonus() {
        let modifiers = cohesion_modifiers(90.0);
        assert!((modifiers.damage_multiplier - 1.0).abs() < 0.001);
        assert!(!modifiers.collapse_risk);
    }

    #[test]
    fn low_cohesion_triggers_collapse_risk() {
        let modifiers = cohesion_modifiers(10.0);
        assert!(!modifiers.collapse_risk);
        assert!(modifiers.damage_multiplier < 1.0);
        assert!(cohesion_modifiers(0.0).collapse_risk);
    }

    #[test]
    fn low_morale_ratio_counts_sub_threshold_members() {
        let morale = [0.9, 0.4, 0.2, 0.8];
        let ratio = low_morale_ratio(&morale, 0.5);
        assert!((ratio - 0.5).abs() < 0.0001);
    }

    #[test]
    fn average_morale_ratio_returns_mean_or_one_when_empty() {
        let morale = [0.8, 0.6, 0.4];
        assert!((average_morale_ratio(&morale) - 0.6).abs() < 0.0001);
        assert!((average_morale_ratio(&[]) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn damage_losses_scale_with_damage_amount() {
        assert!(unit_morale_loss_from_damage(12.0) > unit_morale_loss_from_damage(3.0));
        assert!(friendly_cohesion_loss_from_damage(12.0) > friendly_cohesion_loss_from_damage(3.0));
        assert!(
            friendly_army_morale_loss_from_damage(12.0)
                > friendly_army_morale_loss_from_damage(3.0)
        );
    }

    #[test]
    fn friendly_death_penalty_scales_with_health_and_commander_kind() {
        let recruit_cohesion =
            friendly_death_cohesion_loss(95.0, UnitKind::ChristianPeasantInfantry);
        let commander_cohesion = friendly_death_cohesion_loss(120.0, UnitKind::Commander);
        let recruit_morale = friendly_death_morale_loss(95.0, UnitKind::ChristianPeasantInfantry);
        let commander_morale = friendly_death_morale_loss(120.0, UnitKind::Commander);

        assert!((commander_cohesion - 0.0).abs() < 0.001);
        assert!((recruit_cohesion - 0.0).abs() < 0.001);
        assert!(commander_morale > recruit_morale);
    }

    #[test]
    fn enemy_kill_rewards_apply_every_third_kill() {
        assert!(!should_apply_enemy_kill_reward(1, 3));
        assert!(!should_apply_enemy_kill_reward(2, 3));
        assert!(should_apply_enemy_kill_reward(3, 3));
        assert!(!should_apply_enemy_kill_reward(4, 3));
        assert!(should_apply_enemy_kill_reward(6, 3));
    }

    #[test]
    fn authority_loss_multiplier_applies_only_in_aura() {
        let buffs = GlobalBuffs {
            authority_friendly_loss_resistance: 0.25,
            ..GlobalBuffs::default()
        };
        assert!((friendly_loss_multiplier_from_authority(false, Some(&buffs)) - 1.0).abs() < 0.001);
        assert!((friendly_loss_multiplier_from_authority(true, Some(&buffs)) - 0.75).abs() < 0.001);
    }

    #[test]
    fn commander_aura_radius_includes_upgrade_bonus() {
        let data = GameData::load_from_dir(std::path::Path::new("assets/data")).expect("load data");
        let buffs = GlobalBuffs {
            commander_aura_radius_bonus: 20.0,
            ..GlobalBuffs::default()
        };
        let inventory = InventoryState::default();
        assert!(
            commander_aura_radius(
                &data,
                Some(&buffs),
                Some(&inventory),
                crate::model::PlayerFaction::Christian,
                true,
            ) > data.units.commander_christian.aura_radius
        );
    }

    #[test]
    fn morale_movement_penalty_caps_at_twenty_five_percent() {
        assert!((morale_movement_multiplier(1.0) - 1.0).abs() < 0.001);
        assert!((morale_movement_multiplier(0.5) - 1.0).abs() < 0.001);
        assert!((morale_movement_multiplier(0.25) - 0.875).abs() < 0.001);
        assert!((morale_movement_multiplier(0.0) - 0.75).abs() < 0.001);
    }

    #[test]
    fn cohesion_collapse_casualties_are_ten_percent_with_minimum_one() {
        assert_eq!(cohesion_collapse_casualty_count(0), 0);
        assert_eq!(cohesion_collapse_casualty_count(1), 1);
        assert_eq!(cohesion_collapse_casualty_count(10), 1);
        assert_eq!(cohesion_collapse_casualty_count(40), 4);
    }

    #[test]
    fn fury_mitigation_reduces_friendly_cohesion_losses_from_damage_events() {
        let mut app = App::new();
        app.add_event::<UnitDamagedEvent>();
        app.add_event::<UnitDiedEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(GlobalBuffs::default());
        app.insert_resource(ConditionalUpgradeEffects {
            friendly_morale_loss_multiplier: 0.75,
            friendly_cohesion_loss_multiplier: 0.75,
            ..ConditionalUpgradeEffects::default()
        });
        app.insert_resource(Cohesion { value: 100.0 });
        app.insert_resource(EnemyKillRewardCounter::default());
        app.add_systems(Update, apply_morale_and_cohesion_events);

        let friendly = app
            .world_mut()
            .spawn((
                FriendlyUnit,
                Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianPeasantInfantry,
                    level: 1,
                },
                Morale::new(100.0),
                Transform::default(),
            ))
            .id();

        app.world_mut()
            .resource_mut::<Events<UnitDamagedEvent>>()
            .send(UnitDamagedEvent {
                target: friendly,
                team: Team::Friendly,
                amount: 25.0,
            });
        app.update();

        let morale = app
            .world()
            .entity(friendly)
            .get::<Morale>()
            .copied()
            .expect("morale");
        let cohesion = app.world().resource::<Cohesion>().value;
        assert!((morale.current - 100.0).abs() < 0.001);
        let expected = 100.0 - (friendly_cohesion_loss_from_damage(25.0) * 0.75 * 0.88);
        assert!((cohesion - expected).abs() < 0.01);
    }
}
