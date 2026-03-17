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

use std::path::PathBuf;

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
