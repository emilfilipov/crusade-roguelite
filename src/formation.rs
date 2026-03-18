use bevy::prelude::*;

use crate::banner::BannerMovementPenalty;
use crate::data::GameData;
use crate::model::{CommanderUnit, FriendlyUnit, GameState};

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActiveFormation {
    #[default]
    Square,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct FormationModifiers {
    pub offense_multiplier: f32,
    pub defense_multiplier: f32,
}

impl Default for FormationModifiers {
    fn default() -> Self {
        Self {
            offense_multiplier: 1.0,
            defense_multiplier: 1.0,
        }
    }
}

pub struct FormationPlugin;

impl Plugin for FormationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveFormation>()
            .init_resource::<FormationModifiers>()
            .add_systems(OnEnter(GameState::MainMenu), load_square_modifiers)
            .add_systems(
                Update,
                (apply_square_formation, sync_friendly_depth_sorting)
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn load_square_modifiers(mut modifiers: ResMut<FormationModifiers>, data: Res<GameData>) {
    modifiers.offense_multiplier = data.formations.square.offense_multiplier;
    modifiers.defense_multiplier = data.formations.square.defense_multiplier;
}

#[allow(clippy::type_complexity)]
fn apply_square_formation(
    time: Res<Time>,
    data: Res<GameData>,
    formation: Res<ActiveFormation>,
    banner_penalty: Option<Res<BannerMovementPenalty>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut friendlies: Query<(Entity, &mut Transform), (With<FriendlyUnit>, Without<CommanderUnit>)>,
) {
    if *formation != ActiveFormation::Square {
        return;
    }
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };

    let spacing = data.formations.square.slot_spacing;
    let mut members: Vec<(Entity, Mut<Transform>)> = friendlies.iter_mut().collect();
    members.sort_by_key(|(entity, _)| entity.index());
    let offsets = square_offsets_excluding_commander_slot(members.len(), spacing);
    let speed_multiplier = banner_penalty
        .as_ref()
        .map(|penalty| penalty.friendly_speed_multiplier)
        .unwrap_or(1.0);

    for ((_, mut transform), offset) in members.into_iter().zip(offsets.into_iter()) {
        let target = commander_transform.translation.truncate() + offset;
        let current = transform.translation.truncate();
        let smooth = (time.delta_seconds() * 10.0 * speed_multiplier).clamp(0.0, 1.0);
        let next = current.lerp(target, smooth);
        transform.translation.x = next.x;
        transform.translation.y = next.y;
    }
}

#[allow(clippy::type_complexity)]
fn sync_friendly_depth_sorting(
    mut units: Query<&mut Transform, (With<FriendlyUnit>, Without<Camera>)>,
) {
    for mut transform in &mut units {
        transform.translation.z = depth_z_for_world_y(transform.translation.y);
    }
}

fn square_offsets_excluding_commander_slot(recruit_count: usize, spacing: f32) -> Vec<Vec2> {
    if recruit_count == 0 {
        return Vec::new();
    }

    let mut offsets = square_offsets(recruit_count + 1, spacing);
    let commander_slot = offsets
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.length_squared()
                .partial_cmp(&b.length_squared())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let commander_offset = offsets.remove(commander_slot);
    for offset in &mut offsets {
        *offset -= commander_offset;
    }
    offsets
}

pub fn depth_z_for_world_y(y: f32) -> f32 {
    20.0 - y * 0.001
}

pub fn square_offsets(count: usize, spacing: f32) -> Vec<Vec2> {
    if count == 0 {
        return Vec::new();
    }
    let side = (count as f32).sqrt().ceil() as i32;
    let half = (side as f32 - 1.0) * 0.5;
    let mut result = Vec::with_capacity(count);
    for idx in 0..count {
        let row = (idx as i32) / side;
        let col = (idx as i32) % side;
        let x = (col as f32 - half) * spacing;
        let y = (row as f32 - half) * spacing;
        result.push(Vec2::new(x, y));
    }
    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bevy::prelude::Vec2;

    use crate::formation::{
        depth_z_for_world_y, square_offsets, square_offsets_excluding_commander_slot,
    };

    #[test]
    fn square_offsets_return_expected_count() {
        let offsets = square_offsets(7, 12.0);
        assert_eq!(offsets.len(), 7);
    }

    #[test]
    fn square_offsets_are_unique() {
        let offsets = square_offsets(9, 10.0);
        let mut set = HashSet::new();
        for offset in offsets {
            set.insert((offset.x as i32, offset.y as i32));
        }
        assert_eq!(set.len(), 9);
    }

    #[test]
    fn zero_count_returns_empty() {
        let offsets = square_offsets(0, 10.0);
        assert_eq!(offsets, Vec::<Vec2>::new());
    }

    #[test]
    fn formation_reserves_commander_slot() {
        let offsets = square_offsets_excluding_commander_slot(8, 12.0);
        assert_eq!(offsets.len(), 8);
        assert!(
            !offsets
                .iter()
                .any(|offset| offset.length_squared() < 0.0001)
        );
    }

    #[test]
    fn depth_sorting_places_lower_units_on_top() {
        let upper = depth_z_for_world_y(100.0);
        let lower = depth_z_for_world_y(-100.0);
        assert!(lower > upper);
    }
}
