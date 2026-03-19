use bevy::prelude::*;

use crate::data::GameData;
use crate::formation::FormationModifiers;
use crate::model::{
    AttackCooldown, AttackProfile, CommanderUnit, DamageEvent, EnemyUnit, FriendlyUnit, GameState,
    GlobalBuffs, Health, Morale, SpawnExpPackEvent, Team, Unit, UnitDamagedEvent, UnitDiedEvent,
    UnitKind,
};
use crate::morale::CohesionCombatModifiers;
use crate::projectiles::Projectile;
use crate::upgrades::Progression;
use crate::visuals::ArtAssets;

pub const MIN_FRIENDLY_COMBAT_MULTIPLIER: f32 = 0.55;
const LOW_MORALE_THRESHOLD: f32 = 0.5;
const LOW_MORALE_MIN_MULTIPLIER: f32 = 0.75;
const ENEMY_DROP_PICKUP_DELAY_SECS: f32 = 0.9;
const INSIDE_FORMATION_DAMAGE_MULTIPLIER: f32 = 1.2;
const FORMATION_BOUNDS_PADDING_SLOTS: f32 = 0.35;
const COMMANDER_ARROW_HIT_RADIUS: f32 = 10.0;
const COMMANDER_ARROW_RENDER_SIZE: f32 = 16.0;
const COMMANDER_ARROW_RENDER_Z: f32 = 28.0;

#[derive(Component, Clone, Copy, Debug)]
pub struct CommanderRangedAttackProfile {
    pub damage: f32,
    pub range: f32,
    pub projectile_speed: f32,
    pub projectile_max_distance: f32,
}

#[derive(Component, Clone, Debug)]
pub struct CommanderRangedAttackCooldown(pub Timer);

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_attack_timers,
                commander_ranged_attacks,
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
fn commander_ranged_attacks(
    mut commands: Commands,
    time: Res<Time>,
    art: Res<ArtAssets>,
    data: Res<GameData>,
    formation_mods: Res<FormationModifiers>,
    cohesion_mods: Res<CohesionCombatModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    mut commanders: Query<
        (
            &Unit,
            Option<&Morale>,
            &Transform,
            &AttackProfile,
            &CommanderRangedAttackProfile,
            &mut CommanderRangedAttackCooldown,
        ),
        With<CommanderUnit>,
    >,
    friendlies: Query<&Unit, With<FriendlyUnit>>,
    enemies: Query<(&Transform, &Health, &Unit), With<EnemyUnit>>,
) {
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    for (
        commander_unit,
        commander_morale,
        commander_transform,
        melee_profile,
        ranged_profile,
        mut ranged_cooldown,
    ) in &mut commanders
    {
        let recruit_count = friendlies
            .iter()
            .filter(|unit| unit.kind != UnitKind::Commander)
            .count();
        let morale_multiplier = commander_morale
            .copied()
            .map(|value| morale_effect_multiplier(value.ratio()))
            .unwrap_or(1.0);
        let mut attack_speed =
            cohesion_mods.attack_speed_multiplier * morale_multiplier * level_multiplier;
        if let Some(buff) = &global_buffs {
            attack_speed *= buff.attack_speed_multiplier;
        }
        attack_speed = attack_speed.max(MIN_FRIENDLY_COMBAT_MULTIPLIER);
        ranged_cooldown.0.tick(std::time::Duration::from_secs_f32(
            time.delta_seconds() * attack_speed,
        ));
        if !ranged_cooldown.0.finished() {
            continue;
        }

        let commander_position = commander_transform.translation.truncate();
        let melee_range_sq = melee_profile.range * melee_profile.range;
        let ranged_range_sq = ranged_profile.range * ranged_profile.range;

        let mut best_target: Option<(Vec2, f32, UnitKind)> = None;
        for (enemy_transform, enemy_health, enemy_unit) in &enemies {
            if enemy_health.current <= 0.0 {
                continue;
            }
            let enemy_position = enemy_transform.translation.truncate();
            let distance_sq = commander_position.distance_squared(enemy_position);
            if distance_sq <= melee_range_sq || distance_sq > ranged_range_sq {
                continue;
            }
            let candidate = (enemy_position, distance_sq, enemy_unit.kind);
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

        let base_multiplier = friendly_outgoing_multiplier(
            formation_mods.offense_multiplier,
            cohesion_mods.damage_multiplier,
            global_buffs
                .as_ref()
                .map(|buff| buff.damage_multiplier)
                .unwrap_or(1.0),
            level_multiplier,
            morale_multiplier,
        );
        let formation_multiplier = inside_formation_damage_multiplier(
            &Some(FriendlyFormationContext {
                commander_position,
                recruit_count,
            }),
            target_position,
            target_kind,
            data.formations.square.slot_spacing,
        );
        let projectile_damage =
            (ranged_profile.damage * base_multiplier * formation_multiplier).max(1.0);

        ranged_cooldown.0.reset();
        commands.spawn((
            Projectile {
                velocity: direction_normalized * ranged_profile.projectile_speed,
                damage: projectile_damage,
                remaining_distance: ranged_profile.projectile_max_distance,
                radius: COMMANDER_ARROW_HIT_RADIUS,
                source_team: commander_unit.team,
            },
            SpriteBundle {
                texture: art.arrow_projectile.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(COMMANDER_ARROW_RENDER_SIZE)),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(
                        commander_position.x,
                        commander_position.y,
                        COMMANDER_ARROW_RENDER_Z,
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

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn emit_damage_events(
    mut damage_events: EventWriter<DamageEvent>,
    data: Res<GameData>,
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
            let base_armor = armor.map(|value| value.0).unwrap_or(0.0);
            let effective_armor = if unit.team == Team::Friendly {
                base_armor
                    + global_buffs
                        .as_ref()
                        .map(|buff| buff.armor_bonus)
                        .unwrap_or(0.0)
            } else {
                base_armor
            };
            (
                entity,
                *unit,
                transform.translation.truncate(),
                health.current,
                effective_armor,
            )
        })
        .collect();
    let has_non_commander_friendlies = target_snapshot.iter().any(|(_, unit, _, health, _)| {
        unit.team == Team::Friendly && unit.kind != UnitKind::Commander && *health > 0.0
    });
    let formation_context = friendly_formation_context(&target_snapshot);

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

        let mut closest_target: Option<(Entity, f32, f32, Vec2, UnitKind)> = None;
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
                let candidate = (
                    *target_entity,
                    dist_sq,
                    *target_armor,
                    *target_pos,
                    target_unit.kind,
                );
                match closest_target {
                    Some((_, best_dist, _, _, _)) if dist_sq >= best_dist => {}
                    _ => closest_target = Some(candidate),
                }
            }
        }

        if let Some((target_entity, _, armor, target_position, target_kind)) = closest_target {
            attack_cd.0.reset();
            let morale_multiplier = attacker_morale
                .copied()
                .map(|value| morale_effect_multiplier(value.ratio()))
                .unwrap_or(1.0);
            let outgoing_multiplier = if attacker_unit.team == Team::Friendly {
                let base = friendly_outgoing_multiplier(
                    formation_mods.offense_multiplier,
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
                    data.formations.square.slot_spacing,
                );
                base * inside_multiplier
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
    slot_spacing: f32,
) -> f32 {
    let Some(context) = formation_context else {
        return 1.0;
    };
    if context.recruit_count == 0 || target_kind != UnitKind::EnemyBanditRaider {
        return 1.0;
    }
    if inside_square_formation_bounds(
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

pub fn inside_square_formation_bounds(
    commander_position: Vec2,
    target_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
) -> bool {
    if recruit_count == 0 || slot_spacing <= 0.0 {
        return false;
    }
    let side = ((recruit_count + 1) as f32).sqrt().ceil();
    let half_extent = ((side - 1.0) * 0.5 + FORMATION_BOUNDS_PADDING_SLOTS) * slot_spacing;
    let delta = target_position - commander_position;
    delta.x.abs() <= half_extent && delta.y.abs() <= half_extent
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
        enemy_target_allowed, friendly_formation_context, friendly_outgoing_multiplier,
        inside_formation_damage_multiplier, inside_square_formation_bounds,
        morale_effect_multiplier,
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
            30.0,
        );
        let outside = inside_formation_damage_multiplier(
            &context,
            Vec2::new(220.0, 0.0),
            UnitKind::EnemyBanditRaider,
            30.0,
        );
        assert!((inside - 1.2).abs() < 0.0001);
        assert!((outside - 1.0).abs() < 0.0001);
    }

    #[test]
    fn formation_bounds_check_requires_recruits() {
        assert!(!inside_square_formation_bounds(
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
                    kind: UnitKind::InfantryKnight,
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
}
