use bevy::prelude::*;

use crate::model::{DamageEvent, GameState, Health, Team, Unit};

#[derive(Component, Clone, Copy, Debug)]
pub struct Projectile {
    pub velocity: Vec2,
    pub damage: f32,
    pub lifetime_secs: f32,
    pub radius: f32,
    pub source_team: Team,
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
        transform.translation.x += projectile.velocity.x * dt;
        transform.translation.y += projectile.velocity.y * dt;
        projectile.lifetime_secs -= dt;
        if projectile.lifetime_secs <= 0.0 {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn projectile_collisions(
    mut commands: Commands,
    mut damage_events: EventWriter<DamageEvent>,
    projectiles: Query<(Entity, &Transform, &Projectile)>,
    targets: Query<(Entity, &Unit, &Transform, &Health)>,
) {
    for (projectile_entity, projectile_transform, projectile) in &projectiles {
        let projectile_pos = projectile_transform.translation.truncate();
        let mut hit = false;
        for (target_entity, target_unit, target_transform, target_health) in &targets {
            if target_unit.team == projectile.source_team || target_health.current <= 0.0 {
                continue;
            }
            let target_pos = target_transform.translation.truncate();
            if projectile_pos.distance(target_pos) <= projectile.radius {
                damage_events.send(DamageEvent {
                    target: target_entity,
                    source_team: projectile.source_team,
                    amount: projectile.damage,
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
            lifetime_secs: 1.0,
            radius: 4.0,
            source_team: crate::model::Team::Friendly,
        };
        let mut transform = Transform::from_xyz(0.0, 0.0, 0.0);
        let dt = 0.5;
        transform.translation.x += projectile.velocity.x * dt;
        transform.translation.y += projectile.velocity.y * dt;
        projectile.lifetime_secs -= dt;
        assert!((transform.translation.x - 50.0).abs() < 0.001);
        assert!((projectile.lifetime_secs - 0.5).abs() < 0.001);
    }
}
