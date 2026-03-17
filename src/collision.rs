use bevy::prelude::*;

use crate::map::MapBounds;
use crate::model::{ColliderRadius, GameState, Unit};

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
    mut unit_queries: ParamSet<(
        Query<(Entity, &ColliderRadius, &Transform), With<Unit>>,
        Query<&mut Transform, With<Unit>>,
    )>,
    bounds: Option<Res<MapBounds>>,
) {
    let snapshot: Vec<(Entity, f32, Vec2)> = {
        let read_units = unit_queries.p0();
        read_units
            .iter()
            .map(|(entity, radius, transform)| {
                (entity, radius.0.max(0.0), transform.translation.truncate())
            })
            .collect()
    };
    if snapshot.len() < 2 {
        return;
    }

    let mut corrections = vec![Vec2::ZERO; snapshot.len()];
    for i in 0..snapshot.len() {
        for j in (i + 1)..snapshot.len() {
            let (_, radius_a, position_a) = snapshot[i];
            let (_, radius_b, position_b) = snapshot[j];
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

    for (index, (entity, _, _)) in snapshot.iter().enumerate() {
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

    use crate::collision::pair_push;

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
}
