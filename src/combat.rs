use bevy::prelude::*;

use crate::data::GameData;
use crate::formation::{
    ActiveFormation, FormationModifiers, active_formation_config, formation_contains_position,
};
use crate::inventory::{InventoryState, gear_bonuses_for_unit};
use crate::model::{
    AttackCooldown, AttackProfile, DamageEvent, DamageTextEvent, EnemyUnit, GameState, GlobalBuffs,
    Health, Morale, SpawnExpPackEvent, Team, Unit, UnitDamagedEvent, UnitDiedEvent, UnitKind,
    UnitTier,
};
use crate::morale::CohesionCombatModifiers;
use crate::projectiles::Projectile;
use crate::squad::{
    CommanderMotionState, PriestAttackSpeedBlessing, priest_attack_speed_multiplier,
};
use crate::upgrades::{ConditionalUpgradeEffects, Progression};
use crate::visuals::ArtAssets;

pub const MIN_FRIENDLY_COMBAT_MULTIPLIER: f32 = 0.55;
const LOW_MORALE_THRESHOLD: f32 = 0.5;
const LOW_MORALE_MIN_MULTIPLIER: f32 = 0.75;
const ENEMY_DROP_PICKUP_DELAY_SECS: f32 = 0.9;
const INSIDE_FORMATION_DAMAGE_MULTIPLIER: f32 = 1.2;
const FORMATION_BOUNDS_PADDING_SLOTS: f32 = 0.35;
const RANGED_PROJECTILE_HIT_RADIUS: f32 = 10.0;
const RANGED_PROJECTILE_RENDER_SIZE: f32 = 16.0;
const RANGED_PROJECTILE_RENDER_Z: f32 = 28.0;

#[derive(Component, Clone, Copy, Debug)]
pub struct RangedAttackProfile {
    pub damage: f32,
    pub range: f32,
    pub projectile_speed: f32,
    pub projectile_max_distance: f32,
}

#[derive(Component, Clone, Debug)]
pub struct RangedAttackCooldown(pub Timer);

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_attack_timers,
                emit_ranged_projectile_attacks,
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
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    mut attackers: Query<(
        &Unit,
        Option<&Morale>,
        Option<&PriestAttackSpeedBlessing>,
        &mut AttackCooldown,
    )>,
) {
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    for (unit, morale, priest_blessing, mut cooldown) in &mut attackers {
        let morale_scale = morale
            .copied()
            .map(|value| morale_effect_multiplier(value.ratio()))
            .unwrap_or(1.0);

        let speed_scale = if unit.team == Team::Friendly {
            let mut value = cohesion_mods.attack_speed_multiplier * morale_scale * level_multiplier;
            if let Some(buff) = &global_buffs {
                value *= buff.attack_speed_multiplier;
            }
            value *= priest_attack_speed_multiplier(priest_blessing);
            if let Some(conditional) = &conditional_effects {
                value *= conditional.friendly_attack_speed_multiplier;
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
fn emit_ranged_projectile_attacks(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<ArtAssets>,
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    formation_mods: Res<FormationModifiers>,
    cohesion_mods: Res<CohesionCombatModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    commander_motion: Option<Res<CommanderMotionState>>,
    inventory: Res<InventoryState>,
    mut ranged_attackers: Query<(
        Entity,
        &Unit,
        Option<&UnitTier>,
        Option<&Morale>,
        &Transform,
        &AttackProfile,
        &RangedAttackProfile,
        &mut RangedAttackCooldown,
    )>,
    targets: Query<(Entity, &Transform, &Health, &Unit)>,
) {
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    let target_snapshot: Vec<(Entity, Vec2, f32, f32, Unit)> = targets
        .iter()
        .map(|(entity, transform, health, unit)| {
            (
                entity,
                transform.translation.truncate(),
                health.current,
                health.max,
                *unit,
            )
        })
        .collect();
    let has_non_commander_friendlies = target_snapshot.iter().any(|(_, _, health, _, unit)| {
        unit.team == Team::Friendly && unit.kind != UnitKind::Commander && *health > 0.0
    });
    let formation_targets: Vec<(Entity, Unit, Vec2, f32, f32)> = target_snapshot
        .iter()
        .map(|(entity, position, health, _max_health, unit)| {
            (*entity, *unit, *position, *health, 0.0)
        })
        .collect();
    let formation_context = friendly_formation_context(&formation_targets);
    let slot_spacing = active_formation_config(&data, *active_formation).slot_spacing;

    for (
        _attacker_entity,
        commander_unit,
        attacker_tier,
        commander_morale,
        commander_transform,
        melee_profile,
        ranged_profile,
        mut ranged_cooldown,
    ) in &mut ranged_attackers
    {
        if ranged_profile.range <= melee_profile.range || ranged_profile.damage <= 0.0 {
            continue;
        }

        let attacker_team = commander_unit.team;
        let opposite_team = match attacker_team {
            Team::Friendly => Team::Enemy,
            Team::Enemy => Team::Friendly,
            Team::Neutral => continue,
        };
        let morale_multiplier = commander_morale
            .copied()
            .map(|value| morale_effect_multiplier(value.ratio()))
            .unwrap_or(1.0);
        let mut attack_speed = morale_multiplier;
        if attacker_team == Team::Friendly {
            attack_speed *= cohesion_mods.attack_speed_multiplier * level_multiplier;
            if let Some(buff) = &global_buffs {
                attack_speed *= buff.attack_speed_multiplier;
            }
            attack_speed = attack_speed.max(MIN_FRIENDLY_COMBAT_MULTIPLIER);
        }

        ranged_cooldown.0.tick(std::time::Duration::from_secs_f32(
            time.delta_seconds() * attack_speed,
        ));
        if !ranged_cooldown.0.finished() {
            continue;
        }

        let commander_position = commander_transform.translation.truncate();

        let mut best_target: Option<(Vec2, f32, UnitKind)> = None;
        for (_, target_position, target_health, _, target_unit) in &target_snapshot {
            if target_unit.team != opposite_team || *target_health <= 0.0 {
                continue;
            }
            if !enemy_target_allowed(
                attacker_team,
                target_unit.kind,
                has_non_commander_friendlies,
            ) {
                continue;
            }
            let distance_sq = commander_position.distance_squared(*target_position);
            if !ranged_target_in_window(distance_sq, melee_profile.range, ranged_profile.range) {
                continue;
            }
            let candidate = (*target_position, distance_sq, target_unit.kind);
            match best_target {
                Some((_, best_distance, _)) if distance_sq >= best_distance => {}
                _ => best_target = Some(candidate),
            }
        }

        let Some((target_position, _, target_kind)) = best_target else {
            continue;
        };
        let direction = target_position - commander_position;
        if direction.length_squared() <= 0.001 {
            continue;
        }
        let direction_normalized = direction.normalize();

        let outgoing_multiplier = if attacker_team == Team::Friendly {
            let base_multiplier = friendly_outgoing_multiplier(
                effective_formation_offense_multiplier(
                    &formation_mods,
                    commander_motion.as_deref(),
                ),
                cohesion_mods.damage_multiplier,
                global_buffs
                    .as_ref()
                    .map(|buff| buff.damage_multiplier)
                    .unwrap_or(1.0),
                level_multiplier,
                morale_multiplier,
            );
            let formation_multiplier = inside_formation_damage_multiplier(
                &formation_context,
                target_position,
                target_kind,
                *active_formation,
                slot_spacing,
            );
            base_multiplier
                * formation_multiplier
                * conditional_effects
                    .as_deref()
                    .map(|effects| effects.friendly_damage_multiplier)
                    .unwrap_or(1.0)
        } else {
            morale_multiplier
        };
        let ranged_bonus = if attacker_team == Team::Friendly {
            gear_bonuses_for_unit(
                &inventory,
                commander_unit.kind,
                attacker_tier.copied().map(|tier| tier.0),
            )
            .ranged_damage_bonus
        } else {
            0.0
        };
        let projectile_damage =
            ((ranged_profile.damage + ranged_bonus).max(0.0) * outgoing_multiplier).max(1.0);

        ranged_cooldown.0.reset();
        commands.spawn((
            Projectile {
                velocity: direction_normalized * ranged_profile.projectile_speed,
                damage: projectile_damage,
                remaining_distance: ranged_profile.projectile_max_distance,
                radius: RANGED_PROJECTILE_HIT_RADIUS,
                source_team: commander_unit.team,
            },
            SpriteBundle {
                texture: art.arrow_projectile.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(RANGED_PROJECTILE_RENDER_SIZE)),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        commander_position.x,
                        commander_position.y,
                        RANGED_PROJECTILE_RENDER_Z,
                    ),
                    rotation: Quat::from_rotation_z(
                        direction_normalized.y.atan2(direction_normalized.x),
                    ),
                    ..default()
                },
                ..default()
            },
        ));
    }
}

pub fn ranged_target_in_window(distance_sq: f32, melee_range: f32, ranged_range: f32) -> bool {
    if melee_range <= 0.0 || ranged_range <= 0.0 || ranged_range <= melee_range {
        return false;
    }
    let melee_range_sq = melee_range * melee_range;
    let ranged_range_sq = ranged_range * ranged_range;
    distance_sq > melee_range_sq && distance_sq <= ranged_range_sq
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn emit_damage_events(
    mut damage_events: EventWriter<DamageEvent>,
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    formation_mods: Res<FormationModifiers>,
    cohesion_mods: Res<CohesionCombatModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    commander_motion: Option<Res<CommanderMotionState>>,
    inventory: Res<InventoryState>,
    mut attackers: Query<(
        Entity,
        &Unit,
        Option<&UnitTier>,
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
        Option<&UnitTier>,
        Option<&crate::model::Armor>,
    )>,
) {
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    let target_snapshot: Vec<(Entity, Unit, Vec2, f32, f32, f32)> = targets
        .iter()
        .map(|(entity, unit, transform, health, tier, armor)| {
            let base_armor = armor.map(|value| value.0).unwrap_or(0.0);
            let effective_armor = if unit.team == Team::Friendly {
                let gear_armor_bonus = gear_bonuses_for_unit(
                    &inventory,
                    unit.kind,
                    tier.copied().map(|value| value.0),
                )
                .armor_bonus;
                let armor_with_buffs = base_armor
                    + gear_armor_bonus
                    + global_buffs
                        .as_ref()
                        .map(|buff| buff.armor_bonus)
                        .unwrap_or(0.0);
                (armor_with_buffs.max(0.0)
                    * formation_mods.defense_multiplier
                    * cohesion_mods.defense_multiplier)
                    .max(0.0)
            } else {
                base_armor
            };
            (
                entity,
                *unit,
                transform.translation.truncate(),
                health.current,
                health.max,
                effective_armor,
            )
        })
        .collect();
    let formation_targets: Vec<(Entity, Unit, Vec2, f32, f32)> = target_snapshot
        .iter()
        .map(|(entity, unit, position, health, _max_health, armor)| {
            (*entity, *unit, *position, *health, *armor)
        })
        .collect();
    let has_non_commander_friendlies = target_snapshot.iter().any(|(_, unit, _, health, _, _)| {
        unit.team == Team::Friendly && unit.kind != UnitKind::Commander && *health > 0.0
    });
    let formation_context = friendly_formation_context(&formation_targets);

    for (
        _,
        attacker_unit,
        attacker_tier,
        attacker_morale,
        attacker_transform,
        attack_profile,
        mut attack_cd,
    ) in &mut attackers
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

        let mut closest_target: Option<(Entity, f32, f32, f32, f32, Vec2, UnitKind)> = None;
        for (
            target_entity,
            target_unit,
            target_pos,
            target_health,
            target_max_health,
            target_armor,
        ) in &target_snapshot
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
                let candidate = (
                    *target_entity,
                    dist_sq,
                    *target_armor,
                    *target_health,
                    *target_max_health,
                    *target_pos,
                    target_unit.kind,
                );
                match closest_target {
                    Some((_, best_dist, _, _, _, _, _)) if dist_sq >= best_dist => {}
                    _ => closest_target = Some(candidate),
                }
            }
        }

        if let Some((
            target_entity,
            _,
            armor,
            target_health,
            target_max_health,
            target_position,
            target_kind,
        )) = closest_target
        {
            attack_cd.0.reset();
            let morale_multiplier = attacker_morale
                .copied()
                .map(|value| morale_effect_multiplier(value.ratio()))
                .unwrap_or(1.0);
            let outgoing_multiplier = if attacker_unit.team == Team::Friendly {
                let base = friendly_outgoing_multiplier(
                    effective_formation_offense_multiplier(
                        &formation_mods,
                        commander_motion.as_deref(),
                    ),
                    cohesion_mods.damage_multiplier,
                    global_buffs
                        .as_ref()
                        .map(|buff| buff.damage_multiplier)
                        .unwrap_or(1.0),
                    level_multiplier,
                    morale_multiplier,
                );
                let inside_multiplier = inside_formation_damage_multiplier(
                    &formation_context,
                    target_position,
                    target_kind,
                    *active_formation,
                    active_formation_config(&data, *active_formation).slot_spacing,
                );
                base * inside_multiplier
                    * conditional_effects
                        .as_deref()
                        .map(|effects| effects.friendly_damage_multiplier)
                        .unwrap_or(1.0)
            } else {
                morale_multiplier
            };

            let melee_bonus = if attacker_unit.team == Team::Friendly {
                gear_bonuses_for_unit(
                    &inventory,
                    attacker_unit.kind,
                    attacker_tier.copied().map(|tier| tier.0),
                )
                .melee_damage_bonus
            } else {
                0.0
            };
            let mut damage = compute_damage(
                (attack_profile.damage + melee_bonus).max(0.0),
                armor,
                outgoing_multiplier,
            );
            let execute_threshold = conditional_effects
                .as_deref()
                .map(|effects| effects.execute_below_health_ratio)
                .unwrap_or(0.0);
            let target_team = if attacker_unit.team == Team::Friendly {
                Team::Enemy
            } else {
                Team::Friendly
            };
            let execute = should_execute_target(
                attacker_unit.team,
                target_team,
                target_health,
                target_max_health,
                execute_threshold,
            );
            if execute {
                damage = target_health + armor + 1.0;
            }

            damage_events.send(DamageEvent {
                target: target_entity,
                source_team: attacker_unit.team,
                amount: damage,
                execute,
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

pub fn effective_formation_offense_multiplier(
    formation_modifiers: &FormationModifiers,
    commander_motion: Option<&CommanderMotionState>,
) -> f32 {
    let moving_multiplier = if commander_motion
        .map(|state| state.is_moving)
        .unwrap_or(false)
    {
        formation_modifiers.offense_while_moving_multiplier
    } else {
        1.0
    };
    formation_modifiers.offense_multiplier * moving_multiplier
}

pub fn compute_damage(base_damage: f32, armor: f32, outgoing_multiplier: f32) -> f32 {
    (base_damage * outgoing_multiplier - armor).max(1.0)
}

pub fn should_execute_target(
    source_team: Team,
    target_team: Team,
    target_health: f32,
    target_max_health: f32,
    execute_threshold: f32,
) -> bool {
    source_team == Team::Friendly
        && target_team == Team::Enemy
        && execute_threshold > 0.0
        && target_max_health > 0.0
        && (target_health / target_max_health) <= execute_threshold
}

#[derive(Clone, Copy, Debug)]
pub struct FriendlyFormationContext {
    pub commander_position: Vec2,
    pub recruit_count: usize,
}

pub fn friendly_formation_context(
    targets: &[(Entity, Unit, Vec2, f32, f32)],
) -> Option<FriendlyFormationContext> {
    let commander_position = targets.iter().find_map(|(_, unit, position, health, _)| {
        if unit.team == Team::Friendly && unit.kind == UnitKind::Commander && *health > 0.0 {
            Some(*position)
        } else {
            None
        }
    })?;
    let recruit_count = targets
        .iter()
        .filter(|(_, unit, _, health, _)| {
            unit.team == Team::Friendly && unit.kind != UnitKind::Commander && *health > 0.0
        })
        .count();
    Some(FriendlyFormationContext {
        commander_position,
        recruit_count,
    })
}

pub fn inside_formation_damage_multiplier(
    formation_context: &Option<FriendlyFormationContext>,
    target_position: Vec2,
    target_kind: UnitKind,
    active_formation: ActiveFormation,
    slot_spacing: f32,
) -> f32 {
    let Some(context) = formation_context else {
        return 1.0;
    };
    if context.recruit_count == 0 || target_kind != UnitKind::EnemyBanditRaider {
        return 1.0;
    }
    if inside_active_formation_bounds(
        active_formation,
        context.commander_position,
        target_position,
        context.recruit_count,
        slot_spacing,
    ) {
        INSIDE_FORMATION_DAMAGE_MULTIPLIER
    } else {
        1.0
    }
}

pub fn inside_active_formation_bounds(
    active_formation: ActiveFormation,
    commander_position: Vec2,
    target_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
) -> bool {
    formation_contains_position(
        active_formation,
        commander_position,
        target_position,
        recruit_count,
        slot_spacing,
        FORMATION_BOUNDS_PADDING_SLOTS,
    )
}

fn apply_damage_events(
    mut damage_events: EventReader<DamageEvent>,
    mut damage_text_events: EventWriter<DamageTextEvent>,
    mut damaged_events: EventWriter<UnitDamagedEvent>,
    mut health_query: Query<(&mut Health, &Unit, &Transform)>,
) {
    for event in damage_events.read() {
        if event.amount <= 0.0 {
            continue;
        }
        if let Ok((mut health, unit, transform)) = health_query.get_mut(event.target) {
            let applied_damage = event.amount.min(health.current.max(0.0));
            if applied_damage <= 0.0 {
                continue;
            }
            health.current -= applied_damage;
            damaged_events.send(UnitDamagedEvent {
                target: event.target,
                team: unit.team,
                amount: applied_damage,
            });
            damage_text_events.send(DamageTextEvent {
                world_position: transform.translation.truncate(),
                target_team: unit.team,
                amount: applied_damage,
                execute: event.execute,
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
                max_health: health.max,
                world_position: transform.translation.truncate(),
            });
            if unit.team == Team::Enemy {
                exp_pack_events.send(SpawnExpPackEvent {
                    world_position: transform.translation.truncate(),
                    xp_value_override: None,
                    pickup_delay_secs: Some(ENEMY_DROP_PICKUP_DELAY_SECS),
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
    use bevy::prelude::{Entity, Vec2};

    use crate::combat::{
        FriendlyFormationContext, commander_level_combat_multiplier, compute_damage,
        effective_formation_offense_multiplier, enemy_target_allowed, friendly_formation_context,
        friendly_outgoing_multiplier, inside_active_formation_bounds,
        inside_formation_damage_multiplier, morale_effect_multiplier, ranged_target_in_window,
        should_execute_target,
    };
    use crate::formation::{ActiveFormation, FormationModifiers};
    use crate::model::{Team, UnitKind};
    use crate::squad::CommanderMotionState;

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
    fn moving_formation_offense_bonus_applies_only_while_moving() {
        let mods = FormationModifiers {
            offense_multiplier: 1.0,
            offense_while_moving_multiplier: 1.2,
            defense_multiplier: 0.9,
            move_speed_multiplier: 1.0,
        };
        let idle = effective_formation_offense_multiplier(&mods, None);
        let moving = effective_formation_offense_multiplier(
            &mods,
            Some(&CommanderMotionState { is_moving: true }),
        );
        assert!((idle - 1.0).abs() < 0.0001);
        assert!((moving - 1.2).abs() < 0.0001);
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
            UnitKind::ChristianPeasantInfantry,
            true
        ));
    }

    #[test]
    fn commander_level_multiplier_gains_one_percent_per_level() {
        assert!((commander_level_combat_multiplier(1) - 1.0).abs() < 0.0001);
        assert!((commander_level_combat_multiplier(8) - 1.07).abs() < 0.0001);
    }

    #[test]
    fn enemy_inside_formation_gets_damage_bonus_multiplier() {
        let context = Some(FriendlyFormationContext {
            commander_position: Vec2::ZERO,
            recruit_count: 9,
        });
        let inside = inside_formation_damage_multiplier(
            &context,
            Vec2::new(20.0, 15.0),
            UnitKind::EnemyBanditRaider,
            ActiveFormation::Square,
            30.0,
        );
        let outside = inside_formation_damage_multiplier(
            &context,
            Vec2::new(220.0, 0.0),
            UnitKind::EnemyBanditRaider,
            ActiveFormation::Square,
            30.0,
        );
        assert!((inside - 1.2).abs() < 0.0001);
        assert!((outside - 1.0).abs() < 0.0001);
    }

    #[test]
    fn formation_bounds_check_requires_recruits() {
        assert!(!inside_active_formation_bounds(
            ActiveFormation::Square,
            Vec2::ZERO,
            Vec2::new(1.0, 1.0),
            0,
            30.0
        ));
    }

    #[test]
    fn formation_context_extracts_commander_and_recruit_count() {
        let targets = vec![
            (
                Entity::from_raw(1),
                crate::model::Unit {
                    team: Team::Friendly,
                    kind: UnitKind::Commander,
                    level: 1,
                },
                Vec2::new(10.0, 20.0),
                100.0,
                0.0,
            ),
            (
                Entity::from_raw(2),
                crate::model::Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianPeasantInfantry,
                    level: 1,
                },
                Vec2::new(40.0, 20.0),
                80.0,
                0.0,
            ),
            (
                Entity::from_raw(3),
                crate::model::Unit {
                    team: Team::Enemy,
                    kind: UnitKind::EnemyBanditRaider,
                    level: 1,
                },
                Vec2::new(90.0, 20.0),
                60.0,
                0.0,
            ),
        ];
        let context = friendly_formation_context(&targets).expect("formation context");
        assert_eq!(context.commander_position, Vec2::new(10.0, 20.0));
        assert_eq!(context.recruit_count, 1);
    }

    #[test]
    fn ranged_target_window_requires_outside_melee_and_inside_ranged() {
        assert!(ranged_target_in_window(64.0, 6.0, 12.0));
        assert!(!ranged_target_in_window(25.0, 6.0, 12.0));
        assert!(!ranged_target_in_window(225.0, 6.0, 12.0));
        assert!(!ranged_target_in_window(64.0, 10.0, 10.0));
    }

    #[test]
    fn justice_execute_requires_enemy_below_threshold() {
        assert!(should_execute_target(
            Team::Friendly,
            Team::Enemy,
            9.0,
            100.0,
            0.10
        ));
        assert!(!should_execute_target(
            Team::Friendly,
            Team::Enemy,
            11.0,
            100.0,
            0.10
        ));
        assert!(!should_execute_target(
            Team::Friendly,
            Team::Enemy,
            9.0,
            100.0,
            0.0
        ));
        assert!(!should_execute_target(
            Team::Enemy,
            Team::Friendly,
            9.0,
            100.0,
            0.10
        ));
    }
}
