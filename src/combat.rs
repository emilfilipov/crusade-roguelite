use bevy::prelude::*;

use crate::formation::FormationModifiers;
use crate::model::{
    AttackCooldown, AttackProfile, DamageEvent, EnemyUnit, GameState, GlobalBuffs, Health, Morale,
    SpawnExpPackEvent, Team, Unit, UnitDamagedEvent, UnitDiedEvent, UnitKind,
};
use crate::morale::CohesionCombatModifiers;
use crate::upgrades::Progression;

pub const MIN_FRIENDLY_COMBAT_MULTIPLIER: f32 = 0.55;
const LOW_MORALE_THRESHOLD: f32 = 0.5;
const LOW_MORALE_MIN_MULTIPLIER: f32 = 0.75;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_attack_timers,
                emit_damage_events,
                apply_damage_events,
                resolve_deaths,
            )
                .run_if(in_state(GameState::InRun)),
        );
    }
}

fn tick_attack_timers(
    time: Res<Time>,
    cohesion_mods: Res<CohesionCombatModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    mut attackers: Query<(&Unit, Option<&Morale>, &mut AttackCooldown)>,
) {
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    for (unit, morale, mut cooldown) in &mut attackers {
        let morale_scale = morale
            .copied()
            .map(|value| morale_effect_multiplier(value.ratio()))
            .unwrap_or(1.0);

        let speed_scale = if unit.team == Team::Friendly {
            let mut value = cohesion_mods.attack_speed_multiplier * morale_scale * level_multiplier;
            if let Some(buff) = &global_buffs {
                value *= buff.attack_speed_multiplier;
            }
            value.max(MIN_FRIENDLY_COMBAT_MULTIPLIER)
        } else {
            morale_scale
        };

        cooldown.0.tick(std::time::Duration::from_secs_f32(
            time.delta_seconds() * speed_scale,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn emit_damage_events(
    mut damage_events: EventWriter<DamageEvent>,
    formation_mods: Res<FormationModifiers>,
    cohesion_mods: Res<CohesionCombatModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    mut attackers: Query<(
        Entity,
        &Unit,
        Option<&Morale>,
        &Transform,
        &AttackProfile,
        &mut AttackCooldown,
    )>,
    targets: Query<(
        Entity,
        &Unit,
        &Transform,
        &Health,
        Option<&crate::model::Armor>,
    )>,
) {
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    let target_snapshot: Vec<(Entity, Unit, Vec2, f32, f32)> = targets
        .iter()
        .map(|(entity, unit, transform, health, armor)| {
            (
                entity,
                *unit,
                transform.translation.truncate(),
                health.current,
                armor.map(|value| value.0).unwrap_or(0.0),
            )
        })
        .collect();
    let has_non_commander_friendlies = target_snapshot.iter().any(|(_, unit, _, health, _)| {
        unit.team == Team::Friendly && unit.kind != UnitKind::Commander && *health > 0.0
    });

    for (_, attacker_unit, attacker_morale, attacker_transform, attack_profile, mut attack_cd) in
        &mut attackers
    {
        if !attack_cd.0.finished() {
            continue;
        }

        let attacker_position = attacker_transform.translation.truncate();
        let opposite_team = match attacker_unit.team {
            Team::Friendly => Team::Enemy,
            Team::Enemy => Team::Friendly,
            Team::Neutral => continue,
        };

        let mut closest_target: Option<(Entity, f32, f32)> = None;
        for (target_entity, target_unit, target_pos, target_health, target_armor) in
            &target_snapshot
        {
            if target_unit.team != opposite_team || *target_health <= 0.0 {
                continue;
            }
            if !enemy_target_allowed(
                attacker_unit.team,
                target_unit.kind,
                has_non_commander_friendlies,
            ) {
                continue;
            }
            let dist_sq = attacker_position.distance_squared(*target_pos);
            if dist_sq <= attack_profile.range * attack_profile.range {
                let candidate = (*target_entity, dist_sq, *target_armor);
                match closest_target {
                    Some((_, best_dist, _)) if dist_sq >= best_dist => {}
                    _ => closest_target = Some(candidate),
                }
            }
        }

        if let Some((target_entity, _, armor)) = closest_target {
            attack_cd.0.reset();
            let morale_multiplier = attacker_morale
                .copied()
                .map(|value| morale_effect_multiplier(value.ratio()))
                .unwrap_or(1.0);
            let outgoing_multiplier = if attacker_unit.team == Team::Friendly {
                friendly_outgoing_multiplier(
                    formation_mods.offense_multiplier,
                    cohesion_mods.damage_multiplier,
                    global_buffs
                        .as_ref()
                        .map(|buff| buff.damage_multiplier)
                        .unwrap_or(1.0),
                    level_multiplier,
                    morale_multiplier,
                )
            } else {
                morale_multiplier
            };

            let damage = compute_damage(attack_profile.damage, armor, outgoing_multiplier);

            damage_events.send(DamageEvent {
                target: target_entity,
                source_team: attacker_unit.team,
                amount: damage,
            });
        }
    }
}

pub fn enemy_target_allowed(
    attacker_team: Team,
    target_kind: UnitKind,
    has_non_commander_friendlies: bool,
) -> bool {
    if attacker_team == Team::Enemy
        && has_non_commander_friendlies
        && target_kind == UnitKind::Commander
    {
        return false;
    }
    true
}

pub fn commander_level_combat_multiplier(level: u32) -> f32 {
    1.0 + level.saturating_sub(1) as f32 * 0.01
}

pub fn morale_effect_multiplier(morale_ratio: f32) -> f32 {
    if morale_ratio >= LOW_MORALE_THRESHOLD {
        return 1.0;
    }
    let normalized = (morale_ratio / LOW_MORALE_THRESHOLD).clamp(0.0, 1.0);
    LOW_MORALE_MIN_MULTIPLIER + normalized * (1.0 - LOW_MORALE_MIN_MULTIPLIER)
}

pub fn friendly_outgoing_multiplier(
    formation_offense: f32,
    cohesion_damage_multiplier: f32,
    global_damage_multiplier: f32,
    commander_level_multiplier: f32,
    morale_multiplier: f32,
) -> f32 {
    (formation_offense
        * cohesion_damage_multiplier
        * global_damage_multiplier
        * commander_level_multiplier
        * morale_multiplier)
        .max(MIN_FRIENDLY_COMBAT_MULTIPLIER)
}

pub fn compute_damage(base_damage: f32, armor: f32, outgoing_multiplier: f32) -> f32 {
    (base_damage * outgoing_multiplier - armor).max(1.0)
}

fn apply_damage_events(
    mut damage_events: EventReader<DamageEvent>,
    mut damaged_events: EventWriter<UnitDamagedEvent>,
    mut health_query: Query<(&mut Health, &Unit)>,
) {
    for event in damage_events.read() {
        if let Ok((mut health, unit)) = health_query.get_mut(event.target) {
            health.current -= event.amount;
            damaged_events.send(UnitDamagedEvent {
                target: event.target,
                team: unit.team,
                amount: event.amount,
            });
        }
    }
}

fn resolve_deaths(
    mut commands: Commands,
    mut death_events: EventWriter<UnitDiedEvent>,
    mut exp_pack_events: EventWriter<SpawnExpPackEvent>,
    dead_units: Query<(Entity, &Unit, &Health, &Transform)>,
) {
    for (entity, unit, health, transform) in &dead_units {
        if health.current <= 0.0 {
            death_events.send(UnitDiedEvent {
                team: unit.team,
                kind: unit.kind,
                world_position: transform.translation.truncate(),
            });
            if unit.team == Team::Enemy {
                exp_pack_events.send(SpawnExpPackEvent {
                    world_position: transform.translation.truncate(),
                    xp_value_override: None,
                    pickup_delay_secs: Some(0.45),
                });
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[allow(dead_code)]
fn _satisfy_marker(_enemy: Option<EnemyUnit>) {}

#[cfg(test)]
mod tests {
    use crate::combat::{
        commander_level_combat_multiplier, compute_damage, enemy_target_allowed,
        friendly_outgoing_multiplier, morale_effect_multiplier,
    };
    use crate::model::{Team, UnitKind};

    #[test]
    fn damage_formula_respects_armor_floor() {
        let damage = compute_damage(3.0, 10.0, 1.0);
        assert_eq!(damage, 1.0);
    }

    #[test]
    fn low_morale_scales_down_to_minimum_multiplier() {
        assert!((morale_effect_multiplier(1.0) - 1.0).abs() < 0.0001);
        assert!((morale_effect_multiplier(0.5) - 1.0).abs() < 0.0001);
        assert!((morale_effect_multiplier(0.25) - 0.875).abs() < 0.0001);
        assert!((morale_effect_multiplier(0.0) - 0.75).abs() < 0.0001);
    }

    #[test]
    fn friendly_multiplier_has_floor() {
        let multiplier = friendly_outgoing_multiplier(0.6, 0.7, 0.8, 0.9, 0.75);
        assert!((multiplier - 0.55).abs() < 0.0001);
    }

    #[test]
    fn enemies_prioritize_retinue_over_commander() {
        assert!(!enemy_target_allowed(
            Team::Enemy,
            UnitKind::Commander,
            true
        ));
        assert!(enemy_target_allowed(
            Team::Enemy,
            UnitKind::Commander,
            false
        ));
        assert!(enemy_target_allowed(
            Team::Enemy,
            UnitKind::InfantryKnight,
            true
        ));
    }

    #[test]
    fn commander_level_multiplier_gains_one_percent_per_level() {
        assert!((commander_level_combat_multiplier(1) - 1.0).abs() < 0.0001);
        assert!((commander_level_combat_multiplier(8) - 1.07).abs() < 0.0001);
    }
}
