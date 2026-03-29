use bevy::prelude::*;

use crate::banner::BannerState;
use crate::data::GameData;
use crate::formation::{
    ActiveFormation, FormationModifiers, active_formation_config, formation_contains_position,
    formation_melee_reflect_ratio, formation_shielded_block_bonus,
};
use crate::inventory::{
    EquipmentArmyEffects, InventoryState, gear_bonuses_for_unit_with_banner_state,
};
use crate::model::{
    AttackCooldown, AttackProfile, BaseMaxHealth, DamageEvent, DamageTextEvent, DamageTextKind,
    EnemySpawnSource, EnemyUnit, GameDifficulty, GameState, GlobalBuffs, Health,
    MatchSetupSelection, Morale, SpawnGoldPackEvent, Team, Unit, UnitArmorClass, UnitDamagedEvent,
    UnitDiedEvent, UnitKind, UnitRoleTag, UnitTier,
};
use crate::morale::{morale_armor_multiplier, morale_damage_multiplier};
use crate::projectiles::Projectile;
use crate::squad::{
    ArmorLockedZero, CommanderMotionState, HeroSubtypeUnit, PriestAttackSpeedBlessing,
    hero_subtype_combat_profile, hero_subtype_matchup_multiplier, priest_attack_speed_multiplier,
};
use crate::upgrades::{ConditionalUpgradeEffects, Progression};
use crate::visuals::ArtAssets;

pub const MIN_FRIENDLY_COMBAT_MULTIPLIER: f32 = 0.55;

const ENEMY_DROP_PICKUP_DELAY_SECS: f32 = 0.9;
const FORMATION_BOUNDS_PADDING_SLOTS: f32 = 0.35;
const RANGED_PROJECTILE_HIT_RADIUS: f32 = 10.0;
const RANGED_PROJECTILE_RENDER_SIZE: f32 = 16.0;
const RANGED_PROJECTILE_RENDER_Z: f32 = 28.0;
const DEFAULT_CRIT_DAMAGE_MULTIPLIER: f32 = 1.2;
const MAX_CRIT_CHANCE: f32 = 0.95;
const ARMOR_DIMINISHING_SCALE: f32 = 90.0;
const MAX_ARMOR_REDUCTION_RATIO: f32 = 0.90;
const DEATH_HEALTH_EPSILON: f32 = 0.01;
const PEASANT_INFANTRY_BLOCK_CHANCE: f32 = 0.15;
const INFIDELS_BLOCK_CHANCE_BONUS: f32 = 0.08;
const BLOCK_DAMAGE_MITIGATION_RATIO: f32 = 0.65;
const COUNTER_MULTIPLIER_MIN: f32 = 0.65;
const COUNTER_MULTIPLIER_MAX: f32 = 1.45;
const ANTI_CAVALRY_BONUS: f32 = 0.35;
const CAVALRY_VS_FRONTLINE_BONUS: f32 = 0.18;
const CAVALRY_VS_ANTI_CAVALRY_PENALTY: f32 = 0.24;
const SKIRMISHER_VS_SUPPORT_BONUS: f32 = 0.15;
const ARCHER_VS_CAVALRY_PENALTY: f32 = 0.10;
const SUPPORT_VS_FRONTLINE_PENALTY: f32 = 0.12;

#[derive(Clone, Copy, Debug)]
struct CombatRngState {
    state: u64,
}

impl Default for CombatRngState {
    fn default() -> Self {
        Self {
            state: 0xC0DE_A710_C001_BA11_u64,
        }
    }
}

impl CombatRngState {
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

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

#[allow(clippy::type_complexity)]
fn tick_attack_timers(
    time: Res<Time>,
    banner_state: Option<Res<BannerState>>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    inventory: Res<InventoryState>,
    mut attackers: Query<(
        &Unit,
        Option<&UnitTier>,
        Option<&PriestAttackSpeedBlessing>,
        Option<&HeroSubtypeUnit>,
        &mut AttackCooldown,
    )>,
) {
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    for (unit, tier, priest_blessing, hero_subtype, mut cooldown) in &mut attackers {
        let priest_scale = priest_attack_speed_multiplier(priest_blessing);
        let (upgrade_attack_speed_bonus, gear_attack_speed_bonus) = if unit.team == Team::Friendly {
            let gear = gear_bonuses_for_unit_with_banner_state(
                &inventory,
                unit.kind,
                tier.map(|value| value.0),
                banner_item_active,
            );
            let upgrade_bonus = global_buffs
                .as_ref()
                .map(|buff| buff.attack_speed_multiplier - 1.0)
                .unwrap_or(0.0);
            (upgrade_bonus, gear.attack_speed_multiplier)
        } else {
            (0.0, 0.0)
        };

        let speed_scale = if unit.team == Team::Friendly {
            let mut value = level_multiplier;
            value *= priest_scale;
            value *= combined_percentage_multiplier(
                upgrade_attack_speed_bonus + gear_attack_speed_bonus,
                0.1,
            );
            if let Some(hero) = hero_subtype {
                value *= hero_subtype_combat_profile(hero.subtype).attack_speed_multiplier;
            }
            if let Some(conditional) = &conditional_effects {
                value *= conditional.friendly_attack_speed_multiplier;
            }
            value.max(MIN_FRIENDLY_COMBAT_MULTIPLIER)
        } else {
            priest_scale
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
    banner_state: Option<Res<BannerState>>,
    active_formation: Res<ActiveFormation>,
    formation_mods: Res<FormationModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    commander_motion: Option<Res<CommanderMotionState>>,
    mut crit_rng: Local<CombatRngState>,
    inventory: Res<InventoryState>,
    mut ranged_attackers: Query<(
        Entity,
        &Unit,
        Option<&UnitTier>,
        Option<&PriestAttackSpeedBlessing>,
        Option<&HeroSubtypeUnit>,
        Option<&Morale>,
        &Transform,
        &AttackProfile,
        &RangedAttackProfile,
        &mut RangedAttackCooldown,
    )>,
    targets: Query<(Entity, &Transform, &Health, &Unit)>,
) {
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
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
    let inside_formation_bonus_multiplier = global_buffs
        .as_ref()
        .map(|buff| buff.inside_formation_damage_multiplier)
        .unwrap_or(1.0);

    for (
        _attacker_entity,
        commander_unit,
        attacker_tier,
        priest_blessing,
        hero_subtype,
        attacker_morale,
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
        let mut attack_speed = priest_attack_speed_multiplier(priest_blessing);
        if attacker_team == Team::Friendly {
            let gear = gear_bonuses_for_unit_with_banner_state(
                &inventory,
                commander_unit.kind,
                attacker_tier.copied().map(|tier| tier.0),
                banner_item_active,
            );
            attack_speed *= level_multiplier;
            let upgrade_attack_speed_bonus = global_buffs
                .as_ref()
                .map(|buff| buff.attack_speed_multiplier - 1.0)
                .unwrap_or(0.0);
            attack_speed *= combined_percentage_multiplier(
                upgrade_attack_speed_bonus + gear.attack_speed_multiplier,
                0.1,
            );
            if let Some(hero) = hero_subtype {
                attack_speed *= hero_subtype_combat_profile(hero.subtype).attack_speed_multiplier;
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
        let friendly_gear = if attacker_team == Team::Friendly {
            Some(gear_bonuses_for_unit_with_banner_state(
                &inventory,
                commander_unit.kind,
                attacker_tier.copied().map(|tier| tier.0),
                banner_item_active,
            ))
        } else {
            None
        };
        let effective_ranged_range = (ranged_profile.range
            + friendly_gear
                .as_ref()
                .map(|gear| gear.ranged_range_bonus)
                .unwrap_or(0.0))
        .max(melee_profile.range + 1.0);

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
            if !ranged_target_in_window(distance_sq, melee_profile.range, effective_ranged_range) {
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

        let morale_damage_multiplier = attacker_morale
            .copied()
            .map(|value| morale_damage_multiplier_from_ratio(value.ratio()))
            .unwrap_or(1.0);
        let outgoing_multiplier = if attacker_team == Team::Friendly {
            let base_multiplier = friendly_outgoing_multiplier(
                effective_formation_offense_multiplier(
                    &formation_mods,
                    commander_motion.as_deref(),
                ),
                1.0,
                level_multiplier,
            );
            let formation_multiplier = inside_formation_damage_multiplier(
                &formation_context,
                target_position,
                target_kind,
                *active_formation,
                slot_spacing,
                inside_formation_bonus_multiplier,
            );
            base_multiplier
                * formation_multiplier
                * conditional_effects
                    .as_deref()
                    .map(|effects| effects.friendly_damage_multiplier)
                    .unwrap_or(1.0)
                * morale_damage_multiplier
        } else {
            morale_damage_multiplier
        };
        let ranged_bonus_mult = if attacker_team == Team::Friendly {
            friendly_gear
                .as_ref()
                .map(|gear| gear.ranged_damage_multiplier)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let upgrade_damage_bonus = if attacker_team == Team::Friendly {
            global_buffs
                .as_ref()
                .map(|buff| buff.damage_multiplier - 1.0)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let mut projectile_damage = (apply_percent_increase_to_base_plus_additive(
            ranged_profile.damage,
            0.0,
            upgrade_damage_bonus + ranged_bonus_mult,
        ) * outgoing_multiplier)
            .max(1.0);
        if let Some(hero) = hero_subtype {
            projectile_damage *=
                hero_subtype_combat_profile(hero.subtype).outgoing_damage_multiplier;
        }
        let mut projectile_is_critical = false;
        if attacker_team == Team::Friendly {
            let (crit_chance, crit_multiplier) =
                friendly_critical_parameters(global_buffs.as_deref());
            let is_critical = roll_critical_hit(crit_chance, &mut crit_rng);
            projectile_damage =
                apply_critical_multiplier(projectile_damage, is_critical, crit_multiplier).max(1.0);
            projectile_is_critical = is_critical;
        }

        ranged_cooldown.0.reset();
        commands.spawn((
            Projectile {
                velocity: direction_normalized * ranged_profile.projectile_speed,
                damage: projectile_damage,
                remaining_distance: ranged_profile.projectile_max_distance,
                radius: RANGED_PROJECTILE_HIT_RADIUS,
                source_team: commander_unit.team,
                source_kind: commander_unit.kind,
                source_hero_subtype: hero_subtype.map(|value| value.subtype),
                is_critical: projectile_is_critical,
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
    banner_state: Option<Res<BannerState>>,
    active_formation: Res<ActiveFormation>,
    formation_mods: Res<FormationModifiers>,
    progression: Option<Res<Progression>>,
    global_buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    equipment_effects: Option<Res<EquipmentArmyEffects>>,
    commander_motion: Option<Res<CommanderMotionState>>,
    mut crit_rng: Local<CombatRngState>,
    inventory: Res<InventoryState>,
    mut attackers: Query<(
        Entity,
        &Unit,
        Option<&UnitTier>,
        Option<&HeroSubtypeUnit>,
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
        Option<&ArmorLockedZero>,
        Option<&Morale>,
    )>,
) {
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    let level_multiplier = progression
        .as_ref()
        .map(|value| commander_level_combat_multiplier(value.level))
        .unwrap_or(1.0);
    let target_snapshot: Vec<(Entity, Unit, Vec2, f32, f32, f32)> = targets
        .iter()
        .map(
            |(entity, unit, transform, health, tier, armor, armor_locked, target_morale)| {
                let base_armor = armor.map(|value| value.0).unwrap_or(0.0);
                let morale_defense_multiplier = target_morale
                    .copied()
                    .map(|value| morale_armor_multiplier(value.ratio()))
                    .unwrap_or(1.0);
                let effective_armor = if armor_locked.is_some() {
                    0.0
                } else if unit.team == Team::Friendly {
                    let gear_armor_bonus = gear_bonuses_for_unit_with_banner_state(
                        &inventory,
                        unit.kind,
                        tier.copied().map(|value| value.0),
                        banner_item_active,
                    )
                    .armor_bonus;
                    let temporary_armor_bonus = equipment_effects
                        .as_deref()
                        .map(|effects| effects.temporary_armor_bonus)
                        .unwrap_or(0.0);
                    let armor_with_buffs = base_armor
                        + gear_armor_bonus
                        + temporary_armor_bonus
                        + global_buffs
                            .as_ref()
                            .map(|buff| buff.armor_bonus)
                            .unwrap_or(0.0);
                    (armor_with_buffs.max(0.0)
                        * formation_mods.defense_multiplier
                        * morale_defense_multiplier)
                        .max(0.0)
                } else {
                    (base_armor.max(0.0) * morale_defense_multiplier).max(0.0)
                };
                (
                    entity,
                    *unit,
                    transform.translation.truncate(),
                    health.current,
                    health.max,
                    effective_armor,
                )
            },
        )
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
    let inside_formation_bonus_multiplier = global_buffs
        .as_ref()
        .map(|buff| buff.inside_formation_damage_multiplier)
        .unwrap_or(1.0);

    for (
        attacker_entity,
        attacker_unit,
        attacker_tier,
        attacker_hero_subtype,
        attacker_morale,
        attacker_transform,
        attack_profile,
        mut attack_cd,
    ) in &mut attackers
    {
        if !attack_cd.0.finished() {
            continue;
        }
        if unit_is_non_damaging_support(*attacker_unit) {
            // Keep timer cadence without emitting any damage for pure support units.
            attack_cd.0.reset();
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
            let morale_damage_multiplier = attacker_morale
                .copied()
                .map(|value| morale_damage_multiplier_from_ratio(value.ratio()))
                .unwrap_or(1.0);
            let outgoing_multiplier = if attacker_unit.team == Team::Friendly {
                let base = friendly_outgoing_multiplier(
                    effective_formation_offense_multiplier(
                        &formation_mods,
                        commander_motion.as_deref(),
                    ),
                    1.0,
                    level_multiplier,
                );
                let inside_multiplier = inside_formation_damage_multiplier(
                    &formation_context,
                    target_position,
                    target_kind,
                    *active_formation,
                    active_formation_config(&data, *active_formation).slot_spacing,
                    inside_formation_bonus_multiplier,
                );
                base * inside_multiplier
                    * conditional_effects
                        .as_deref()
                        .map(|effects| effects.friendly_damage_multiplier)
                        .unwrap_or(1.0)
                    * morale_damage_multiplier
            } else {
                morale_damage_multiplier
            };

            let melee_bonus_mult = if attacker_unit.team == Team::Friendly {
                gear_bonuses_for_unit_with_banner_state(
                    &inventory,
                    attacker_unit.kind,
                    attacker_tier.copied().map(|tier| tier.0),
                    banner_item_active,
                )
                .melee_damage_multiplier
            } else {
                0.0
            };
            let upgrade_damage_bonus = if attacker_unit.team == Team::Friendly {
                global_buffs
                    .as_ref()
                    .map(|buff| buff.damage_multiplier - 1.0)
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            let counter_multiplier =
                role_counter_damage_multiplier(attacker_unit.kind, target_kind);
            let hero_profile = attacker_hero_subtype
                .map(|value| hero_subtype_combat_profile(value.subtype))
                .unwrap_or_default();
            let hero_matchup_multiplier = attacker_hero_subtype
                .map(|value| hero_subtype_matchup_multiplier(value.subtype, target_kind))
                .unwrap_or(1.0);
            let mut outgoing_damage = apply_percent_increase_to_base_plus_additive(
                attack_profile.damage,
                0.0,
                upgrade_damage_bonus + melee_bonus_mult,
            ) * outgoing_multiplier
                * counter_multiplier
                * hero_profile.outgoing_damage_multiplier
                * hero_matchup_multiplier;
            let mut is_critical = false;
            if attacker_unit.team == Team::Friendly {
                let (crit_chance, crit_multiplier) =
                    friendly_critical_parameters(global_buffs.as_deref());
                is_critical = roll_critical_hit(crit_chance, &mut crit_rng);
                outgoing_damage =
                    apply_critical_multiplier(outgoing_damage, is_critical, crit_multiplier);
            }
            let mut damage = compute_damage(outgoing_damage, armor, 1.0);
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
                damage = target_health + 1.0;
            }

            damage_events.send(DamageEvent {
                target: target_entity,
                source_team: attacker_unit.team,
                source_entity: Some(attacker_entity),
                amount: damage,
                execute,
                critical: is_critical,
            });
        }
    }
}

pub fn unit_is_non_damaging_support(unit: Unit) -> bool {
    unit.kind.is_priest()
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

pub fn friendly_outgoing_multiplier(
    formation_offense: f32,
    global_damage_multiplier: f32,
    commander_level_multiplier: f32,
) -> f32 {
    (formation_offense * global_damage_multiplier * commander_level_multiplier)
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
    let scaled_damage = (base_damage * outgoing_multiplier).max(0.0);
    let reduction_ratio = armor_reduction_ratio(armor);
    (scaled_damage * (1.0 - reduction_ratio)).max(1.0)
}

pub fn role_counter_damage_multiplier(attacker_kind: UnitKind, defender_kind: UnitKind) -> f32 {
    let mut multiplier = 1.0;

    if attacker_kind.has_role_tag(UnitRoleTag::AntiCavalry)
        && defender_kind.has_role_tag(UnitRoleTag::Cavalry)
    {
        multiplier += ANTI_CAVALRY_BONUS;
    }

    if attacker_kind.has_role_tag(UnitRoleTag::Cavalry)
        && defender_kind.has_role_tag(UnitRoleTag::Frontline)
    {
        multiplier += CAVALRY_VS_FRONTLINE_BONUS;
    }

    if attacker_kind.has_role_tag(UnitRoleTag::Cavalry)
        && defender_kind.has_role_tag(UnitRoleTag::AntiCavalry)
    {
        multiplier -= CAVALRY_VS_ANTI_CAVALRY_PENALTY;
    }

    if attacker_kind.has_role_tag(UnitRoleTag::AntiArmor) {
        multiplier += match defender_kind.armor_class() {
            UnitArmorClass::Heavy => 0.30,
            UnitArmorClass::Armored => 0.20,
            UnitArmorClass::Light => 0.05,
            UnitArmorClass::Unarmored => -0.10,
        };
    }

    if attacker_kind.has_role_tag(UnitRoleTag::Skirmisher)
        && defender_kind.has_role_tag(UnitRoleTag::Support)
    {
        multiplier += SKIRMISHER_VS_SUPPORT_BONUS;
    }

    if attacker_kind.is_archer_line()
        && !attacker_kind.has_role_tag(UnitRoleTag::AntiCavalry)
        && defender_kind.has_role_tag(UnitRoleTag::Cavalry)
    {
        multiplier -= ARCHER_VS_CAVALRY_PENALTY;
    }

    if attacker_kind.has_role_tag(UnitRoleTag::Support)
        && defender_kind.has_role_tag(UnitRoleTag::Frontline)
    {
        multiplier -= SUPPORT_VS_FRONTLINE_PENALTY;
    }

    multiplier.clamp(COUNTER_MULTIPLIER_MIN, COUNTER_MULTIPLIER_MAX)
}

fn morale_damage_multiplier_from_ratio(morale_ratio: f32) -> f32 {
    morale_damage_multiplier(morale_ratio)
}

pub fn apply_percent_increase_to_base_plus_additive(
    base_stat: f32,
    additive_bonus: f32,
    percent_bonus: f32,
) -> f32 {
    let base_plus_additive = (base_stat + additive_bonus).max(0.0);
    (base_plus_additive * (1.0 + percent_bonus)).max(0.0)
}

fn combined_percentage_multiplier(total_percent_bonus: f32, min_multiplier: f32) -> f32 {
    (1.0 + total_percent_bonus).max(min_multiplier)
}

pub fn armor_reduction_ratio(armor: f32) -> f32 {
    let clamped_armor = armor.max(0.0);
    if clamped_armor <= 0.0 {
        return 0.0;
    }
    (clamped_armor / (clamped_armor + ARMOR_DIMINISHING_SCALE))
        .clamp(0.0, MAX_ARMOR_REDUCTION_RATIO)
}

fn critical_hit(roll: f32, crit_chance: f32) -> bool {
    let clamped_chance = crit_chance.clamp(0.0, MAX_CRIT_CHANCE);
    if clamped_chance <= 0.0 {
        return false;
    }
    roll.clamp(0.0, 1.0) < clamped_chance
}

fn roll_critical_hit(crit_chance: f32, rng: &mut CombatRngState) -> bool {
    critical_hit(rng.next_f32(), crit_chance)
}

fn apply_critical_multiplier(damage: f32, is_critical: bool, crit_multiplier: f32) -> f32 {
    if is_critical {
        damage * crit_multiplier.max(1.0)
    } else {
        damage
    }
}

fn friendly_critical_parameters(global_buffs: Option<&GlobalBuffs>) -> (f32, f32) {
    let crit_chance = global_buffs
        .map(|buff| buff.crit_chance_bonus)
        .unwrap_or(0.0);
    let crit_multiplier = global_buffs
        .map(|buff| buff.crit_damage_multiplier)
        .unwrap_or(DEFAULT_CRIT_DAMAGE_MULTIPLIER);
    (crit_chance, crit_multiplier)
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
    inside_bonus_multiplier: f32,
) -> f32 {
    let Some(context) = formation_context else {
        return 1.0;
    };
    if inside_bonus_multiplier <= 1.0 {
        return 1.0;
    }
    if context.recruit_count == 0 || !target_kind.is_friendly_recruit() {
        return 1.0;
    }
    if inside_active_formation_bounds(
        active_formation,
        context.commander_position,
        target_position,
        context.recruit_count,
        slot_spacing,
    ) {
        inside_bonus_multiplier
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

fn is_shielded_block_target_kind(kind: UnitKind) -> bool {
    kind.has_shielded_trait()
}

fn enemy_block_chance_for_difficulty(difficulty: GameDifficulty, block_enabled: bool) -> f32 {
    if !block_enabled {
        return 0.0;
    }
    match difficulty {
        GameDifficulty::Recruit => 0.0,
        GameDifficulty::Experienced => PEASANT_INFANTRY_BLOCK_CHANCE,
        GameDifficulty::AloneAgainstTheInfidels => {
            (PEASANT_INFANTRY_BLOCK_CHANCE + INFIDELS_BLOCK_CHANCE_BONUS).clamp(0.0, 0.95)
        }
    }
}

fn should_enemy_block_hit(
    target_team: Team,
    target_kind: UnitKind,
    block_chance: f32,
    rng: &mut CombatRngState,
) -> bool {
    target_team == Team::Enemy
        && block_chance > 0.0
        && is_shielded_block_target_kind(target_kind)
        && rng.next_f32() < block_chance
}

fn should_friendly_block_hit(
    target_team: Team,
    target_kind: UnitKind,
    block_chance: f32,
    rng: &mut CombatRngState,
) -> bool {
    target_team == Team::Friendly
        && block_chance > 0.0
        && is_shielded_block_target_kind(target_kind)
        && rng.next_f32() < block_chance
}

fn should_block_hit(
    target_team: Team,
    target_kind: UnitKind,
    enemy_block_chance: f32,
    friendly_block_chance: f32,
    rng: &mut CombatRngState,
) -> bool {
    should_enemy_block_hit(target_team, target_kind, enemy_block_chance, rng)
        || should_friendly_block_hit(target_team, target_kind, friendly_block_chance, rng)
}

fn is_melee_reflect_eligible_source_kind(kind: UnitKind) -> bool {
    !kind.is_archer_line()
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn apply_damage_events(
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    mut damage_events: EventReader<DamageEvent>,
    mut damage_text_events: EventWriter<DamageTextEvent>,
    mut damaged_events: EventWriter<UnitDamagedEvent>,
    mut health_query: Query<(
        &mut Health,
        &Unit,
        &Transform,
        Option<&BaseMaxHealth>,
        Option<&HeroSubtypeUnit>,
    )>,
    mut block_rng: Local<CombatRngState>,
) {
    let difficulty = setup_selection
        .as_deref()
        .map(|selection| selection.difficulty)
        .unwrap_or(GameDifficulty::Recruit);
    let block_chance = enemy_block_chance_for_difficulty(
        difficulty,
        data.difficulties
            .for_difficulty(difficulty)
            .enemy_block_enabled,
    );
    let friendly_block_bonus = formation_shielded_block_bonus(&data, *active_formation);
    let melee_reflect_ratio = formation_melee_reflect_ratio(&data, *active_formation);
    for event in damage_events.read() {
        if event.amount <= 0.0 {
            continue;
        }
        let mut pending_life_leech: Option<(Entity, f32)> = None;
        let mut pending_melee_reflect: Option<(Entity, f32)> = None;
        {
            let Ok((mut health, unit, transform, _, hero_subtype)) =
                health_query.get_mut(event.target)
            else {
                continue;
            };
            let blocked = should_block_hit(
                unit.team,
                unit.kind,
                block_chance,
                friendly_block_bonus,
                &mut block_rng,
            );
            if blocked {
                damage_text_events.send(DamageTextEvent {
                    world_position: transform.translation.truncate(),
                    target_team: unit.team,
                    kind: DamageTextKind::Blocked,
                });
            }

            let incoming_multiplier = hero_subtype
                .map(|value| hero_subtype_combat_profile(value.subtype).incoming_damage_multiplier)
                .unwrap_or(1.0);
            let block_adjusted_amount = blocked_damage_amount(event.amount, blocked);
            let applied_damage =
                (block_adjusted_amount * incoming_multiplier).min(health.current.max(0.0));
            if applied_damage <= 0.0 {
                continue;
            }
            health.current = (health.current - applied_damage).max(0.0);
            if is_dead_health(health.current) {
                health.current = 0.0;
            }
            damaged_events.send(UnitDamagedEvent {
                target: event.target,
                team: unit.team,
                amount: applied_damage,
            });
            if event.critical {
                damage_text_events.send(DamageTextEvent {
                    world_position: transform.translation.truncate(),
                    target_team: unit.team,
                    kind: DamageTextKind::CriticalHit,
                });
            }
            if let Some(source_entity) = event.source_entity {
                pending_life_leech = Some((source_entity, applied_damage));
                if melee_reflect_ratio > 0.0
                    && event.source_team == Team::Enemy
                    && unit.team == Team::Friendly
                    && source_entity != event.target
                {
                    let reflected = (applied_damage * melee_reflect_ratio).max(0.0);
                    if reflected > 0.0 {
                        pending_melee_reflect = Some((source_entity, reflected));
                    }
                }
            }
        }

        if let Some((source_entity, applied_damage)) = pending_life_leech
            && source_entity != event.target
            && let Ok((mut source_health, source_unit, _, source_base_max, _)) =
                health_query.get_mut(source_entity)
            && source_health.current > 0.0
        {
            let life_leech_ratio = fanatic_life_leech_ratio(source_unit.kind, &data);
            if life_leech_ratio > 0.0 {
                let max_health = source_base_max
                    .map(|value| value.0)
                    .unwrap_or(source_health.max)
                    .max(1.0);
                let heal = (applied_damage * life_leech_ratio).max(0.0);
                source_health.current = (source_health.current + heal).clamp(0.0, max_health);
            }
        }

        if let Some((source_entity, reflected_damage)) = pending_melee_reflect {
            if source_entity == event.target {
                continue;
            }
            let Ok((mut source_health, source_unit, _, _, _)) = health_query.get_mut(source_entity)
            else {
                continue;
            };
            if source_health.current <= 0.0
                || source_unit.team != Team::Enemy
                || !is_melee_reflect_eligible_source_kind(source_unit.kind)
            {
                continue;
            }
            let applied_reflect = reflected_damage.min(source_health.current.max(0.0));
            if applied_reflect <= 0.0 {
                continue;
            }
            source_health.current = (source_health.current - applied_reflect).max(0.0);
            if is_dead_health(source_health.current) {
                source_health.current = 0.0;
            }
            damaged_events.send(UnitDamagedEvent {
                target: source_entity,
                team: source_unit.team,
                amount: applied_reflect,
            });
        }
    }
}

fn blocked_damage_amount(base_damage: f32, blocked: bool) -> f32 {
    if !blocked {
        return base_damage.max(0.0);
    }
    let mitigation = BLOCK_DAMAGE_MITIGATION_RATIO.clamp(0.0, 0.95);
    (base_damage * (1.0 - mitigation)).max(0.0)
}

fn fanatic_life_leech_ratio(kind: UnitKind, data: &GameData) -> f32 {
    if kind.is_fanatic_line() {
        data.roster_tuning.behavior.fanatic_life_leech_ratio
    } else {
        0.0
    }
}

fn resolve_deaths(
    mut commands: Commands,
    mut death_events: EventWriter<UnitDiedEvent>,
    mut gold_pack_events: EventWriter<SpawnGoldPackEvent>,
    dead_units: Query<(
        Entity,
        &Unit,
        &Health,
        &Transform,
        Option<&EnemySpawnSource>,
    )>,
) {
    for (entity, unit, health, transform, enemy_spawn_source) in &dead_units {
        if is_dead_health(health.current) {
            death_events.send(UnitDiedEvent {
                team: unit.team,
                kind: unit.kind,
                max_health: health.max,
                world_position: transform.translation.truncate(),
                enemy_spawn_lane: enemy_spawn_lane_from_source(enemy_spawn_source),
            });
            if unit.team == Team::Enemy {
                gold_pack_events.send(SpawnGoldPackEvent {
                    world_position: transform.translation.truncate(),
                    gold_value_override: None,
                    pickup_delay_secs: Some(ENEMY_DROP_PICKUP_DELAY_SECS),
                });
            }
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn enemy_spawn_lane_from_source(
    source: Option<&EnemySpawnSource>,
) -> Option<crate::model::EnemySpawnLane> {
    source.map(|value| value.lane)
}

fn is_dead_health(current_health: f32) -> bool {
    current_health <= DEATH_HEALTH_EPSILON
}

#[allow(dead_code)]
fn _satisfy_marker(_enemy: Option<EnemyUnit>) {}

#[cfg(test)]
mod tests {
    use bevy::ecs::event::ManualEventReader;
    use bevy::prelude::{App, Entity, Events, MinimalPlugins, Transform, Update, Vec2};

    use crate::combat::{
        FriendlyFormationContext, apply_critical_multiplier, armor_reduction_ratio,
        commander_level_combat_multiplier, compute_damage, critical_hit,
        effective_formation_offense_multiplier, enemy_block_chance_for_difficulty,
        enemy_target_allowed, fanatic_life_leech_ratio, friendly_critical_parameters,
        friendly_formation_context, friendly_outgoing_multiplier, inside_active_formation_bounds,
        inside_formation_damage_multiplier, is_dead_health, ranged_target_in_window,
        role_counter_damage_multiplier, should_block_hit, should_enemy_block_hit,
        should_execute_target, unit_is_non_damaging_support,
    };
    use crate::data::GameData;
    use crate::formation::{
        ActiveFormation, FormationModifiers, formation_melee_reflect_ratio,
        formation_shielded_block_bonus,
    };
    use crate::model::{
        DamageEvent, DamageTextEvent, DamageTextKind, EnemySpawnLane, EnemySpawnSource,
        GameDifficulty, GlobalBuffs, Health, Team, Unit, UnitDamagedEvent, UnitKind,
    };
    use crate::squad::CommanderMotionState;

    #[test]
    fn damage_formula_respects_armor_floor() {
        let damage = compute_damage(3.0, 10_000.0, 1.0);
        assert_eq!(damage, 1.0);
    }

    #[test]
    fn armor_reduction_uses_diminishing_returns_and_caps_at_ninety_percent() {
        let low = armor_reduction_ratio(25.0);
        let mid = armor_reduction_ratio(100.0);
        let high = armor_reduction_ratio(400.0);
        let extreme = armor_reduction_ratio(100_000.0);

        assert!(low > 0.0);
        assert!(mid > low);
        assert!(high > mid);
        assert!(high - mid < mid - low);
        assert!(extreme <= 0.90 + 0.0001);
        assert!(extreme >= 0.89);
    }

    #[test]
    fn armor_cap_preserves_at_least_ten_percent_of_high_incoming_damage() {
        let damage = compute_damage(100.0, 100_000.0, 1.0);
        assert!((damage - 10.0).abs() < 0.001);
    }

    #[test]
    fn dead_health_uses_small_positive_epsilon() {
        assert!(is_dead_health(0.0));
        assert!(is_dead_health(0.009));
        assert!(!is_dead_health(0.02));
    }

    #[test]
    fn enemy_spawn_lane_mapping_passes_through_optional_source() {
        let source = EnemySpawnSource {
            lane: EnemySpawnLane::Major,
        };
        assert_eq!(
            super::enemy_spawn_lane_from_source(Some(&source)),
            Some(EnemySpawnLane::Major)
        );
        assert_eq!(super::enemy_spawn_lane_from_source(None), None);
    }

    #[test]
    fn friendly_multiplier_has_floor() {
        let multiplier = friendly_outgoing_multiplier(0.6, 0.8, 0.9);
        assert!((multiplier - 0.55).abs() < 0.0001);
    }

    #[test]
    fn critical_hit_threshold_respects_roll_and_chance() {
        assert!(critical_hit(0.19, 0.20));
        assert!(!critical_hit(0.20, 0.20));
        assert!(!critical_hit(0.05, 0.0));
    }

    #[test]
    fn critical_damage_multiplier_applies_only_for_critical_hits() {
        assert!((apply_critical_multiplier(10.0, false, 2.0) - 10.0).abs() < 0.0001);
        assert!((apply_critical_multiplier(10.0, true, 2.0) - 20.0).abs() < 0.0001);
        assert!((apply_critical_multiplier(10.0, true, 0.5) - 10.0).abs() < 0.0001);
    }

    #[test]
    fn critical_hit_chance_is_capped_at_ninety_five_percent() {
        assert!(critical_hit(0.949, 5.0));
        assert!(!critical_hit(0.95, 5.0));
    }

    #[test]
    fn friendly_critical_parameters_default_without_buffs() {
        let (chance, multiplier) = friendly_critical_parameters(None);
        assert!((chance - 0.0).abs() < 0.0001);
        assert!((multiplier - 1.2).abs() < 0.0001);
    }

    #[test]
    fn friendly_critical_parameters_read_global_buffs() {
        let buffs = GlobalBuffs {
            crit_chance_bonus: 0.17,
            crit_damage_multiplier: 1.75,
            ..GlobalBuffs::default()
        };
        let (chance, multiplier) = friendly_critical_parameters(Some(&buffs));
        assert!((chance - 0.17).abs() < 0.0001);
        assert!((multiplier - 1.75).abs() < 0.0001);
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
            UnitKind::MuslimPeasantInfantry,
            ActiveFormation::Square,
            30.0,
            1.2,
        );
        let outside = inside_formation_damage_multiplier(
            &context,
            Vec2::new(220.0, 0.0),
            UnitKind::MuslimPeasantInfantry,
            ActiveFormation::Square,
            30.0,
            1.2,
        );
        assert!((inside - 1.2).abs() < 0.0001);
        assert!((outside - 1.0).abs() < 0.0001);
    }

    #[test]
    fn inside_formation_bonus_requires_upgrade_multiplier_above_one() {
        let context = Some(FriendlyFormationContext {
            commander_position: Vec2::ZERO,
            recruit_count: 9,
        });
        let without_upgrade = inside_formation_damage_multiplier(
            &context,
            Vec2::new(20.0, 15.0),
            UnitKind::MuslimPeasantInfantry,
            ActiveFormation::Square,
            30.0,
            1.0,
        );
        assert!((without_upgrade - 1.0).abs() < 0.0001);
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
                    kind: UnitKind::MuslimPeasantInfantry,
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

    #[test]
    fn priest_is_always_non_damaging_support() {
        let friendly_priest = Unit {
            team: Team::Friendly,
            kind: UnitKind::ChristianPeasantPriest,
            level: 1,
        };
        let enemy_priest = Unit {
            team: Team::Enemy,
            kind: UnitKind::MuslimPeasantPriest,
            level: 1,
        };
        let enemy_raider = Unit {
            team: Team::Enemy,
            kind: UnitKind::MuslimPeasantInfantry,
            level: 1,
        };
        let friendly_infantry = Unit {
            team: Team::Friendly,
            kind: UnitKind::ChristianPeasantInfantry,
            level: 1,
        };
        assert!(unit_is_non_damaging_support(friendly_priest));
        assert!(unit_is_non_damaging_support(enemy_priest));
        assert!(!unit_is_non_damaging_support(enemy_raider));
        assert!(!unit_is_non_damaging_support(friendly_infantry));
    }

    #[test]
    fn enemy_block_chance_scales_by_difficulty_when_enabled() {
        assert_eq!(
            enemy_block_chance_for_difficulty(GameDifficulty::Recruit, false),
            0.0
        );
        assert_eq!(
            enemy_block_chance_for_difficulty(GameDifficulty::Recruit, true),
            0.0
        );
        let experienced = enemy_block_chance_for_difficulty(GameDifficulty::Experienced, true);
        let infidels =
            enemy_block_chance_for_difficulty(GameDifficulty::AloneAgainstTheInfidels, true);
        assert!(experienced > 0.0);
        assert!(infidels > experienced);
    }

    #[test]
    fn enemy_block_roll_requires_shielded_trait() {
        let mut rng = super::CombatRngState {
            state: 0xAABB_CCDD_0011_2233,
        };
        assert!(should_enemy_block_hit(
            Team::Enemy,
            UnitKind::MuslimPeasantInfantry,
            1.0,
            &mut rng
        ));
        assert!(!should_enemy_block_hit(
            Team::Enemy,
            UnitKind::MuslimSpearman,
            1.0,
            &mut rng
        ));
        assert!(!should_enemy_block_hit(
            Team::Friendly,
            UnitKind::ChristianPeasantInfantry,
            1.0,
            &mut rng
        ));
    }

    #[test]
    fn shield_wall_bonus_applies_to_shielded_friendlies_only() {
        let data = GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data");
        let bonus = formation_shielded_block_bonus(&data, ActiveFormation::ShieldWall);
        assert!(bonus > 0.0);

        let mut rng = super::CombatRngState { state: 0 };
        assert!(should_block_hit(
            Team::Friendly,
            UnitKind::ChristianPeasantInfantry,
            0.0,
            1.0,
            &mut rng,
        ));
        assert!(!should_block_hit(
            Team::Friendly,
            UnitKind::ChristianSpearman,
            0.0,
            1.0,
            &mut rng,
        ));
    }

    #[test]
    fn blocked_damage_mitigates_but_does_not_zero_out_damage() {
        let blocked = super::blocked_damage_amount(10.0, true);
        let unblocked = super::blocked_damage_amount(10.0, false);
        assert!(blocked > 0.0);
        assert!(blocked < unblocked);
        assert!((blocked - 3.5).abs() < 0.001);
    }

    #[test]
    fn fanatic_life_leech_ratio_is_defined_for_fanatic_branch_only() {
        let data = GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data");
        let fanatic_ratio = fanatic_life_leech_ratio(UnitKind::ChristianFanatic, &data);
        let flagellant_ratio = fanatic_life_leech_ratio(UnitKind::ChristianFlagellant, &data);
        let elite_flagellant_ratio =
            fanatic_life_leech_ratio(UnitKind::ChristianEliteFlagellant, &data);
        let divine_judge_ratio = fanatic_life_leech_ratio(UnitKind::ChristianDivineJudge, &data);
        assert!(fanatic_ratio > 0.0);
        assert!(flagellant_ratio > 0.0);
        assert!(elite_flagellant_ratio > 0.0);
        assert!(divine_judge_ratio > 0.0);
        assert_eq!(
            fanatic_life_leech_ratio(UnitKind::ChristianPeasantInfantry, &data),
            0.0
        );
    }

    #[test]
    fn counter_multiplier_boosts_anti_cavalry_vs_cavalry() {
        let anti_cavalry = UnitKind::ChristianArmoredHalberdier;
        let cavalry = UnitKind::MuslimEliteShockCavalry;
        let baseline = role_counter_damage_multiplier(
            UnitKind::ChristianPeasantInfantry,
            UnitKind::MuslimPeasantInfantry,
        );
        let boosted = role_counter_damage_multiplier(anti_cavalry, cavalry);
        assert!(boosted > baseline);
    }

    #[test]
    fn counter_multiplier_boosts_anti_armor_vs_heavy_and_penalizes_vs_unarmored() {
        let anti_armor = UnitKind::ChristianSiegeCrossbowman;
        let heavy = UnitKind::MuslimCitadelGuard;
        let unarmored = UnitKind::MuslimPeasantArcher;
        let versus_heavy = role_counter_damage_multiplier(anti_armor, heavy);
        let versus_unarmored = role_counter_damage_multiplier(anti_armor, unarmored);
        assert!(versus_heavy > 1.0);
        assert!(versus_unarmored < 1.0);
    }

    #[test]
    fn counter_multiplier_penalizes_cavalry_into_anti_cavalry() {
        let cavalry = UnitKind::ChristianEliteShockCavalry;
        let anti_cavalry = UnitKind::MuslimShieldedSpearman;
        let into_frontline = role_counter_damage_multiplier(cavalry, UnitKind::MuslimMenAtArms);
        let into_counter = role_counter_damage_multiplier(cavalry, anti_cavalry);
        assert!(into_counter < into_frontline);
    }

    #[test]
    fn shield_wall_reflect_uses_post_mitigation_applied_damage() {
        let data = GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data");
        let reflect_ratio = formation_melee_reflect_ratio(&data, ActiveFormation::ShieldWall);
        assert!(reflect_ratio > 0.0);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(data);
        app.insert_resource(ActiveFormation::ShieldWall);
        app.add_event::<DamageEvent>();
        app.add_event::<DamageTextEvent>();
        app.add_event::<UnitDamagedEvent>();
        app.add_systems(Update, super::apply_damage_events);

        let target = app
            .world_mut()
            .spawn((
                Health {
                    current: 500.0,
                    max: 500.0,
                },
                Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianPeasantArcher,
                    level: 1,
                },
                Transform::default(),
            ))
            .id();
        let source = app
            .world_mut()
            .spawn((
                Health {
                    current: 500.0,
                    max: 500.0,
                },
                Unit {
                    team: Team::Enemy,
                    kind: UnitKind::MuslimPeasantInfantry,
                    level: 1,
                },
                Transform::default(),
            ))
            .id();

        let incoming_damage = 60.0;
        {
            let mut events = app.world_mut().resource_mut::<Events<DamageEvent>>();
            events.send(DamageEvent {
                target,
                source_team: Team::Enemy,
                source_entity: Some(source),
                amount: incoming_damage,
                execute: false,
                critical: false,
            });
        }

        app.update();

        let target_health = app
            .world()
            .entity(target)
            .get::<Health>()
            .expect("target health")
            .current;
        let source_health = app
            .world()
            .entity(source)
            .get::<Health>()
            .expect("source health")
            .current;
        let expected_reflect = incoming_damage * reflect_ratio;
        assert!((target_health - (500.0 - incoming_damage)).abs() < 0.001);
        assert!(
            (source_health - (500.0 - expected_reflect)).abs() < 0.001,
            "source_health={source_health} expected={} reflect_ratio={reflect_ratio}",
            500.0 - expected_reflect
        );

        let damage_events = app.world().resource::<Events<UnitDamagedEvent>>();
        let mut reader = ManualEventReader::<UnitDamagedEvent>::default();
        let emitted = reader.read(damage_events).copied().collect::<Vec<_>>();
        assert_eq!(emitted.len(), 2);
        assert!(
            emitted
                .iter()
                .any(|event| event.target == target
                    && (event.amount - incoming_damage).abs() < 0.001)
        );
        assert!(
            emitted
                .iter()
                .any(|event| event.target == source
                    && (event.amount - expected_reflect).abs() < 0.001)
        );
    }

    #[test]
    fn shield_wall_reflect_preserves_critical_hit_amount_without_reflect_crit_text() {
        let data = GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data");
        let reflect_ratio = formation_melee_reflect_ratio(&data, ActiveFormation::ShieldWall);
        assert!(reflect_ratio > 0.0);

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(data);
        app.insert_resource(ActiveFormation::ShieldWall);
        app.add_event::<DamageEvent>();
        app.add_event::<DamageTextEvent>();
        app.add_event::<UnitDamagedEvent>();
        app.add_systems(Update, super::apply_damage_events);

        let target = app
            .world_mut()
            .spawn((
                Health {
                    current: 1_000.0,
                    max: 1_000.0,
                },
                Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianPeasantArcher,
                    level: 1,
                },
                Transform::default(),
            ))
            .id();
        let source = app
            .world_mut()
            .spawn((
                Health {
                    current: 1_000.0,
                    max: 1_000.0,
                },
                Unit {
                    team: Team::Enemy,
                    kind: UnitKind::MuslimPeasantInfantry,
                    level: 1,
                },
                Transform::default(),
            ))
            .id();

        let normal = 40.0;
        let critical = 90.0;
        {
            let mut events = app.world_mut().resource_mut::<Events<DamageEvent>>();
            events.send(DamageEvent {
                target,
                source_team: Team::Enemy,
                source_entity: Some(source),
                amount: normal,
                execute: false,
                critical: false,
            });
            events.send(DamageEvent {
                target,
                source_team: Team::Enemy,
                source_entity: Some(source),
                amount: critical,
                execute: false,
                critical: true,
            });
        }

        app.update();

        let expected_reflect_total = (normal + critical) * reflect_ratio;
        let source_health = app
            .world()
            .entity(source)
            .get::<Health>()
            .expect("source health")
            .current;
        assert!(
            (source_health - (1_000.0 - expected_reflect_total)).abs() < 0.001,
            "source_health={source_health} expected={} reflect_ratio={reflect_ratio}",
            1_000.0 - expected_reflect_total
        );

        let mut damage_reader = ManualEventReader::<UnitDamagedEvent>::default();
        let emitted_damage = damage_reader
            .read(app.world().resource::<Events<UnitDamagedEvent>>())
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            emitted_damage
                .iter()
                .filter(|event| event.target == source)
                .count(),
            2
        );

        let mut text_reader = ManualEventReader::<DamageTextEvent>::default();
        let emitted_text = text_reader
            .read(app.world().resource::<Events<DamageTextEvent>>())
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            emitted_text
                .iter()
                .filter(|event| event.kind == DamageTextKind::CriticalHit)
                .count(),
            1
        );
    }
}
