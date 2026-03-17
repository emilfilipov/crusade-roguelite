pub mod banner;
pub mod combat;
pub mod core;
pub mod data;
pub mod enemies;
pub mod formation;
pub mod map;
pub mod model;
pub mod morale;
pub mod projectiles;
pub mod rescue;
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
use tracing_appender::non_blocking::WorkerGuard;

use crate::model::{
    DamageEvent, GainXpEvent, GameState, RecruitEvent, RunSession, StartRunEvent, UnitDiedEvent,
};

#[derive(Resource)]
#[allow(dead_code)]
struct LogFileGuard(WorkerGuard);

pub fn configure_game_app(app: &mut App) {
    app.init_state::<GameState>()
        .insert_resource(ClearColor(Color::srgb(0.79, 0.68, 0.51)))
        .init_resource::<RunSession>()
        .add_event::<StartRunEvent>()
        .add_event::<RecruitEvent>()
        .add_event::<DamageEvent>()
        .add_event::<UnitDiedEvent>()
        .add_event::<GainXpEvent>()
        .add_plugins((
            data::DataPlugin,
            core::CorePlugin,
            visuals::VisualPlugin,
            map::MapPlugin,
            squad::SquadPlugin,
            formation::FormationPlugin,
            rescue::RescuePlugin,
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
