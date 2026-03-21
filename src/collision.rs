use bevy::prelude::*;
use std::collections::HashMap;

use crate::data::GameData;
use crate::formation::{ActiveFormation, active_formation_config};
use crate::map::MapBounds;
use crate::model::{ColliderRadius, GameState, Team, Unit, UnitKind};

const COLLISION_CORRECTION_DAMPING: f32 = 0.84;
const COLLISION_MAX_PUSH_PER_FRAME: f32 = 10.0;
const COLLISION_MIN_OVERLAP_TO_RESOLVE: f32 = 0.05;
const COLLISION_MIN_CELL_SIZE: f32 = 24.0;
const COLLISION_SOLVER_PASSES: usize = 2;
const COLLISION_PAIR_MAX_PUSH: f32 = 6.0;
const ENEMY_ENEMY_COLLISION_DISTANCE_MULTIPLIER: f32 = 1.14;

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
    time: Res<Time>,
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
            .filter_map(|(entity, radius, transform, unit)| {
                if unit.team == Team::Neutral {
                    return None;
                }
                Some((
                    entity,
                    radius.0.max(0.0),
                    transform.translation.truncate(),
                    *unit,
                ))
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
    let max_radius = snapshot
        .iter()
        .map(|(_, radius, _, _)| *radius)
        .fold(0.0_f32, f32::max);
    let cell_size = (max_radius * 2.0).max(COLLISION_MIN_CELL_SIZE);
    let mut positions: Vec<Vec2> = snapshot
        .iter()
        .map(|(_, _, position, _)| *position)
        .collect();
    let original_positions = positions.clone();
    let per_pass_push_cap = COLLISION_MAX_PUSH_PER_FRAME / COLLISION_SOLVER_PASSES as f32;

    for _ in 0..COLLISION_SOLVER_PASSES {
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (index, (_, radius, _, _)) in snapshot.iter().enumerate() {
            if *radius <= 0.0 {
                continue;
            }
            grid.entry(spatial_cell_key(positions[index], cell_size))
                .or_default()
                .push(index);
        }

        let mut corrections = vec![Vec2::ZERO; snapshot.len()];
        for i in 0..snapshot.len() {
            let (_, radius_a, _, unit_a) = snapshot[i];
            if radius_a <= 0.0 {
                continue;
            }
            let position_a = positions[i];
            let cell = spatial_cell_key(position_a, cell_size);
            for neighbor_cell in neighboring_cells(cell) {
                let Some(candidates) = grid.get(&neighbor_cell) else {
                    continue;
                };
                for &j in candidates {
                    if j <= i {
                        continue;
                    }
                    let (_, radius_b, _, unit_b) = snapshot[j];
                    if radius_b <= 0.0 {
                        continue;
                    }
                    let position_b = positions[j];
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
                    let min_distance = pair_min_distance(radius_a, unit_a, radius_b, unit_b);
                    if min_distance <= 0.0 {
                        continue;
                    }
                    if let Some(push) =
                        pair_push(position_a, position_b, min_distance, (i + j) as u32)
                    {
                        corrections[i] -= push;
                        corrections[j] += push;
                    }
                }
            }
        }

        for (index, correction) in corrections.into_iter().enumerate() {
            let damped = damp_collision_correction(
                correction,
                time.delta_seconds(),
                COLLISION_CORRECTION_DAMPING,
                per_pass_push_cap,
            );
            if damped.length_squared() <= 0.000001 {
                continue;
            }
            positions[index] += damped;
            if let Some(map_bounds) = &bounds {
                positions[index].x = positions[index]
                    .x
                    .clamp(-map_bounds.half_width, map_bounds.half_width);
                positions[index].y = positions[index]
                    .y
                    .clamp(-map_bounds.half_height, map_bounds.half_height);
            }
        }
    }

    for (index, (entity, _, _, _)) in snapshot.iter().enumerate() {
        let delta = positions[index] - original_positions[index];
        if delta.length_squared() <= 0.000001 {
            continue;
        }
        if let Ok(mut transform) = unit_queries.p1().get_mut(*entity) {
            transform.translation.x += delta.x;
            transform.translation.y += delta.y;
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

fn spatial_cell_key(position: Vec2, cell_size: f32) -> (i32, i32) {
    let inv = 1.0 / cell_size.max(1.0);
    (
        (position.x * inv).floor() as i32,
        (position.y * inv).floor() as i32,
    )
}

fn neighboring_cells(cell: (i32, i32)) -> [(i32, i32); 9] {
    let (x, y) = cell;
    [
        (x - 1, y - 1),
        (x, y - 1),
        (x + 1, y - 1),
        (x - 1, y),
        (x, y),
        (x + 1, y),
        (x - 1, y + 1),
        (x, y + 1),
        (x + 1, y + 1),
    ]
}

pub fn damp_collision_correction(
    correction: Vec2,
    delta_seconds: f32,
    damping: f32,
    max_push_per_frame: f32,
) -> Vec2 {
    if correction.length_squared() <= 0.000001 || damping <= 0.0 || max_push_per_frame <= 0.0 {
        return Vec2::ZERO;
    }
    // Clamp frame scaling to avoid low-FPS over-corrections that create visible jitter.
    let frame_scale = (delta_seconds.max(0.0) * 60.0).clamp(0.75, 1.0);
    let mut damped = correction * damping * frame_scale;
    let max_len = max_push_per_frame.max(0.0);
    let len = damped.length();
    if len > max_len {
        damped = damped / len * max_len;
    }
    damped
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

fn pair_min_distance(radius_a: f32, unit_a: Unit, radius_b: f32, unit_b: Unit) -> f32 {
    let base = radius_a + radius_b;
    if base <= 0.0 {
        return 0.0;
    }
    if unit_a.team == Team::Enemy && unit_b.team == Team::Enemy {
        base * ENEMY_ENEMY_COLLISION_DISTANCE_MULTIPLIER
    } else {
        base
    }
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
    if overlap <= COLLISION_MIN_OVERLAP_TO_RESOLVE {
        return None;
    }
    Some(direction * (overlap * 0.5).min(COLLISION_PAIR_MAX_PUSH))
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::collision::{
        damp_collision_correction, pair_min_distance, pair_push, should_resolve_collision_pair,
    };
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
    fn separation_solver_converges_towards_minimum_distance() {
        let min_distance = 18.0;
        let mut a = Vec2::ZERO;
        let mut b = Vec2::new(6.0, 0.0);
        for step in 0..24 {
            let Some(push) = pair_push(a, b, min_distance, step) else {
                break;
            };
            let a_correction = damp_collision_correction(-push, 1.0 / 60.0, 0.62, 6.0);
            let b_correction = damp_collision_correction(push, 1.0 / 60.0, 0.62, 6.0);
            a += a_correction;
            b += b_correction;
        }
        assert!(a.distance(b) >= min_distance - 0.4);
    }

    #[test]
    fn damped_correction_is_stable_across_frame_deltas() {
        let correction = Vec2::new(8.0, -2.0);
        let fast = damp_collision_correction(correction, 1.0 / 120.0, 0.62, 6.0);
        let medium = damp_collision_correction(correction, 1.0 / 60.0, 0.62, 6.0);
        let slow = damp_collision_correction(correction, 1.0 / 20.0, 0.62, 6.0);

        assert!(fast.length() > 0.0);
        assert!(medium.length() >= fast.length());
        assert!(slow.length() >= medium.length());
        assert!(slow.length() <= 6.0 + 0.001);
    }

    #[test]
    fn collision_rules_match_enemy_and_inner_ring_design() {
        let commander_pos = Some(Vec2::ZERO);
        let inner_radius = 24.0;
        let enemy = unit(Team::Enemy, UnitKind::EnemyBanditRaider);
        let commander = unit(Team::Friendly, UnitKind::Commander);
        let inner_retinue = unit(Team::Friendly, UnitKind::ChristianPeasantInfantry);
        let outer_retinue = unit(Team::Friendly, UnitKind::ChristianPeasantInfantry);

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

    #[test]
    fn enemy_enemy_pairs_use_larger_spacing_radius() {
        let enemy_a = unit(Team::Enemy, UnitKind::EnemyBanditRaider);
        let enemy_b = unit(Team::Enemy, UnitKind::EnemyBanditRaider);
        let friendly = unit(Team::Friendly, UnitKind::ChristianPeasantInfantry);
        let enemy_spacing = pair_min_distance(10.0, enemy_a, 10.0, enemy_b);
        let mixed_spacing = pair_min_distance(10.0, enemy_a, 10.0, friendly);
        assert!(enemy_spacing > mixed_spacing);
    }
}
