use bevy::prelude::*;

use crate::banner::BannerState;
use crate::enemies::WaveRuntime;
use crate::model::{GameState, Health, Team, Unit};
use crate::morale::Cohesion;
use crate::squad::SquadRoster;
use crate::upgrades::Progression;

#[derive(Resource, Clone, Debug, Default)]
pub struct HudSnapshot {
    pub cohesion: f32,
    pub banner_dropped: bool,
    pub squad_size: usize,
    pub xp: f32,
    pub wave_index: usize,
}

#[derive(Component, Clone, Copy, Debug)]
struct HasHealthBar;

#[derive(Component, Clone, Copy, Debug)]
struct HealthBarFill;

const HEALTH_BAR_WIDTH: f32 = 22.0;
const HEALTH_BAR_HEIGHT: f32 = 3.0;
const HEALTH_BAR_Y_OFFSET: f32 = 24.0;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSnapshot>()
            .add_systems(
                Update,
                refresh_hud_snapshot.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (attach_health_bars_to_units, update_health_bar_fills)
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn refresh_hud_snapshot(
    cohesion: Res<Cohesion>,
    banner_state: Res<BannerState>,
    roster: Res<SquadRoster>,
    progression: Res<Progression>,
    waves: Res<WaveRuntime>,
    mut hud: ResMut<HudSnapshot>,
) {
    *hud = HudSnapshot {
        cohesion: cohesion.value,
        banner_dropped: banner_state.is_dropped,
        squad_size: roster.friendly_count,
        xp: progression.xp,
        wave_index: waves.next_wave_index,
    };
}

#[allow(clippy::type_complexity)]
fn attach_health_bars_to_units(
    mut commands: Commands,
    units_without_bars: Query<(Entity, &Unit), (With<Health>, Without<HasHealthBar>)>,
) {
    for (entity, unit) in &units_without_bars {
        if matches!(unit.team, Team::Neutral) {
            commands.entity(entity).insert(HasHealthBar);
            continue;
        }
        commands.entity(entity).insert(HasHealthBar);
        commands.entity(entity).with_children(|parent| {
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::srgba(0.05, 0.05, 0.05, 0.8),
                    custom_size: Some(Vec2::new(HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT + 1.0)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, HEALTH_BAR_Y_OFFSET, 20.0),
                ..default()
            });
            parent.spawn((
                HealthBarFill,
                SpriteBundle {
                    sprite: Sprite {
                        color: health_bar_team_color(unit.team),
                        custom_size: Some(Vec2::new(HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, HEALTH_BAR_Y_OFFSET, 21.0),
                    ..default()
                },
            ));
        });
    }
}

fn update_health_bar_fills(
    mut fill_query: Query<(&Parent, &mut Sprite, &mut Transform), With<HealthBarFill>>,
    health_query: Query<(&Health, &Unit)>,
) {
    for (parent, mut sprite, mut transform) in &mut fill_query {
        let Ok((health, unit)) = health_query.get(parent.get()) else {
            continue;
        };
        let fill_width = health_bar_fill_width(health.current, health.max, HEALTH_BAR_WIDTH);
        sprite.custom_size = Some(Vec2::new(fill_width, HEALTH_BAR_HEIGHT));
        sprite.color = health_bar_team_color(unit.team);
        transform.translation.x = -(HEALTH_BAR_WIDTH - fill_width) * 0.5;
    }
}

fn health_bar_team_color(team: Team) -> Color {
    match team {
        Team::Friendly => Color::srgb(0.2, 0.75, 0.24),
        Team::Enemy => Color::srgb(0.85, 0.24, 0.19),
        Team::Neutral => Color::srgb(0.77, 0.77, 0.72),
    }
}

pub fn health_bar_fill_width(current: f32, max: f32, full_width: f32) -> f32 {
    if max <= 0.0 {
        return 0.0;
    }
    (current / max).clamp(0.0, 1.0) * full_width
}

#[cfg(test)]
mod tests {
    use crate::ui::{HudSnapshot, health_bar_fill_width};

    #[test]
    fn snapshot_holds_expected_values() {
        let snapshot = HudSnapshot {
            cohesion: 70.0,
            banner_dropped: true,
            squad_size: 5,
            xp: 12.0,
            wave_index: 2,
        };
        assert!(snapshot.banner_dropped);
        assert_eq!(snapshot.squad_size, 5);
    }

    #[test]
    fn health_bar_width_clamps_from_zero_to_full() {
        assert_eq!(health_bar_fill_width(0.0, 100.0, 22.0), 0.0);
        assert_eq!(health_bar_fill_width(100.0, 100.0, 22.0), 22.0);
        assert_eq!(health_bar_fill_width(150.0, 100.0, 22.0), 22.0);
        assert_eq!(health_bar_fill_width(-10.0, 100.0, 22.0), 0.0);
    }
}
