use bevy::prelude::*;

use crate::combat::{compute_damage, should_execute_target};
use crate::inventory::{EquipmentArmyEffects, InventoryState, gear_bonuses_for_unit};
use crate::model::{DamageEvent, GameState, GlobalBuffs, Health, Team, Unit};
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
    buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    inventory: Res<InventoryState>,
    equipment_effects: Option<Res<EquipmentArmyEffects>>,
    projectiles: Query<(Entity, &Transform, &Projectile)>,
    targets: Query<(
        Entity,
        &Unit,
        &Transform,
        &Health,
        Option<&crate::model::UnitTier>,
        Option<&crate::model::Armor>,
    )>,
) {
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
        ) in &targets
        {
            if target_unit.team == projectile.source_team || target_health.current <= 0.0 {
                continue;
            }
            let target_pos = target_transform.translation.truncate();
            if projectile_pos.distance(target_pos) <= projectile.radius {
                let base_armor = target_armor.map(|value| value.0).unwrap_or(0.0);
                let effective_armor = if target_unit.team == Team::Friendly {
                    let gear_armor_bonus = gear_bonuses_for_unit(
                        &inventory,
                        target_unit.kind,
                        target_tier.copied().map(|value| value.0),
                    )
                    .armor_bonus;
                    let temporary_armor_bonus = equipment_effects
                        .as_deref()
                        .map(|effects| effects.temporary_armor_bonus)
                        .unwrap_or(0.0);
                    base_armor
                        + gear_armor_bonus
                        + temporary_armor_bonus
                        + buffs.as_ref().map(|value| value.armor_bonus).unwrap_or(0.0)
                } else {
                    base_armor
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

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::projectiles::Projectile;

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
}
