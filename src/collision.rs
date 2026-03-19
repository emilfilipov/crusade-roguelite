use bevy::prelude::*;

use crate::data::GameData;
use crate::formation::{ActiveFormation, active_formation_config};
use crate::map::MapBounds;
use crate::model::{ColliderRadius, GameState, Team, Unit, UnitKind};

pub struct CollisionPlugin;

impl Plugin for CollisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            resolve_unit_collisions.run_if(in_state(GameState::InRun)),
        );
    }
}

#[allow(clippy::type_complexity)]
fn resolve_unit_collisions(
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    mut unit_queries: ParamSet<(
        Query<(Entity, &ColliderRadius, &Transform, &Unit), With<Unit>>,
        Query<&mut Transform, With<Unit>>,
    )>,
    bounds: Option<Res<MapBounds>>,
) {
    let snapshot: Vec<(Entity, f32, Vec2, Unit)> = {
        let read_units = unit_queries.p0();
        read_units
            .iter()
            .map(|(entity, radius, transform, unit)| {
                (
                    entity,
                    radius.0.max(0.0),
                    transform.translation.truncate(),
                    *unit,
                )
            })
            .collect()
    };
    if snapshot.len() < 2 {
        return;
    }
    let commander_position = snapshot.iter().find_map(|(_, _, position, unit)| {
        if unit.kind == UnitKind::Commander {
            Some(*position)
        } else {
            None
        }
    });
    let inner_retinue_radius = active_formation_config(&data, *active_formation).slot_spacing * 1.6;

    let mut corrections = vec![Vec2::ZERO; snapshot.len()];
    for i in 0..snapshot.len() {
        for j in (i + 1)..snapshot.len() {
            let (_, radius_a, position_a, unit_a) = snapshot[i];
            let (_, radius_b, position_b, unit_b) = snapshot[j];
            if !should_resolve_collision_pair(
                unit_a,
                position_a,
                unit_b,
                position_b,
                commander_position,
                inner_retinue_radius,
            ) {
                continue;
            }
            let min_distance = radius_a + radius_b;
            if min_distance <= 0.0 {
                continue;
            }
            if let Some(push) = pair_push(position_a, position_b, min_distance, (i + j) as u32) {
                corrections[i] -= push;
                corrections[j] += push;
            }
        }
    }

    for (index, (entity, _, _, _)) in snapshot.iter().enumerate() {
        let correction = corrections[index];
        if correction.length_squared() <= 0.000001 {
            continue;
        }
        if let Ok(mut transform) = unit_queries.p1().get_mut(*entity) {
            transform.translation.x += correction.x;
            transform.translation.y += correction.y;
            if let Some(map_bounds) = &bounds {
                transform.translation.x = transform
                    .translation
                    .x
                    .clamp(-map_bounds.half_width, map_bounds.half_width);
                transform.translation.y = transform
                    .translation
                    .y
                    .clamp(-map_bounds.half_height, map_bounds.half_height);
            }
        }
    }
}

pub fn should_resolve_collision_pair(
    unit_a: Unit,
    position_a: Vec2,
    unit_b: Unit,
    position_b: Vec2,
    commander_position: Option<Vec2>,
    inner_retinue_radius: f32,
) -> bool {
    let enemy_a = unit_a.team == Team::Enemy;
    let enemy_b = unit_b.team == Team::Enemy;
    if enemy_a && enemy_b {
        return true;
    }
    if enemy_a == enemy_b {
        return false;
    }
    if enemy_a {
        return is_inner_ring_retinue(unit_b, position_b, commander_position, inner_retinue_radius);
    }
    is_inner_ring_retinue(unit_a, position_a, commander_position, inner_retinue_radius)
}

fn is_inner_ring_retinue(
    unit: Unit,
    position: Vec2,
    commander_position: Option<Vec2>,
    inner_retinue_radius: f32,
) -> bool {
    if unit.team != Team::Friendly || unit.kind == UnitKind::Commander {
        return false;
    }
    let Some(commander_pos) = commander_position else {
        return false;
    };
    position.distance_squared(commander_pos) <= inner_retinue_radius * inner_retinue_radius
}

pub fn pair_push(position_a: Vec2, position_b: Vec2, min_distance: f32, seed: u32) -> Option<Vec2> {
    let delta = position_b - position_a;
    let distance_sq = delta.length_squared();
    let min_distance_sq = min_distance * min_distance;
    if distance_sq >= min_distance_sq {
        return None;
    }

    let direction = if distance_sq <= 0.000001 {
        let angle = (seed as f32 * 0.618_033_95) * std::f32::consts::TAU;
        Vec2::new(angle.cos(), angle.sin())
    } else {
        delta.normalize()
    };
    let distance = distance_sq.sqrt();
    let overlap = (min_distance - distance).max(0.0);
    if overlap <= 0.0 {
        return None;
    }
    Some(direction * overlap * 0.5)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::collision::{pair_push, should_resolve_collision_pair};
    use crate::model::{Team, Unit, UnitKind};

    fn unit(team: Team, kind: UnitKind) -> Unit {
        Unit {
            team,
            kind,
            level: 1,
        }
    }

    #[test]
    fn overlapping_units_generate_push_vector() {
        let push = pair_push(Vec2::ZERO, Vec2::new(5.0, 0.0), 10.0, 1).expect("push");
        assert!(push.x > 0.0);
        assert!(push.y.abs() < 0.001);
    }

    #[test]
    fn separated_units_have_no_push_vector() {
        assert_eq!(pair_push(Vec2::ZERO, Vec2::new(20.0, 0.0), 10.0, 3), None);
    }

    #[test]
    fn exact_overlap_uses_deterministic_fallback_direction() {
        let push = pair_push(Vec2::ZERO, Vec2::ZERO, 10.0, 7).expect("push");
        assert!(push.length() > 0.0);
    }

    #[test]
    fn collision_rules_match_enemy_and_inner_ring_design() {
        let commander_pos = Some(Vec2::ZERO);
        let inner_radius = 24.0;
        let enemy = unit(Team::Enemy, UnitKind::EnemyBanditRaider);
        let commander = unit(Team::Friendly, UnitKind::Commander);
        let inner_retinue = unit(Team::Friendly, UnitKind::InfantryKnight);
        let outer_retinue = unit(Team::Friendly, UnitKind::InfantryKnight);

        assert!(should_resolve_collision_pair(
            enemy,
            Vec2::new(10.0, 0.0),
            unit(Team::Enemy, UnitKind::EnemyBanditRaider),
            Vec2::new(13.0, 0.0),
            commander_pos,
            inner_radius,
        ));
        assert!(!should_resolve_collision_pair(
            enemy,
            Vec2::new(10.0, 0.0),
            commander,
            Vec2::ZERO,
            commander_pos,
            inner_radius,
        ));
        assert!(should_resolve_collision_pair(
            enemy,
            Vec2::new(10.0, 0.0),
            inner_retinue,
            Vec2::new(8.0, 0.0),
            commander_pos,
            inner_radius,
        ));
        assert!(!should_resolve_collision_pair(
            enemy,
            Vec2::new(10.0, 0.0),
            outer_retinue,
            Vec2::new(50.0, 0.0),
            commander_pos,
            inner_radius,
        ));
    }
}
