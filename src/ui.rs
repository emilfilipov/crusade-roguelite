use bevy::app::AppExit;
use bevy::prelude::*;

use crate::banner::{BannerState, banner_pickup_progress_ratio};
use crate::data::GameData;
use crate::drops::ExpPack;
use crate::enemies::WaveRuntime;
use crate::formation::{FormationSkillBar, SkillBarSkillKind};
use crate::map::MapBounds;
use crate::model::{
    FrameRateCap, FriendlyUnit, GameState, Health, Morale, RescuableUnit, RunSession,
    StartRunEvent, Team, Unit, UnitKind,
};
use crate::morale::{Cohesion, average_morale_ratio};
use crate::rescue::RescueProgress;
use crate::settings::AppSettings;
use crate::squad::SquadRoster;
use crate::upgrades::{
    Progression, SelectUpgradeEvent, UpgradeCardIcon, UpgradeDraft, upgrade_card_icon,
    upgrade_display_description, upgrade_display_title,
};

#[derive(Resource, Clone, Debug)]
pub struct HudSnapshot {
    pub cohesion: f32,
    pub banner_dropped: bool,
    pub squad_size: usize,
    pub level: u32,
    pub xp: f32,
    pub next_level_xp: f32,
    pub wave_index: usize,
    pub current_wave: u32,
    pub elapsed_seconds: f32,
    pub average_morale_ratio: f32,
}

impl Default for HudSnapshot {
    fn default() -> Self {
        Self {
            cohesion: 100.0,
            banner_dropped: false,
            squad_size: 1,
            level: 1,
            xp: 0.0,
            next_level_xp: 30.0,
            wave_index: 0,
            current_wave: 1,
            elapsed_seconds: 0.0,
            average_morale_ratio: 1.0,
        }
    }
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
    Settings,
    Exit,
}

#[derive(Component, Clone, Copy, Debug)]
struct SettingsMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsMenuAction {
    Back,
}

#[derive(Component, Clone, Copy, Debug)]
struct GameOverMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum GameOverMenuAction {
    Restart,
    MainMenu,
}

#[derive(Component, Clone, Copy, Debug)]
struct PauseMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum PauseMenuAction {
    Resume,
    Restart,
    MainMenuOrQuit,
}

#[derive(Component, Clone, Copy, Debug)]
struct LevelUpMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct LevelUpOptionAction {
    index: usize,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct FpsCapButton {
    cap: FrameRateCap,
}

#[derive(Component, Clone, Copy, Debug)]
struct InRunHudRoot;

#[derive(Component, Clone, Copy, Debug)]
struct WaveHudText;

#[derive(Component, Clone, Copy, Debug)]
struct TimeHudText;

#[derive(Component, Clone, Copy, Debug)]
struct CommanderLevelHudText;

#[derive(Component, Clone, Copy, Debug)]
struct XpBarFill;

#[derive(Component, Clone, Copy, Debug)]
struct RescueProgressBarsRoot;

#[derive(Component, Clone, Copy, Debug)]
struct MoraleBarFill;

#[derive(Component, Clone, Copy, Debug)]
struct CohesionBarFill;

#[derive(Component, Clone, Copy, Debug)]
struct MinimapDotsRoot;

#[derive(Component, Clone, Copy, Debug)]
struct SkillBarRoot;

#[derive(Component, Clone, Copy, Debug)]
struct SkillBarSlotNode {
    index: usize,
}

#[derive(Component, Clone, Copy, Debug)]
struct SkillBarSlotIcon {
    index: usize,
}

#[derive(Resource, Clone, Debug)]
struct MinimapRefreshRuntime {
    timer: Timer,
}

impl Default for MinimapRefreshRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.12, TimerMode::Repeating),
        }
    }
}

const MENU_BACKGROUND: Color = Color::srgb(0.12, 0.1, 0.08);
const MENU_BUTTON_TEXT_NORMAL: Color = Color::srgb(0.92, 0.88, 0.8);
const MENU_BUTTON_TEXT_HOVERED: Color = Color::srgb(0.98, 0.96, 0.88);
const MENU_BUTTON_BORDER_HOVERED: Color = Color::srgb(0.86, 0.78, 0.62);
const MENU_FPS_BOX_BORDER: Color = Color::srgba(0.86, 0.78, 0.62, 0.7);
const HUD_TEXT_COLOR: Color = Color::srgb(0.97, 0.95, 0.9);
const HUD_BAR_BG: Color = Color::srgba(0.12, 0.1, 0.08, 0.8);
const HUD_BAR_FILL: Color = Color::srgb(0.88, 0.72, 0.28);
const HUD_VERTICAL_BAR_BG: Color = Color::srgba(0.08, 0.07, 0.06, 0.85);
const MINIMAP_SIZE: f32 = 170.0;
const MINIMAP_BORDER: Color = Color::srgb(0.84, 0.76, 0.62);
const MINIMAP_BG: Color = Color::srgba(0.08, 0.07, 0.06, 0.75);
const MINIMAP_COMMANDER_COLOR: Color = Color::srgb(1.0, 0.96, 0.78);
const MINIMAP_FRIENDLY_COLOR: Color = Color::srgb(0.38, 0.79, 0.36);
const MINIMAP_ENEMY_COLOR: Color = Color::srgb(0.9, 0.28, 0.22);
const MINIMAP_RESCUABLE_COLOR: Color = Color::srgb(0.45, 0.72, 0.94);
const MINIMAP_DROPPED_BANNER_COLOR: Color = Color::srgb(0.95, 0.8, 0.32);
const MINIMAP_EXP_COLOR: Color = Color::srgb(0.98, 0.87, 0.22);
const MINIMAP_MAX_ENEMY_BLIPS: usize = 220;
const MINIMAP_MAX_FRIENDLY_BLIPS: usize = 260;
const MINIMAP_MAX_RESCUABLE_BLIPS: usize = 80;
const MINIMAP_MAX_EXP_BLIPS: usize = 320;
const SKILL_BAR_SLOT_BG: Color = Color::srgba(0.05, 0.045, 0.04, 0.82);
const SKILL_BAR_SLOT_BORDER: Color = Color::srgba(0.78, 0.72, 0.58, 0.4);
const SKILL_BAR_SLOT_ACTIVE_BORDER: Color = Color::srgb(0.94, 0.82, 0.43);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSnapshot>()
            .init_resource::<MinimapRefreshRuntime>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
            .add_systems(OnEnter(GameState::Settings), spawn_settings_menu)
            .add_systems(OnExit(GameState::Settings), despawn_settings_menu)
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over_menu)
            .add_systems(OnExit(GameState::GameOver), despawn_game_over_menu)
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), despawn_pause_menu)
            .add_systems(OnEnter(GameState::LevelUp), spawn_level_up_menu)
            .add_systems(OnExit(GameState::LevelUp), despawn_level_up_menu)
            .add_systems(OnEnter(GameState::MainMenu), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::Settings), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::GameOver), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::InRun), spawn_in_run_hud)
            .add_systems(
                Update,
                handle_main_menu_buttons.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(
                Update,
                (
                    handle_settings_menu_buttons,
                    handle_fps_cap_buttons,
                    refresh_fps_cap_button_visuals,
                )
                    .chain()
                    .run_if(in_state(GameState::Settings)),
            )
            .add_systems(
                Update,
                handle_game_over_buttons.run_if(in_state(GameState::GameOver)),
            )
            .add_systems(
                Update,
                handle_pause_menu_buttons.run_if(in_state(GameState::Paused)),
            )
            .add_systems(
                Update,
                handle_level_up_buttons.run_if(in_state(GameState::LevelUp)),
            )
            .add_systems(
                Update,
                refresh_hud_snapshot.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (
                    update_in_run_hud,
                    update_rescue_progress_hud,
                    update_skill_bar_hud,
                )
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                update_minimap_hud.run_if(in_state(GameState::InRun)),
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
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(18.0),
                    ..default()
                },
                background_color: BackgroundColor(MENU_BACKGROUND),
                z_index: ZIndex::Global(100),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(18.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|menu_buttons| {
                    spawn_menu_button(menu_buttons, MainMenuAction::Start, "START");
                    spawn_menu_button(menu_buttons, MainMenuAction::Settings, "SETTINGS");
                    spawn_menu_button(menu_buttons, MainMenuAction::Exit, "EXIT");
                });
        });
}

fn spawn_fps_selector(parent: &mut ChildBuilder, selected: FrameRateCap) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(8.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            border_color: BorderColor(MENU_FPS_BOX_BORDER),
            ..default()
        })
        .with_children(|row| {
            row.spawn(TextBundle::from_section(
                "FPS",
                TextStyle {
                    font_size: 18.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));
            for cap in FrameRateCap::all() {
                spawn_fps_button(row, cap, selected == cap);
            }
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
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::NONE),
                ..default()
            },
            action,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 28.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));
        });
}

fn spawn_fps_button(parent: &mut ChildBuilder, cap: FrameRateCap, selected: bool) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(56.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(if selected {
                    MENU_BUTTON_BORDER_HOVERED
                } else {
                    Color::NONE
                }),
                ..default()
            },
            FpsCapButton { cap },
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                frame_cap_label(cap),
                TextStyle {
                    font_size: 18.0,
                    color: if selected {
                        MENU_BUTTON_TEXT_HOVERED
                    } else {
                        MENU_BUTTON_TEXT_NORMAL
                    },
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

fn spawn_settings_menu(mut commands: Commands, frame_cap: Res<FrameRateCap>) {
    commands
        .spawn((
            SettingsMenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(22.0),
                    ..default()
                },
                background_color: BackgroundColor(MENU_BACKGROUND),
                z_index: ZIndex::Global(100),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "SETTINGS",
                TextStyle {
                    font_size: 42.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|settings| {
                    settings.spawn(TextBundle::from_section(
                        "Frame Rate Cap",
                        TextStyle {
                            font_size: 22.0,
                            color: MENU_BUTTON_TEXT_NORMAL,
                            ..default()
                        },
                    ));
                    spawn_fps_selector(settings, *frame_cap);
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|actions| {
                    spawn_settings_button(actions, SettingsMenuAction::Back, "BACK");
                });
        });
}

fn spawn_settings_button(parent: &mut ChildBuilder, action: SettingsMenuAction, label: &str) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(220.0),
                    height: Val::Px(56.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::NONE),
                ..default()
            },
            action,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 28.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));
        });
}

fn despawn_settings_menu(mut commands: Commands, roots: Query<Entity, With<SettingsMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_game_over_menu(mut commands: Commands) {
    commands
        .spawn((
            GameOverMenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.03, 0.03, 0.03, 0.55)),
                z_index: ZIndex::Global(110),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "DEFEAT",
                TextStyle {
                    font_size: 44.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|buttons| {
                    spawn_game_over_button(buttons, GameOverMenuAction::Restart, "RESTART");
                    spawn_game_over_button(buttons, GameOverMenuAction::MainMenu, "MAIN MENU");
                });
        });
}

fn spawn_game_over_button(parent: &mut ChildBuilder, action: GameOverMenuAction, label: &str) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(240.0),
                    height: Val::Px(56.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::NONE),
                ..default()
            },
            action,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 28.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));
        });
}

fn despawn_game_over_menu(mut commands: Commands, roots: Query<Entity, With<GameOverMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_pause_menu(mut commands: Commands) {
    commands
        .spawn((
            PauseMenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.03, 0.03, 0.03, 0.58)),
                z_index: ZIndex::Global(115),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "PAUSED",
                TextStyle {
                    font_size: 42.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|buttons| {
                    spawn_pause_menu_button(buttons, PauseMenuAction::Resume, "RESUME");
                    spawn_pause_menu_button(buttons, PauseMenuAction::Restart, "RESTART");
                    spawn_pause_menu_button(buttons, PauseMenuAction::MainMenuOrQuit, "MAIN MENU");
                });
        });
}

fn spawn_pause_menu_button(parent: &mut ChildBuilder, action: PauseMenuAction, label: &str) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(260.0),
                    height: Val::Px(56.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::NONE),
                ..default()
            },
            action,
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 28.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));
        });
}

fn despawn_pause_menu(mut commands: Commands, roots: Query<Entity, With<PauseMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_level_up_menu(
    mut commands: Commands,
    draft: Res<UpgradeDraft>,
    art: Res<crate::visuals::ArtAssets>,
) {
    commands
        .spawn((
            LevelUpMenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(18.0),
                    padding: UiRect::all(Val::Px(16.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.03, 0.03, 0.03, 0.64)),
                z_index: ZIndex::Global(120),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "LEVEL UP - CHOOSE ONE",
                TextStyle {
                    font_size: 40.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            parent.spawn(TextBundle::from_section(
                "Selection is required to continue.",
                TextStyle {
                    font_size: 18.0,
                    color: HUD_TEXT_COLOR,
                    ..default()
                },
            ));

            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Stretch,
                        column_gap: Val::Px(14.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|cards| {
                    for (index, upgrade) in draft.options.iter().take(3).enumerate() {
                        spawn_level_up_card(
                            cards,
                            index,
                            upgrade_display_title(upgrade),
                            &upgrade_display_description(upgrade),
                            upgrade_icon_for(upgrade_card_icon(upgrade), &art),
                        );
                    }
                });
        });
}

fn spawn_level_up_card(
    parent: &mut ChildBuilder,
    index: usize,
    title: &str,
    description: &str,
    icon: Handle<Image>,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(185.0),
                    height: Val::Px(320.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.74)),
                border_color: BorderColor(Color::srgba(0.82, 0.76, 0.64, 0.34)),
                ..default()
            },
            LevelUpOptionAction { index },
        ))
        .with_children(|card| {
            card.spawn(TextBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    ..default()
                },
                text: Text::from_section(
                    title,
                    TextStyle {
                        font_size: 22.0,
                        color: MENU_BUTTON_TEXT_HOVERED,
                        ..default()
                    },
                )
                .with_justify(JustifyText::Center),
                ..default()
            });
            card.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(96.0),
                    height: Val::Px(96.0),
                    ..default()
                },
                image: UiImage::new(icon),
                background_color: BackgroundColor(Color::NONE),
                ..default()
            });
            card.spawn(TextBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    ..default()
                },
                text: Text::from_section(
                    description,
                    TextStyle {
                        font_size: 15.0,
                        color: HUD_TEXT_COLOR,
                        ..default()
                    },
                )
                .with_justify(JustifyText::Center),
                ..default()
            });
        });
}

fn upgrade_icon_for(icon_kind: UpgradeCardIcon, art: &crate::visuals::ArtAssets) -> Handle<Image> {
    match icon_kind {
        UpgradeCardIcon::Damage => art.upgrade_damage_icon.clone(),
        UpgradeCardIcon::AttackSpeed => art.upgrade_attack_speed_icon.clone(),
        UpgradeCardIcon::Armor => art.upgrade_armor_icon.clone(),
        UpgradeCardIcon::PickupRadius => art.upgrade_pickup_radius_icon.clone(),
        UpgradeCardIcon::AuraRadius => art.upgrade_aura_radius_icon.clone(),
        UpgradeCardIcon::AuthorityAura => art.upgrade_authority_icon.clone(),
        UpgradeCardIcon::MoveSpeed => art.upgrade_move_speed_icon.clone(),
        UpgradeCardIcon::HospitalierAura => art.upgrade_hospitalier_icon.clone(),
        UpgradeCardIcon::FormationSquare => art.formation_square_icon.clone(),
        UpgradeCardIcon::FormationDiamond => art.formation_diamond_icon.clone(),
    }
}

fn despawn_level_up_menu(mut commands: Commands, roots: Query<Entity, With<LevelUpMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_in_run_hud(
    mut commands: Commands,
    existing: Query<Entity, With<InRunHudRoot>>,
    art: Res<crate::visuals::ArtAssets>,
) {
    if !existing.is_empty() {
        return;
    }

    commands
        .spawn((
            InRunHudRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                z_index: ZIndex::Global(90),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(12.0),
                    left: Val::Px(12.0),
                    right: Val::Px(12.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            })
            .with_children(|top_row| {
                top_row.spawn((
                    WaveHudText,
                    TextBundle::from_section(
                        "Wave 1",
                        TextStyle {
                            font_size: 26.0,
                            color: HUD_TEXT_COLOR,
                            ..default()
                        },
                    ),
                ));

                top_row
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(340.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(6.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    })
                    .with_children(|center| {
                        center.spawn((
                            CommanderLevelHudText,
                            TextBundle::from_section(
                                "Commander Lv 1",
                                TextStyle {
                                    font_size: 24.0,
                                    color: HUD_TEXT_COLOR,
                                    ..default()
                                },
                            ),
                        ));

                        center
                            .spawn(NodeBundle {
                                style: Style {
                                    width: Val::Px(320.0),
                                    height: Val::Px(14.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: BackgroundColor(HUD_BAR_BG),
                                border_color: BorderColor(HUD_TEXT_COLOR),
                                ..default()
                            })
                            .with_children(|bar| {
                                bar.spawn((
                                    XpBarFill,
                                    NodeBundle {
                                        style: Style {
                                            width: Val::Percent(0.0),
                                            height: Val::Percent(100.0),
                                            ..default()
                                        },
                                        background_color: BackgroundColor(HUD_BAR_FILL),
                                        ..default()
                                    },
                                ));
                            });

                        center.spawn((
                            RescueProgressBarsRoot,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(320.0),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Stretch,
                                    row_gap: Val::Px(4.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::NONE),
                                ..default()
                            },
                        ));
                    });

                top_row.spawn((
                    TimeHudText,
                    TextBundle::from_section(
                        "00:00",
                        TextStyle {
                            font_size: 26.0,
                            color: HUD_TEXT_COLOR,
                            ..default()
                        },
                    ),
                ));
            });

            root.spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(12.0),
                    bottom: Val::Px(12.0),
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            })
            .with_children(|bottom_left| {
                spawn_vertical_meter(
                    bottom_left,
                    "MORALE",
                    MoraleBarFill,
                    Color::srgb(0.83, 0.63, 0.27),
                );
                spawn_vertical_meter(
                    bottom_left,
                    "COHESION",
                    CohesionBarFill,
                    Color::srgb(0.38, 0.69, 0.9),
                );
            });

            spawn_minimap(root);
            spawn_skill_bar(root, &art);
        });
}

fn despawn_in_run_hud(mut commands: Commands, roots: Query<Entity, With<InRunHudRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

fn spawn_vertical_meter<T: Component + Clone>(
    parent: &mut ChildBuilder,
    label: &str,
    fill_component: T,
    fill_color: Color,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(4.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|meter| {
            meter.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 13.0,
                    color: HUD_TEXT_COLOR,
                    ..default()
                },
            ));
            meter
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(18.0),
                        height: Val::Px(108.0),
                        border: UiRect::all(Val::Px(1.0)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexEnd,
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    background_color: BackgroundColor(HUD_VERTICAL_BAR_BG),
                    border_color: BorderColor(HUD_TEXT_COLOR),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn((
                        fill_component,
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(fill_color),
                            ..default()
                        },
                    ));
                });
        });
}

fn spawn_minimap(parent: &mut ChildBuilder) {
    parent
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                bottom: Val::Px(12.0),
                width: Val::Px(MINIMAP_SIZE),
                height: Val::Px(MINIMAP_SIZE),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(MINIMAP_BG),
            border_color: BorderColor(MINIMAP_BORDER),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                MinimapDotsRoot,
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(MINIMAP_SIZE),
                        height: Val::Px(MINIMAP_SIZE),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                },
            ));
        });
}

fn spawn_skill_bar(parent: &mut ChildBuilder, art: &crate::visuals::ArtAssets) {
    parent
        .spawn((
            SkillBarRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    bottom: Val::Px(12.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.05, 0.045, 0.04, 0.28)),
                ..default()
            },
        ))
        .with_children(|bar| {
            for index in 0..10 {
                bar.spawn((
                    SkillBarSlotNode { index },
                    NodeBundle {
                        style: Style {
                            width: Val::Px(44.0),
                            height: Val::Px(44.0),
                            border: UiRect::all(Val::Px(1.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(SKILL_BAR_SLOT_BG),
                        border_color: BorderColor(SKILL_BAR_SLOT_BORDER),
                        ..default()
                    },
                ))
                .with_children(|slot| {
                    slot.spawn((
                        SkillBarSlotIcon { index },
                        ImageBundle {
                            style: Style {
                                width: Val::Px(28.0),
                                height: Val::Px(28.0),
                                ..default()
                            },
                            image: UiImage::new(art.formation_square_icon.clone()),
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                    ));
                    slot.spawn(TextBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            top: Val::Px(2.0),
                            left: Val::Px(3.0),
                            ..default()
                        },
                        text: Text::from_section(
                            skillbar_hotkey_label(index),
                            TextStyle {
                                font_size: 11.0,
                                color: HUD_TEXT_COLOR,
                                ..default()
                            },
                        ),
                        ..default()
                    });
                });
            }
        });
}

fn skillbar_hotkey_label(index: usize) -> &'static str {
    match index {
        0 => "1",
        1 => "2",
        2 => "3",
        3 => "4",
        4 => "5",
        5 => "6",
        6 => "7",
        7 => "8",
        8 => "9",
        9 => "0",
        _ => "?",
    }
}

fn skillbar_icon_handle(kind: SkillBarSkillKind, art: &crate::visuals::ArtAssets) -> Handle<Image> {
    match kind {
        SkillBarSkillKind::Formation(crate::formation::ActiveFormation::Square) => {
            art.formation_square_icon.clone()
        }
        SkillBarSkillKind::Formation(crate::formation::ActiveFormation::Diamond) => {
            art.formation_diamond_icon.clone()
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_main_menu_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &MainMenuAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut start_run_events: EventWriter<StartRunEvent>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for (interaction, action, children, mut border_color, mut background) in &mut buttons {
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = match *interaction {
                Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                Interaction::None => MENU_BUTTON_TEXT_NORMAL,
            };
        }
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
                match action {
                    MainMenuAction::Start => {
                        info!("Start run requested from MainMenu button.");
                        *run_session = RunSession::default();
                        next_state.set(GameState::InRun);
                        start_run_events.send(StartRunEvent);
                    }
                    MainMenuAction::Settings => {
                        info!("Opening Settings screen from MainMenu.");
                        next_state.set(GameState::Settings);
                    }
                    MainMenuAction::Exit => {
                        info!("Exit requested from MainMenu button.");
                        app_exit_events.send(AppExit::Success);
                    }
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(Color::NONE);
                *background = BackgroundColor(Color::NONE);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_settings_menu_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &SettingsMenuAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, action, children, mut border_color, mut background) in &mut buttons {
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = match *interaction {
                Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                Interaction::None => MENU_BUTTON_TEXT_NORMAL,
            };
        }
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
                match action {
                    SettingsMenuAction::Back => {
                        info!("Returning from Settings to MainMenu.");
                        next_state.set(GameState::MainMenu);
                    }
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(Color::NONE);
                *background = BackgroundColor(Color::NONE);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_game_over_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &GameOverMenuAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut start_run_events: EventWriter<StartRunEvent>,
) {
    for (interaction, action, children, mut border_color, mut background) in &mut buttons {
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = match *interaction {
                Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                Interaction::None => MENU_BUTTON_TEXT_NORMAL,
            };
        }

        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
                match action {
                    GameOverMenuAction::Restart => {
                        info!("Restart requested from GameOver.");
                        *run_session = RunSession::default();
                        start_run_events.send(StartRunEvent);
                        next_state.set(GameState::InRun);
                    }
                    GameOverMenuAction::MainMenu => {
                        info!("Returning to MainMenu from GameOver.");
                        next_state.set(GameState::MainMenu);
                    }
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(Color::NONE);
                *background = BackgroundColor(Color::NONE);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_pause_menu_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &PauseMenuAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut start_run_events: EventWriter<StartRunEvent>,
) {
    for (interaction, action, children, mut border_color, mut background) in &mut buttons {
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = match *interaction {
                Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                Interaction::None => MENU_BUTTON_TEXT_NORMAL,
            };
        }
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
                match action {
                    PauseMenuAction::Resume => {
                        info!("Resuming run from pause menu.");
                        next_state.set(GameState::InRun);
                    }
                    PauseMenuAction::Restart => {
                        info!("Restart requested from pause menu.");
                        *run_session = RunSession::default();
                        start_run_events.send(StartRunEvent);
                        next_state.set(GameState::InRun);
                    }
                    PauseMenuAction::MainMenuOrQuit => {
                        info!("Returning to MainMenu from pause menu.");
                        next_state.set(GameState::MainMenu);
                    }
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(Color::NONE);
                *background = BackgroundColor(Color::NONE);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_level_up_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &LevelUpOptionAction,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut select_events: EventWriter<SelectUpgradeEvent>,
) {
    for (interaction, option, mut border_color, mut background) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::srgba(0.14, 0.12, 0.09, 0.82));
                select_events.send(SelectUpgradeEvent {
                    option_index: option.index,
                });
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background = BackgroundColor(Color::srgba(0.11, 0.09, 0.08, 0.78));
            }
            Interaction::None => {
                *border_color = BorderColor(Color::srgba(0.82, 0.76, 0.64, 0.34));
                *background = BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.74));
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_fps_cap_buttons(
    mut buttons: Query<(&Interaction, &FpsCapButton), (Changed<Interaction>, With<Button>)>,
    mut frame_cap: ResMut<FrameRateCap>,
    mut settings: ResMut<AppSettings>,
) {
    for (interaction, fps_button) in &mut buttons {
        if *interaction == Interaction::Pressed && *frame_cap != fps_button.cap {
            *frame_cap = fps_button.cap;
            settings.frame_rate_cap = fps_button.cap;
            info!("Set frame rate cap to {} FPS.", fps_button.cap.as_u32());
        }
    }
}

#[allow(clippy::type_complexity)]
fn refresh_fps_cap_button_visuals(
    frame_cap: Res<FrameRateCap>,
    mut buttons: Query<
        (
            &Interaction,
            &FpsCapButton,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (With<Button>, With<FpsCapButton>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, fps_button, children, mut border_color, mut background_color) in &mut buttons
    {
        let is_selected = *frame_cap == fps_button.cap;
        let is_hovered = matches!(*interaction, Interaction::Hovered | Interaction::Pressed);
        *border_color = BorderColor(if is_selected || is_hovered {
            MENU_BUTTON_BORDER_HOVERED
        } else {
            Color::NONE
        });
        *background_color = BackgroundColor(Color::NONE);

        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if is_selected || is_hovered {
                MENU_BUTTON_TEXT_HOVERED
            } else {
                MENU_BUTTON_TEXT_NORMAL
            };
        }
    }
}

pub fn frame_cap_label(cap: FrameRateCap) -> &'static str {
    match cap {
        FrameRateCap::Fps60 => "60",
        FrameRateCap::Fps90 => "90",
        FrameRateCap::Fps120 => "120",
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_hud_snapshot(
    cohesion: Res<Cohesion>,
    banner_state: Res<BannerState>,
    roster: Res<SquadRoster>,
    progression: Res<Progression>,
    waves: Res<WaveRuntime>,
    run_session: Res<RunSession>,
    friendlies: Query<&Morale, With<FriendlyUnit>>,
    mut hud: ResMut<HudSnapshot>,
) {
    let morale_ratios: Vec<f32> = friendlies.iter().map(|morale| morale.ratio()).collect();
    *hud = HudSnapshot {
        cohesion: cohesion.value,
        banner_dropped: banner_state.is_dropped,
        squad_size: roster.friendly_count,
        level: progression.level,
        xp: progression.xp,
        next_level_xp: progression.next_level_xp,
        wave_index: waves.next_wave_index,
        current_wave: displayed_wave_number(&waves),
        elapsed_seconds: run_session.survived_seconds,
        average_morale_ratio: average_morale_ratio(&morale_ratios),
    };
}

#[allow(clippy::type_complexity)]
fn update_in_run_hud(
    hud: Res<HudSnapshot>,
    mut texts: ParamSet<(
        Query<&mut Text, With<WaveHudText>>,
        Query<&mut Text, With<TimeHudText>>,
        Query<&mut Text, With<CommanderLevelHudText>>,
    )>,
    mut bar_styles: ParamSet<(
        Query<&mut Style, With<XpBarFill>>,
        Query<&mut Style, With<MoraleBarFill>>,
        Query<&mut Style, With<CohesionBarFill>>,
    )>,
) {
    if let Ok(mut text) = texts.p0().get_single_mut() {
        text.sections[0].value = format!("Wave {}", hud.current_wave);
    }
    if let Ok(mut text) = texts.p1().get_single_mut() {
        text.sections[0].value = format_elapsed_mm_ss(hud.elapsed_seconds);
    }
    if let Ok(mut text) = texts.p2().get_single_mut() {
        text.sections[0].value = format!("Commander Lv {}", hud.level);
    }
    let xp_ratio = if hud.next_level_xp <= 0.0 {
        0.0
    } else {
        (hud.xp / hud.next_level_xp).clamp(0.0, 1.0)
    };
    if let Ok(mut style) = bar_styles.p0().get_single_mut() {
        style.width = Val::Percent(xp_ratio * 100.0);
    }
    if let Ok(mut style) = bar_styles.p1().get_single_mut() {
        style.height = Val::Percent(hud.average_morale_ratio.clamp(0.0, 1.0) * 100.0);
    }
    if let Ok(mut style) = bar_styles.p2().get_single_mut() {
        style.height = Val::Percent((hud.cohesion / 100.0).clamp(0.0, 1.0) * 100.0);
    }
}

#[allow(clippy::type_complexity)]
fn update_skill_bar_hud(
    skillbar: Res<FormationSkillBar>,
    art: Res<crate::visuals::ArtAssets>,
    mut slot_nodes: Query<
        (&SkillBarSlotNode, &mut BorderColor, &mut BackgroundColor),
        With<SkillBarSlotNode>,
    >,
    mut slot_icons: Query<(&SkillBarSlotIcon, &mut UiImage), With<SkillBarSlotIcon>>,
) {
    for (slot, mut border_color, mut background) in &mut slot_nodes {
        let is_active = skillbar.active_slot == Some(slot.index);
        *border_color = BorderColor(if is_active {
            SKILL_BAR_SLOT_ACTIVE_BORDER
        } else {
            SKILL_BAR_SLOT_BORDER
        });
        *background = BackgroundColor(if is_active {
            Color::srgba(0.12, 0.1, 0.08, 0.9)
        } else {
            SKILL_BAR_SLOT_BG
        });
    }

    for (slot_icon, mut image) in &mut slot_icons {
        let Some(entry) = skillbar.slots.get(slot_icon.index) else {
            image.color = Color::srgba(1.0, 1.0, 1.0, 0.0);
            continue;
        };
        image.texture = skillbar_icon_handle(entry.kind, &art);
        image.color = Color::WHITE;
    }
}

fn update_rescue_progress_hud(
    mut commands: Commands,
    data: Res<GameData>,
    banner_state: Res<BannerState>,
    rescue_bars_root: Query<Entity, With<RescueProgressBarsRoot>>,
    rescuables: Query<&RescueProgress, With<RescuableUnit>>,
) {
    let Ok(root_entity) = rescue_bars_root.get_single() else {
        return;
    };
    commands.entity(root_entity).despawn_descendants();

    let duration = data.rescue.rescue_duration_secs;
    let mut bars: Vec<(f32, Color)> = Vec::new();
    if let Some(progress_ratio) = banner_pickup_progress_ratio(&banner_state) {
        bars.push((progress_ratio, Color::srgb(0.94, 0.68, 0.32)));
    }
    bars.extend(
        rescuables
            .iter()
            .filter_map(|progress| rescue_progress_ratio(progress.elapsed, duration))
            .map(|ratio| (ratio, Color::srgb(0.56, 0.78, 0.95))),
    );
    if bars.is_empty() {
        return;
    }
    bars.sort_by(|a, b| b.0.total_cmp(&a.0));

    commands.entity(root_entity).with_children(|parent| {
        for (ratio, color) in bars {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(320.0),
                        height: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(HUD_BAR_BG),
                    border_color: BorderColor(HUD_TEXT_COLOR),
                    ..default()
                })
                .with_children(|bar| {
                    bar.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(ratio * 100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: BackgroundColor(color),
                        ..default()
                    });
                });
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn update_minimap_hud(
    mut commands: Commands,
    time: Res<Time>,
    banner_state: Res<BannerState>,
    bounds: Option<Res<MapBounds>>,
    mut runtime: ResMut<MinimapRefreshRuntime>,
    minimap_roots: Query<Entity, With<MinimapDotsRoot>>,
    units: Query<(&Unit, &Transform)>,
    rescuables: Query<&Transform, With<RescuableUnit>>,
    exp_packs: Query<&Transform, With<ExpPack>>,
) {
    runtime.timer.tick(time.delta());
    if !runtime.timer.just_finished() {
        return;
    }

    let Some(bounds) = bounds else {
        return;
    };
    let Ok(root) = minimap_roots.get_single() else {
        return;
    };

    commands.entity(root).despawn_descendants();
    commands.entity(root).with_children(|parent| {
        let mut friendly_count = 0usize;
        let mut enemy_count = 0usize;
        let mut rescuable_count = 0usize;
        let mut exp_count = 0usize;
        for (unit, transform) in &units {
            let position = transform.translation.truncate();
            let Some(draw_pos) = world_to_minimap_pos(position, *bounds, MINIMAP_SIZE) else {
                continue;
            };

            match unit.team {
                Team::Friendly => {
                    let (color, dot_size) = if unit.kind == UnitKind::Commander {
                        (MINIMAP_COMMANDER_COLOR, 4.0)
                    } else {
                        if friendly_count >= MINIMAP_MAX_FRIENDLY_BLIPS {
                            continue;
                        }
                        friendly_count += 1;
                        (MINIMAP_FRIENDLY_COLOR, 2.5)
                    };
                    spawn_minimap_dot(parent, draw_pos, dot_size, color);
                }
                Team::Enemy => {
                    if enemy_count >= MINIMAP_MAX_ENEMY_BLIPS {
                        continue;
                    }
                    enemy_count += 1;
                    spawn_minimap_dot(parent, draw_pos, 2.3, MINIMAP_ENEMY_COLOR);
                }
                Team::Neutral => {}
            }
        }

        for transform in &rescuables {
            if rescuable_count >= MINIMAP_MAX_RESCUABLE_BLIPS {
                break;
            }
            let position = transform.translation.truncate();
            let Some(draw_pos) = world_to_minimap_pos(position, *bounds, MINIMAP_SIZE) else {
                continue;
            };
            rescuable_count += 1;
            spawn_minimap_dot(parent, draw_pos, 2.7, MINIMAP_RESCUABLE_COLOR);
        }

        for transform in &exp_packs {
            if exp_count >= MINIMAP_MAX_EXP_BLIPS {
                break;
            }
            let position = transform.translation.truncate();
            let Some(draw_pos) = world_to_minimap_pos(position, *bounds, MINIMAP_SIZE) else {
                continue;
            };
            exp_count += 1;
            spawn_minimap_dot(parent, draw_pos, 2.1, MINIMAP_EXP_COLOR);
        }

        if banner_state.is_dropped
            && let Some(draw_pos) =
                world_to_minimap_pos(banner_state.world_position, *bounds, MINIMAP_SIZE)
        {
            spawn_minimap_dot(parent, draw_pos, 3.6, MINIMAP_DROPPED_BANNER_COLOR);
        }
    });
}

fn spawn_minimap_dot(parent: &mut ChildBuilder, draw_pos: Vec2, dot_size: f32, color: Color) {
    parent.spawn(NodeBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(draw_pos.x - dot_size * 0.5),
            top: Val::Px(draw_pos.y - dot_size * 0.5),
            width: Val::Px(dot_size),
            height: Val::Px(dot_size),
            ..default()
        },
        background_color: BackgroundColor(color),
        ..default()
    });
}

pub fn world_to_minimap_pos(position: Vec2, bounds: MapBounds, minimap_size: f32) -> Option<Vec2> {
    if bounds.half_width <= 0.0 || bounds.half_height <= 0.0 || minimap_size <= 0.0 {
        return None;
    }
    let u = (position.x + bounds.half_width) / (bounds.half_width * 2.0);
    let v = (position.y + bounds.half_height) / (bounds.half_height * 2.0);
    if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
        return None;
    }

    Some(Vec2::new(
        u * minimap_size,
        (1.0 - v) * minimap_size, // UI Y axis grows downward.
    ))
}

pub fn rescue_progress_ratio(elapsed: f32, duration: f32) -> Option<f32> {
    if duration <= 0.0 || elapsed <= 0.0 || elapsed >= duration {
        return None;
    }
    Some((elapsed / duration).clamp(0.0, 1.0))
}

pub fn displayed_wave_number(runtime: &WaveRuntime) -> u32 {
    let spawned = runtime.next_wave_index as u32 + runtime.infinite_wave_index;
    spawned.max(1)
}

pub fn format_elapsed_mm_ss(seconds: f32) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    let minutes = total_seconds / 60;
    let secs = total_seconds % 60;
    format!("{minutes:02}:{secs:02}")
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
    use crate::enemies::WaveRuntime;
    use crate::map::MapBounds;
    use crate::model::FrameRateCap;
    use crate::ui::{
        HudSnapshot, displayed_wave_number, format_elapsed_mm_ss, frame_cap_label,
        health_bar_fill_width, rescue_progress_ratio, world_to_minimap_pos,
    };

    #[test]
    fn snapshot_holds_expected_values() {
        let snapshot = HudSnapshot {
            cohesion: 70.0,
            banner_dropped: true,
            squad_size: 5,
            level: 2,
            xp: 12.0,
            next_level_xp: 45.0,
            wave_index: 2,
            current_wave: 2,
            elapsed_seconds: 61.0,
            average_morale_ratio: 0.74,
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

    #[test]
    fn frame_cap_labels_match_expected_values() {
        assert_eq!(frame_cap_label(FrameRateCap::Fps60), "60");
        assert_eq!(frame_cap_label(FrameRateCap::Fps90), "90");
        assert_eq!(frame_cap_label(FrameRateCap::Fps120), "120");
    }

    #[test]
    fn elapsed_time_formats_as_minutes_seconds() {
        assert_eq!(format_elapsed_mm_ss(0.0), "00:00");
        assert_eq!(format_elapsed_mm_ss(65.3), "01:05");
        assert_eq!(format_elapsed_mm_ss(600.9), "10:00");
    }

    #[test]
    fn displayed_wave_number_never_below_one() {
        let mut runtime = WaveRuntime::default();
        assert_eq!(displayed_wave_number(&runtime), 1);
        runtime.next_wave_index = 3;
        assert_eq!(displayed_wave_number(&runtime), 3);
        runtime.infinite_wave_index = 4;
        assert_eq!(displayed_wave_number(&runtime), 7);
    }

    #[test]
    fn rescue_progress_ratio_visible_only_while_channeling() {
        assert_eq!(rescue_progress_ratio(0.0, 2.0), None);
        assert_eq!(rescue_progress_ratio(2.0, 2.0), None);
        assert_eq!(rescue_progress_ratio(2.4, 2.0), None);
        assert!(rescue_progress_ratio(0.5, 2.0).is_some());
    }

    #[test]
    fn minimap_position_maps_world_center_to_panel_center() {
        let bounds = MapBounds {
            half_width: 1200.0,
            half_height: 900.0,
        };
        let pos = world_to_minimap_pos(bevy::prelude::Vec2::ZERO, bounds, 170.0)
            .expect("center should be visible");
        assert!((pos.x - 85.0).abs() < 0.01);
        assert!((pos.y - 85.0).abs() < 0.01);
    }
}
