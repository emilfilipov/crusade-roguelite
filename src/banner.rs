use bevy::prelude::*;

use crate::model::{CommanderUnit, GameState, StartRunEvent};
use crate::morale::Cohesion;
use crate::squad::SquadRoster;

#[derive(Component, Clone, Copy, Debug)]
pub struct BannerMarker;

#[derive(Resource, Clone, Copy, Debug)]
pub struct BannerState {
    pub is_dropped: bool,
    pub world_position: Vec2,
}

impl Default for BannerState {
    fn default() -> Self {
        Self {
            is_dropped: false,
            world_position: Vec2::ZERO,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct BannerCombatModifiers {
    pub attack_multiplier: f32,
    pub defense_multiplier: f32,
}

impl Default for BannerCombatModifiers {
    fn default() -> Self {
        Self {
            attack_multiplier: 1.0,
            defense_multiplier: 1.0,
        }
    }
}

pub struct BannerPlugin;

impl Plugin for BannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BannerState>()
            .init_resource::<BannerCombatModifiers>()
            .add_systems(Update, reset_banner_on_run_start)
            .add_systems(
                Update,
                (
                    follow_commander_when_banner_up,
                    drop_banner_condition,
                    recover_banner_nearby,
                    refresh_banner_modifiers,
                )
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_banner_on_run_start(
    mut commands: Commands,
    mut start_events: EventReader<StartRunEvent>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    existing_banner: Query<Entity, With<BannerMarker>>,
    mut banner_state: ResMut<BannerState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    for entity in existing_banner.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let commander_pos = commanders
        .get_single()
        .map(|t| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    banner_state.is_dropped = false;
    banner_state.world_position = commander_pos;
    commands.spawn((
        BannerMarker,
        Transform::from_xyz(commander_pos.x, commander_pos.y, 3.0),
        GlobalTransform::default(),
    ));
}

fn follow_commander_when_banner_up(
    mut banner_state: ResMut<BannerState>,
    mut banner_query: Query<&mut Transform, (With<BannerMarker>, Without<CommanderUnit>)>,
    commanders: Query<&Transform, (With<CommanderUnit>, Without<BannerMarker>)>,
) {
    if banner_state.is_dropped {
        return;
    }
    let Ok(commander) = commanders.get_single() else {
        return;
    };
    let new_position = commander.translation.truncate();
    banner_state.world_position = new_position;
    if let Ok(mut banner_transform) = banner_query.get_single_mut() {
        banner_transform.translation.x = new_position.x;
        banner_transform.translation.y = new_position.y;
    }
}

fn drop_banner_condition(
    cohesion: Res<Cohesion>,
    roster: Res<SquadRoster>,
    mut banner_state: ResMut<BannerState>,
) {
    if !banner_state.is_dropped && should_drop_banner(cohesion.value, roster.casualties) {
        banner_state.is_dropped = true;
    }
}

pub fn should_drop_banner(cohesion: f32, casualties: u32) -> bool {
    cohesion < 25.0 && casualties > 0
}

fn recover_banner_nearby(
    mut banner_state: ResMut<BannerState>,
    commanders: Query<&Transform, With<CommanderUnit>>,
) {
    if !banner_state.is_dropped {
        return;
    }
    let Ok(commander) = commanders.get_single() else {
        return;
    };
    let commander_pos = commander.translation.truncate();
    if commander_pos.distance(banner_state.world_position) <= 40.0 {
        banner_state.is_dropped = false;
    }
}

fn refresh_banner_modifiers(
    banner_state: Res<BannerState>,
    mut modifiers: ResMut<BannerCombatModifiers>,
) {
    if banner_state.is_dropped {
        modifiers.attack_multiplier = 0.8;
        modifiers.defense_multiplier = 0.85;
    } else {
        modifiers.attack_multiplier = 1.0;
        modifiers.defense_multiplier = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use crate::banner::should_drop_banner;

    #[test]
    fn banner_drops_when_cohesion_critical_and_casualties_exist() {
        assert!(should_drop_banner(20.0, 1));
        assert!(!should_drop_banner(30.0, 3));
        assert!(!should_drop_banner(20.0, 0));
    }
}
