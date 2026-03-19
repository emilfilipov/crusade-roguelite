use bevy::prelude::*;

use crate::banner::BannerMarker;
use crate::drops::ExpPack;
use crate::model::{CommanderUnit, GameState, RunSession, Unit};
use crate::projectiles::Projectile;

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Boot), boot_to_menu)
            .add_systems(
                OnEnter(GameState::MainMenu),
                (
                    cleanup_run_entities_on_menu_enter,
                    set_main_menu_clear_color,
                ),
            )
            .add_systems(OnEnter(GameState::Settings), set_main_menu_clear_color)
            .add_systems(OnExit(GameState::MainMenu), set_in_run_clear_color)
            .add_systems(Update, (pause_toggle, resume_from_pause))
            .add_systems(
                PostUpdate,
                detect_game_over.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                tick_survival_time.run_if(in_state(GameState::InRun)),
            );
    }
}

fn boot_to_menu(mut next_state: ResMut<NextState<GameState>>) {
    info!("Transition Boot -> MainMenu");
    next_state.set(GameState::MainMenu);
}

fn cleanup_run_entities_on_menu_enter(
    mut commands: Commands,
    units: Query<Entity, With<Unit>>,
    drops: Query<Entity, With<ExpPack>>,
    banners: Query<Entity, With<BannerMarker>>,
    projectiles: Query<Entity, With<Projectile>>,
    mut run_session: ResMut<RunSession>,
) {
    for entity in &units {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &drops {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &banners {
        commands.entity(entity).despawn_recursive();
    }
    for entity in &projectiles {
        commands.entity(entity).despawn_recursive();
    }
    *run_session = RunSession::default();
}

fn set_main_menu_clear_color(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = Color::srgb(0.12, 0.1, 0.08);
}

fn set_in_run_clear_color(mut clear_color: ResMut<ClearColor>) {
    clear_color.0 = Color::srgb(0.79, 0.68, 0.51);
}

fn pause_toggle(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    if *state.get() != GameState::InRun {
        return;
    }
    if keyboard
        .as_ref()
        .map(|keys| keys.just_pressed(KeyCode::Escape))
        .unwrap_or(false)
    {
        next_state.set(GameState::Paused);
    }
}

fn resume_from_pause(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    if *state.get() != GameState::Paused {
        return;
    }
    if keyboard
        .as_ref()
        .map(|keys| keys.just_pressed(KeyCode::Escape))
        .unwrap_or(false)
    {
        next_state.set(GameState::InRun);
    }
}

fn tick_survival_time(time: Res<Time>, mut session: ResMut<RunSession>) {
    session.survived_seconds += time.delta_seconds();
}

fn detect_game_over(
    commanders: Query<Entity, With<CommanderUnit>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if commanders.is_empty() {
        warn!("Commander defeated; returning to MainMenu.");
        next_state.set(GameState::MainMenu);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::configure_game_app;
    use crate::model::{CommanderUnit, GameState, StartRunEvent};

    #[test]
    fn transitions_boot_to_main_menu() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        configure_game_app(&mut app);

        app.update();
        assert_eq!(
            app.world().resource::<State<GameState>>().get(),
            &GameState::MainMenu
        );
    }

    #[test]
    fn in_run_start_event_spawns_commander_without_immediate_game_over() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        configure_game_app(&mut app);

        app.update();
        assert_eq!(
            app.world().resource::<State<GameState>>().get(),
            &GameState::MainMenu
        );

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.world_mut().send_event(StartRunEvent);
        app.update();

        assert_eq!(
            app.world().resource::<State<GameState>>().get(),
            &GameState::InRun
        );

        let commander_count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<CommanderUnit>>();
            query.iter(world).count()
        };
        assert_eq!(commander_count, 1);
    }
}
