use bevy::prelude::*;

use crate::banner::BannerCombatModifiers;
use crate::formation::FormationModifiers;
use crate::model::{
    AttackCooldown, AttackProfile, DamageEvent, EnemyUnit, GainXpEvent, GameState, GlobalBuffs,
    Health, Team, Unit, UnitDiedEvent, UnitKind,
};
use crate::morale::CohesionCombatModifiers;

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
    global_buffs: Option<Res<GlobalBuffs>>,
    mut attackers: Query<(&Unit, &mut AttackCooldown)>,
) {
    for (unit, mut cooldown) in &mut attackers {
        let mut speed_scale = 1.0;
        if unit.team == Team::Friendly {
            speed_scale *= cohesion_mods.attack_speed_multiplier;
            if let Some(buff) = &global_buffs {
                speed_scale *= buff.attack_speed_multiplier;
            }
        }
        cooldown.0.tick(std::time::Duration::from_secs_f32(
            time.delta_seconds() * speed_scale,
        ));
    }
}

fn emit_damage_events(
    mut damage_events: EventWriter<DamageEvent>,
    formation_mods: Res<FormationModifiers>,
    cohesion_mods: Res<CohesionCombatModifiers>,
    banner_mods: Res<BannerCombatModifiers>,
    global_buffs: Option<Res<GlobalBuffs>>,
    mut attackers: Query<(
        Entity,
        &Unit,
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

    for (_, attacker_unit, attacker_transform, attack_profile, mut attack_cd) in &mut attackers {
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
            let base_damage = attack_profile.damage;
            let damage = compute_damage(
                base_damage,
                armor,
                attacker_unit.team,
                formation_mods.offense_multiplier,
                cohesion_mods.damage_multiplier,
                banner_mods.attack_multiplier,
                global_buffs
                    .as_ref()
                    .map(|buff| buff.damage_multiplier)
                    .unwrap_or(1.0),
            );

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

pub fn compute_damage(
    base_damage: f32,
    armor: f32,
    source_team: Team,
    formation_offense: f32,
    cohesion_damage_multiplier: f32,
    banner_attack_multiplier: f32,
    global_damage_multiplier: f32,
) -> f32 {
    let mut scaled = base_damage;
    if source_team == Team::Friendly {
        scaled *= formation_offense
            * cohesion_damage_multiplier
            * banner_attack_multiplier
            * global_damage_multiplier;
    }
    (scaled - armor).max(1.0)
}

fn apply_damage_events(
    mut damage_events: EventReader<DamageEvent>,
    mut health_query: Query<&mut Health>,
) {
    for event in damage_events.read() {
        if let Ok(mut health) = health_query.get_mut(event.target) {
            health.current -= event.amount;
        }
    }
}

fn resolve_deaths(
    mut commands: Commands,
    mut death_events: EventWriter<UnitDiedEvent>,
    mut xp_events: EventWriter<GainXpEvent>,
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
                xp_events.send(GainXpEvent(5.0));
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[allow(dead_code)]
fn _satisfy_marker(_enemy: Option<EnemyUnit>) {}

#[cfg(test)]
mod tests {
    use crate::combat::{compute_damage, enemy_target_allowed};
    use crate::model::{Team, UnitKind};

    #[test]
    fn damage_formula_respects_armor_floor() {
        let damage = compute_damage(3.0, 10.0, Team::Friendly, 1.0, 1.0, 1.0, 1.0);
        assert_eq!(damage, 1.0);
    }

    #[test]
    fn damage_formula_applies_multipliers_for_friendlies() {
        let damage = compute_damage(10.0, 0.0, Team::Friendly, 1.1, 0.9, 0.8, 1.2);
        assert!((damage - 9.504).abs() < 0.01);
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
}
