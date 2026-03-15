use bevy::prelude::*;

use crate::model::{CommanderUnit, GameState, RunSession, StartRunEvent};

pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Boot), boot_to_menu)
            .add_systems(
                Update,
                (
                    start_run_from_main_menu,
                    pause_toggle,
                    resume_from_pause,
                    detect_game_over,
                    restart_from_game_over,
                ),
            )
            .add_systems(
                Update,
                tick_survival_time.run_if(in_state(GameState::InRun)),
            );
    }
}

fn boot_to_menu(mut next_state: ResMut<NextState<GameState>>) {
    next_state.set(GameState::MainMenu);
}

fn start_run_from_main_menu(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut run_session: ResMut<RunSession>,
    mut start_run_events: EventWriter<StartRunEvent>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    if *state.get() != GameState::MainMenu {
        return;
    }

    let should_start = keyboard
        .as_ref()
        .map(|keys| keys.just_pressed(KeyCode::Enter))
        .unwrap_or(false);

    if should_start {
        *run_session = RunSession::default();
        next_state.set(GameState::InRun);
        start_run_events.send(StartRunEvent);
    }
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
    state: Res<State<GameState>>,
    commanders: Query<Entity, With<CommanderUnit>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if *state.get() == GameState::InRun && commanders.is_empty() {
        next_state.set(GameState::GameOver);
    }
}

fn restart_from_game_over(
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut session: ResMut<RunSession>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    if *state.get() != GameState::GameOver {
        return;
    }
    if keyboard
        .as_ref()
        .map(|keys| keys.just_pressed(KeyCode::Enter))
        .unwrap_or(false)
    {
        *session = RunSession::default();
        next_state.set(GameState::MainMenu);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::configure_game_app;
    use crate::model::GameState;

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
}
