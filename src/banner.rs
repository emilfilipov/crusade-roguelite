use bevy::prelude::*;

use crate::model::{CommanderUnit, FriendlyUnit, GameState, StartRunEvent};
use crate::morale::Cohesion;
use crate::visuals::ArtAssets;

const BANNER_DROP_COHESION_THRESHOLD: f32 = 20.0;
const BANNER_PICKUP_UNLOCK_SECS: f32 = 10.0;
const BANNER_PICKUP_DURATION_SECS: f32 = 5.0;
const BANNER_RECOVERY_RADIUS: f32 = 42.0;
const BANNER_COHESION_RESTORE: f32 = 65.0;
const BANNER_REDROP_GRACE_SECS: f32 = 10.0;
const BANNER_DROPPED_SPEED_MULTIPLIER: f32 = 0.72;
const BANNER_FOLLOW_Y_OFFSET: f32 = 18.0;
const BANNER_FOLLOW_Z: f32 = 9.0;
const BANNER_DROPPED_Z: f32 = 3.0;

pub fn banner_follow_translation(position: Vec2) -> Vec3 {
    Vec3::new(
        position.x,
        position.y + BANNER_FOLLOW_Y_OFFSET,
        BANNER_FOLLOW_Z,
    )
}

fn banner_dropped_translation(position: Vec2) -> Vec3 {
    Vec3::new(position.x, position.y, BANNER_DROPPED_Z)
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BannerMarker;

#[derive(Resource, Clone, Copy, Debug)]
pub struct BannerState {
    pub is_dropped: bool,
    pub world_position: Vec2,
    pub pickup_unlock_remaining: f32,
    pub pickup_progress: f32,
    pub redrop_grace_remaining: f32,
}

impl Default for BannerState {
    fn default() -> Self {
        Self {
            is_dropped: false,
            world_position: Vec2::ZERO,
            pickup_unlock_remaining: 0.0,
            pickup_progress: 0.0,
            redrop_grace_remaining: 0.0,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct BannerMovementPenalty {
    pub friendly_speed_multiplier: f32,
}

impl Default for BannerMovementPenalty {
    fn default() -> Self {
        Self {
            friendly_speed_multiplier: 1.0,
        }
    }
}

pub struct BannerPlugin;

impl Plugin for BannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BannerState>()
            .init_resource::<BannerMovementPenalty>()
            .add_systems(Update, reset_banner_on_run_start)
            .add_systems(
                Update,
                (
                    follow_commander_when_banner_up,
                    drop_banner_on_low_cohesion,
                    tick_banner_recovery,
                    sync_banner_visual,
                    refresh_banner_speed_penalty,
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
    art: Res<ArtAssets>,
    mut banner_state: ResMut<BannerState>,
    mut movement_penalty: ResMut<BannerMovementPenalty>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    for entity in &existing_banner {
        commands.entity(entity).despawn_recursive();
    }

    let commander_pos = commanders
        .get_single()
        .map(|t| t.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    *banner_state = BannerState {
        is_dropped: false,
        world_position: commander_pos,
        pickup_unlock_remaining: 0.0,
        pickup_progress: 0.0,
        redrop_grace_remaining: 0.0,
    };
    movement_penalty.friendly_speed_multiplier = 1.0;

    commands.spawn((
        BannerMarker,
        SpriteBundle {
            texture: art.banner_upright.clone(),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(32.0)),
                ..default()
            },
            transform: Transform::from_translation(banner_follow_translation(commander_pos)),
            ..default()
        },
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
        banner_transform.translation = banner_follow_translation(new_position);
    }
}

fn drop_banner_on_low_cohesion(cohesion: Res<Cohesion>, mut banner_state: ResMut<BannerState>) {
    if should_drop_banner(
        cohesion.value,
        banner_state.is_dropped,
        banner_state.redrop_grace_remaining,
    ) {
        banner_state.is_dropped = true;
        banner_state.pickup_unlock_remaining = BANNER_PICKUP_UNLOCK_SECS;
        banner_state.pickup_progress = 0.0;
        banner_state.redrop_grace_remaining = 0.0;
    }
}

#[allow(clippy::type_complexity)]
fn tick_banner_recovery(
    time: Res<Time>,
    mut cohesion: ResMut<Cohesion>,
    mut banner_state: ResMut<BannerState>,
    friendlies: Query<&Transform, With<FriendlyUnit>>,
) {
    if banner_state.redrop_grace_remaining > 0.0 {
        banner_state.redrop_grace_remaining =
            (banner_state.redrop_grace_remaining - time.delta_seconds()).max(0.0);
    }

    if !banner_state.is_dropped {
        banner_state.pickup_progress = 0.0;
        banner_state.pickup_unlock_remaining = 0.0;
        return;
    }

    if banner_state.pickup_unlock_remaining > 0.0 {
        banner_state.pickup_unlock_remaining =
            (banner_state.pickup_unlock_remaining - time.delta_seconds()).max(0.0);
        banner_state.pickup_progress = 0.0;
        return;
    }

    let in_recovery_range = friendlies.iter().any(|transform| {
        transform
            .translation
            .truncate()
            .distance_squared(banner_state.world_position)
            <= BANNER_RECOVERY_RADIUS * BANNER_RECOVERY_RADIUS
    });

    if in_recovery_range {
        banner_state.pickup_progress += time.delta_seconds();
    } else {
        banner_state.pickup_progress = 0.0;
    }

    if banner_state.pickup_progress >= BANNER_PICKUP_DURATION_SECS {
        banner_state.is_dropped = false;
        banner_state.pickup_progress = 0.0;
        banner_state.pickup_unlock_remaining = 0.0;
        banner_state.redrop_grace_remaining = BANNER_REDROP_GRACE_SECS;
        cohesion.value = BANNER_COHESION_RESTORE;
    }
}

fn sync_banner_visual(
    banner_state: Res<BannerState>,
    art: Res<ArtAssets>,
    mut banner_query: Query<(&mut Handle<Image>, &mut Transform), With<BannerMarker>>,
) {
    if let Ok((mut texture, mut transform)) = banner_query.get_single_mut() {
        if banner_state.is_dropped {
            // Keep dropped banner highly visible by using the upright banner asset.
            *texture = art.banner_upright.clone();
            transform.translation = banner_dropped_translation(banner_state.world_position);
        } else {
            *texture = art.banner_upright.clone();
        }
    }
}

fn refresh_banner_speed_penalty(
    banner_state: Res<BannerState>,
    mut movement_penalty: ResMut<BannerMovementPenalty>,
) {
    movement_penalty.friendly_speed_multiplier = if banner_state.is_dropped {
        BANNER_DROPPED_SPEED_MULTIPLIER
    } else {
        1.0
    };
}

pub fn should_drop_banner(cohesion: f32, is_dropped: bool, redrop_grace_remaining: f32) -> bool {
    !is_dropped && redrop_grace_remaining <= 0.0 && cohesion < BANNER_DROP_COHESION_THRESHOLD
}

pub fn banner_pickup_progress_ratio(state: &BannerState) -> Option<f32> {
    if !state.is_dropped || state.pickup_unlock_remaining > 0.0 || state.pickup_progress <= 0.0 {
        return None;
    }
    Some((state.pickup_progress / BANNER_PICKUP_DURATION_SECS).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec2;

    use crate::banner::{
        BannerState, banner_follow_translation, banner_pickup_progress_ratio, should_drop_banner,
    };

    #[test]
    fn banner_drops_only_when_low_cohesion_and_not_graced() {
        assert!(should_drop_banner(19.0, false, 0.0));
        assert!(!should_drop_banner(25.0, false, 0.0));
        assert!(!should_drop_banner(10.0, true, 0.0));
        assert!(!should_drop_banner(10.0, false, 2.0));
    }

    #[test]
    fn pickup_ratio_visible_only_during_active_channel() {
        let mut state = BannerState::default();
        assert_eq!(banner_pickup_progress_ratio(&state), None);

        state.is_dropped = true;
        state.pickup_unlock_remaining = 1.0;
        state.pickup_progress = 2.0;
        assert_eq!(banner_pickup_progress_ratio(&state), None);

        state.pickup_unlock_remaining = 0.0;
        assert!(banner_pickup_progress_ratio(&state).is_some());
    }

    #[test]
    fn follow_translation_offsets_banner_above_commander() {
        let translation = banner_follow_translation(Vec2::new(10.0, 25.0));
        assert!((translation.x - 10.0).abs() < 0.001);
        assert!(translation.y > 25.0);
        assert!(translation.z > 0.0);
    }
}
