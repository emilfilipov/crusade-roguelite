use bevy::prelude::{Entity, Vec2};

pub fn should_move_towards_target(
    was_moving: bool,
    distance_to_target: f32,
    stop_distance: f32,
    resume_distance: f32,
) -> bool {
    if was_moving {
        return distance_to_target > stop_distance;
    }
    distance_to_target > resume_distance
}

pub fn chase_step_distance(distance_to_target: f32, stop_distance: f32, max_step: f32) -> f32 {
    if max_step <= 0.0 {
        return 0.0;
    }
    (distance_to_target - stop_distance).max(0.0).min(max_step)
}

pub fn chase_target_positions(all_friendlies: &[(Vec2, bool)]) -> Vec<Vec2> {
    if all_friendlies.is_empty() {
        return Vec::new();
    }
    let has_retinue = all_friendlies.iter().any(|(_, is_commander)| !is_commander);
    all_friendlies
        .iter()
        .filter_map(|(position, is_commander)| {
            if has_retinue && *is_commander {
                None
            } else {
                Some(*position)
            }
        })
        .collect()
}

pub fn choose_nearest(origin: Vec2, candidates: &[Vec2]) -> Option<Vec2> {
    candidates.iter().copied().min_by(|a, b| {
        let da = origin.distance_squared(*a);
        let db = origin.distance_squared(*b);
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    })
}

pub fn choose_support_follow_target(
    self_entity: Entity,
    origin: Vec2,
    preferred_allies: &[(Entity, Vec2)],
    fallback_allies: &[(Entity, Vec2)],
) -> Option<Vec2> {
    choose_nearest_entity_position(origin, preferred_allies, self_entity)
        .or_else(|| choose_nearest_entity_position(origin, fallback_allies, self_entity))
}

fn choose_nearest_entity_position(
    origin: Vec2,
    candidates: &[(Entity, Vec2)],
    excluded_entity: Entity,
) -> Option<Vec2> {
    candidates
        .iter()
        .filter(|(entity, _)| *entity != excluded_entity)
        .min_by(|(_, a), (_, b)| {
            let da = origin.distance_squared(*a);
            let db = origin.distance_squared(*b);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, position)| *position)
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Entity, Vec2};

    use crate::ai::{
        chase_step_distance, chase_target_positions, choose_nearest, choose_support_follow_target,
    };

    #[test]
    fn chooses_nearest_target() {
        let origin = Vec2::new(0.0, 0.0);
        let targets = [
            Vec2::new(5.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(10.0, 0.0),
        ];
        let nearest = choose_nearest(origin, &targets).expect("target");
        assert_eq!(nearest, Vec2::new(2.0, 0.0));
    }

    #[test]
    fn no_targets_returns_none() {
        assert_eq!(choose_nearest(Vec2::ZERO, &[]), None);
    }

    #[test]
    fn chase_targets_exclude_commander_when_retinue_exists() {
        let commander = (Vec2::new(0.0, 0.0), true);
        let retinue = (Vec2::new(10.0, 0.0), false);
        let targets = chase_target_positions(&[commander, retinue]);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], retinue.0);
    }

    #[test]
    fn chase_step_distance_prevents_overshoot_into_stop_range() {
        let step = chase_step_distance(24.0, 20.0, 10.0);
        assert!((step - 4.0).abs() < 0.001);
        assert_eq!(chase_step_distance(19.5, 20.0, 10.0), 0.0);
        assert_eq!(chase_step_distance(30.0, 20.0, 0.0), 0.0);
    }

    #[test]
    fn support_follow_prefers_non_support_allies() {
        let self_entity = Entity::from_raw(1);
        let preferred = [(Entity::from_raw(2), Vec2::new(20.0, 0.0))];
        let fallback = [
            (Entity::from_raw(2), Vec2::new(20.0, 0.0)),
            (Entity::from_raw(3), Vec2::new(5.0, 0.0)),
        ];
        let target = choose_support_follow_target(self_entity, Vec2::ZERO, &preferred, &fallback)
            .expect("target");
        assert_eq!(target, Vec2::new(20.0, 0.0));
    }

    #[test]
    fn support_follow_falls_back_to_any_ally_when_no_preferred_exists() {
        let self_entity = Entity::from_raw(1);
        let preferred = [];
        let fallback = [
            (Entity::from_raw(1), Vec2::new(1.0, 0.0)),
            (Entity::from_raw(3), Vec2::new(6.0, 0.0)),
            (Entity::from_raw(4), Vec2::new(3.0, 0.0)),
        ];
        let target = choose_support_follow_target(self_entity, Vec2::ZERO, &preferred, &fallback)
            .expect("target");
        assert_eq!(target, Vec2::new(3.0, 0.0));
    }
}
