use bevy::prelude::*;

use crate::banner::BannerState;
use crate::enemies::WaveRuntime;
use crate::model::GameState;
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

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudSnapshot>().add_systems(
            Update,
            refresh_hud_snapshot.run_if(in_state(GameState::InRun)),
        );
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

#[cfg(test)]
mod tests {
    use crate::ui::HudSnapshot;

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
}
