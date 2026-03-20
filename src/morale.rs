use bevy::prelude::*;

use crate::data::GameData;
use crate::model::{
    CommanderUnit, EnemyUnit, FriendlyUnit, GameState, GlobalBuffs, Health, Morale, StartRunEvent,
    Team, UnitDamagedEvent, UnitDiedEvent, UnitKind,
};
use crate::upgrades::ConditionalUpgradeEffects;

const STARTING_COHESION: f32 = 100.0;
const LOW_MORALE_RATIO_THRESHOLD: f32 = 0.5;
const LOW_MORALE_COHESION_DRAIN_PER_SEC: f32 = 3.0;
const STABLE_COHESION_RECOVERY_PER_SEC: f32 = 0.25;
const DAMAGE_TO_UNIT_MORALE_FACTOR: f32 = 0.32;
const DAMAGE_TO_UNIT_MORALE_MIN: f32 = 0.35;
const FRIENDLY_DAMAGE_COHESION_FACTOR: f32 = 0.12;
const FRIENDLY_DAMAGE_ARMY_MORALE_FACTOR: f32 = 0.06;
const FRIENDLY_DAMAGE_ARMY_MORALE_MIN: f32 = 0.12;
const FRIENDLY_DEATH_COHESION_FACTOR: f32 = 0.04;
const FRIENDLY_DEATH_COHESION_MIN: f32 = 2.5;
const FRIENDLY_DEATH_ARMY_MORALE_FACTOR: f32 = 0.05;
const FRIENDLY_DEATH_ARMY_MORALE_MIN: f32 = 3.0;
const COMMANDER_DEATH_PENALTY_MULTIPLIER: f32 = 1.6;
const ENEMY_KILL_REWARD_EVERY_N: u32 = 3;
const ENEMY_KILL_COHESION_GAIN: f32 = 1.0;
const ENEMY_KILL_MORALE_GAIN: f32 = 2.0;
const ENEMY_DEATH_MORALE_LOSS: f32 = 0.8;
const ENEMY_MORALE_GAIN_ON_FRIENDLY_DEATH: f32 = 1.2;
const MAX_AUTHORITY_LOSS_RESISTANCE: f32 = 0.75;

#[derive(Resource, Clone, Copy, Debug, Default)]
struct EnemyKillRewardCounter {
    enemy_deaths: u32,
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
            .add_systems(Update, reset_morale_state_on_run_start)
            .add_systems(
                Update,
                (
                    apply_morale_and_cohesion_events,
                    apply_authority_enemy_morale_drain,
                    apply_hospitalier_aura_regen,
                    apply_low_morale_cohesion_pressure,
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
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    cohesion.value = STARTING_COHESION;
    *modifiers = cohesion_modifiers(cohesion.value);
    kill_counter.enemy_deaths = 0;
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn apply_morale_and_cohesion_events(
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    mut damaged_events: EventReader<UnitDamagedEvent>,
    mut death_events: EventReader<UnitDiedEvent>,
    mut cohesion: ResMut<Cohesion>,
    mut kill_counter: ResMut<EnemyKillRewardCounter>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    transforms: Query<&Transform>,
    mut morale_sets: ParamSet<(
        Query<&mut Morale>,
        Query<&mut Morale, With<FriendlyUnit>>,
        Query<&mut Morale, With<EnemyUnit>>,
    )>,
) {
    let friendly_loss_immunity = conditional_effects
        .as_deref()
        .map(|effects| effects.friendly_loss_immunity)
        .unwrap_or(false);
    let aura_context = commanders.get_single().ok().map(|transform| {
        (
            transform.translation.truncate(),
            commander_aura_radius(&data, buffs.as_deref()),
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
        let loss_multiplier =
            friendly_loss_multiplier_from_authority(friendly_in_aura, buffs.as_deref());
        if let Ok(mut morale) = morale_sets.p0().get_mut(event.target) {
            let morale_loss = unit_morale_loss_from_damage(event.amount)
                * if event.team == Team::Friendly {
                    loss_multiplier
                } else {
                    1.0
                };
            if !(friendly_loss_immunity && event.team == Team::Friendly) {
                morale.current = (morale.current - morale_loss).clamp(0.0, morale.max);
            }
        }
        if event.team == Team::Friendly && !friendly_loss_immunity {
            cohesion.value -= friendly_cohesion_loss_from_damage(event.amount) * loss_multiplier;
            friendly_morale_loss +=
                friendly_army_morale_loss_from_damage(event.amount) * loss_multiplier;
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
                    cohesion.value += ENEMY_KILL_COHESION_GAIN;
                    friendly_morale_gain += ENEMY_KILL_MORALE_GAIN;
                }
                enemy_morale_loss += ENEMY_DEATH_MORALE_LOSS;
            }
            Team::Friendly => {
                let in_aura = in_commander_aura(event.world_position, aura_context);
                let loss_multiplier =
                    friendly_loss_multiplier_from_authority(in_aura, buffs.as_deref());
                if !friendly_loss_immunity {
                    cohesion.value -= friendly_death_cohesion_loss(event.max_health, event.kind)
                        * loss_multiplier;
                    friendly_morale_loss +=
                        friendly_death_morale_loss(event.max_health, event.kind) * loss_multiplier;
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

fn apply_authority_enemy_morale_drain(
    time: Res<Time>,
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut enemies: Query<(&Transform, &mut Morale), With<EnemyUnit>>,
) {
    let Some(buffs) = buffs.as_ref() else {
        return;
    };
    if buffs.authority_enemy_morale_drain_per_sec <= 0.0 {
        return;
    }
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let aura_radius = commander_aura_radius(&data, Some(buffs));
    let commander_position = commander_transform.translation.truncate();
    let morale_drain = buffs.authority_enemy_morale_drain_per_sec * time.delta_seconds();
    for (transform, mut morale) in &mut enemies {
        let in_aura = in_commander_aura(
            transform.translation.truncate(),
            Some((commander_position, aura_radius)),
        );
        if !in_aura {
            continue;
        }
        morale.current = (morale.current - morale_drain).clamp(0.0, morale.max);
    }
}

fn apply_hospitalier_aura_regen(
    time: Res<Time>,
    data: Res<GameData>,
    buffs: Option<Res<GlobalBuffs>>,
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
    let aura_radius = commander_aura_radius(&data, Some(buffs));
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
        health.current =
            (health.current + buffs.hospitalier_hp_regen_per_sec * dt).clamp(0.0, health.max);
        morale.current =
            (morale.current + buffs.hospitalier_morale_regen_per_sec * dt).clamp(0.0, morale.max);
    }

    if total_count > 0 && in_aura_count > 0 {
        let coverage = in_aura_count as f32 / total_count as f32;
        cohesion.value += buffs.hospitalier_cohesion_regen_per_sec * dt * coverage;
        cohesion.value = cohesion.value.clamp(0.0, 100.0);
    }
}

fn apply_low_morale_cohesion_pressure(
    time: Res<Time>,
    mut cohesion: ResMut<Cohesion>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    retinue_morale: Query<&Morale, (With<FriendlyUnit>, Without<CommanderUnit>)>,
) {
    if conditional_effects
        .as_deref()
        .map(|effects| effects.friendly_loss_immunity)
        .unwrap_or(false)
    {
        return;
    }
    let ratios: Vec<f32> = retinue_morale.iter().map(|morale| morale.ratio()).collect();
    let low_ratio = low_morale_ratio(&ratios, LOW_MORALE_RATIO_THRESHOLD);
    if low_ratio >= LOW_MORALE_RATIO_THRESHOLD {
        cohesion.value -= LOW_MORALE_COHESION_DRAIN_PER_SEC * time.delta_seconds();
    } else {
        cohesion.value += STABLE_COHESION_RECOVERY_PER_SEC * time.delta_seconds();
    }
    cohesion.value = cohesion.value.clamp(0.0, 100.0);
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

pub fn unit_morale_loss_from_damage(damage: f32) -> f32 {
    (damage.max(0.0) * DAMAGE_TO_UNIT_MORALE_FACTOR).max(DAMAGE_TO_UNIT_MORALE_MIN)
}

pub fn friendly_cohesion_loss_from_damage(damage: f32) -> f32 {
    damage.max(0.0) * FRIENDLY_DAMAGE_COHESION_FACTOR
}

pub fn friendly_army_morale_loss_from_damage(damage: f32) -> f32 {
    (damage.max(0.0) * FRIENDLY_DAMAGE_ARMY_MORALE_FACTOR).max(FRIENDLY_DAMAGE_ARMY_MORALE_MIN)
}

pub fn friendly_death_cohesion_loss(max_health: f32, kind: UnitKind) -> f32 {
    let base =
        (max_health.max(1.0) * FRIENDLY_DEATH_COHESION_FACTOR).max(FRIENDLY_DEATH_COHESION_MIN);
    if kind == UnitKind::Commander {
        base * COMMANDER_DEATH_PENALTY_MULTIPLIER
    } else {
        base
    }
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

pub fn commander_aura_radius(data: &GameData, buffs: Option<&GlobalBuffs>) -> f32 {
    let base_radius = data.units.commander.aura_radius.max(0.0);
    let bonus = buffs
        .map(|value| value.commander_aura_radius_bonus)
        .unwrap_or(0.0);
    (base_radius + bonus).max(0.0)
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

pub fn cohesion_modifiers(value: f32) -> CohesionCombatModifiers {
    if value >= 80.0 {
        CohesionCombatModifiers {
            damage_multiplier: 1.08,
            defense_multiplier: 1.05,
            attack_speed_multiplier: 1.08,
            collapse_risk: false,
        }
    } else if value >= 60.0 {
        CohesionCombatModifiers {
            damage_multiplier: 1.0,
            defense_multiplier: 1.0,
            attack_speed_multiplier: 1.0,
            collapse_risk: false,
        }
    } else if value >= 40.0 {
        CohesionCombatModifiers {
            damage_multiplier: 0.9,
            defense_multiplier: 0.93,
            attack_speed_multiplier: 0.9,
            collapse_risk: false,
        }
    } else if value >= 20.0 {
        CohesionCombatModifiers {
            damage_multiplier: 0.8,
            defense_multiplier: 0.86,
            attack_speed_multiplier: 0.8,
            collapse_risk: false,
        }
    } else {
        CohesionCombatModifiers {
            damage_multiplier: 0.7,
            defense_multiplier: 0.8,
            attack_speed_multiplier: 0.7,
            collapse_risk: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::UnitKind;
    use crate::morale::{
        Cohesion, average_morale_ratio, cohesion_modifiers, friendly_army_morale_loss_from_damage,
        friendly_cohesion_loss_from_damage, friendly_death_cohesion_loss,
        friendly_death_morale_loss, friendly_loss_multiplier_from_authority, low_morale_ratio,
        should_apply_enemy_kill_reward, unit_morale_loss_from_damage,
    };
    use crate::{data::GameData, model::GlobalBuffs, morale::commander_aura_radius};

    #[test]
    fn cohesion_starts_full() {
        assert!((Cohesion::default().value - 100.0).abs() < 0.0001);
    }

    #[test]
    fn high_cohesion_has_positive_bonus() {
        let modifiers = cohesion_modifiers(90.0);
        assert!(modifiers.damage_multiplier > 1.0);
        assert!(!modifiers.collapse_risk);
    }

    #[test]
    fn low_cohesion_triggers_collapse_risk() {
        let modifiers = cohesion_modifiers(10.0);
        assert!(modifiers.collapse_risk);
        assert!(modifiers.damage_multiplier < 1.0);
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

        assert!(commander_cohesion > recruit_cohesion);
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
        assert!(commander_aura_radius(&data, Some(&buffs)) > data.units.commander.aura_radius);
    }
}
