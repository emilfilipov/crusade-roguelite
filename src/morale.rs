use bevy::prelude::*;

use crate::banner::BannerState;
use crate::model::{GameState, GlobalBuffs};
use crate::squad::SquadRoster;

#[derive(Resource, Clone, Copy, Debug)]
pub struct Cohesion {
    pub value: f32,
}

impl Default for Cohesion {
    fn default() -> Self {
        Self { value: 100.0 }
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct CohesionCombatModifiers {
    pub damage_multiplier: f32,
    pub defense_multiplier: f32,
    pub attack_speed_multiplier: f32,
    pub collapse_risk: bool,
}

impl Default for CohesionCombatModifiers {
    fn default() -> Self {
        Self {
            damage_multiplier: 1.0,
            defense_multiplier: 1.0,
            attack_speed_multiplier: 1.0,
            collapse_risk: false,
        }
    }
}

pub struct MoralePlugin;

impl Plugin for MoralePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cohesion>()
            .init_resource::<CohesionCombatModifiers>()
            .add_systems(
                Update,
                update_cohesion_and_modifiers.run_if(in_state(GameState::InRun)),
            );
    }
}

fn update_cohesion_and_modifiers(
    roster: Res<SquadRoster>,
    banner_state: Res<BannerState>,
    buffs: Option<Res<GlobalBuffs>>,
    mut cohesion: ResMut<Cohesion>,
    mut modifiers: ResMut<CohesionCombatModifiers>,
) {
    let mut next_value = 100.0 - roster.casualties as f32 * 6.0;
    if banner_state.is_dropped {
        next_value -= 20.0;
    }
    if let Some(buff) = buffs {
        next_value += buff.cohesion_bonus;
    }
    cohesion.value = next_value.clamp(0.0, 100.0);
    *modifiers = cohesion_modifiers(cohesion.value);
}

pub fn cohesion_modifiers(value: f32) -> CohesionCombatModifiers {
    if value >= 70.0 {
        CohesionCombatModifiers {
            damage_multiplier: 1.0,
            defense_multiplier: 1.0,
            attack_speed_multiplier: 1.0,
            collapse_risk: false,
        }
    } else if value >= 40.0 {
        CohesionCombatModifiers {
            damage_multiplier: 0.9,
            defense_multiplier: 0.95,
            attack_speed_multiplier: 0.95,
            collapse_risk: false,
        }
    } else if value >= 20.0 {
        CohesionCombatModifiers {
            damage_multiplier: 0.8,
            defense_multiplier: 0.9,
            attack_speed_multiplier: 0.9,
            collapse_risk: false,
        }
    } else {
        CohesionCombatModifiers {
            damage_multiplier: 0.7,
            defense_multiplier: 0.8,
            attack_speed_multiplier: 0.85,
            collapse_risk: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::morale::cohesion_modifiers;

    #[test]
    fn high_cohesion_has_no_penalty() {
        let modifiers = cohesion_modifiers(90.0);
        assert_eq!(modifiers.damage_multiplier, 1.0);
        assert!(!modifiers.collapse_risk);
    }

    #[test]
    fn low_cohesion_triggers_collapse_risk() {
        let modifiers = cohesion_modifiers(10.0);
        assert!(modifiers.collapse_risk);
        assert!(modifiers.damage_multiplier < 1.0);
    }
}
