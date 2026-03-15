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

use bevy::prelude::*;

use crate::model::{
    DamageEvent, GainXpEvent, GameState, RecruitEvent, RunSession, StartRunEvent, UnitDiedEvent,
};

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

pub fn build_runtime_app() -> App {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Crusade Roguelite".to_string(),
            resolution: (1280.0, 720.0).into(),
            resizable: true,
            ..default()
        }),
        ..default()
    }));
    configure_game_app(&mut app);
    app
}

pub fn build_headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    configure_game_app(&mut app);
    app
}
