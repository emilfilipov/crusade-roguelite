use bevy::app::AppExit;
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::archive::{ArchiveCategory, ArchiveDataset, ArchiveEntry};
use crate::banner::{BannerState, banner_pickup_progress_ratio};
use crate::data::GameData;
use crate::drops::{ExpPack, MagnetPickup};
use crate::enemies::WaveRuntime;
use crate::formation::{ActiveFormation, FormationModifiers, FormationSkillBar, SkillBarSkillKind};
use crate::inventory::{EquipmentUnitType, InventoryState};
use crate::map::MapBounds;
use crate::model::{
    CommanderUnit, DamageTextEvent, EnemyUnit, FrameRateCap, FriendlyUnit, GameState, Health,
    MatchSetupSelection, Morale, PlayerFaction, RescuableUnit, RunModalAction,
    RunModalRequestEvent, RunModalScreen, RunModalState, RunSession, StartRunEvent, Team, Unit,
    UnitKind, level_cap_from_locked_budget,
};
use crate::morale::{Cohesion, average_morale_ratio, commander_aura_radius};
use crate::rescue::{RescueProgress, effective_rescue_duration};
use crate::settings::AppSettings;
use crate::squad::{
    PriestAttackSpeedBlessing, PromoteUnitsEvent, RosterEconomy, RosterEconomyFeedback,
    SquadRoster, friendly_tier_for_kind, is_upgrade_tier_unlocked, promotion_step_cost,
    unit_kind_label, unlock_wave_for_tier,
};
use crate::upgrades::{
    ConditionalUpgradeEffects, ConditionalUpgradeStatus, Progression, ProgressionLockFeedback,
    SelectUpgradeEvent, SkillBookLog, UpgradeCardIcon, UpgradeDraft, UpgradeValueTier,
    commander_level_hp_bonus, skill_book_entry_cumulative_description, upgrade_card_icon,
    upgrade_display_description, upgrade_display_title, upgrade_value_tier,
};
use crate::visuals::ArtAssets;

#[derive(Resource, Clone, Debug)]
pub struct HudSnapshot {
    pub cohesion: f32,
    pub banner_dropped: bool,
    pub squad_size: usize,
    pub retinue_count: usize,
    pub level: u32,
    pub allowed_max_level: u32,
    pub xp: f32,
    pub next_level_xp: f32,
    pub wave_index: usize,
    pub current_wave: u32,
    pub elapsed_seconds: f32,
    pub alive_enemy_count: usize,
    pub average_morale_ratio: f32,
    pub progression_lock_reason: Option<String>,
}

impl Default for HudSnapshot {
    fn default() -> Self {
        Self {
            cohesion: 100.0,
            banner_dropped: false,
            squad_size: 1,
            retinue_count: 0,
            level: 1,
            allowed_max_level: 200,
            xp: 0.0,
            next_level_xp: 30.0,
            wave_index: 0,
            current_wave: 1,
            elapsed_seconds: 0.0,
            alive_enemy_count: 0,
            average_morale_ratio: 1.0,
            progression_lock_reason: None,
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
struct FloatingDamageText;

#[derive(Component, Clone, Copy, Debug)]
struct FloatingDamageTextRuntime {
    age_secs: f32,
    lifetime_secs: f32,
    rise_speed: f32,
    base_rgb: Vec3,
}

#[derive(Component, Clone, Copy, Debug)]
struct MainMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum MainMenuAction {
    PlayOffline,
    PlayOnline,
    Settings,
    Bestiary,
    Exit,
}

#[derive(Component, Clone, Copy, Debug)]
struct DisabledMenuAction;

#[derive(Component, Clone, Copy, Debug)]
struct MatchSetupRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct MatchSetupFactionButton {
    faction: PlayerFaction,
}

#[derive(Component, Clone, Debug, Eq, PartialEq)]
struct MatchSetupMapButton {
    map_id: String,
}

#[derive(Component, Clone, Copy, Debug)]
struct MatchSetupButtonDisabled;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum MatchSetupAction {
    Start,
    Back,
}

#[derive(Component, Clone, Copy, Debug)]
struct SettingsMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum SettingsMenuAction {
    Back,
}

#[derive(Component, Clone, Copy, Debug)]
struct ArchiveMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum ArchiveMenuAction {
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
struct VictoryMenuRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum VictoryMenuAction {
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

#[derive(Component, Clone, Copy, Debug)]
struct LevelUpCardTierStyle {
    base_border: Color,
    hover_border: Color,
    base_glow: Color,
    hover_glow: Color,
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
struct EnemyCountHudText;

#[derive(Component, Clone, Copy, Debug)]
struct RetinueCountHudText;

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

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct RunModalRoot {
    screen: RunModalScreen,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
enum RunModalButtonAction {
    Close,
}

#[derive(Component, Clone, Copy, Debug)]
struct UtilityBarRoot;

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct UtilityBarButton {
    screen: RunModalScreen,
}

#[derive(Component, Clone, Copy, Debug)]
struct ScrollViewport {
    max_height: f32,
}

#[derive(Component, Clone, Copy, Debug)]
struct ScrollContent {
    offset: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
struct UnitUpgradeUiState {
    selected_source: UnitKind,
}

impl Default for UnitUpgradeUiState {
    fn default() -> Self {
        Self {
            selected_source: UnitKind::ChristianPeasantInfantry,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct UnitUpgradeSourceButton {
    kind: UnitKind,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
struct UnitUpgradePromoteButton {
    from: UnitKind,
    to: UnitKind,
    quantity: UnitUpgradeQuantity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UnitUpgradeQuantity {
    One,
    Five,
    Max,
}

#[derive(Clone, Debug)]
struct StatsPanelData {
    rows: Vec<StatsPanelRow>,
    active_buffs: Vec<String>,
}

#[derive(Clone, Debug)]
struct StatsPanelRow {
    label: &'static str,
    bonus_value: f32,
    bonus_text: String,
}

#[derive(Clone, Debug)]
struct SkillBookPanelData {
    sections: Vec<SkillBookPanelSection>,
}

#[derive(Clone, Debug)]
struct SkillBookPanelSection {
    label: &'static str,
    entries: Vec<SkillBookPanelEntry>,
}

#[derive(Clone, Debug)]
struct SkillBookPanelEntry {
    title: String,
    description: String,
    icon: UpgradeCardIcon,
    stacks: u32,
    active: Option<bool>,
}

#[derive(Clone, Debug)]
struct UnitUpgradePanelData {
    commander_level: u32,
    allowed_max_level: u32,
    locked_levels: u32,
    current_wave: u32,
    selected_source: UnitKind,
    blocked_upgrade_reason: Option<String>,
    progression_lock_reason: Option<String>,
    roster_entries: Vec<UnitUpgradeRosterEntry>,
    promotion_options: Vec<UnitPromotionOption>,
}

#[derive(Clone, Debug)]
struct UnitUpgradeRosterEntry {
    kind: UnitKind,
    tier: u8,
    count: u32,
}

#[derive(Clone, Debug)]
struct UnitPromotionOption {
    to_kind: UnitKind,
    to_tier: u8,
    tier_unlocked: bool,
    unlock_wave: Option<u32>,
    source_count: u32,
    max_affordable: u32,
}

#[derive(Resource, Clone, Debug)]
struct MinimapRefreshRuntime {
    timer: Timer,
}

impl Default for MinimapRefreshRuntime {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.2, TimerMode::Repeating),
        }
    }
}

#[derive(Clone, Debug)]
struct FloatingDamageTextSpawnData {
    translation: Vec3,
    text: String,
    base_rgb: Vec3,
    font_size: f32,
}

#[derive(SystemParam)]
struct RunModalOverlayDeps<'w, 's> {
    inventory: Res<'w, InventoryState>,
    data: Res<'w, GameData>,
    archive: Res<'w, ArchiveDataset>,
    progression: Res<'w, Progression>,
    waves: Res<'w, WaveRuntime>,
    progression_lock_feedback: Res<'w, ProgressionLockFeedback>,
    roster_economy: Res<'w, RosterEconomy>,
    roster_feedback: Res<'w, RosterEconomyFeedback>,
    unit_upgrade_state: Res<'w, UnitUpgradeUiState>,
    buffs: Res<'w, crate::model::GlobalBuffs>,
    skill_book: Res<'w, SkillBookLog>,
    conditional_status: Res<'w, ConditionalUpgradeStatus>,
    skillbar: Res<'w, FormationSkillBar>,
    art: Res<'w, crate::visuals::ArtAssets>,
    active_formation: Res<'w, ActiveFormation>,
    formation_modifiers: Res<'w, FormationModifiers>,
    setup_selection: Res<'w, MatchSetupSelection>,
    roots: Query<'w, 's, (Entity, &'static RunModalRoot)>,
}

const MENU_BACKGROUND: Color = Color::srgb(0.12, 0.1, 0.08);
const MENU_BUTTON_TEXT_NORMAL: Color = Color::srgb(0.92, 0.88, 0.8);
const MENU_BUTTON_TEXT_HOVERED: Color = Color::srgb(0.98, 0.96, 0.88);
const MENU_BUTTON_TEXT_DISABLED: Color = Color::srgba(0.7, 0.66, 0.6, 0.72);
const MENU_BUTTON_BORDER_HOVERED: Color = Color::srgb(0.86, 0.78, 0.62);
const MENU_BUTTON_BORDER_DISABLED: Color = Color::srgba(0.62, 0.57, 0.5, 0.2);
const MENU_FPS_BOX_BORDER: Color = Color::srgba(0.86, 0.78, 0.62, 0.7);
const HUD_TEXT_COLOR: Color = Color::srgb(0.97, 0.95, 0.9);
const HUD_BAR_BG: Color = Color::srgba(0.12, 0.1, 0.08, 0.8);
const HUD_BAR_FILL: Color = Color::srgb(0.88, 0.72, 0.28);
const HUD_VERTICAL_BAR_BG: Color = Color::srgba(0.08, 0.07, 0.06, 0.85);
const MINIMAP_SIZE: f32 = 204.0;
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
const MINIMAP_MAX_MAGNET_BLIPS: usize = 4;
const SKILL_BAR_SLOT_BG: Color = Color::srgba(0.05, 0.045, 0.04, 0.82);
const SKILL_BAR_SLOT_BORDER: Color = Color::srgba(0.78, 0.72, 0.58, 0.4);
const SKILL_BAR_SLOT_ACTIVE_BORDER: Color = Color::srgb(0.94, 0.82, 0.43);
const UTILITY_BAR_BG: Color = Color::srgba(0.05, 0.045, 0.04, 0.72);
const UTILITY_BAR_BORDER: Color = Color::srgba(0.78, 0.72, 0.58, 0.35);
const FLOATING_DAMAGE_TEXT_START_Y_OFFSET: f32 = 24.0;
const FLOATING_DAMAGE_TEXT_Z: f32 = 60.0;
const FLOATING_DAMAGE_TEXT_FONT_SIZE: f32 = 18.0;
const FLOATING_DAMAGE_TEXT_CRIT_FONT_SIZE: f32 = 28.0;
const FLOATING_DAMAGE_TEXT_LIFETIME_SECS: f32 = 0.72;
const FLOATING_DAMAGE_TEXT_RISE_SPEED: f32 = 44.0;
const FLOATING_DAMAGE_TEXT_MAX_ACTIVE: usize = 320;
const FLOATING_DAMAGE_TEXT_MAX_SPAWNS_PER_FRAME: usize = 56;
const UI_REFERENCE_WIDTH: f32 = 1280.0;
const UI_REFERENCE_HEIGHT: f32 = 720.0;
const UI_SCALE_MIN: f32 = 0.7;
const UI_SCALE_MAX: f32 = 3.0;
const BASE_CRIT_DAMAGE_MULTIPLIER: f32 = 1.2;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSnapshot>()
            .init_resource::<MinimapRefreshRuntime>()
            .init_resource::<UnitUpgradeUiState>()
            .add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_main_menu)
            .add_systems(OnEnter(GameState::MatchSetup), spawn_match_setup_menu)
            .add_systems(OnExit(GameState::MatchSetup), despawn_match_setup_menu)
            .add_systems(OnEnter(GameState::Settings), spawn_settings_menu)
            .add_systems(OnExit(GameState::Settings), despawn_settings_menu)
            .add_systems(OnEnter(GameState::Archive), spawn_archive_menu)
            .add_systems(OnExit(GameState::Archive), despawn_archive_menu)
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over_menu)
            .add_systems(OnExit(GameState::GameOver), despawn_game_over_menu)
            .add_systems(OnEnter(GameState::Victory), spawn_victory_menu)
            .add_systems(OnExit(GameState::Victory), despawn_victory_menu)
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), despawn_pause_menu)
            .add_systems(OnEnter(GameState::LevelUp), spawn_level_up_menu)
            .add_systems(OnExit(GameState::LevelUp), despawn_level_up_menu)
            .add_systems(OnEnter(GameState::MainMenu), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::MatchSetup), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::Settings), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::GameOver), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::Victory), despawn_in_run_hud)
            .add_systems(OnEnter(GameState::InRun), spawn_in_run_hud)
            .add_systems(
                OnExit(GameState::InRun),
                (despawn_run_modal_overlay, despawn_floating_damage_text),
            )
            .add_systems(
                Update,
                handle_main_menu_buttons.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(Update, sync_responsive_ui_scale)
            .add_systems(
                Update,
                (
                    handle_match_setup_faction_buttons,
                    handle_match_setup_map_buttons,
                    handle_match_setup_action_buttons,
                    refresh_match_setup_faction_button_visuals,
                    refresh_match_setup_map_button_visuals,
                    refresh_match_setup_action_button_visuals,
                )
                    .chain()
                    .run_if(in_state(GameState::MatchSetup)),
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
                handle_archive_menu_buttons.run_if(in_state(GameState::Archive)),
            )
            .add_systems(
                Update,
                handle_game_over_buttons.run_if(in_state(GameState::GameOver)),
            )
            .add_systems(
                Update,
                handle_victory_buttons.run_if(in_state(GameState::Victory)),
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
                handle_unit_upgrade_buttons.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (
                    sync_run_modal_overlay,
                    handle_run_modal_buttons,
                    handle_utility_bar_buttons,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
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
                draw_commander_aura_footprint
                    .run_if(in_state(GameState::InRun))
                    .run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
            )
            .add_systems(
                Update,
                (spawn_floating_damage_text, animate_floating_damage_text)
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (attach_health_bars_to_units, update_health_bar_fills)
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                handle_scroll_views.run_if(resource_exists::<Events<MouseWheel>>),
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
                    spawn_menu_button(
                        menu_buttons,
                        MainMenuAction::PlayOffline,
                        "PLAY OFFLINE",
                        false,
                    );
                    spawn_menu_button(
                        menu_buttons,
                        MainMenuAction::PlayOnline,
                        "PLAY ONLINE",
                        true,
                    );
                    spawn_menu_button(menu_buttons, MainMenuAction::Settings, "SETTINGS", false);
                    spawn_menu_button(menu_buttons, MainMenuAction::Bestiary, "BESTIARY", false);
                    spawn_menu_button(menu_buttons, MainMenuAction::Exit, "EXIT", false);
                });
            parent.spawn(TextBundle::from_section(
                "Online mode is not available in this build.",
                TextStyle {
                    font_size: 16.0,
                    color: MENU_BUTTON_TEXT_DISABLED,
                    ..default()
                },
            ));
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

fn spawn_menu_button(
    parent: &mut ChildBuilder,
    action: MainMenuAction,
    label: &str,
    disabled: bool,
) {
    let mut entity = parent.spawn((
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
            border_color: BorderColor(if disabled {
                MENU_BUTTON_BORDER_DISABLED
            } else {
                Color::NONE
            }),
            ..default()
        },
        action,
    ));
    if disabled {
        entity.insert(DisabledMenuAction);
    }
    entity.with_children(|button| {
        button.spawn(TextBundle::from_section(
            label,
            TextStyle {
                font_size: 28.0,
                color: if disabled {
                    MENU_BUTTON_TEXT_DISABLED
                } else {
                    MENU_BUTTON_TEXT_NORMAL
                },
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

fn spawn_match_setup_menu(
    mut commands: Commands,
    data: Res<GameData>,
    mut setup_selection: ResMut<MatchSetupSelection>,
) {
    if (setup_selection.map_id.is_empty() || data.map.find_map(&setup_selection.map_id).is_none())
        && let Some(first_map) = data.map.first_map()
    {
        setup_selection.map_id = first_map.id.clone();
    }
    if !can_select_match_setup_faction(setup_selection.faction) {
        setup_selection.faction = PlayerFaction::Christian;
    }

    commands
        .spawn((
            MatchSetupRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.0),
                    padding: UiRect::all(Val::Px(18.0)),
                    ..default()
                },
                background_color: BackgroundColor(MENU_BACKGROUND),
                z_index: ZIndex::Global(105),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(TextBundle::from_section(
                "MATCH SETUP",
                TextStyle {
                    font_size: 42.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            root.spawn(TextBundle::from_section(
                "Choose faction and map before starting an offline run.",
                TextStyle {
                    font_size: 16.0,
                    color: MENU_BUTTON_TEXT_NORMAL,
                    ..default()
                },
            ));

            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(760.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            })
            .with_children(|panel| {
                panel.spawn(TextBundle::from_section(
                    "Faction",
                    TextStyle {
                        font_size: 22.0,
                        color: MENU_BUTTON_TEXT_NORMAL,
                        ..default()
                    },
                ));
                panel
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    })
                    .with_children(|row| {
                        spawn_match_setup_faction_button(row, PlayerFaction::Christian);
                        spawn_match_setup_faction_button(row, PlayerFaction::Muslim);
                    });

                panel.spawn(TextBundle::from_section(
                    "Map",
                    TextStyle {
                        font_size: 22.0,
                        color: MENU_BUTTON_TEXT_NORMAL,
                        ..default()
                    },
                ));
                panel
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    })
                    .with_children(|maps| {
                        for map in &data.map.maps {
                            spawn_match_setup_map_button(
                                maps,
                                &map.id,
                                &format!("{} - {}", map.name, map.description),
                            );
                        }
                    });
            });

            root.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            })
            .with_children(|actions| {
                spawn_match_setup_action_button(actions, MatchSetupAction::Start, "START");
                spawn_match_setup_action_button(actions, MatchSetupAction::Back, "BACK");
            });
        });
}

fn spawn_match_setup_faction_button(parent: &mut ChildBuilder, faction: PlayerFaction) {
    let disabled = !can_select_match_setup_faction(faction);
    let mut entity = parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(190.0),
                height: Val::Px(52.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            border_color: BorderColor(if disabled {
                MENU_BUTTON_BORDER_DISABLED
            } else {
                Color::NONE
            }),
            ..default()
        },
        MatchSetupFactionButton { faction },
    ));
    if disabled {
        entity.insert(MatchSetupButtonDisabled);
    }
    entity.with_children(|button| {
        button.spawn(TextBundle::from_section(
            faction.label(),
            TextStyle {
                font_size: 24.0,
                color: if disabled {
                    MENU_BUTTON_TEXT_DISABLED
                } else {
                    MENU_BUTTON_TEXT_NORMAL
                },
                ..default()
            },
        ));
    });
}

fn spawn_match_setup_map_button(parent: &mut ChildBuilder, map_id: &str, label: &str) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(760.0),
                    min_height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(10.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::NONE),
                ..default()
            },
            MatchSetupMapButton {
                map_id: map_id.to_string(),
            },
        ))
        .with_children(|button| {
            button.spawn(TextBundle {
                style: Style {
                    max_width: Val::Px(720.0),
                    ..default()
                },
                text: Text::from_section(
                    label,
                    TextStyle {
                        font_size: 18.0,
                        color: MENU_BUTTON_TEXT_NORMAL,
                        ..default()
                    },
                ),
                ..default()
            });
        });
}

fn spawn_match_setup_action_button(
    parent: &mut ChildBuilder,
    action: MatchSetupAction,
    label: &str,
) {
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

fn despawn_match_setup_menu(mut commands: Commands, roots: Query<Entity, With<MatchSetupRoot>>) {
    for entity in &roots {
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

fn spawn_archive_menu(
    mut commands: Commands,
    archive: Res<ArchiveDataset>,
    art: Res<crate::visuals::ArtAssets>,
) {
    commands
        .spawn((
            ArchiveMenuRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(MENU_BACKGROUND),
                z_index: ZIndex::Global(110),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(980.0),
                        height: Val::Px(620.0),
                        border: UiRect::all(Val::Px(1.0)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        padding: UiRect::all(Val::Px(16.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.95)),
                    border_color: BorderColor(MINIMAP_BORDER),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn(TextBundle::from_section(
                        "BESTIARY",
                        TextStyle {
                            font_size: 42.0,
                            color: MENU_BUTTON_TEXT_HOVERED,
                            ..default()
                        },
                    ));
                    spawn_archive_sections(panel, &archive, &art, 460.0);
                    panel
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
                            ArchiveMenuAction::Back,
                        ))
                        .with_children(|button| {
                            button.spawn(TextBundle::from_section(
                                "BACK",
                                TextStyle {
                                    font_size: 28.0,
                                    color: MENU_BUTTON_TEXT_NORMAL,
                                    ..default()
                                },
                            ));
                        });
                });
        });
}

fn despawn_archive_menu(mut commands: Commands, roots: Query<Entity, With<ArchiveMenuRoot>>) {
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

fn spawn_victory_menu(mut commands: Commands) {
    commands
        .spawn((
            VictoryMenuRoot,
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
                "VICTORY",
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
                    spawn_victory_button(buttons, VictoryMenuAction::Restart, "RESTART");
                    spawn_victory_button(buttons, VictoryMenuAction::MainMenu, "MAIN MENU");
                });
        });
}

fn spawn_victory_button(parent: &mut ChildBuilder, action: VictoryMenuAction, label: &str) {
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

fn despawn_victory_menu(mut commands: Commands, roots: Query<Entity, With<VictoryMenuRoot>>) {
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
                "Level Up!",
                TextStyle {
                    font_size: 40.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            spawn_level_up_tier_legend(parent);

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
                            upgrade_value_tier(upgrade),
                        );
                    }
                });
        });
}

fn spawn_level_up_tier_legend(parent: &mut ChildBuilder) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                flex_wrap: FlexWrap::Wrap,
                width: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|row| {
            for (tier, label) in LEVEL_UP_TIER_LEGEND {
                row.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                })
                .with_children(|entry| {
                    let color = level_up_tier_border_color(tier);
                    entry.spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(14.0),
                            height: Val::Px(14.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.7)),
                        border_color: BorderColor(color),
                        ..default()
                    });
                    entry.spawn(TextBundle::from_section(
                        label,
                        TextStyle {
                            font_size: 13.0,
                            color,
                            ..default()
                        },
                    ));
                });
            }
        });
}

const LEVEL_UP_TIER_LEGEND: [(UpgradeValueTier, &str); 6] = [
    (UpgradeValueTier::Common, "Common"),
    (UpgradeValueTier::Uncommon, "Uncommon"),
    (UpgradeValueTier::Rare, "Rare"),
    (UpgradeValueTier::Epic, "Epic"),
    (UpgradeValueTier::Mythical, "Mythical"),
    (UpgradeValueTier::Unique, "Unique"),
];

fn spawn_level_up_card(
    parent: &mut ChildBuilder,
    index: usize,
    title: &str,
    description: &str,
    icon: Handle<Image>,
    tier: UpgradeValueTier,
) {
    let style = level_up_card_tier_style(tier);
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
                border_color: BorderColor(style.base_border),
                ..default()
            },
            Outline::new(Val::Px(3.0), Val::Px(1.0), style.base_glow),
            style,
            LevelUpOptionAction { index },
        ))
        .with_children(|card| {
            card.spawn(TextBundle::from_section(
                tier_label(tier),
                TextStyle {
                    font_size: 12.0,
                    color: level_up_tier_border_color(tier),
                    ..default()
                },
            ));
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

fn level_up_card_tier_style(tier: UpgradeValueTier) -> LevelUpCardTierStyle {
    let base = level_up_tier_border_color(tier);
    LevelUpCardTierStyle {
        base_border: base.with_alpha(0.96),
        hover_border: base.with_alpha(0.98),
        base_glow: base.with_alpha(0.42),
        hover_glow: base.with_alpha(0.62),
    }
}

fn level_up_tier_border_color(tier: UpgradeValueTier) -> Color {
    match tier {
        UpgradeValueTier::Common => Color::srgb(0.82, 0.8, 0.76),
        UpgradeValueTier::Uncommon => Color::srgb(0.34, 0.58, 0.95),
        UpgradeValueTier::Rare => Color::srgb(0.1, 0.95, 0.28),
        UpgradeValueTier::Epic => Color::srgb(0.67, 0.38, 0.91),
        UpgradeValueTier::Mythical => Color::srgb(1.0, 0.56, 0.08),
        UpgradeValueTier::Unique => Color::srgb(0.5, 0.31, 0.18),
    }
}

fn tier_label(tier: UpgradeValueTier) -> &'static str {
    match tier {
        UpgradeValueTier::Common => "COMMON",
        UpgradeValueTier::Uncommon => "UNCOMMON",
        UpgradeValueTier::Rare => "RARE",
        UpgradeValueTier::Epic => "EPIC",
        UpgradeValueTier::Mythical => "MYTHICAL",
        UpgradeValueTier::Unique => "UNIQUE",
    }
}

fn upgrade_icon_for(icon_kind: UpgradeCardIcon, art: &crate::visuals::ArtAssets) -> Handle<Image> {
    match icon_kind {
        UpgradeCardIcon::Damage => art.upgrade_damage_icon.clone(),
        UpgradeCardIcon::AttackSpeed => art.upgrade_attack_speed_icon.clone(),
        UpgradeCardIcon::FastLearner => art.upgrade_pickup_radius_icon.clone(),
        UpgradeCardIcon::CritChance => art.upgrade_crit_chance_icon.clone(),
        UpgradeCardIcon::CritDamage => art.upgrade_crit_damage_icon.clone(),
        UpgradeCardIcon::Armor => art.upgrade_armor_icon.clone(),
        UpgradeCardIcon::PickupRadius => art.upgrade_pickup_radius_icon.clone(),
        UpgradeCardIcon::AuraRadius => art.upgrade_aura_radius_icon.clone(),
        UpgradeCardIcon::AuthorityAura => art.upgrade_authority_icon.clone(),
        UpgradeCardIcon::MoveSpeed => art.upgrade_move_speed_icon.clone(),
        UpgradeCardIcon::HospitalierAura => art.upgrade_hospitalier_icon.clone(),
        UpgradeCardIcon::FormationSquare => art.formation_square_icon.clone(),
        UpgradeCardIcon::FormationDiamond => art.formation_diamond_icon.clone(),
        UpgradeCardIcon::MobFury => art.upgrade_damage_icon.clone(),
        UpgradeCardIcon::MobJustice => art.upgrade_attack_speed_icon.clone(),
        UpgradeCardIcon::MobMercy => art.upgrade_pickup_radius_icon.clone(),
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
                top_row
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(280.0),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::FlexStart,
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    })
                    .with_children(|left| {
                        left.spawn((
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
                        left.spawn((
                            TimeHudText,
                            TextBundle::from_section(
                                "00:00",
                                TextStyle {
                                    font_size: 20.0,
                                    color: HUD_TEXT_COLOR,
                                    ..default()
                                },
                            ),
                        ));
                        left.spawn((
                            EnemyCountHudText,
                            TextBundle::from_section(
                                "Enemies: 0",
                                TextStyle {
                                    font_size: 18.0,
                                    color: HUD_TEXT_COLOR,
                                    ..default()
                                },
                            ),
                        ));
                        left.spawn((
                            RetinueCountHudText,
                            TextBundle::from_section(
                                "Retinue: 0",
                                TextStyle {
                                    font_size: 18.0,
                                    color: HUD_TEXT_COLOR,
                                    ..default()
                                },
                            ),
                        ));
                    });

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
                top_row.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(280.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::NONE),
                    ..default()
                });
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

            spawn_utility_bar(root, &art);
            spawn_minimap(root);
            spawn_skill_bar(root, &art);
        });
}

fn despawn_in_run_hud(mut commands: Commands, roots: Query<Entity, With<InRunHudRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_run_modal_overlay(
    mut commands: Commands,
    modal_state: Res<RunModalState>,
    deps: RunModalOverlayDeps,
    priest_blessings: Query<&PriestAttackSpeedBlessing, With<FriendlyUnit>>,
) {
    let existing = deps.roots.get_single().ok();
    match *modal_state {
        RunModalState::None => {
            if let Some((entity, _)) = existing {
                commands.entity(entity).despawn_recursive();
            }
        }
        RunModalState::Open(screen) => {
            let should_refresh_unit_upgrade = screen == RunModalScreen::UnitUpgrade
                && (deps.roster_economy.is_changed()
                    || deps.roster_feedback.is_changed()
                    || deps.unit_upgrade_state.is_changed());
            if let Some((_, root)) = existing
                && root.screen == screen
                && !should_refresh_unit_upgrade
            {
                return;
            }
            if let Some((entity, _)) = existing {
                commands.entity(entity).despawn_recursive();
            }
            let stats = build_stats_panel_data(
                &deps.data,
                &deps.progression,
                &deps.buffs,
                *deps.active_formation,
                &deps.formation_modifiers,
                &deps.conditional_status,
                priest_blessings
                    .iter()
                    .map(|blessing| blessing.remaining_secs)
                    .fold(0.0, f32::max),
            );
            let skill_book_panel = build_skill_book_panel_data(
                &deps.skill_book,
                &deps.conditional_status,
                &deps.skillbar,
                *deps.active_formation,
                &deps.data,
            );
            let unit_upgrade_panel = build_unit_upgrade_panel_data(
                &deps.roster_economy,
                &deps.roster_feedback,
                &deps.progression,
                deps.waves.current_wave.max(1),
                &deps.progression_lock_feedback,
                &deps.unit_upgrade_state,
                deps.setup_selection.faction,
            );
            spawn_run_modal_overlay(
                &mut commands,
                screen,
                &deps.inventory,
                &stats,
                &skill_book_panel,
                &unit_upgrade_panel,
                &deps.archive,
                &deps.art,
            );
        }
    }
}

fn despawn_run_modal_overlay(mut commands: Commands, roots: Query<Entity, With<RunModalRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn_recursive();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_run_modal_overlay(
    commands: &mut Commands,
    screen: RunModalScreen,
    inventory: &InventoryState,
    stats: &StatsPanelData,
    skill_book_panel: &SkillBookPanelData,
    unit_upgrade_panel: &UnitUpgradePanelData,
    archive: &ArchiveDataset,
    art: &crate::visuals::ArtAssets,
) {
    let (title, subtitle) = run_modal_titles(screen);
    let (panel_width, panel_height) = match screen {
        RunModalScreen::Inventory => (Val::Px(920.0), Val::Px(520.0)),
        RunModalScreen::Stats => (Val::Px(980.0), Val::Px(560.0)),
        RunModalScreen::SkillBook => (Val::Px(900.0), Val::Px(640.0)),
        RunModalScreen::Archive => (Val::Px(980.0), Val::Px(620.0)),
        RunModalScreen::UnitUpgrade => (Val::Px(980.0), Val::Px(560.0)),
    };
    commands
        .spawn((
            RunModalRoot { screen },
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.02, 0.02, 0.02, 0.55)),
                z_index: ZIndex::Global(112),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: panel_width,
                    height: panel_height,
                    border: UiRect::all(Val::Px(1.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(14.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.95)),
                border_color: BorderColor(MINIMAP_BORDER),
                ..default()
            })
            .with_children(|panel| {
                panel
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            min_height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            flex_direction: FlexDirection::Row,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn(TextBundle::from_section(
                            title,
                            TextStyle {
                                font_size: 34.0,
                                color: MENU_BUTTON_TEXT_HOVERED,
                                ..default()
                            },
                        ));
                    });
                if let Some(subtitle) = subtitle {
                    panel.spawn(TextBundle {
                        style: Style {
                            max_width: Val::Px(640.0),
                            ..default()
                        },
                        text: Text::from_section(
                            subtitle,
                            TextStyle {
                                font_size: 16.0,
                                color: HUD_TEXT_COLOR,
                                ..default()
                            },
                        )
                        .with_justify(JustifyText::Center),
                        ..default()
                    });
                }
                if matches!(screen, RunModalScreen::Inventory) {
                    spawn_inventory_modal_sections(panel, inventory);
                }
                if matches!(screen, RunModalScreen::Stats) {
                    spawn_stats_modal_sections(panel, stats);
                }
                if matches!(screen, RunModalScreen::SkillBook) {
                    spawn_skill_book_modal_sections(panel, skill_book_panel, art);
                }
                if matches!(screen, RunModalScreen::Archive) {
                    spawn_archive_sections(panel, archive, art, 430.0);
                }
                if matches!(screen, RunModalScreen::UnitUpgrade) {
                    spawn_unit_upgrade_modal_sections(panel, unit_upgrade_panel);
                }
                panel
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(154.0),
                                height: Val::Px(40.0),
                                border: UiRect::all(Val::Px(1.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect {
                                    top: Val::Px(6.0),
                                    ..default()
                                },
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            border_color: BorderColor(Color::NONE),
                            ..default()
                        },
                        RunModalButtonAction::Close,
                    ))
                    .with_children(|button| {
                        button.spawn(TextBundle::from_section(
                            "CLOSE",
                            TextStyle {
                                font_size: 18.0,
                                color: MENU_BUTTON_TEXT_NORMAL,
                                ..default()
                            },
                        ));
                    });
            });
        });
}

fn run_modal_titles(screen: RunModalScreen) -> (&'static str, Option<&'static str>) {
    match screen {
        RunModalScreen::Inventory => ("INVENTORY", None),
        RunModalScreen::Stats => (
            "STATS",
            Some(
                "Per level: +1 Health, +1% Damage, +1% Attack Speed to commander and all friendlies.",
            ),
        ),
        RunModalScreen::SkillBook => ("SKILL BOOK", Some("Chosen skills, formations, and auras.")),
        RunModalScreen::Archive => ("BESTIARY", None),
        RunModalScreen::UnitUpgrade => (
            "UNIT UPGRADES",
            Some("Manage roster promotions and level-cost budget usage."),
        ),
    }
}

fn stats_bonus_row(label: &'static str, bonus_value: f32, bonus_text: String) -> StatsPanelRow {
    StatsPanelRow {
        label,
        bonus_value,
        bonus_text,
    }
}

fn format_percent_bonus(percent: f32) -> String {
    if percent.abs() <= 0.0001 {
        "0%".to_string()
    } else {
        format!("{:+.1}%", percent)
    }
}

fn format_raw_bonus(raw: f32) -> String {
    if raw.abs() <= 0.0001 {
        "0".to_string()
    } else {
        format_signed_stat_value(raw)
    }
}

fn format_raw_and_percent_bonus(raw: f32, percent: f32) -> String {
    let mut parts = Vec::with_capacity(2);
    if raw.abs() > 0.0001 {
        parts.push(format_raw_bonus(raw));
    }
    if percent.abs() > 0.0001 {
        parts.push(format_percent_bonus(percent));
    }
    if parts.is_empty() {
        "0".to_string()
    } else {
        parts.join(" ")
    }
}

fn build_stats_panel_data(
    _data: &GameData,
    progression: &Progression,
    buffs: &crate::model::GlobalBuffs,
    active_formation: ActiveFormation,
    formation_modifiers: &FormationModifiers,
    conditional_status: &ConditionalUpgradeStatus,
    max_priest_blessing_secs: f32,
) -> StatsPanelData {
    let level = progression.level.max(1);
    let hp_bonus = commander_level_hp_bonus(level);
    let passive_level_percent = level.saturating_sub(1) as f32;
    let damage_upgrade_percent = (buffs.damage_multiplier - 1.0) * 100.0;
    let attack_speed_upgrade_percent = (buffs.attack_speed_multiplier - 1.0) * 100.0;
    let crit_chance_percent = buffs.crit_chance_bonus * 100.0;
    let crit_damage_bonus_percent =
        (buffs.crit_damage_multiplier - BASE_CRIT_DAMAGE_MULTIPLIER) * 100.0;
    let authority_resist_percent = buffs.authority_friendly_loss_resistance * 100.0;

    let mut active_buffs = vec![format!("Formation: {}", active_formation.display_name())];
    if buffs.authority_friendly_loss_resistance > 0.0
        || buffs.authority_enemy_morale_drain_per_sec > 0.0
    {
        active_buffs.push("Authority Aura".to_string());
    }
    if buffs.hospitalier_hp_regen_per_sec > 0.0
        || buffs.hospitalier_cohesion_regen_per_sec > 0.0
        || buffs.hospitalier_morale_regen_per_sec > 0.0
    {
        active_buffs.push("Hospitalier Aura".to_string());
    }
    if buffs.inside_formation_damage_multiplier > 1.0 {
        active_buffs.push("Encirclement Doctrine".to_string());
    }
    if let Some(active) = conditional_upgrade_active_state(conditional_status, "mob_fury")
        && active
    {
        active_buffs.push("Mob's Fury".to_string());
    }
    if let Some(active) = conditional_upgrade_active_state(conditional_status, "mob_justice")
        && active
    {
        active_buffs.push("Mob's Justice".to_string());
    }
    if let Some(active) = conditional_upgrade_active_state(conditional_status, "mob_mercy")
        && active
    {
        active_buffs.push("Mob's Mercy".to_string());
    }
    if max_priest_blessing_secs > 0.0 {
        active_buffs.push(format!("Priest Blessing ({max_priest_blessing_secs:.1}s)"));
    }

    StatsPanelData {
        rows: vec![
            stats_bonus_row("Health", hp_bonus, format_raw_bonus(hp_bonus)),
            stats_bonus_row(
                "Damage",
                passive_level_percent + damage_upgrade_percent,
                format_percent_bonus(passive_level_percent + damage_upgrade_percent),
            ),
            stats_bonus_row(
                "Attack Speed",
                passive_level_percent + attack_speed_upgrade_percent,
                format_percent_bonus(passive_level_percent + attack_speed_upgrade_percent),
            ),
            stats_bonus_row(
                "XP Gain",
                (buffs.xp_gain_multiplier - 1.0) * 100.0,
                format_percent_bonus((buffs.xp_gain_multiplier - 1.0) * 100.0),
            ),
            stats_bonus_row(
                "Crit Chance",
                crit_chance_percent,
                format_percent_bonus(crit_chance_percent),
            ),
            stats_bonus_row(
                "Crit Damage",
                crit_damage_bonus_percent,
                format_percent_bonus(crit_damage_bonus_percent),
            ),
            stats_bonus_row(
                "Armor",
                buffs.armor_bonus,
                format_raw_bonus(buffs.armor_bonus),
            ),
            stats_bonus_row(
                "Move Speed",
                buffs.move_speed_bonus,
                format_raw_and_percent_bonus(
                    buffs.move_speed_bonus,
                    (formation_modifiers.move_speed_multiplier - 1.0) * 100.0,
                ),
            ),
            stats_bonus_row(
                "Pickup Radius",
                buffs.pickup_radius_bonus,
                format_raw_bonus(buffs.pickup_radius_bonus),
            ),
            stats_bonus_row(
                "Morale Loss Resist",
                authority_resist_percent,
                format_percent_bonus(authority_resist_percent),
            ),
            stats_bonus_row(
                "Cohesion Loss Resist",
                authority_resist_percent,
                format_percent_bonus(authority_resist_percent),
            ),
            stats_bonus_row(
                "Health Regen/s",
                buffs.hospitalier_hp_regen_per_sec,
                format_raw_bonus(buffs.hospitalier_hp_regen_per_sec),
            ),
            stats_bonus_row(
                "Morale Regen/s",
                buffs.hospitalier_morale_regen_per_sec,
                format_raw_bonus(buffs.hospitalier_morale_regen_per_sec),
            ),
            stats_bonus_row(
                "Cohesion Regen/s",
                buffs.hospitalier_cohesion_regen_per_sec,
                format_raw_bonus(buffs.hospitalier_cohesion_regen_per_sec),
            ),
        ],
        active_buffs,
    }
}

fn spawn_stats_modal_sections(parent: &mut ChildBuilder, stats: &StatsPanelData) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                min_height: Val::Px(360.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|layout| {
            layout
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(72.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.06, 0.06, 0.06, 0.4)),
                    border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.24)),
                    ..default()
                })
                .with_children(|table| {
                    table
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Px(30.0),
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.1, 0.09, 0.08, 0.7)),
                            border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.24)),
                            ..default()
                        })
                        .with_children(|header| {
                            spawn_stats_cell(header, "Stat", 54.0, MENU_BUTTON_TEXT_HOVERED);
                            spawn_stats_cell(header, "Bonus", 46.0, MENU_BUTTON_TEXT_HOVERED);
                        });
                    spawn_scrollable_panel(table, 272.0, |rows| {
                        for row in &stats.rows {
                            rows.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    min_height: Val::Px(28.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.08, 0.07, 0.06, 0.62,
                                )),
                                border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.16)),
                                ..default()
                            })
                            .with_children(|line| {
                                spawn_stats_cell(line, row.label, 54.0, HUD_TEXT_COLOR);
                                spawn_stats_cell(
                                    line,
                                    row.bonus_text.as_str(),
                                    46.0,
                                    stat_bonus_color(row.bonus_value),
                                );
                            });
                        }
                    });
                });
            layout
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(28.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                    border_color: BorderColor(UTILITY_BAR_BORDER),
                    ..default()
                })
                .with_children(|buffs_col| {
                    buffs_col.spawn(TextBundle::from_section(
                        "Active Buffs",
                        TextStyle {
                            font_size: 19.0,
                            color: MENU_BUTTON_TEXT_HOVERED,
                            ..default()
                        },
                    ));
                    spawn_scrollable_panel(buffs_col, 280.0, |buffs_list| {
                        if stats.active_buffs.is_empty() {
                            buffs_list.spawn(TextBundle::from_section(
                                "No active buffs.",
                                TextStyle {
                                    font_size: 14.0,
                                    color: MENU_BUTTON_TEXT_DISABLED,
                                    ..default()
                                },
                            ));
                            return;
                        }

                        for buff in &stats.active_buffs {
                            buffs_list
                                .spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Percent(100.0),
                                        border: UiRect::all(Val::Px(1.0)),
                                        padding: UiRect::all(Val::Px(6.0)),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::srgba(
                                        0.08, 0.07, 0.06, 0.62,
                                    )),
                                    border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.16)),
                                    ..default()
                                })
                                .with_children(|line| {
                                    line.spawn(TextBundle::from_section(
                                        buff,
                                        TextStyle {
                                            font_size: 13.0,
                                            color: HUD_TEXT_COLOR,
                                            ..default()
                                        },
                                    ));
                                });
                        }
                    });
                });
        });
}

fn spawn_stats_cell(parent: &mut ChildBuilder, text: &str, width_percent: f32, color: Color) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(width_percent),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|cell| {
            cell.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font_size: 13.0,
                    color,
                    ..default()
                },
            ));
        });
}

fn format_stat_value(value: f32) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.1}")
    } else if value.abs() >= 10.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.3}")
    }
}

fn format_signed_stat_value(value: f32) -> String {
    if value > 0.0 {
        format!("+{}", format_stat_value(value))
    } else if value < 0.0 {
        format!("-{}", format_stat_value(value.abs()))
    } else {
        "0".to_string()
    }
}

fn stat_bonus_color(value: f32) -> Color {
    if value > 0.001 {
        Color::srgb(0.52, 0.9, 0.5)
    } else if value < -0.001 {
        Color::srgb(0.94, 0.42, 0.36)
    } else {
        HUD_TEXT_COLOR
    }
}

#[cfg(test)]
fn find_stats_row<'a>(rows: &'a [StatsPanelRow], label: &str) -> Option<&'a StatsPanelRow> {
    rows.iter().find(|row| row.label == label)
}

fn build_skill_book_panel_data(
    skill_book: &SkillBookLog,
    conditional_status: &ConditionalUpgradeStatus,
    skillbar: &FormationSkillBar,
    active_formation: ActiveFormation,
    data: &GameData,
) -> SkillBookPanelData {
    let mut formations: Vec<SkillBookPanelEntry> = Vec::new();
    for slot in &skillbar.slots {
        let SkillBarSkillKind::Formation(formation) = slot.kind;
        let icon = match formation {
            ActiveFormation::Square => UpgradeCardIcon::FormationSquare,
            ActiveFormation::Diamond => UpgradeCardIcon::FormationDiamond,
        };
        formations.push(SkillBookPanelEntry {
            title: slot.label.clone(),
            description: formation_skill_description(formation, data),
            icon,
            stacks: 1,
            active: Some(formation == active_formation),
        });
    }

    let mut auras: Vec<SkillBookPanelEntry> = Vec::new();
    let mut combat: Vec<SkillBookPanelEntry> = Vec::new();
    let mut utility: Vec<SkillBookPanelEntry> = Vec::new();

    for entry in &skill_book.entries {
        if entry.kind == "unlock_formation" {
            continue;
        }
        let status = conditional_status
            .entries
            .iter()
            .find(|status| status.id == entry.id);
        let mut description = skill_book_entry_cumulative_description(entry);
        let mut active = None;
        if let Some(status) = status {
            active = Some(status.active);
            if !status.active
                && let Some(detail) = status.detail.as_deref()
            {
                description.push(' ');
                description.push_str(detail);
            }
        }
        let mapped = SkillBookPanelEntry {
            title: entry.title.clone(),
            description,
            icon: entry.icon,
            stacks: entry.stacks,
            active,
        };
        match skill_book_category(entry.kind.as_str()) {
            "Auras" => auras.push(mapped),
            "Utility" => utility.push(mapped),
            _ => combat.push(mapped),
        }
    }

    let mut sections = vec![
        SkillBookPanelSection {
            label: "Formations",
            entries: formations,
        },
        SkillBookPanelSection {
            label: "Auras",
            entries: auras,
        },
        SkillBookPanelSection {
            label: "Combat",
            entries: combat,
        },
        SkillBookPanelSection {
            label: "Utility",
            entries: utility,
        },
    ];
    sections.retain(|section| !section.entries.is_empty());
    SkillBookPanelData { sections }
}

fn formation_skill_description(formation: ActiveFormation, data: &GameData) -> String {
    let config = match formation {
        ActiveFormation::Square => &data.formations.square,
        ActiveFormation::Diamond => &data.formations.diamond,
    };
    format!(
        "Offense x{:.2}, Moving offense x{:.2}, Defense x{:.2}, Move x{:.2}.",
        config.offense_multiplier,
        config.offense_while_moving_multiplier,
        config.defense_multiplier,
        config.move_speed_multiplier,
    )
}

fn skill_book_category(kind: &str) -> &'static str {
    match kind {
        "authority_aura" | "hospitalier_aura" => "Auras",
        "pickup_radius" | "aura_radius" | "move_speed" | "mob_mercy" | "fast_learner" => "Utility",
        _ => "Combat",
    }
}

fn spawn_scrollable_panel(
    parent: &mut ChildBuilder,
    max_height: f32,
    content_builder: impl FnOnce(&mut ChildBuilder),
) {
    parent
        .spawn((
            ScrollViewport { max_height },
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(max_height),
                    border: UiRect::all(Val::Px(1.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::FlexStart,
                    position_type: PositionType::Relative,
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                border_color: BorderColor(UTILITY_BAR_BORDER),
                ..default()
            },
        ))
        .with_children(|viewport| {
            viewport
                .spawn((
                    ScrollContent { offset: 0.0 },
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            flex_shrink: 0.0,
                            align_self: AlignSelf::FlexStart,
                            position_type: PositionType::Relative,
                            top: Val::Px(0.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    },
                ))
                .with_children(content_builder);
        });
}

fn spawn_skill_book_modal_sections(
    parent: &mut ChildBuilder,
    skill_book: &SkillBookPanelData,
    art: &crate::visuals::ArtAssets,
) {
    spawn_scrollable_panel(parent, 420.0, |root| {
        if skill_book.sections.is_empty() {
            root.spawn(TextBundle::from_section(
                "No skills selected yet.",
                TextStyle {
                    font_size: 17.0,
                    color: HUD_TEXT_COLOR,
                    ..default()
                },
            ));
            return;
        }

        for section in &skill_book.sections {
            root.spawn(TextBundle::from_section(
                section.label,
                TextStyle {
                    font_size: 19.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            for entry in &section.entries {
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(6.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.07, 0.07, 0.07, 0.42)),
                    border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.18)),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn(ImageBundle {
                        style: Style {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            ..default()
                        },
                        image: UiImage::new(upgrade_icon_for(entry.icon, art)),
                        background_color: BackgroundColor(Color::NONE),
                        ..default()
                    });

                    let mut header = entry.title.clone();
                    if entry.stacks > 1 {
                        header.push_str(&format!(" x{}", entry.stacks));
                    }
                    if let Some(active) = entry.active {
                        header.push_str(if active { " [ACTIVE]" } else { " [INACTIVE]" });
                    }

                    row.spawn(TextBundle {
                        style: Style {
                            max_width: Val::Px(680.0),
                            ..default()
                        },
                        text: Text::from_section(
                            format!("{header} - {}", entry.description),
                            TextStyle {
                                font_size: 14.0,
                                color: HUD_TEXT_COLOR,
                                ..default()
                            },
                        ),
                        ..default()
                    });
                });
            }
        }
    });
}

fn archive_entries_for_category(
    archive: &ArchiveDataset,
    category: ArchiveCategory,
) -> Vec<&ArchiveEntry> {
    archive
        .entries
        .iter()
        .filter(|entry| entry.category == category)
        .collect()
}

fn spawn_archive_sections(
    parent: &mut ChildBuilder,
    archive: &ArchiveDataset,
    art: &crate::visuals::ArtAssets,
    max_height: f32,
) {
    spawn_scrollable_panel(parent, max_height, |root| {
        if archive.entries.is_empty() {
            root.spawn(TextBundle::from_section(
                "Bestiary data not available yet.",
                TextStyle {
                    font_size: 17.0,
                    color: HUD_TEXT_COLOR,
                    ..default()
                },
            ));
            return;
        }

        for category in ArchiveCategory::all() {
            let entries = archive_entries_for_category(archive, category);
            if entries.is_empty() {
                continue;
            }
            root.spawn(TextBundle::from_section(
                category.label(),
                TextStyle {
                    font_size: 19.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            for entry in entries {
                root.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(6.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::FlexStart,
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.07, 0.07, 0.07, 0.42)),
                    border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.18)),
                    ..default()
                })
                .with_children(|row| {
                    if let Some(icon) = entry.icon {
                        row.spawn(ImageBundle {
                            style: Style {
                                width: Val::Px(22.0),
                                height: Val::Px(22.0),
                                ..default()
                            },
                            image: UiImage::new(upgrade_icon_for(icon, art)),
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        });
                    } else {
                        row.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(22.0),
                                height: Val::Px(22.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        });
                    }
                    row.spawn(TextBundle {
                        style: Style {
                            max_width: Val::Px(720.0),
                            ..default()
                        },
                        text: Text::from_section(
                            format!("{} - {}", entry.title, entry.description),
                            TextStyle {
                                font_size: 14.0,
                                color: HUD_TEXT_COLOR,
                                ..default()
                            },
                        ),
                        ..default()
                    });
                });
            }
        }
    });
}

#[cfg(test)]
fn find_skill_section<'a>(
    panel: &'a SkillBookPanelData,
    label: &str,
) -> Option<&'a SkillBookPanelSection> {
    panel.sections.iter().find(|section| section.label == label)
}

fn spawn_inventory_modal_sections(parent: &mut ChildBuilder, inventory: &InventoryState) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::SpaceBetween,
                column_gap: Val::Px(10.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        })
        .with_children(|layout| {
            layout
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(50.0),
                        min_height: Val::Px(380.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                    border_color: BorderColor(UTILITY_BAR_BORDER),
                    ..default()
                })
                .with_children(|bag| {
                    bag.spawn(TextBundle::from_section(
                        "Backpack",
                        TextStyle {
                            font_size: 20.0,
                            color: MENU_BUTTON_TEXT_HOVERED,
                            ..default()
                        },
                    ));
                    const BACKPACK_ROWS: usize = 5;
                    const BACKPACK_COLS: usize = 6;
                    const BACKPACK_SLOTS: usize = BACKPACK_ROWS * BACKPACK_COLS;
                    bag
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                border: UiRect::all(Val::Px(1.0)),
                                padding: UiRect::all(Val::Px(8.0)),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(6.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                            border_color: BorderColor(UTILITY_BAR_BORDER),
                            ..default()
                        })
                        .with_children(|bag_grid| {
                            for row in 0..BACKPACK_ROWS {
                                bag_grid
                                    .spawn(NodeBundle {
                                        style: Style {
                                            width: Val::Percent(100.0),
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(6.0),
                                            ..default()
                                        },
                                        background_color: BackgroundColor(Color::NONE),
                                        ..default()
                                    })
                                    .with_children(|row_builder| {
                                        for col in 0..BACKPACK_COLS {
                                            let index = row * BACKPACK_COLS + col;
                                            let maybe_item = inventory.bag.get(index);
                                            row_builder
                                                .spawn(NodeBundle {
                                                    style: Style {
                                                        width: Val::Px(60.0),
                                                        height: Val::Px(60.0),
                                                        border: UiRect::all(Val::Px(1.0)),
                                                        justify_content: JustifyContent::Center,
                                                        align_items: AlignItems::Center,
                                                        padding: UiRect::all(Val::Px(3.0)),
                                                        ..default()
                                                    },
                                                    background_color: BackgroundColor(Color::srgba(
                                                        0.09, 0.08, 0.07, 0.72,
                                                    )),
                                                    border_color: BorderColor(Color::srgba(
                                                        0.78, 0.72, 0.58, 0.24,
                                                    )),
                                                    ..default()
                                                })
                                                .with_children(|slot| {
                                                    if let Some(item) = maybe_item {
                                                        slot.spawn(TextBundle::from_section(
                                                            truncate_inventory_label(&item.name, 10),
                                                            TextStyle {
                                                                font_size: 10.5,
                                                                color: MENU_BUTTON_TEXT_HOVERED,
                                                                ..default()
                                                            },
                                                        ));
                                                    } else {
                                                        slot.spawn(TextBundle::from_section(
                                                            "--",
                                                            TextStyle {
                                                                font_size: 10.0,
                                                                color: MENU_BUTTON_TEXT_DISABLED,
                                                                ..default()
                                                            },
                                                        ));
                                                    }
                                                });
                                        }
                                    });
                            }
                            if inventory.bag.len() > BACKPACK_SLOTS {
                                bag_grid.spawn(TextBundle::from_section(
                                    format!(
                                        "+{} more items in reserve (backpack UI currently shows first 25).",
                                        inventory.bag.len() - BACKPACK_SLOTS
                                    ),
                                    TextStyle {
                                        font_size: 11.0,
                                        color: MENU_BUTTON_TEXT_DISABLED,
                                        ..default()
                                    },
                                ));
                            }
                        });
                });

            layout
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(50.0),
                        min_height: Val::Px(380.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::all(Val::Px(8.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                    border_color: BorderColor(UTILITY_BAR_BORDER),
                    ..default()
                })
                .with_children(|setups| {
                    setups.spawn(TextBundle::from_section(
                        "Equipment",
                        TextStyle {
                            font_size: 20.0,
                            color: MENU_BUTTON_TEXT_HOVERED,
                            ..default()
                        },
                    ));
                    spawn_scrollable_panel(setups, 340.0, |setups_scroll| {
                        for unit_type in EquipmentUnitType::all() {
                            let Some(setup) = inventory.setup_for(unit_type) else {
                                continue;
                            };
                            setups_scroll
                                .spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Percent(100.0),
                                        min_height: Val::Px(68.0),
                                        padding: UiRect::axes(Val::Px(2.0), Val::Px(2.0)),
                                        flex_direction: FlexDirection::Row,
                                        align_items: AlignItems::Center,
                                        column_gap: Val::Px(6.0),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::NONE),
                                    ..default()
                                })
                                .with_children(|unit_setup| {
                                    unit_setup
                                        .spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Px(36.0),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                ..default()
                                            },
                                            background_color: BackgroundColor(Color::NONE),
                                            ..default()
                                        })
                                        .with_children(|tag| {
                                            tag.spawn(TextBundle::from_section(
                                                equipment_unit_short_label(setup.unit_type),
                                                TextStyle {
                                                    font_size: 14.0,
                                                    color: MENU_BUTTON_TEXT_HOVERED,
                                                    ..default()
                                                },
                                            ));
                                        });
                                    unit_setup
                                        .spawn(NodeBundle {
                                            style: Style {
                                                width: Val::Percent(92.0),
                                                flex_direction: FlexDirection::Row,
                                                column_gap: Val::Px(6.0),
                                                ..default()
                                            },
                                            background_color: BackgroundColor(Color::NONE),
                                            ..default()
                                        })
                                        .with_children(|slot_row| {
                                            for slot in &setup.slots {
                                                slot_row
                                                    .spawn(NodeBundle {
                                                        style: Style {
                                                            width: Val::Px(62.0),
                                                            height: Val::Px(62.0),
                                                            border: UiRect::all(Val::Px(1.0)),
                                                            justify_content: JustifyContent::Center,
                                                            align_items: AlignItems::Center,
                                                            padding: UiRect::all(Val::Px(2.0)),
                                                            ..default()
                                                        },
                                                        background_color: BackgroundColor(
                                                            Color::srgba(0.1, 0.085, 0.075, 0.74),
                                                        ),
                                                        border_color: BorderColor(Color::srgba(
                                                            0.78, 0.72, 0.58, 0.24,
                                                        )),
                                                        ..default()
                                                    })
                                                    .with_children(|slot_cell| {
                                                        let label = match &slot.item_id {
                                                            Some(item_id) => {
                                                                truncate_inventory_label(
                                                                    item_id, 10,
                                                                )
                                                            }
                                                            None => truncate_inventory_label(
                                                                &slot.display_name,
                                                                10,
                                                            ),
                                                        };
                                                        let color = if slot.item_id.is_some() {
                                                            MENU_BUTTON_TEXT_HOVERED
                                                        } else {
                                                            HUD_TEXT_COLOR
                                                        };
                                                        slot_cell.spawn(TextBundle::from_section(
                                                            label,
                                                            TextStyle {
                                                                font_size: 10.0,
                                                                color,
                                                                ..default()
                                                            },
                                                        ));
                                                    });
                                            }
                                        });
                                });
                        }
                    });
                });
        });
}

fn truncate_inventory_label(label: &str, max_chars: usize) -> String {
    let mut chars = label.chars();
    let clipped: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{clipped}...")
    } else {
        clipped
    }
}

fn equipment_unit_short_label(unit_type: EquipmentUnitType) -> &'static str {
    match unit_type {
        EquipmentUnitType::Commander => "C",
        EquipmentUnitType::Tier0 => "T0",
        EquipmentUnitType::Tier1 => "T1",
        EquipmentUnitType::Tier2 => "T2",
        EquipmentUnitType::Tier3 => "T3",
        EquipmentUnitType::Tier4 => "T4",
        EquipmentUnitType::Tier5 => "T5",
        EquipmentUnitType::Hero => "H",
    }
}

fn build_unit_upgrade_panel_data(
    economy: &RosterEconomy,
    feedback: &RosterEconomyFeedback,
    progression: &Progression,
    current_wave: u32,
    lock_feedback: &ProgressionLockFeedback,
    ui_state: &UnitUpgradeUiState,
    player_faction: PlayerFaction,
) -> UnitUpgradePanelData {
    let roster_kinds = roster_upgrade_kinds_for_faction(player_faction);
    let roster_entries = roster_kinds
        .iter()
        .map(|kind| UnitUpgradeRosterEntry {
            kind: *kind,
            tier: friendly_tier_for_kind(*kind).unwrap_or(0),
            count: roster_count_for_kind(economy, *kind),
        })
        .collect::<Vec<_>>();
    let selected_source =
        resolve_selected_source(ui_state.selected_source, &roster_entries, roster_kinds[0]);
    let source_count = roster_count_for_kind(economy, selected_source);
    let current_level = progression.level.max(1);
    let promotion_options = promotion_targets_for(selected_source)
        .iter()
        .filter_map(|to_kind| {
            let to_tier = friendly_tier_for_kind(*to_kind)?;
            let tier_unlocked = is_upgrade_tier_unlocked(to_tier, current_wave);
            Some(UnitPromotionOption {
                to_kind: *to_kind,
                to_tier,
                tier_unlocked,
                unlock_wave: (!tier_unlocked).then(|| unlock_wave_for_tier(to_tier).unwrap_or(1)),
                source_count,
                max_affordable: max_affordable_promotions(
                    current_wave,
                    current_level,
                    economy.locked_levels,
                    source_count,
                    selected_source,
                    *to_kind,
                ),
            })
        })
        .collect();

    UnitUpgradePanelData {
        commander_level: current_level,
        allowed_max_level: economy.allowed_max_level,
        locked_levels: economy.locked_levels,
        current_wave,
        selected_source,
        blocked_upgrade_reason: feedback.blocked_upgrade_reason.clone(),
        progression_lock_reason: lock_feedback.message.clone(),
        roster_entries,
        promotion_options,
    }
}

fn spawn_unit_upgrade_modal_sections(parent: &mut ChildBuilder, panel_data: &UnitUpgradePanelData) {
    let budget_text = format!(
        "Commander Level Budget: {}/{}  |  Locked by Roster: {}",
        panel_data.commander_level, panel_data.allowed_max_level, panel_data.locked_levels
    );
    let wave_text = format!("Current Wave: {}", panel_data.current_wave);
    let upgrade_feedback = panel_data
        .blocked_upgrade_reason
        .as_deref()
        .unwrap_or("No blocked promotion event in the current frame.");
    let progression_feedback = panel_data
        .progression_lock_reason
        .as_deref()
        .unwrap_or("Level progression is currently not budget-locked.");

    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                min_height: Val::Px(360.0),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::all(Val::Px(8.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
            border_color: BorderColor(UTILITY_BAR_BORDER),
            ..default()
        })
        .with_children(|root| {
            root.spawn(TextBundle::from_section(
                budget_text,
                TextStyle {
                    font_size: 16.0,
                    color: MENU_BUTTON_TEXT_HOVERED,
                    ..default()
                },
            ));
            root.spawn(TextBundle::from_section(
                wave_text,
                TextStyle {
                    font_size: 14.0,
                    color: MENU_BUTTON_TEXT_DISABLED,
                    ..default()
                },
            ));
            root.spawn(TextBundle::from_section(
                format!("Upgrade feedback: {upgrade_feedback}"),
                TextStyle {
                    font_size: 13.0,
                    color: HUD_TEXT_COLOR,
                    ..default()
                },
            ));
            root.spawn(TextBundle::from_section(
                format!("Progression feedback: {progression_feedback}"),
                TextStyle {
                    font_size: 13.0,
                    color: HUD_TEXT_COLOR,
                    ..default()
                },
            ));
            root.spawn(TextBundle::from_section(
                "Promotions are shown in a grid table. Upgrades consume level budget per step.",
                TextStyle {
                    font_size: 13.0,
                    color: MENU_BUTTON_TEXT_DISABLED,
                    ..default()
                },
            ));

            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(10.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                ..default()
            })
            .with_children(|layout| {
                layout
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(38.0),
                            border: UiRect::all(Val::Px(1.0)),
                            padding: UiRect::all(Val::Px(8.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(6.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                        border_color: BorderColor(UTILITY_BAR_BORDER),
                        ..default()
                    })
                    .with_children(|roster| {
                        roster.spawn(TextBundle::from_section(
                            "Roster",
                            TextStyle {
                                font_size: 18.0,
                                color: MENU_BUTTON_TEXT_HOVERED,
                                ..default()
                            },
                        ));
                        for entry in &panel_data.roster_entries {
                            let selected = entry.kind == panel_data.selected_source;
                            roster
                                .spawn((
                                    ButtonBundle {
                                        style: Style {
                                            width: Val::Percent(100.0),
                                            min_height: Val::Px(34.0),
                                            justify_content: JustifyContent::FlexStart,
                                            align_items: AlignItems::Center,
                                            border: UiRect::all(Val::Px(1.0)),
                                            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                            ..default()
                                        },
                                        background_color: BackgroundColor(Color::srgba(
                                            0.08, 0.07, 0.06, 0.62,
                                        )),
                                        border_color: BorderColor(if selected {
                                            MENU_BUTTON_BORDER_HOVERED
                                        } else {
                                            Color::srgba(0.78, 0.72, 0.58, 0.24)
                                        }),
                                        ..default()
                                    },
                                    UnitUpgradeSourceButton { kind: entry.kind },
                                ))
                                .with_children(|button| {
                                    button.spawn(TextBundle::from_section(
                                        format!(
                                            "{} (Tier {}) x{}",
                                            unit_kind_label(entry.kind),
                                            entry.tier,
                                            entry.count
                                        ),
                                        TextStyle {
                                            font_size: 14.0,
                                            color: if selected {
                                                MENU_BUTTON_TEXT_HOVERED
                                            } else {
                                                HUD_TEXT_COLOR
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                        }
                    });

                layout
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(60.0),
                            border: UiRect::all(Val::Px(1.0)),
                            padding: UiRect::all(Val::Px(8.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(8.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.04, 0.04, 0.04, 0.34)),
                        border_color: BorderColor(UTILITY_BAR_BORDER),
                        ..default()
                    })
                    .with_children(|tree| {
                        let selected_tier =
                            friendly_tier_for_kind(panel_data.selected_source).unwrap_or(0);
                        tree.spawn(TextBundle::from_section(
                            format!(
                                "Selected: {} (Tier {})",
                                unit_kind_label(panel_data.selected_source),
                                selected_tier
                            ),
                            TextStyle {
                                font_size: 18.0,
                                color: MENU_BUTTON_TEXT_HOVERED,
                                ..default()
                            },
                        ));
                        tree.spawn(TextBundle::from_section(
                            "Promotion Table: Target | Tier | Source | Max Affordable | Actions",
                            TextStyle {
                                font_size: 13.0,
                                color: MENU_BUTTON_TEXT_DISABLED,
                                ..default()
                            },
                        ));

                        if panel_data.promotion_options.is_empty() {
                            tree.spawn(TextBundle::from_section(
                                "No promotion paths available for this unit type.",
                                TextStyle {
                                    font_size: 14.0,
                                    color: HUD_TEXT_COLOR,
                                    ..default()
                                },
                            ));
                            return;
                        }

                        tree.spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                min_height: Val::Px(28.0),
                                border: UiRect::all(Val::Px(1.0)),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.1, 0.09, 0.08, 0.7)),
                            border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.24)),
                            ..default()
                        })
                        .with_children(|header| {
                            spawn_stats_cell(header, "Target", 30.0, MENU_BUTTON_TEXT_HOVERED);
                            spawn_stats_cell(header, "Tier", 12.0, MENU_BUTTON_TEXT_HOVERED);
                            spawn_stats_cell(header, "Source", 14.0, MENU_BUTTON_TEXT_HOVERED);
                            spawn_stats_cell(header, "Max", 16.0, MENU_BUTTON_TEXT_HOVERED);
                            spawn_stats_cell(header, "Actions", 28.0, MENU_BUTTON_TEXT_HOVERED);
                        });

                        for option in &panel_data.promotion_options {
                            tree.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    padding: UiRect::all(Val::Px(4.0)),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    column_gap: Val::Px(6.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgba(
                                    0.07, 0.07, 0.07, 0.46,
                                )),
                                border_color: BorderColor(Color::srgba(0.78, 0.72, 0.58, 0.24)),
                                ..default()
                            })
                            .with_children(|row| {
                                let target_label = if let Some(unlock_wave) = option.unlock_wave {
                                    format!(
                                        "{} (unlocks W{})",
                                        unit_kind_label(option.to_kind),
                                        unlock_wave
                                    )
                                } else {
                                    unit_kind_label(option.to_kind).to_string()
                                };
                                spawn_stats_cell(
                                    row,
                                    target_label.as_str(),
                                    30.0,
                                    if option.tier_unlocked {
                                        HUD_TEXT_COLOR
                                    } else {
                                        MENU_BUTTON_TEXT_DISABLED
                                    },
                                );
                                spawn_stats_cell(
                                    row,
                                    format!("{}", option.to_tier).as_str(),
                                    12.0,
                                    HUD_TEXT_COLOR,
                                );
                                spawn_stats_cell(
                                    row,
                                    format!("{}", option.source_count).as_str(),
                                    14.0,
                                    HUD_TEXT_COLOR,
                                );
                                spawn_stats_cell(
                                    row,
                                    format!("{}", option.max_affordable).as_str(),
                                    16.0,
                                    HUD_TEXT_COLOR,
                                );
                                row.spawn(NodeBundle {
                                    style: Style {
                                        width: Val::Percent(28.0),
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(6.0),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::NONE),
                                    ..default()
                                })
                                .with_children(|actions| {
                                    for quantity in [
                                        UnitUpgradeQuantity::One,
                                        UnitUpgradeQuantity::Five,
                                        UnitUpgradeQuantity::Max,
                                    ] {
                                        let enabled = option.max_affordable > 0;
                                        actions
                                            .spawn((
                                                ButtonBundle {
                                                    style: Style {
                                                        width: Val::Px(64.0),
                                                        height: Val::Px(28.0),
                                                        justify_content: JustifyContent::Center,
                                                        align_items: AlignItems::Center,
                                                        border: UiRect::all(Val::Px(1.0)),
                                                        ..default()
                                                    },
                                                    background_color: BackgroundColor(
                                                        Color::srgba(0.08, 0.07, 0.06, 0.7),
                                                    ),
                                                    border_color: BorderColor(if enabled {
                                                        Color::srgba(0.78, 0.72, 0.58, 0.24)
                                                    } else {
                                                        MENU_BUTTON_BORDER_DISABLED
                                                    }),
                                                    ..default()
                                                },
                                                UnitUpgradePromoteButton {
                                                    from: panel_data.selected_source,
                                                    to: option.to_kind,
                                                    quantity,
                                                },
                                            ))
                                            .with_children(|button| {
                                                button.spawn(TextBundle::from_section(
                                                    quantity.button_label(),
                                                    TextStyle {
                                                        font_size: 13.0,
                                                        color: if enabled {
                                                            HUD_TEXT_COLOR
                                                        } else {
                                                            MENU_BUTTON_TEXT_DISABLED
                                                        },
                                                        ..default()
                                                    },
                                                ));
                                            });
                                    }
                                });
                            });
                        }
                    });
            });
        });
}

fn roster_count_for_kind(economy: &RosterEconomy, kind: UnitKind) -> u32 {
    match kind {
        UnitKind::ChristianPeasantInfantry | UnitKind::MuslimPeasantInfantry => {
            economy.infantry_count
        }
        UnitKind::ChristianPeasantArcher | UnitKind::MuslimPeasantArcher => economy.archer_count,
        UnitKind::ChristianPeasantPriest | UnitKind::MuslimPeasantPriest => economy.priest_count,
        _ => 0,
    }
}

fn roster_upgrade_kinds_for_faction(faction: PlayerFaction) -> [UnitKind; 3] {
    match faction {
        PlayerFaction::Christian => [
            UnitKind::ChristianPeasantInfantry,
            UnitKind::ChristianPeasantArcher,
            UnitKind::ChristianPeasantPriest,
        ],
        PlayerFaction::Muslim => [
            UnitKind::MuslimPeasantInfantry,
            UnitKind::MuslimPeasantArcher,
            UnitKind::MuslimPeasantPriest,
        ],
    }
}

fn promotion_targets_for(kind: UnitKind) -> &'static [UnitKind] {
    const CHRISTIAN_INFANTRY_PROMOTIONS: [UnitKind; 2] = [
        UnitKind::ChristianPeasantArcher,
        UnitKind::ChristianPeasantPriest,
    ];
    const MUSLIM_INFANTRY_PROMOTIONS: [UnitKind; 2] =
        [UnitKind::MuslimPeasantArcher, UnitKind::MuslimPeasantPriest];
    const NO_PROMOTIONS: [UnitKind; 0] = [];
    match kind {
        UnitKind::ChristianPeasantInfantry => &CHRISTIAN_INFANTRY_PROMOTIONS,
        UnitKind::MuslimPeasantInfantry => &MUSLIM_INFANTRY_PROMOTIONS,
        _ => &NO_PROMOTIONS,
    }
}

fn resolve_selected_source(
    requested: UnitKind,
    roster_entries: &[UnitUpgradeRosterEntry],
    fallback: UnitKind,
) -> UnitKind {
    if roster_entries.iter().any(|entry| entry.kind == requested) {
        return requested;
    }
    roster_entries
        .first()
        .map(|entry| entry.kind)
        .unwrap_or(fallback)
}

fn max_affordable_promotions(
    current_wave: u32,
    current_level: u32,
    locked_levels: u32,
    source_count: u32,
    from_kind: UnitKind,
    to_kind: UnitKind,
) -> u32 {
    let Some(step_cost) = promotion_step_cost(from_kind, to_kind) else {
        return 0;
    };
    let Some(to_tier) = friendly_tier_for_kind(to_kind) else {
        return 0;
    };
    if !is_upgrade_tier_unlocked(to_tier, current_wave) {
        return 0;
    }
    if step_cost == 0 || source_count == 0 {
        return 0;
    }
    let level = current_level.max(1);
    let mut affordable = 0u32;
    for requested in 1..=source_count {
        let predicted_locked = locked_levels.saturating_add(step_cost.saturating_mul(requested));
        if level_cap_from_locked_budget(predicted_locked) < level {
            break;
        }
        affordable = requested;
    }
    affordable
}

fn requested_promotion_count(quantity: UnitUpgradeQuantity, max_affordable: u32) -> u32 {
    let raw_requested = match quantity {
        UnitUpgradeQuantity::One => 1,
        UnitUpgradeQuantity::Five => 5,
        UnitUpgradeQuantity::Max => max_affordable,
    };
    raw_requested.min(max_affordable)
}

impl UnitUpgradeQuantity {
    fn button_label(self) -> &'static str {
        match self {
            UnitUpgradeQuantity::One => "+1",
            UnitUpgradeQuantity::Five => "+5",
            UnitUpgradeQuantity::Max => "MAX",
        }
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
                top: Val::Px(12.0),
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
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
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

fn spawn_utility_bar(parent: &mut ChildBuilder, art: &crate::visuals::ArtAssets) {
    parent
        .spawn((
            UtilityBarRoot,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(12.0),
                    bottom: Val::Px(12.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(1.0)),
                    padding: UiRect::all(Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(UTILITY_BAR_BG),
                border_color: BorderColor(UTILITY_BAR_BORDER),
                ..default()
            },
        ))
        .with_children(|bar| {
            for screen in [
                RunModalScreen::Inventory,
                RunModalScreen::Stats,
                RunModalScreen::SkillBook,
                RunModalScreen::Archive,
                RunModalScreen::UnitUpgrade,
            ] {
                spawn_utility_bar_button(bar, screen, art);
            }
        });
}

fn spawn_utility_bar_button(
    parent: &mut ChildBuilder,
    screen: RunModalScreen,
    art: &crate::visuals::ArtAssets,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::NONE),
                border_color: BorderColor(Color::NONE),
                ..default()
            },
            UtilityBarButton { screen },
        ))
        .with_children(|button| {
            button.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(20.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                image: UiImage::new(utility_bar_icon(screen, art)),
                background_color: BackgroundColor(Color::NONE),
                ..default()
            });
            button.spawn(TextBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(1.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                text: Text::from_section(
                    utility_bar_hotkey_label(screen),
                    TextStyle {
                        font_size: 9.0,
                        color: Color::srgba(0.95, 0.93, 0.86, 0.82),
                        ..default()
                    },
                ),
                ..default()
            });
        });
}

fn utility_bar_icon(screen: RunModalScreen, art: &crate::visuals::ArtAssets) -> Handle<Image> {
    match screen {
        RunModalScreen::Inventory => art.upgrade_armor_icon.clone(),
        RunModalScreen::Stats => art.upgrade_damage_icon.clone(),
        RunModalScreen::SkillBook => art.upgrade_hospitalier_icon.clone(),
        RunModalScreen::Archive => art.upgrade_authority_icon.clone(),
        RunModalScreen::UnitUpgrade => art.upgrade_attack_speed_icon.clone(),
    }
}

fn utility_bar_hotkey_label(screen: RunModalScreen) -> &'static str {
    match screen {
        RunModalScreen::Inventory => "I",
        RunModalScreen::Stats => "O",
        RunModalScreen::SkillBook => "K",
        RunModalScreen::Archive => "B",
        RunModalScreen::UnitUpgrade => "U",
    }
}

pub fn modal_action_for_utility_button(screen: RunModalScreen) -> RunModalAction {
    RunModalAction::Toggle(screen)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MainMenuDispatch {
    OpenMatchSetup,
    OpenSettings,
    OpenBestiary,
    Exit,
    DisabledOnline,
}

fn main_menu_dispatch(action: MainMenuAction) -> MainMenuDispatch {
    match action {
        MainMenuAction::PlayOffline => MainMenuDispatch::OpenMatchSetup,
        MainMenuAction::PlayOnline => MainMenuDispatch::DisabledOnline,
        MainMenuAction::Settings => MainMenuDispatch::OpenSettings,
        MainMenuAction::Bestiary => MainMenuDispatch::OpenBestiary,
        MainMenuAction::Exit => MainMenuDispatch::Exit,
    }
}

#[allow(clippy::type_complexity)]
fn handle_main_menu_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &MainMenuAction,
            Option<&DisabledMenuAction>,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for (interaction, action, disabled, children, mut border_color, mut background) in &mut buttons
    {
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if disabled.is_some() {
                MENU_BUTTON_TEXT_DISABLED
            } else {
                match *interaction {
                    Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                    Interaction::None => MENU_BUTTON_TEXT_NORMAL,
                }
            };
        }
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(if disabled.is_some() {
                    MENU_BUTTON_BORDER_DISABLED
                } else {
                    MENU_BUTTON_BORDER_HOVERED
                });
                *background = BackgroundColor(Color::NONE);
                match main_menu_dispatch(*action) {
                    MainMenuDispatch::OpenMatchSetup => {
                        info!("Opening Match Setup screen from MainMenu.");
                        next_state.set(GameState::MatchSetup);
                    }
                    MainMenuDispatch::OpenSettings => {
                        info!("Opening Settings screen from MainMenu.");
                        next_state.set(GameState::Settings);
                    }
                    MainMenuDispatch::OpenBestiary => {
                        info!("Opening Bestiary screen from MainMenu.");
                        next_state.set(GameState::Archive);
                    }
                    MainMenuDispatch::Exit => {
                        info!("Exit requested from MainMenu button.");
                        app_exit_events.send(AppExit::Success);
                    }
                    MainMenuDispatch::DisabledOnline => {
                        info!("Play Online is disabled in the current build.");
                    }
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(if disabled.is_some() {
                    MENU_BUTTON_BORDER_DISABLED
                } else {
                    MENU_BUTTON_BORDER_HOVERED
                });
                *background = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(if disabled.is_some() {
                    MENU_BUTTON_BORDER_DISABLED
                } else {
                    Color::NONE
                });
                *background = BackgroundColor(Color::NONE);
            }
        }
    }
}

fn can_select_match_setup_faction(faction: PlayerFaction) -> bool {
    matches!(faction, PlayerFaction::Christian | PlayerFaction::Muslim)
}

fn map_allows_faction(map: &crate::data::MapDefinitionConfig, faction: PlayerFaction) -> bool {
    map.allowed_factions
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(faction.config_key()))
}

fn match_setup_can_start(data: &GameData, setup_selection: &MatchSetupSelection) -> bool {
    if !can_select_match_setup_faction(setup_selection.faction) {
        return false;
    }
    let Some(map) = data.map.find_map(&setup_selection.map_id) else {
        return false;
    };
    map_allows_faction(map, setup_selection.faction)
}

#[allow(clippy::type_complexity)]
fn handle_match_setup_faction_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &MatchSetupFactionButton,
            Option<&MatchSetupButtonDisabled>,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut setup_selection: ResMut<MatchSetupSelection>,
) {
    for (interaction, faction_button, disabled, children) in &mut buttons {
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if disabled.is_some() {
                MENU_BUTTON_TEXT_DISABLED
            } else {
                match *interaction {
                    Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                    Interaction::None => MENU_BUTTON_TEXT_NORMAL,
                }
            };
        }
        if *interaction != Interaction::Pressed {
            continue;
        }
        if disabled.is_some() || !can_select_match_setup_faction(faction_button.faction) {
            info!(
                "Faction '{}' is disabled in current build.",
                faction_button.faction.label()
            );
            continue;
        }
        setup_selection.faction = faction_button.faction;
    }
}

#[allow(clippy::type_complexity)]
fn handle_match_setup_map_buttons(
    mut buttons: Query<(&Interaction, &MatchSetupMapButton), (Changed<Interaction>, With<Button>)>,
    mut setup_selection: ResMut<MatchSetupSelection>,
) {
    for (interaction, map_button) in &mut buttons {
        if *interaction == Interaction::Pressed {
            setup_selection.map_id = map_button.map_id.clone();
            info!("Selected map '{}'.", setup_selection.map_id);
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_match_setup_action_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &MatchSetupAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    data: Res<GameData>,
    setup_selection: Res<MatchSetupSelection>,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut start_run_events: EventWriter<StartRunEvent>,
) {
    let can_start = match_setup_can_start(&data, &setup_selection);
    for (interaction, action, children, mut border_color, mut background) in &mut buttons {
        let is_start = matches!(action, MatchSetupAction::Start);
        let disabled = is_start && !can_start;
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if disabled {
                MENU_BUTTON_TEXT_DISABLED
            } else {
                match *interaction {
                    Interaction::Hovered | Interaction::Pressed => MENU_BUTTON_TEXT_HOVERED,
                    Interaction::None => MENU_BUTTON_TEXT_NORMAL,
                }
            };
        }

        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(if disabled {
                    MENU_BUTTON_BORDER_DISABLED
                } else {
                    MENU_BUTTON_BORDER_HOVERED
                });
                *background = BackgroundColor(Color::NONE);
                if disabled {
                    info!("Cannot start match: invalid faction/map selection.");
                    continue;
                }
                match action {
                    MatchSetupAction::Start => {
                        *run_session = RunSession::default();
                        start_run_events.send(StartRunEvent);
                        next_state.set(GameState::InRun);
                    }
                    MatchSetupAction::Back => {
                        next_state.set(GameState::MainMenu);
                    }
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(if disabled {
                    MENU_BUTTON_BORDER_DISABLED
                } else {
                    MENU_BUTTON_BORDER_HOVERED
                });
                *background = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(if disabled {
                    MENU_BUTTON_BORDER_DISABLED
                } else {
                    Color::NONE
                });
                *background = BackgroundColor(Color::NONE);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn refresh_match_setup_faction_button_visuals(
    setup_selection: Res<MatchSetupSelection>,
    mut faction_buttons: Query<
        (
            &Interaction,
            &MatchSetupFactionButton,
            Option<&MatchSetupButtonDisabled>,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (With<Button>, With<MatchSetupFactionButton>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, faction_button, disabled, children, mut border, mut background) in
        &mut faction_buttons
    {
        let selected = setup_selection.faction == faction_button.faction;
        let hovered = matches!(*interaction, Interaction::Hovered | Interaction::Pressed);
        let is_disabled = disabled.is_some();
        *border = BorderColor(if is_disabled {
            MENU_BUTTON_BORDER_DISABLED
        } else if selected || hovered {
            MENU_BUTTON_BORDER_HOVERED
        } else {
            Color::NONE
        });
        *background = BackgroundColor(Color::NONE);
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if is_disabled {
                MENU_BUTTON_TEXT_DISABLED
            } else if selected || hovered {
                MENU_BUTTON_TEXT_HOVERED
            } else {
                MENU_BUTTON_TEXT_NORMAL
            };
        }
    }
}

#[allow(clippy::type_complexity)]
fn refresh_match_setup_map_button_visuals(
    setup_selection: Res<MatchSetupSelection>,
    mut map_buttons: Query<
        (
            &Interaction,
            &MatchSetupMapButton,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (With<Button>, With<MatchSetupMapButton>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, map_button, children, mut border, mut background) in &mut map_buttons {
        let selected = setup_selection.map_id == map_button.map_id;
        let hovered = matches!(*interaction, Interaction::Hovered | Interaction::Pressed);
        *border = BorderColor(if selected || hovered {
            MENU_BUTTON_BORDER_HOVERED
        } else {
            Color::NONE
        });
        *background = BackgroundColor(Color::NONE);
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if selected || hovered {
                MENU_BUTTON_TEXT_HOVERED
            } else {
                MENU_BUTTON_TEXT_NORMAL
            };
        }
    }
}

#[allow(clippy::type_complexity)]
fn refresh_match_setup_action_button_visuals(
    data: Res<GameData>,
    setup_selection: Res<MatchSetupSelection>,
    mut action_buttons: Query<
        (
            &Interaction,
            &MatchSetupAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (With<Button>, With<MatchSetupAction>),
    >,
    mut text_query: Query<&mut Text>,
) {
    let can_start = match_setup_can_start(&data, &setup_selection);
    for (interaction, action, children, mut border, mut background) in &mut action_buttons {
        let disabled = matches!(action, MatchSetupAction::Start) && !can_start;
        let hovered = matches!(*interaction, Interaction::Hovered | Interaction::Pressed);
        *border = BorderColor(if disabled {
            MENU_BUTTON_BORDER_DISABLED
        } else if hovered {
            MENU_BUTTON_BORDER_HOVERED
        } else {
            Color::NONE
        });
        *background = BackgroundColor(Color::NONE);
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if disabled {
                MENU_BUTTON_TEXT_DISABLED
            } else if hovered {
                MENU_BUTTON_TEXT_HOVERED
            } else {
                MENU_BUTTON_TEXT_NORMAL
            };
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
fn handle_archive_menu_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &ArchiveMenuAction,
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
                    ArchiveMenuAction::Back => {
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
    mut run_modal_state: ResMut<RunModalState>,
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
                        request_run_restart(
                            &mut run_session,
                            &mut run_modal_state,
                            &mut start_run_events,
                            &mut next_state,
                        );
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
fn handle_victory_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &VictoryMenuAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut run_modal_state: ResMut<RunModalState>,
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
                    VictoryMenuAction::Restart => {
                        info!("Restart requested from Victory screen.");
                        request_run_restart(
                            &mut run_session,
                            &mut run_modal_state,
                            &mut start_run_events,
                            &mut next_state,
                        );
                    }
                    VictoryMenuAction::MainMenu => {
                        info!("Returning to MainMenu from Victory screen.");
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
    mut run_modal_state: ResMut<RunModalState>,
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
                        request_run_restart(
                            &mut run_session,
                            &mut run_modal_state,
                            &mut start_run_events,
                            &mut next_state,
                        );
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

fn request_run_restart(
    run_session: &mut RunSession,
    run_modal_state: &mut RunModalState,
    start_run_events: &mut EventWriter<StartRunEvent>,
    next_state: &mut NextState<GameState>,
) {
    *run_session = RunSession::default();
    *run_modal_state = RunModalState::None;
    start_run_events.send(StartRunEvent);
    next_state.set(GameState::InRun);
}

#[allow(clippy::type_complexity)]
fn handle_level_up_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &LevelUpOptionAction,
            &LevelUpCardTierStyle,
            &mut BorderColor,
            &mut BackgroundColor,
            &mut Outline,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut select_events: EventWriter<SelectUpgradeEvent>,
) {
    for (interaction, option, tier_style, mut border_color, mut background, mut outline) in
        &mut buttons
    {
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(tier_style.hover_border);
                *background = BackgroundColor(Color::srgba(0.14, 0.12, 0.09, 0.82));
                outline.color = tier_style.hover_glow;
                outline.width = Val::Px(4.0);
                select_events.send(SelectUpgradeEvent {
                    option_index: option.index,
                });
            }
            Interaction::Hovered => {
                *border_color = BorderColor(tier_style.hover_border);
                *background = BackgroundColor(Color::srgba(0.11, 0.09, 0.08, 0.78));
                outline.color = tier_style.hover_glow;
                outline.width = Val::Px(4.0);
            }
            Interaction::None => {
                *border_color = BorderColor(tier_style.base_border);
                *background = BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.74));
                outline.color = tier_style.base_glow;
                outline.width = Val::Px(3.0);
            }
        }
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn handle_unit_upgrade_buttons(
    modal_state: Res<RunModalState>,
    mut source_buttons: Query<
        (
            &Interaction,
            &UnitUpgradeSourceButton,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (
            Changed<Interaction>,
            With<Button>,
            Without<UnitUpgradePromoteButton>,
        ),
    >,
    mut promote_buttons: Query<
        (
            &Interaction,
            &UnitUpgradePromoteButton,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (
            Changed<Interaction>,
            With<Button>,
            Without<UnitUpgradeSourceButton>,
        ),
    >,
    mut text_query: Query<&mut Text>,
    mut ui_state: ResMut<UnitUpgradeUiState>,
    waves: Res<WaveRuntime>,
    economy: Res<RosterEconomy>,
    progression: Res<Progression>,
    mut economy_feedback: ResMut<RosterEconomyFeedback>,
    mut promote_events: EventWriter<PromoteUnitsEvent>,
) {
    if !matches!(
        *modal_state,
        RunModalState::Open(RunModalScreen::UnitUpgrade)
    ) {
        return;
    }

    for (interaction, button, children, mut border_color, mut background_color) in
        &mut source_buttons
    {
        let selected = ui_state.selected_source == button.kind;
        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if selected
                || matches!(*interaction, Interaction::Hovered | Interaction::Pressed)
            {
                MENU_BUTTON_TEXT_HOVERED
            } else {
                HUD_TEXT_COLOR
            };
        }

        match *interaction {
            Interaction::Pressed => {
                ui_state.selected_source = button.kind;
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background_color = BackgroundColor(Color::srgba(0.11, 0.09, 0.08, 0.66));
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background_color = BackgroundColor(Color::srgba(0.1, 0.085, 0.075, 0.62));
            }
            Interaction::None => {
                *border_color = BorderColor(if selected {
                    MENU_BUTTON_BORDER_HOVERED
                } else {
                    Color::srgba(0.78, 0.72, 0.58, 0.24)
                });
                *background_color = BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.62));
            }
        }
    }

    for (interaction, button, children, mut border_color, mut background_color) in
        &mut promote_buttons
    {
        let current_wave = waves.current_wave.max(1);
        let source_count = roster_count_for_kind(&economy, button.from);
        let max_affordable = max_affordable_promotions(
            current_wave,
            progression.level.max(1),
            economy.locked_levels,
            source_count,
            button.from,
            button.to,
        );
        let requested = requested_promotion_count(button.quantity, max_affordable);
        let enabled = requested > 0;

        if let Some(&text_entity) = children.first()
            && let Ok(mut text) = text_query.get_mut(text_entity)
        {
            text.sections[0].style.color = if enabled {
                if matches!(*interaction, Interaction::Hovered | Interaction::Pressed) {
                    MENU_BUTTON_TEXT_HOVERED
                } else {
                    HUD_TEXT_COLOR
                }
            } else {
                MENU_BUTTON_TEXT_DISABLED
            };
        }

        match *interaction {
            Interaction::Pressed => {
                if enabled {
                    promote_events.send(PromoteUnitsEvent {
                        from_kind: button.from,
                        to_kind: button.to,
                        count: requested,
                    });
                    economy_feedback.blocked_upgrade_reason = None;
                    *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                    *background_color = BackgroundColor(Color::srgba(0.11, 0.09, 0.08, 0.7));
                } else {
                    let tier_locked_reason = friendly_tier_for_kind(button.to).and_then(|tier| {
                        if is_upgrade_tier_unlocked(tier, current_wave) {
                            None
                        } else {
                            Some(format!(
                                "Promotion blocked: tier {tier} upgrades unlock at wave {} (current wave {}).",
                                unlock_wave_for_tier(tier).unwrap_or(1),
                                current_wave
                            ))
                        }
                    });
                    economy_feedback.blocked_upgrade_reason = tier_locked_reason.or_else(|| {
                        Some(format!(
                            "Promotion blocked: '{}' -> '{}' has no affordable quantity at current level budget.",
                            unit_kind_label(button.from),
                            unit_kind_label(button.to)
                        ))
                    });
                    *border_color = BorderColor(MENU_BUTTON_BORDER_DISABLED);
                    *background_color = BackgroundColor(Color::srgba(0.06, 0.055, 0.05, 0.64));
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(if enabled {
                    MENU_BUTTON_BORDER_HOVERED
                } else {
                    MENU_BUTTON_BORDER_DISABLED
                });
                *background_color = BackgroundColor(Color::srgba(0.1, 0.085, 0.075, 0.62));
            }
            Interaction::None => {
                *border_color = BorderColor(if enabled {
                    Color::srgba(0.78, 0.72, 0.58, 0.24)
                } else {
                    MENU_BUTTON_BORDER_DISABLED
                });
                *background_color = BackgroundColor(Color::srgba(0.08, 0.07, 0.06, 0.62));
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_run_modal_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &RunModalButtonAction,
            &Children,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut modal_requests: EventWriter<RunModalRequestEvent>,
    mut modal_state: ResMut<RunModalState>,
) {
    for (interaction, action, children, mut border_color, mut background_color) in &mut buttons {
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
                *background_color = BackgroundColor(Color::NONE);
                if matches!(action, RunModalButtonAction::Close) {
                    *modal_state = RunModalState::None;
                    modal_requests.send(RunModalRequestEvent {
                        action: RunModalAction::Close,
                    });
                }
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background_color = BackgroundColor(Color::NONE);
            }
            Interaction::None => {
                *border_color = BorderColor(Color::NONE);
                *background_color = BackgroundColor(Color::NONE);
            }
        }
    }
}

fn handle_scroll_views(
    mut wheel_events: EventReader<MouseWheel>,
    mut content_query: Query<(&Parent, &Node, &mut Style, &mut ScrollContent), With<ScrollContent>>,
    viewport_query: Query<(&Node, &ScrollViewport), With<ScrollViewport>>,
) {
    let wheel_delta = wheel_events.read().map(|event| event.y).sum::<f32>() * 28.0;

    for (parent, content_node, mut style, mut content) in &mut content_query {
        let Ok((viewport_node, viewport)) = viewport_query.get(parent.get()) else {
            continue;
        };
        let viewport_height = if viewport_node.size().y > 1.0 {
            viewport_node.size().y
        } else {
            viewport.max_height
        };
        let max_negative_offset = (content_node.size().y - viewport_height).max(0.0);
        let next_offset = (content.offset + wheel_delta).clamp(-max_negative_offset, 0.0);
        if (next_offset - content.offset).abs() > f32::EPSILON {
            content.offset = next_offset;
            style.top = Val::Px(next_offset);
        } else if max_negative_offset <= 0.0 && content.offset != 0.0 {
            content.offset = 0.0;
            style.top = Val::Px(0.0);
        }
    }
}

#[allow(clippy::type_complexity)]
fn handle_utility_bar_buttons(
    mut buttons: Query<
        (
            &Interaction,
            &UtilityBarButton,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut modal_requests: EventWriter<RunModalRequestEvent>,
) {
    for (interaction, button, mut border_color, mut background_color) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background_color = BackgroundColor(Color::srgba(0.15, 0.12, 0.09, 0.58));
                modal_requests.send(RunModalRequestEvent {
                    action: modal_action_for_utility_button(button.screen),
                });
            }
            Interaction::Hovered => {
                *border_color = BorderColor(MENU_BUTTON_BORDER_HOVERED);
                *background_color = BackgroundColor(Color::srgba(0.11, 0.09, 0.08, 0.46));
            }
            Interaction::None => {
                *border_color = BorderColor(Color::NONE);
                *background_color = BackgroundColor(Color::NONE);
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

pub fn responsive_ui_scale_for_resolution(width: f32, height: f32) -> f32 {
    if width <= 0.0 || height <= 0.0 {
        return 1.0;
    }
    (width / UI_REFERENCE_WIDTH)
        .min(height / UI_REFERENCE_HEIGHT)
        .clamp(UI_SCALE_MIN, UI_SCALE_MAX)
}

fn sync_responsive_ui_scale(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Option<ResMut<UiScale>>,
) {
    let Some(mut ui_scale) = ui_scale else {
        return;
    };
    let Ok(window) = primary_window.get_single() else {
        return;
    };
    let next_scale =
        responsive_ui_scale_for_resolution(window.resolution.width(), window.resolution.height());
    if (ui_scale.0 - next_scale).abs() > f32::EPSILON {
        ui_scale.0 = next_scale;
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_hud_snapshot(
    cohesion: Res<Cohesion>,
    banner_state: Res<BannerState>,
    roster: Res<SquadRoster>,
    roster_economy: Res<RosterEconomy>,
    progression: Res<Progression>,
    progression_feedback: Res<ProgressionLockFeedback>,
    waves: Res<WaveRuntime>,
    run_session: Res<RunSession>,
    friendlies: Query<&Morale, With<FriendlyUnit>>,
    enemies: Query<(), With<EnemyUnit>>,
    mut hud: ResMut<HudSnapshot>,
) {
    let morale_ratios: Vec<f32> = friendlies.iter().map(|morale| morale.ratio()).collect();
    *hud = HudSnapshot {
        cohesion: cohesion.value,
        banner_dropped: banner_state.is_dropped,
        squad_size: roster.friendly_count,
        retinue_count: roster_economy.total_retinue_count as usize,
        level: progression.level,
        allowed_max_level: roster_economy.allowed_max_level,
        xp: progression.xp,
        next_level_xp: progression.next_level_xp,
        wave_index: waves.current_wave.saturating_sub(1) as usize,
        current_wave: displayed_wave_number(&waves),
        elapsed_seconds: run_session.survived_seconds,
        alive_enemy_count: enemies.iter().len(),
        average_morale_ratio: average_morale_ratio(&morale_ratios),
        progression_lock_reason: progression_feedback.message.clone(),
    };
}

#[allow(clippy::type_complexity)]
fn update_in_run_hud(
    hud: Res<HudSnapshot>,
    mut texts: ParamSet<(
        Query<&mut Text, With<WaveHudText>>,
        Query<&mut Text, With<TimeHudText>>,
        Query<&mut Text, With<EnemyCountHudText>>,
        Query<&mut Text, With<RetinueCountHudText>>,
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
        text.sections[0].value = format_enemy_count(hud.alive_enemy_count);
    }
    if let Ok(mut text) = texts.p3().get_single_mut() {
        text.sections[0].value = format_retinue_count(hud.retinue_count);
    }
    if let Ok(mut text) = texts.p4().get_single_mut() {
        text.sections[0].value = format_commander_level_text(
            hud.level,
            hud.allowed_max_level,
            hud.progression_lock_reason.is_some(),
        );
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
    setup_selection: Option<Res<MatchSetupSelection>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    banner_state: Res<BannerState>,
    rescue_bars_root: Query<Entity, With<RescueProgressBarsRoot>>,
    rescuables: Query<&RescueProgress, With<RescuableUnit>>,
) {
    let Ok(root_entity) = rescue_bars_root.get_single() else {
        return;
    };
    commands.entity(root_entity).despawn_descendants();

    let duration = effective_rescue_duration(
        data.rescue.rescue_duration_secs,
        conditional_effects.as_deref(),
        data.factions
            .for_faction(
                setup_selection
                    .as_deref()
                    .map(|selection| selection.faction)
                    .unwrap_or(PlayerFaction::Christian),
            )
            .rescue_time_multiplier,
    );
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
    art: Res<ArtAssets>,
    banner_state: Res<BannerState>,
    bounds: Option<Res<MapBounds>>,
    mut runtime: ResMut<MinimapRefreshRuntime>,
    minimap_roots: Query<Entity, With<MinimapDotsRoot>>,
    units: Query<(&Unit, &Transform)>,
    rescuables: Query<&Transform, With<RescuableUnit>>,
    exp_packs: Query<&Transform, With<ExpPack>>,
    magnets: Query<(&Transform, &MagnetPickup)>,
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
        let mut commander_seen = false;
        let mut rescuable_count = 0usize;
        let mut exp_count = 0usize;
        let mut magnet_count = 0usize;
        for (unit, transform) in &units {
            if commander_seen
                && friendly_count >= MINIMAP_MAX_FRIENDLY_BLIPS
                && enemy_count >= MINIMAP_MAX_ENEMY_BLIPS
            {
                break;
            }
            let position = transform.translation.truncate();
            let Some(draw_pos) = world_to_minimap_pos(position, *bounds, MINIMAP_SIZE) else {
                continue;
            };

            match unit.team {
                Team::Friendly => {
                    let (color, dot_size) = if unit.kind == UnitKind::Commander {
                        commander_seen = true;
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

        for (transform, magnet) in &magnets {
            if magnet_count >= MINIMAP_MAX_MAGNET_BLIPS {
                break;
            }
            let position = transform.translation.truncate();
            let Some(draw_pos) = world_to_minimap_pos(position, *bounds, MINIMAP_SIZE) else {
                continue;
            };
            magnet_count += 1;
            let texture = match magnet.faction {
                PlayerFaction::Christian => art.magnet_cross_pickup.clone(),
                PlayerFaction::Muslim => art.magnet_crescent_pickup.clone(),
            };
            spawn_minimap_icon(parent, draw_pos, 9.0, texture);
        }

        if banner_state.is_dropped
            && let Some(draw_pos) =
                world_to_minimap_pos(banner_state.world_position, *bounds, MINIMAP_SIZE)
        {
            spawn_minimap_dot(parent, draw_pos, 3.6, MINIMAP_DROPPED_BANNER_COLOR);
        }
    });
}

fn draw_commander_aura_footprint(
    mut gizmos: Gizmos,
    data: Res<GameData>,
    buffs: Res<crate::model::GlobalBuffs>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
) {
    if !has_active_aura_footprint(&buffs) {
        return;
    }
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let player_faction = setup_selection
        .as_deref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let radius = commander_aura_radius(&data, Some(&buffs), player_faction).max(1.0);
    let center = commander_transform.translation.truncate();
    let edge_color = Color::srgba(0.62, 0.86, 1.0, 0.18);
    gizmos.circle_2d(center, radius, edge_color);
}

fn has_active_aura_footprint(buffs: &crate::model::GlobalBuffs) -> bool {
    buffs.authority_friendly_loss_resistance > 0.0
        || buffs.authority_enemy_morale_drain_per_sec > 0.0
        || buffs.hospitalier_hp_regen_per_sec > 0.0
        || buffs.hospitalier_cohesion_regen_per_sec > 0.0
        || buffs.hospitalier_morale_regen_per_sec > 0.0
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

fn spawn_minimap_icon(
    parent: &mut ChildBuilder,
    draw_pos: Vec2,
    icon_size: f32,
    texture: Handle<Image>,
) {
    parent.spawn(ImageBundle {
        style: Style {
            position_type: PositionType::Absolute,
            left: Val::Px(draw_pos.x - icon_size * 0.5),
            top: Val::Px(draw_pos.y - icon_size * 0.5),
            width: Val::Px(icon_size),
            height: Val::Px(icon_size),
            ..default()
        },
        image: UiImage::new(texture),
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

pub fn format_commander_level_text(
    level: u32,
    allowed_max_level: u32,
    is_budget_locked: bool,
) -> String {
    let clamped_level = level.max(1);
    let clamped_allowed = allowed_max_level.max(1);
    if is_budget_locked {
        format!(
            "Commander Lv {}/{} [LOCKED BY ROSTER COST]",
            clamped_level, clamped_allowed
        )
    } else {
        format!("Commander Lv {}/{}", clamped_level, clamped_allowed)
    }
}

pub fn displayed_wave_number(runtime: &WaveRuntime) -> u32 {
    runtime.current_wave.max(1)
}

pub fn format_elapsed_mm_ss(seconds: f32) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    let minutes = total_seconds / 60;
    let secs = total_seconds % 60;
    format!("{minutes:02}:{secs:02}")
}

pub fn format_enemy_count(count: usize) -> String {
    format!("Enemies: {count}")
}

pub fn format_retinue_count(count: usize) -> String {
    format!("Retinue: {count}")
}

pub fn conditional_upgrade_hud_status_text(
    status: &ConditionalUpgradeStatus,
    max_priest_blessing_secs: f32,
) -> String {
    let mut entries = Vec::new();
    if let Some(active) = conditional_upgrade_active_state(status, "mob_fury") {
        entries.push(if active {
            "Mob's Fury: ACTIVE".to_string()
        } else {
            "Mob's Fury: INACTIVE".to_string()
        });
    }
    if let Some(active) = conditional_upgrade_active_state(status, "mob_justice") {
        entries.push(if active {
            "Mob's Justice: EXECUTE <10% HP".to_string()
        } else {
            "Mob's Justice: INACTIVE".to_string()
        });
    }
    if let Some(active) = conditional_upgrade_active_state(status, "mob_mercy") {
        entries.push(if active {
            "Mob's Mercy: RESCUE x0.5".to_string()
        } else {
            "Mob's Mercy: INACTIVE".to_string()
        });
    }
    if max_priest_blessing_secs > 0.0 {
        entries.push(format!("Priest Blessing: {:.1}s", max_priest_blessing_secs));
    }
    entries.join("\n")
}

fn conditional_upgrade_active_state(status: &ConditionalUpgradeStatus, kind: &str) -> Option<bool> {
    status
        .entries
        .iter()
        .rev()
        .find(|entry| entry.kind == kind)
        .map(|entry| entry.active)
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

fn spawn_floating_damage_text(
    mut commands: Commands,
    mut damage_text_events: EventReader<DamageTextEvent>,
    active_texts: Query<Entity, With<FloatingDamageText>>,
) {
    let mut active_count = active_texts.iter().len();
    let mut spawned_this_frame = 0usize;
    for event in damage_text_events.read() {
        if event.amount <= 0.0 {
            continue;
        }
        if active_count >= FLOATING_DAMAGE_TEXT_MAX_ACTIVE {
            continue;
        }
        if spawned_this_frame >= FLOATING_DAMAGE_TEXT_MAX_SPAWNS_PER_FRAME {
            continue;
        }
        let spawn_data = floating_damage_text_spawn_data(event, spawned_this_frame);
        commands.spawn((
            FloatingDamageText,
            FloatingDamageTextRuntime {
                age_secs: 0.0,
                lifetime_secs: FLOATING_DAMAGE_TEXT_LIFETIME_SECS,
                rise_speed: FLOATING_DAMAGE_TEXT_RISE_SPEED,
                base_rgb: spawn_data.base_rgb,
            },
            Text2dBundle {
                text: Text::from_section(
                    spawn_data.text,
                    TextStyle {
                        font_size: spawn_data.font_size,
                        color: Color::srgba(
                            spawn_data.base_rgb.x,
                            spawn_data.base_rgb.y,
                            spawn_data.base_rgb.z,
                            1.0,
                        ),
                        ..default()
                    },
                ),
                transform: Transform::from_translation(spawn_data.translation),
                text_anchor: bevy::sprite::Anchor::Center,
                ..default()
            },
        ));
        active_count += 1;
        spawned_this_frame += 1;
    }
}

fn animate_floating_damage_text(
    mut commands: Commands,
    time: Res<Time>,
    mut floating_texts: Query<
        (
            Entity,
            &mut FloatingDamageTextRuntime,
            &mut Transform,
            &mut Text,
        ),
        With<FloatingDamageText>,
    >,
) {
    let delta_secs = time.delta_seconds();
    for (entity, mut runtime, mut transform, mut text) in &mut floating_texts {
        runtime.age_secs += delta_secs;
        if floating_damage_text_is_expired(runtime.age_secs, runtime.lifetime_secs) {
            commands.entity(entity).despawn_recursive();
            continue;
        }
        transform.translation.y += runtime.rise_speed * delta_secs;
        let alpha = floating_damage_text_alpha(runtime.age_secs, runtime.lifetime_secs);
        text.sections[0].style.color = Color::srgba(
            runtime.base_rgb.x,
            runtime.base_rgb.y,
            runtime.base_rgb.z,
            alpha,
        );
    }
}

fn despawn_floating_damage_text(
    mut commands: Commands,
    floating_texts: Query<Entity, With<FloatingDamageText>>,
) {
    for entity in &floating_texts {
        commands.entity(entity).despawn_recursive();
    }
}

fn floating_damage_text_spawn_data(
    event: &DamageTextEvent,
    spawn_index: usize,
) -> FloatingDamageTextSpawnData {
    const X_JITTER_LANES: [f32; 5] = [-10.0, -5.0, 0.0, 5.0, 10.0];
    let lane_index = spawn_index % X_JITTER_LANES.len();
    let row = ((spawn_index / X_JITTER_LANES.len()) % 3) as f32;
    let x_offset = X_JITTER_LANES[lane_index];
    let y_offset = FLOATING_DAMAGE_TEXT_START_Y_OFFSET + row * 4.0;
    let (text, base_rgb, font_size) = if event.execute {
        (
            "EXECUTE".to_string(),
            Vec3::new(0.98, 0.28, 0.18),
            FLOATING_DAMAGE_TEXT_FONT_SIZE,
        )
    } else if event.critical {
        (
            format!("{}!", format_damage_text_amount(event.amount)),
            Vec3::new(1.0, 0.12, 0.94),
            FLOATING_DAMAGE_TEXT_CRIT_FONT_SIZE,
        )
    } else {
        (
            format_damage_text_amount(event.amount),
            floating_damage_text_team_rgb(event.target_team),
            FLOATING_DAMAGE_TEXT_FONT_SIZE,
        )
    };
    FloatingDamageTextSpawnData {
        translation: Vec3::new(
            event.world_position.x + x_offset,
            event.world_position.y + y_offset,
            FLOATING_DAMAGE_TEXT_Z,
        ),
        text,
        base_rgb,
        font_size,
    }
}

fn format_damage_text_amount(amount: f32) -> String {
    let rounded = amount.max(1.0).round();
    format!("{}", rounded as u32)
}

fn floating_damage_text_team_rgb(team: Team) -> Vec3 {
    match team {
        Team::Friendly => Vec3::new(0.96, 0.33, 0.25),
        Team::Enemy => Vec3::new(0.98, 0.88, 0.23),
        Team::Neutral => Vec3::new(0.84, 0.84, 0.8),
    }
}

fn floating_damage_text_alpha(age_secs: f32, lifetime_secs: f32) -> f32 {
    if lifetime_secs <= 0.0 {
        return 0.0;
    }
    (1.0 - age_secs / lifetime_secs).clamp(0.0, 1.0)
}

fn floating_damage_text_is_expired(age_secs: f32, lifetime_secs: f32) -> bool {
    age_secs >= lifetime_secs
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
    use std::path::Path;

    use crate::archive::{ArchiveCategory, ArchiveDataset, ArchiveEntry};
    use crate::core::hotkey_to_run_modal_screen;
    use crate::data::GameData;
    use crate::enemies::WaveRuntime;
    use crate::formation::{ActiveFormation, FormationModifiers, FormationSkillBar};
    use crate::map::MapBounds;
    use crate::model::{
        DamageTextEvent, FrameRateCap, GlobalBuffs, PlayerFaction, RunModalAction, RunModalScreen,
        Team,
    };
    use crate::ui::{
        FLOATING_DAMAGE_TEXT_CRIT_FONT_SIZE, FLOATING_DAMAGE_TEXT_FONT_SIZE, HudSnapshot,
        LEVEL_UP_TIER_LEGEND, MainMenuAction, MainMenuDispatch, UnitUpgradeQuantity,
        archive_entries_for_category, build_skill_book_panel_data, build_stats_panel_data,
        can_select_match_setup_faction, conditional_upgrade_hud_status_text, displayed_wave_number,
        find_skill_section, find_stats_row, floating_damage_text_alpha,
        floating_damage_text_is_expired, floating_damage_text_spawn_data,
        format_commander_level_text, format_elapsed_mm_ss, format_enemy_count,
        format_retinue_count, frame_cap_label, health_bar_fill_width, level_up_tier_border_color,
        main_menu_dispatch, max_affordable_promotions, modal_action_for_utility_button,
        requested_promotion_count, rescue_progress_ratio, responsive_ui_scale_for_resolution,
        world_to_minimap_pos,
    };
    use crate::upgrades::{
        ConditionalUpgradeStatus, ConditionalUpgradeStatusEntry, Progression, SkillBookEntry,
        SkillBookLog, UpgradeCardIcon,
    };

    #[test]
    fn snapshot_holds_expected_values() {
        let snapshot = HudSnapshot {
            cohesion: 70.0,
            banner_dropped: true,
            squad_size: 5,
            retinue_count: 4,
            level: 2,
            allowed_max_level: 197,
            xp: 12.0,
            next_level_xp: 45.0,
            wave_index: 2,
            current_wave: 2,
            elapsed_seconds: 61.0,
            alive_enemy_count: 37,
            average_morale_ratio: 0.74,
            progression_lock_reason: Some("blocked by budget".to_string()),
        };
        assert!(snapshot.banner_dropped);
        assert_eq!(snapshot.squad_size, 5);
        assert_eq!(snapshot.retinue_count, 4);
    }

    #[test]
    fn health_bar_width_clamps_from_zero_to_full() {
        assert_eq!(health_bar_fill_width(0.0, 100.0, 22.0), 0.0);
        assert_eq!(health_bar_fill_width(100.0, 100.0, 22.0), 22.0);
        assert_eq!(health_bar_fill_width(150.0, 100.0, 22.0), 22.0);
        assert_eq!(health_bar_fill_width(-10.0, 100.0, 22.0), 0.0);
    }

    #[test]
    fn damage_text_spawn_data_maps_event_payload() {
        let event = DamageTextEvent {
            world_position: bevy::prelude::Vec2::new(100.0, -50.0),
            target_team: Team::Enemy,
            amount: 12.6,
            execute: false,
            critical: false,
        };
        let data = floating_damage_text_spawn_data(&event, 0);
        assert_eq!(data.text, "13");
        assert!((data.translation.x - 90.0).abs() < 0.001);
        assert!((data.translation.y - -26.0).abs() < 0.001);
        assert!((data.translation.z - 60.0).abs() < 0.001);
        assert!((data.base_rgb.x - 0.98).abs() < 0.001);
        assert!((data.font_size - FLOATING_DAMAGE_TEXT_FONT_SIZE).abs() < 0.001);
    }

    #[test]
    fn damage_text_spawn_data_uses_critical_feedback_style() {
        let event = DamageTextEvent {
            world_position: bevy::prelude::Vec2::new(0.0, 0.0),
            target_team: Team::Enemy,
            amount: 74.6,
            execute: false,
            critical: true,
        };
        let data = floating_damage_text_spawn_data(&event, 1);
        assert_eq!(data.text, "75!");
        assert!((data.base_rgb.x - 1.0).abs() < 0.001);
        assert!((data.base_rgb.y - 0.12).abs() < 0.001);
        assert!((data.base_rgb.z - 0.94).abs() < 0.001);
        assert!((data.font_size - FLOATING_DAMAGE_TEXT_CRIT_FONT_SIZE).abs() < 0.001);
    }

    #[test]
    fn damage_text_spawn_data_uses_execute_feedback_style() {
        let event = DamageTextEvent {
            world_position: bevy::prelude::Vec2::new(0.0, 0.0),
            target_team: Team::Enemy,
            amount: 999.0,
            execute: true,
            critical: false,
        };
        let data = floating_damage_text_spawn_data(&event, 2);
        assert_eq!(data.text, "EXECUTE");
        assert!((data.base_rgb.x - 0.98).abs() < 0.001);
        assert!((data.base_rgb.y - 0.28).abs() < 0.001);
        assert!((data.base_rgb.z - 0.18).abs() < 0.001);
        assert!((data.font_size - FLOATING_DAMAGE_TEXT_FONT_SIZE).abs() < 0.001);
    }

    #[test]
    fn floating_damage_text_cleanup_logic_expires_at_lifetime() {
        assert!((floating_damage_text_alpha(0.0, 1.0) - 1.0).abs() < 0.001);
        assert!((floating_damage_text_alpha(0.5, 1.0) - 0.5).abs() < 0.001);
        assert_eq!(floating_damage_text_alpha(2.0, 1.0), 0.0);
        assert!(!floating_damage_text_is_expired(0.69, 0.7));
        assert!(floating_damage_text_is_expired(0.7, 0.7));
    }

    #[test]
    fn frame_cap_labels_match_expected_values() {
        assert_eq!(frame_cap_label(FrameRateCap::Fps60), "60");
        assert_eq!(frame_cap_label(FrameRateCap::Fps90), "90");
        assert_eq!(frame_cap_label(FrameRateCap::Fps120), "120");
    }

    #[test]
    fn responsive_ui_scale_uses_reference_resolution_as_baseline() {
        assert!((responsive_ui_scale_for_resolution(1280.0, 720.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn responsive_ui_scale_is_limited_by_shorter_axis() {
        // Ultrawide should scale from height, not width.
        let scale = responsive_ui_scale_for_resolution(2560.0, 1080.0);
        assert!((scale - 1.5).abs() < 0.001);
    }

    #[test]
    fn responsive_ui_scale_is_clamped_to_safe_bounds() {
        assert!((responsive_ui_scale_for_resolution(320.0, 180.0) - 0.7).abs() < 0.001);
        assert!((responsive_ui_scale_for_resolution(7680.0, 4320.0) - 3.0).abs() < 0.001);
    }

    #[test]
    fn elapsed_time_formats_as_minutes_seconds() {
        assert_eq!(format_elapsed_mm_ss(0.0), "00:00");
        assert_eq!(format_elapsed_mm_ss(65.3), "01:05");
        assert_eq!(format_elapsed_mm_ss(600.9), "10:00");
    }

    #[test]
    fn enemy_count_formats_as_counter_text() {
        assert_eq!(format_enemy_count(0), "Enemies: 0");
        assert_eq!(format_enemy_count(128), "Enemies: 128");
    }

    #[test]
    fn retinue_count_formats_as_counter_text() {
        assert_eq!(format_retinue_count(0), "Retinue: 0");
        assert_eq!(format_retinue_count(56), "Retinue: 56");
    }

    #[test]
    fn commander_level_text_includes_allowed_cap_and_lock_marker() {
        assert_eq!(
            format_commander_level_text(54, 170, false),
            "Commander Lv 54/170"
        );
        assert_eq!(
            format_commander_level_text(170, 170, true),
            "Commander Lv 170/170 [LOCKED BY ROSTER COST]"
        );
    }

    #[test]
    fn requested_promotion_count_clamps_to_affordable_limit() {
        assert_eq!(requested_promotion_count(UnitUpgradeQuantity::One, 3), 1);
        assert_eq!(requested_promotion_count(UnitUpgradeQuantity::Five, 3), 3);
        assert_eq!(requested_promotion_count(UnitUpgradeQuantity::Max, 3), 3);
        assert_eq!(requested_promotion_count(UnitUpgradeQuantity::One, 0), 0);
    }

    #[test]
    fn max_affordable_promotions_respects_budget_and_path_rules() {
        assert_eq!(
            max_affordable_promotions(
                1,
                199,
                0,
                10,
                crate::model::UnitKind::ChristianPeasantInfantry,
                crate::model::UnitKind::ChristianPeasantArcher
            ),
            1
        );
        assert_eq!(
            max_affordable_promotions(
                1,
                150,
                20,
                10,
                crate::model::UnitKind::ChristianPeasantInfantry,
                crate::model::UnitKind::ChristianPeasantPriest
            ),
            10
        );
        assert_eq!(
            max_affordable_promotions(
                1,
                120,
                0,
                10,
                crate::model::UnitKind::ChristianPeasantArcher,
                crate::model::UnitKind::ChristianPeasantPriest
            ),
            0
        );
    }

    #[test]
    fn displayed_wave_number_never_below_one() {
        let mut runtime = WaveRuntime::default();
        assert_eq!(displayed_wave_number(&runtime), 1);
        runtime.current_wave = 3;
        assert_eq!(displayed_wave_number(&runtime), 3);
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

    #[test]
    fn level_up_tier_legend_order_matches_card_rarity_scale() {
        assert_eq!(LEVEL_UP_TIER_LEGEND.len(), 6);
        assert_eq!(LEVEL_UP_TIER_LEGEND[0].1, "Common");
        assert_eq!(LEVEL_UP_TIER_LEGEND[5].1, "Unique");
        let common = level_up_tier_border_color(LEVEL_UP_TIER_LEGEND[0].0);
        let unique = level_up_tier_border_color(LEVEL_UP_TIER_LEGEND[5].0);
        assert_ne!(format!("{common:?}"), format!("{unique:?}"));
    }

    #[test]
    fn utility_button_dispatch_matches_modal_toggle_contract() {
        let action = modal_action_for_utility_button(RunModalScreen::Inventory);
        assert_eq!(action, RunModalAction::Toggle(RunModalScreen::Inventory));
    }

    #[test]
    fn utility_buttons_and_hotkeys_target_same_modal_screens() {
        for key in [
            bevy::prelude::KeyCode::KeyI,
            bevy::prelude::KeyCode::KeyO,
            bevy::prelude::KeyCode::KeyK,
            bevy::prelude::KeyCode::KeyB,
            bevy::prelude::KeyCode::KeyU,
        ] {
            let Some(screen) = hotkey_to_run_modal_screen(key) else {
                panic!("expected modal screen mapping for key: {key:?}");
            };
            let action = modal_action_for_utility_button(screen);
            assert_eq!(action, RunModalAction::Toggle(screen));
        }
    }

    #[test]
    fn main_menu_dispatch_maps_to_expected_actions() {
        assert_eq!(
            main_menu_dispatch(MainMenuAction::PlayOffline),
            MainMenuDispatch::OpenMatchSetup
        );
        assert_eq!(
            main_menu_dispatch(MainMenuAction::Settings),
            MainMenuDispatch::OpenSettings
        );
        assert_eq!(
            main_menu_dispatch(MainMenuAction::Bestiary),
            MainMenuDispatch::OpenBestiary
        );
        assert_eq!(
            main_menu_dispatch(MainMenuAction::Exit),
            MainMenuDispatch::Exit
        );
    }

    #[test]
    fn play_online_dispatch_is_explicitly_disabled() {
        assert_eq!(
            main_menu_dispatch(MainMenuAction::PlayOnline),
            MainMenuDispatch::DisabledOnline
        );
    }

    #[test]
    fn muslim_faction_selection_is_enabled() {
        assert!(can_select_match_setup_faction(PlayerFaction::Christian));
        assert!(can_select_match_setup_faction(PlayerFaction::Muslim));
    }

    #[test]
    fn stats_panel_defaults_show_zero_bonus_values_without_upgrades() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load data");
        let progression = Progression::default();
        let buffs = GlobalBuffs::default();
        let modifiers = FormationModifiers::default();
        let conditional_status = ConditionalUpgradeStatus::default();
        let panel = build_stats_panel_data(
            &data,
            &progression,
            &buffs,
            ActiveFormation::Square,
            &modifiers,
            &conditional_status,
            0.0,
        );

        assert!(!panel.rows.is_empty());
        let damage_row = find_stats_row(&panel.rows, "Damage").expect("damage row");
        assert!((damage_row.bonus_value - 0.0).abs() < 0.001);
        let hp_row = find_stats_row(&panel.rows, "Health").expect("hp row");
        assert!((hp_row.bonus_value - 0.0).abs() < 0.001);
    }

    #[test]
    fn stats_panel_applies_level_and_buff_bonuses() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load data");
        let progression = Progression {
            xp: 0.0,
            level: 8,
            next_level_xp: 1.0,
        };
        let buffs = GlobalBuffs {
            damage_multiplier: 1.15,
            armor_bonus: 3.0,
            attack_speed_multiplier: 1.20,
            xp_gain_multiplier: 1.10,
            crit_chance_bonus: 0.08,
            crit_damage_multiplier: 1.75,
            pickup_radius_bonus: 12.0,
            move_speed_bonus: 18.0,
            inside_formation_damage_multiplier: 1.0,
            commander_aura_radius_bonus: 25.0,
            authority_friendly_loss_resistance: 0.0,
            authority_enemy_morale_drain_per_sec: 0.0,
            hospitalier_hp_regen_per_sec: 0.0,
            hospitalier_cohesion_regen_per_sec: 0.0,
            hospitalier_morale_regen_per_sec: 0.0,
        };
        let modifiers = FormationModifiers {
            offense_multiplier: 1.0,
            offense_while_moving_multiplier: 1.0,
            defense_multiplier: 1.0,
            move_speed_multiplier: 1.08,
        };
        let conditional_status = ConditionalUpgradeStatus::default();
        let panel = build_stats_panel_data(
            &data,
            &progression,
            &buffs,
            ActiveFormation::Diamond,
            &modifiers,
            &conditional_status,
            0.0,
        );

        let hp_row = find_stats_row(&panel.rows, "Health").expect("hp row");
        assert!((hp_row.bonus_value - 7.0).abs() < 0.001);
        let damage_row = find_stats_row(&panel.rows, "Damage").expect("damage row");
        assert!(damage_row.bonus_value > 0.0);
        let crit_chance_row = find_stats_row(&panel.rows, "Crit Chance").expect("crit chance");
        assert!(crit_chance_row.bonus_text.contains("+8.0%"));
        let crit_damage_row = find_stats_row(&panel.rows, "Crit Damage").expect("crit damage");
        assert!(crit_damage_row.bonus_text.contains("+55.0%"));
        let move_row = find_stats_row(&panel.rows, "Move Speed").expect("move row");
        assert!(move_row.bonus_text.contains("+18"));
        assert!(move_row.bonus_text.contains("+8.0%"));
        let morale_resist_row =
            find_stats_row(&panel.rows, "Morale Loss Resist").expect("morale resist");
        assert_eq!(morale_resist_row.bonus_text, "0%");
    }

    #[test]
    fn skill_book_panel_groups_entries_and_tracks_active_formation() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load data");
        let mut skillbar = FormationSkillBar::default();
        assert!(skillbar.try_add_formation(ActiveFormation::Diamond));

        let mut skill_book = SkillBookLog::default();
        skill_book.entries.push(SkillBookEntry {
            id: "damage_up".to_string(),
            kind: "damage".to_string(),
            title: "Sharpened Steel".to_string(),
            description: "Increase damage.".to_string(),
            total_value: 8.0,
            icon: UpgradeCardIcon::Damage,
            stacks: 2,
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
        });
        skill_book.entries.push(SkillBookEntry {
            id: "authority_aura".to_string(),
            kind: "authority_aura".to_string(),
            title: "Authority Aura".to_string(),
            description: "Reduce morale/cohesion losses.".to_string(),
            total_value: 0.1,
            icon: UpgradeCardIcon::AuthorityAura,
            stacks: 1,
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
        });

        let panel = build_skill_book_panel_data(
            &skill_book,
            &ConditionalUpgradeStatus::default(),
            &skillbar,
            ActiveFormation::Diamond,
            &data,
        );
        let formation_section =
            find_skill_section(&panel, "Formations").expect("formation section");
        assert!(
            formation_section
                .entries
                .iter()
                .any(|entry| entry.active == Some(true))
        );
        assert!(
            formation_section
                .entries
                .iter()
                .any(|entry| entry.active == Some(false))
        );

        let combat_section = find_skill_section(&panel, "Combat").expect("combat section");
        assert!(
            combat_section
                .entries
                .iter()
                .any(|entry| entry.title == "Sharpened Steel")
        );
        assert!(combat_section.entries.iter().any(|entry| entry.stacks == 2));

        let aura_section = find_skill_section(&panel, "Auras").expect("auras section");
        assert!(
            aura_section
                .entries
                .iter()
                .any(|entry| entry.title == "Authority Aura")
        );
    }

    #[test]
    fn skill_book_panel_marks_owned_conditional_upgrades_inactive_with_reason() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load data");
        let skillbar = FormationSkillBar::default();
        let mut skill_book = SkillBookLog::default();
        skill_book.entries.push(SkillBookEntry {
            id: "mob_fury".to_string(),
            kind: "mob_fury".to_string(),
            title: "Mob's Fury".to_string(),
            description: "Conditional buff.".to_string(),
            total_value: 1.0,
            icon: UpgradeCardIcon::MobFury,
            stacks: 1,
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
        });
        let conditional_status = ConditionalUpgradeStatus {
            entries: vec![ConditionalUpgradeStatusEntry {
                id: "mob_fury".to_string(),
                kind: "mob_fury".to_string(),
                active: false,
                detail: Some("Requires tier-0 share >= 100%.".to_string()),
            }],
        };
        let panel = build_skill_book_panel_data(
            &skill_book,
            &conditional_status,
            &skillbar,
            ActiveFormation::Square,
            &data,
        );
        let combat_section = find_skill_section(&panel, "Combat").expect("combat section");
        let fury_entry = combat_section
            .entries
            .iter()
            .find(|entry| entry.title == "Mob's Fury")
            .expect("fury entry");
        assert_eq!(fury_entry.active, Some(false));
        assert!(fury_entry.description.contains("Requires tier-0 share"));
    }

    #[test]
    fn conditional_upgrade_hud_status_text_summarizes_mob_upgrade_states() {
        let status = ConditionalUpgradeStatus {
            entries: vec![
                ConditionalUpgradeStatusEntry {
                    id: "mob_fury".to_string(),
                    kind: "mob_fury".to_string(),
                    active: true,
                    detail: None,
                },
                ConditionalUpgradeStatusEntry {
                    id: "mob_justice".to_string(),
                    kind: "mob_justice".to_string(),
                    active: false,
                    detail: Some("requires tier-0 share".to_string()),
                },
                ConditionalUpgradeStatusEntry {
                    id: "mob_mercy".to_string(),
                    kind: "mob_mercy".to_string(),
                    active: true,
                    detail: None,
                },
            ],
        };
        let text = conditional_upgrade_hud_status_text(&status, 6.25);
        assert!(text.contains("Mob's Fury: ACTIVE"));
        assert!(text.contains("Mob's Justice: INACTIVE"));
        assert!(text.contains("Mob's Mercy: RESCUE x0.5"));
        assert!(text.contains("Priest Blessing: 6.2s"));
    }

    #[test]
    fn archive_filter_returns_same_data_for_modal_and_menu_paths() {
        let dataset = ArchiveDataset {
            entries: vec![
                ArchiveEntry {
                    category: ArchiveCategory::Skills,
                    title: "Skill A".to_string(),
                    description: "Desc".to_string(),
                    icon: Some(UpgradeCardIcon::Damage),
                },
                ArchiveEntry {
                    category: ArchiveCategory::Units,
                    title: "Unit A".to_string(),
                    description: "Desc".to_string(),
                    icon: None,
                },
            ],
        };

        let modal_view = archive_entries_for_category(&dataset, ArchiveCategory::Skills);
        let menu_view = archive_entries_for_category(&dataset, ArchiveCategory::Skills);
        assert_eq!(modal_view.len(), 1);
        assert_eq!(menu_view.len(), 1);
        assert_eq!(modal_view[0].title, menu_view[0].title);
    }
}
