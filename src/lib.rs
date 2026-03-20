pub mod archive;
pub mod banner;
pub mod collision;
pub mod combat;
pub mod core;
pub mod data;
pub mod drops;
pub mod enemies;
pub mod formation;
pub mod inventory;
pub mod map;
pub mod model;
pub mod morale;
pub mod performance;
pub mod projectiles;
pub mod rescue;
pub mod settings;
pub mod squad;
pub mod steam;
pub mod ui;
pub mod upgrades;
pub mod visuals;

use std::path::{Path, PathBuf};

use bevy::asset::AssetPlugin;
use bevy::log::tracing_subscriber::Layer;
use bevy::log::{BoxedLayer, Level, LogPlugin};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WinitWindows;
use image::imageops::FilterType;
use tracing_appender::non_blocking::WorkerGuard;

use crate::model::{
    DamageEvent, DamageTextEvent, GainXpEvent, GameState, MatchSetupSelection, RecruitEvent,
    RunModalRequestEvent, RunModalState, RunSession, SpawnExpPackEvent, StartRunEvent,
    UnitDamagedEvent, UnitDiedEvent,
};

#[derive(Resource)]
#[allow(dead_code)]
struct LogFileGuard(WorkerGuard);

fn load_window_icon() -> Option<winit::window::Icon> {
    let icon_bytes = include_bytes!("../assets/branding/game_icon.png");
    let decoded = image::load_from_memory(icon_bytes).ok()?;
    let rgba = decoded.into_rgba8();
    let (width, height, raw) = if rgba.width() > 256 || rgba.height() > 256 {
        let resized = image::imageops::resize(&rgba, 256, 256, FilterType::Lanczos3);
        let (rw, rh) = resized.dimensions();
        (rw, rh, resized.into_raw())
    } else {
        let (rw, rh) = rgba.dimensions();
        (rw, rh, rgba.into_raw())
    };
    winit::window::Icon::from_rgba(raw, width, height).ok()
}

fn apply_window_icon_once(
    windows: Option<NonSend<WinitWindows>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut applied: Local<bool>,
) {
    if *applied {
        return;
    }
    let Some(windows) = windows else {
        return;
    };
    let Ok(primary_entity) = primary_window.get_single() else {
        return;
    };
    let Some(primary) = windows.get_window(primary_entity) else {
        return;
    };
    let Some(icon) = load_window_icon() else {
        warn!("Failed to decode runtime window icon from assets/branding/game_icon.png.");
        *applied = true;
        return;
    };
    primary.set_window_icon(Some(icon));
    info!("Applied runtime window icon from assets/branding/game_icon.png.");
    *applied = true;
}

pub fn configure_game_app(app: &mut App) {
    app.init_state::<GameState>()
        .insert_resource(ClearColor(Color::srgb(0.79, 0.68, 0.51)))
        .init_resource::<RunSession>()
        .init_resource::<MatchSetupSelection>()
        .init_resource::<RunModalState>()
        .add_event::<StartRunEvent>()
        .add_event::<RecruitEvent>()
        .add_event::<DamageEvent>()
        .add_event::<UnitDamagedEvent>()
        .add_event::<DamageTextEvent>()
        .add_event::<UnitDiedEvent>()
        .add_event::<GainXpEvent>()
        .add_event::<SpawnExpPackEvent>()
        .add_event::<RunModalRequestEvent>()
        .add_systems(Update, apply_window_icon_once)
        .add_plugins((
            data::DataPlugin,
            archive::ArchivePlugin,
            core::CorePlugin,
            settings::SettingsPlugin,
            performance::PerformancePlugin,
            visuals::VisualPlugin,
            map::MapPlugin,
            inventory::InventoryPlugin,
            squad::SquadPlugin,
            formation::FormationPlugin,
            collision::CollisionPlugin,
            rescue::RescuePlugin,
            drops::DropsPlugin,
        ))
        .add_plugins((
            enemies::EnemyPlugin,
            combat::CombatPlugin,
            projectiles::ProjectilePlugin,
            morale::MoralePlugin,
            banner::BannerPlugin,
            upgrades::UpgradePlugin,
            ui::UiPlugin,
            steam::PlatformPlugin,
        ));
}

fn runtime_log_dir() -> PathBuf {
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(local_app_data)
            .join("CrusadeRoguelite")
            .join("logs")
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("logs")
    }
}

pub fn discover_assets_dir(exe_dir: &Path, max_ancestor_depth: usize) -> Option<PathBuf> {
    let local_assets = exe_dir.join("assets");
    if local_assets.is_dir() {
        return Some(local_assets);
    }

    let mut current = Some(exe_dir);
    for _ in 0..max_ancestor_depth {
        let Some(dir) = current else {
            break;
        };
        let candidate = dir.join("assets");
        if candidate.is_dir() {
            return Some(candidate);
        }
        current = dir.parent();
    }

    None
}

fn resolve_asset_file_path() -> String {
    let fallback = "assets".to_string();
    let Ok(exe_path) = std::env::current_exe() else {
        return fallback;
    };
    let Some(exe_dir) = exe_path.parent() else {
        return fallback;
    };

    discover_assets_dir(exe_dir, 8)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or(fallback)
}

fn custom_file_log_layer(app: &mut App) -> Option<BoxedLayer> {
    let log_dir = runtime_log_dir();
    if std::fs::create_dir_all(&log_dir).is_err() {
        return None;
    }

    let appender = tracing_appender::rolling::never(log_dir, "crusade_roguelite.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    app.insert_resource(LogFileGuard(guard));

    Some(
        bevy::log::tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_writer(writer)
            .boxed(),
    )
}

pub fn build_runtime_app() -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: resolve_asset_file_path(),
                ..default()
            })
            .set(LogPlugin {
                level: Level::INFO,
                filter: "wgpu=error,naga=warn".to_string(),
                custom_layer: custom_file_log_layer,
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Crusade Roguelite".to_string(),
                    resolution: (1280.0, 720.0).into(),
                    resizable: true,
                    ..default()
                }),
                ..default()
            }),
    );
    configure_game_app(&mut app);
    app
}

pub fn build_headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    configure_game_app(&mut app);
    app
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::discover_assets_dir;

    #[test]
    fn discovers_assets_next_to_executable_directory() {
        let tmp = TempDir::new().expect("temp dir");
        let exe_dir = tmp.path().join("bin");
        let assets_dir = exe_dir.join("assets");
        fs::create_dir_all(&assets_dir).expect("create assets");

        let discovered = discover_assets_dir(&exe_dir, 4).expect("assets found");
        assert_eq!(discovered, assets_dir);
    }

    #[test]
    fn discovers_assets_in_ancestor_directory() {
        let tmp = TempDir::new().expect("temp dir");
        let root_assets = tmp.path().join("assets");
        let exe_dir = tmp.path().join("target").join("release");
        fs::create_dir_all(&root_assets).expect("create root assets");
        fs::create_dir_all(&exe_dir).expect("create exe dir");

        let discovered = discover_assets_dir(&exe_dir, 5).expect("assets found");
        assert_eq!(discovered, root_assets);
    }
}
