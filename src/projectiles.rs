use bevy::prelude::*;

use crate::banner::BannerState;
use crate::combat::{compute_damage, should_execute_target};
use crate::data::GameData;
use crate::inventory::{
    EquipmentArmyEffects, InventoryState, gear_bonuses_for_unit_with_banner_state,
};
use crate::model::{
    DamageEvent, GameDifficulty, GameState, GlobalBuffs, Health, MatchSetupSelection, Morale, Team,
    Unit,
};
use crate::squad::ArmorLockedZero;
use crate::upgrades::ConditionalUpgradeEffects;

#[derive(Component, Clone, Copy, Debug)]
pub struct Projectile {
    pub velocity: Vec2,
    pub damage: f32,
    pub remaining_distance: f32,
    pub radius: f32,
    pub source_team: Team,
    pub is_critical: bool,
}

const BRACKET_LOW_THRESHOLD: f32 = 0.5;
const LOW_MORALE_ARMOR_DEBUFF_MAX_RATIO: f32 = 0.12;
const EXPERIENCED_DODGE_CHANCE: f32 = 0.18;
const INFIDELS_DODGE_CHANCE: f32 = 0.30;

#[derive(Clone, Copy, Debug)]
struct ProjectileRngState {
    state: u64,
}

impl Default for ProjectileRngState {
    fn default() -> Self {
        Self {
            state: 0xB5A1_C29D_7FF0_3412,
        }
    }
}

impl ProjectileRngState {
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

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (tick_projectiles, projectile_collisions).run_if(in_state(GameState::InRun)),
        );
    }
}

fn tick_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut projectiles: Query<(Entity, &mut Transform, &mut Projectile)>,
) {
    for (entity, mut transform, mut projectile) in &mut projectiles {
        let dt = time.delta_seconds();
        let travel = projectile.velocity.length() * dt;
        transform.translation.x += projectile.velocity.x * dt;
        transform.translation.y += projectile.velocity.y * dt;
        projectile.remaining_distance -= travel;
        if projectile.remaining_distance <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn projectile_collisions(
    mut commands: Commands,
    mut damage_events: EventWriter<DamageEvent>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    banner_state: Option<Res<BannerState>>,
    buffs: Option<Res<GlobalBuffs>>,

    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    inventory: Res<InventoryState>,
    equipment_effects: Option<Res<EquipmentArmyEffects>>,
    mut dodge_rng: Local<ProjectileRngState>,
    projectiles: Query<(Entity, &Transform, &Projectile)>,
    targets: Query<(
        Entity,
        &Unit,
        &Transform,
        &Health,
        Option<&crate::model::UnitTier>,
        Option<&crate::model::Armor>,
        Option<&ArmorLockedZero>,
        Option<&Morale>,
    )>,
) {
    let difficulty = setup_selection
        .as_deref()
        .map(|selection| selection.difficulty)
        .unwrap_or(GameDifficulty::Recruit);
    let dodge_chance = enemy_ranged_dodge_chance_for_difficulty(
        difficulty,
        data.difficulties
            .for_difficulty(difficulty)
            .enemy_ranged_dodge_enabled,
    );
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    for (projectile_entity, projectile_transform, projectile) in &projectiles {
        let projectile_pos = projectile_transform.translation.truncate();
        let mut hit = false;
        for (
            target_entity,
            target_unit,
            target_transform,
            target_health,
            target_tier,
            target_armor,
            armor_locked,
            target_morale,
        ) in &targets
        {
            if target_unit.team == projectile.source_team || target_health.current <= 0.0 {
                continue;
            }
            let target_pos = target_transform.translation.truncate();
            if projectile_pos.distance(target_pos) <= projectile.radius {
                if should_enemy_dodge_projectile(
                    projectile.source_team,
                    target_unit.team,
                    dodge_chance,
                    &mut dodge_rng,
                ) {
                    hit = true;
                    break;
                }
                let base_armor = target_armor.map(|value| value.0).unwrap_or(0.0);
                let morale_armor_multiplier = target_morale
                    .copied()
                    .map(|value| morale_armor_multiplier_from_ratio(value.ratio()))
                    .unwrap_or(1.0);

                let effective_armor = if armor_locked.is_some() {
                    0.0
                } else if target_unit.team == Team::Friendly {
                    let gear_armor_bonus = gear_bonuses_for_unit_with_banner_state(
                        &inventory,
                        target_unit.kind,
                        target_tier.copied().map(|value| value.0),
                        banner_item_active,
                    )
                    .armor_bonus;
                    let temporary_armor_bonus = equipment_effects
                        .as_deref()
                        .map(|effects| effects.temporary_armor_bonus)
                        .unwrap_or(0.0);
                    (base_armor
                        + gear_armor_bonus
                        + temporary_armor_bonus
                        + buffs.as_ref().map(|value| value.armor_bonus).unwrap_or(0.0))
                        * morale_armor_multiplier
                } else {
                    base_armor * morale_armor_multiplier
                };
                let execute_threshold = conditional_effects
                    .as_deref()
                    .map(|effects| effects.execute_below_health_ratio)
                    .unwrap_or(0.0);
                let execute = should_execute_target(
                    projectile.source_team,
                    target_unit.team,
                    target_health.current,
                    target_health.max,
                    execute_threshold,
                );
                let damage = if execute {
                    target_health.current + 1.0
                } else {
                    compute_damage(projectile.damage, effective_armor, 1.0)
                };
                damage_events.send(DamageEvent {
                    target: target_entity,
                    source_team: projectile.source_team,
                    source_entity: None,
                    amount: damage,
                    execute,
                    critical: projectile.is_critical,
                });
                hit = true;
                break;
            }
        }
        if hit {
            commands.entity(projectile_entity).despawn_recursive();
        }
    }
}

fn morale_armor_multiplier_from_ratio(morale_ratio: f32) -> f32 {
    if morale_ratio >= BRACKET_LOW_THRESHOLD {
        return 1.0;
    }
    let normalized =
        ((BRACKET_LOW_THRESHOLD - morale_ratio) / BRACKET_LOW_THRESHOLD).clamp(0.0, 1.0);
    1.0 - normalized * LOW_MORALE_ARMOR_DEBUFF_MAX_RATIO
}

fn enemy_ranged_dodge_chance_for_difficulty(
    difficulty: GameDifficulty,
    dodge_enabled: bool,
) -> f32 {
    if !dodge_enabled {
        return 0.0;
    }
    match difficulty {
        GameDifficulty::Recruit => 0.0,
        GameDifficulty::Experienced => EXPERIENCED_DODGE_CHANCE,
        GameDifficulty::AloneAgainstTheInfidels => INFIDELS_DODGE_CHANCE,
    }
}

fn should_enemy_dodge_projectile(
    source_team: Team,
    target_team: Team,
    dodge_chance: f32,
    rng: &mut ProjectileRngState,
) -> bool {
    source_team == Team::Friendly
        && target_team == Team::Enemy
        && dodge_chance > 0.0
        && rng.next_f32() < dodge_chance.clamp(0.0, 0.95)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::model::{GameDifficulty, Team};
    use crate::projectiles::{
        Projectile, ProjectileRngState, enemy_ranged_dodge_chance_for_difficulty,
        should_enemy_dodge_projectile,
    };

    #[test]
    fn projectile_travel_math_is_correct() {
        let mut projectile = Projectile {
            velocity: Vec2::new(100.0, 0.0),
            damage: 1.0,
            remaining_distance: 50.0,
            radius: 4.0,
            source_team: crate::model::Team::Friendly,
            is_critical: false,
        };
        let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let dt = 0.5;
        let travel = projectile.velocity.length() * dt;
        transform.translation.x += projectile.velocity.x * dt;
        transform.translation.y += projectile.velocity.y * dt;
        projectile.remaining_distance -= travel;
        assert!((transform.translation.x - 50.0).abs() < 0.001);
        assert!((projectile.remaining_distance - 0.0).abs() < 0.001);
    }

    #[test]
    fn ranged_dodge_chance_depends_on_difficulty_and_flag() {
        assert_eq!(
            enemy_ranged_dodge_chance_for_difficulty(GameDifficulty::Recruit, false),
            0.0
        );
        assert_eq!(
            enemy_ranged_dodge_chance_for_difficulty(GameDifficulty::Recruit, true),
            0.0
        );
        let experienced =
            enemy_ranged_dodge_chance_for_difficulty(GameDifficulty::Experienced, true);
        let infidels =
            enemy_ranged_dodge_chance_for_difficulty(GameDifficulty::AloneAgainstTheInfidels, true);
        assert!(experienced > 0.0);
        assert!(infidels > experienced);
    }

    #[test]
    fn dodge_check_applies_only_against_friendly_projectiles_targeting_enemies() {
        let mut rng = ProjectileRngState {
            state: 0x0000_0000_0000_0001,
        };
        let mut seen = false;
        for _ in 0..64 {
            if should_enemy_dodge_projectile(Team::Friendly, Team::Enemy, 0.95, &mut rng) {
                seen = true;
                break;
            }
        }
        assert!(seen);
        assert!(!should_enemy_dodge_projectile(
            Team::Enemy,
            Team::Friendly,
            0.95,
            &mut rng
        ));
        assert!(!should_enemy_dodge_projectile(
            Team::Friendly,
            Team::Friendly,
            0.95,
            &mut rng
        ));
    }
}
