use bevy::prelude::*;
use bevy::time::Virtual;

use crate::banner::BannerMarker;
use crate::drops::ExpPack;
use crate::enemies::{WaveRuntime, should_trigger_victory};
use crate::model::EnemyUnit;
use crate::model::{
    CommanderUnit, GameState, RunModalAction, RunModalRequestEvent, RunModalScreen, RunModalState,
    RunSession, Unit,
};
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
            .add_systems(OnExit(GameState::InRun), clear_run_modal_state)
            .add_systems(
                Update,
                (
                    dispatch_run_modal_hotkeys,
                    handle_run_modal_requests,
                    sync_virtual_time_pause,
                )
                    .chain(),
            )
            .add_systems(
                PostUpdate,
                (detect_victory, detect_game_over)
                    .chain()
                    .run_if(in_state(GameState::InRun)),
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

fn dispatch_run_modal_hotkeys(
    state: Res<State<GameState>>,
    modal_state: Res<RunModalState>,
    mut modal_requests: EventWriter<RunModalRequestEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    if *state.get() != GameState::InRun {
        return;
    }
    let Some(keys) = keyboard.as_ref() else {
        return;
    };
    if keys.just_pressed(KeyCode::Escape) {
        if modal_state.is_open() {
            modal_requests.send(RunModalRequestEvent {
                action: RunModalAction::Close,
            });
        } else {
            next_state.set(GameState::Paused);
        }
    }
    if keys.just_pressed(KeyCode::KeyI) {
        modal_requests.send(RunModalRequestEvent {
            action: RunModalAction::Toggle(RunModalScreen::Inventory),
        });
    }
    if keys.just_pressed(KeyCode::KeyO) {
        modal_requests.send(RunModalRequestEvent {
            action: RunModalAction::Toggle(RunModalScreen::Stats),
        });
    }
    if keys.just_pressed(KeyCode::KeyP) {
        modal_requests.send(RunModalRequestEvent {
            action: RunModalAction::Toggle(RunModalScreen::SkillBook),
        });
    }
    if keys.just_pressed(KeyCode::KeyK) {
        modal_requests.send(RunModalRequestEvent {
            action: RunModalAction::Toggle(RunModalScreen::Archive),
        });
    }
    if keys.just_pressed(KeyCode::KeyU) {
        modal_requests.send(RunModalRequestEvent {
            action: RunModalAction::Toggle(RunModalScreen::UnitUpgrade),
        });
    }
}

fn handle_run_modal_requests(
    state: Res<State<GameState>>,
    mut modal_state: ResMut<RunModalState>,
    mut requests: EventReader<RunModalRequestEvent>,
) {
    let can_open = *state.get() == GameState::InRun;
    let mut next = *modal_state;
    for request in requests.read() {
        next = reduce_run_modal_state(next, request.action, can_open);
    }
    *modal_state = next;
}

fn sync_virtual_time_pause(
    state: Res<State<GameState>>,
    modal_state: Res<RunModalState>,
    virtual_time: Option<ResMut<Time<Virtual>>>,
) {
    let Some(mut virtual_time) = virtual_time else {
        return;
    };
    let should_pause = *state.get() == GameState::InRun && modal_state.is_open();
    if should_pause {
        if !virtual_time.is_paused() {
            virtual_time.pause();
        }
    } else if virtual_time.is_paused() {
        virtual_time.unpause();
    }
}

fn clear_run_modal_state(mut modal_state: ResMut<RunModalState>) {
    *modal_state = RunModalState::None;
}

pub fn reduce_run_modal_state(
    current: RunModalState,
    action: RunModalAction,
    can_open: bool,
) -> RunModalState {
    match action {
        RunModalAction::Close => RunModalState::None,
        RunModalAction::Open(screen) => {
            if can_open {
                RunModalState::Open(screen)
            } else {
                current
            }
        }
        RunModalAction::Toggle(screen) => {
            if !can_open {
                return current;
            }
            match current {
                RunModalState::Open(active) if active == screen => RunModalState::None,
                _ => RunModalState::Open(screen),
            }
        }
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
        warn!("Commander defeated; entering GameOver.");
        next_state.set(GameState::GameOver);
    }
}

fn detect_victory(
    mut wave_runtime: ResMut<WaveRuntime>,
    enemies: Query<Entity, With<EnemyUnit>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if wave_runtime.victory_announced {
        return;
    }
    let alive_enemies = enemies.iter().count();
    if should_trigger_victory(&wave_runtime, alive_enemies) {
        info!("All 100 waves cleared; entering Victory.");
        wave_runtime.victory_announced = true;
        next_state.set(GameState::Victory);
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use crate::configure_game_app;
    use crate::core::reduce_run_modal_state;
    use crate::model::{
        CommanderUnit, GameState, RunModalAction, RunModalScreen, RunModalState, StartRunEvent,
    };

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

    #[test]
    fn modal_reducer_toggles_open_and_close_for_same_screen() {
        let state = reduce_run_modal_state(
            RunModalState::None,
            RunModalAction::Toggle(RunModalScreen::Inventory),
            true,
        );
        assert_eq!(state, RunModalState::Open(RunModalScreen::Inventory));

        let closed = reduce_run_modal_state(
            state,
            RunModalAction::Toggle(RunModalScreen::Inventory),
            true,
        );
        assert_eq!(closed, RunModalState::None);
    }

    #[test]
    fn modal_open_requests_are_ignored_when_opening_not_allowed() {
        let state = RunModalState::None;
        let ignored = reduce_run_modal_state(
            state,
            RunModalAction::Open(RunModalScreen::SkillBook),
            false,
        );
        assert_eq!(ignored, state);
    }

    #[test]
    fn modal_close_request_always_clears_modal_state() {
        let state = RunModalState::Open(RunModalScreen::Stats);
        let closed = reduce_run_modal_state(state, RunModalAction::Close, false);
        assert_eq!(closed, RunModalState::None);
    }

    #[test]
    fn modal_open_request_supports_all_in_run_screens() {
        let screens = [
            RunModalScreen::Inventory,
            RunModalScreen::Stats,
            RunModalScreen::SkillBook,
            RunModalScreen::Archive,
            RunModalScreen::UnitUpgrade,
        ];
        for screen in screens {
            let state =
                reduce_run_modal_state(RunModalState::None, RunModalAction::Open(screen), true);
            assert_eq!(state, RunModalState::Open(screen));
        }
    }
}
