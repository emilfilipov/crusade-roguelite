use bevy::app::AppExit;
use bevy::prelude::*;

use crate::banner::BannerState;
use crate::enemies::WaveRuntime;
use crate::model::{GameState, Health, RunSession, StartRunEvent, Team, Unit};
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

#[derive(Component, Clone, Copy, Debug)]
struct MainMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum MainMenuAction {
    Start,
    Exit,
}

const MENU_BUTTON_NORMAL: Color = Color::srgb(0.18, 0.16, 0.14);
const MENU_BUTTON_HOVERED: Color = Color::srgb(0.28, 0.24, 0.2);
const MENU_BUTTON_PRESSED: Color = Color::srgb(0.4, 0.32, 0.22);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSnapshot>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
            .add_systems(
                Update,
                handle_main_menu_buttons.run_if(in_state(GameState::MainMenu)),
            )
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

fn spawn_main_menu(mut commands: Commands) {
    commands
        .spawn((
            MainMenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.05, 0.04, 0.03, 0.72)),
                z_index: ZIndex::Global(100),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(320.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.14, 0.12, 0.1, 0.9)),
                    ..default()
                })
                .with_children(|panel| {
                    spawn_menu_button(panel, MainMenuAction::Start, "START");
                    spawn_menu_button(panel, MainMenuAction::Exit, "EXIT");
                });
        });
}

fn spawn_menu_button(parent: &mut ChildBuilder, action: MainMenuAction, label: &str) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(220.0),
                    height: Val::Px(56.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(MENU_BUTTON_NORMAL),
                ..default()
            },
            action,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 28.0,
                    color: Color::srgb(0.92, 0.88, 0.8),
                    ..default()
                },
            ));
        });
}

fn despawn_main_menu(mut commands: Commands, menu_roots: Query<Entity, With<MainMenuRoot>>) {
    for entity in &menu_roots {
        commands.entity(entity).despawn_recursive();
    }
}

#[allow(clippy::type_complexity)]
fn handle_main_menu_buttons(
    mut buttons: Query<
        (&Interaction, &MainMenuAction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut start_run_events: EventWriter<StartRunEvent>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for (interaction, action, mut background) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                *background = MENU_BUTTON_PRESSED.into();
                match action {
                    MainMenuAction::Start => {
                        info!("Start run requested from MainMenu button.");
                        *run_session = RunSession::default();
                        next_state.set(GameState::InRun);
                        start_run_events.send(StartRunEvent);
                    }
                    MainMenuAction::Exit => {
                        info!("Exit requested from MainMenu button.");
                        app_exit_events.send(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => {
                *background = MENU_BUTTON_HOVERED.into();
            }
            Interaction::None => {
                *background = MENU_BUTTON_NORMAL.into();
            }
        }
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
