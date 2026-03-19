use bevy::prelude::*;

use crate::model::{
    CommanderUnit, EnemyUnit, FriendlyUnit, GameState, GlobalBuffs, Morale, StartRunEvent, Team,
    UnitDamagedEvent, UnitDiedEvent, UnitKind,
};

const STARTING_COHESION: f32 = 100.0;
const LOW_MORALE_RATIO_THRESHOLD: f32 = 0.5;
const LOW_MORALE_COHESION_DRAIN_PER_SEC: f32 = 2.0;
const STABLE_COHESION_RECOVERY_PER_SEC: f32 = 0.7;
const DAMAGE_TO_MORALE_FACTOR: f32 = 0.18;
const FRIENDLY_DAMAGE_COHESION_LOSS: f32 = 0.04;
const ENEMY_KILL_COHESION_GAIN: f32 = 0.45;
const RETINUE_DEATH_COHESION_LOSS: f32 = 0.3;
const ENEMY_KILL_MORALE_GAIN: f32 = 1.2;
const ALLY_DEATH_MORALE_LOSS: f32 = 0.8;

#[derive(Resource, Clone, Copy, Debug)]
pub struct Cohesion {
    pub value: f32,
}

impl Default for Cohesion {
    fn default() -> Self {
        Self {
            value: STARTING_COHESION,
        }
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
            .add_systems(Update, reset_morale_state_on_run_start)
            .add_systems(
                Update,
                (
                    apply_morale_and_cohesion_events,
                    apply_low_morale_cohesion_pressure,
                    refresh_cohesion_modifiers,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_morale_state_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut cohesion: ResMut<Cohesion>,
    mut modifiers: ResMut<CohesionCombatModifiers>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    cohesion.value = STARTING_COHESION;
    *modifiers = cohesion_modifiers(cohesion.value);
}

#[allow(clippy::type_complexity)]
fn apply_morale_and_cohesion_events(
    mut damaged_events: EventReader<UnitDamagedEvent>,
    mut death_events: EventReader<UnitDiedEvent>,
    mut cohesion: ResMut<Cohesion>,
    mut morale_sets: ParamSet<(
        Query<&mut Morale>,
        Query<&mut Morale, With<FriendlyUnit>>,
        Query<&mut Morale, With<EnemyUnit>>,
    )>,
) {
    for event in damaged_events.read() {
        if let Ok(mut morale) = morale_sets.p0().get_mut(event.target) {
            let morale_loss = (event.amount * DAMAGE_TO_MORALE_FACTOR).max(0.2);
            morale.current = (morale.current - morale_loss).clamp(0.0, morale.max);
        }
        if event.team == Team::Friendly {
            cohesion.value -= FRIENDLY_DAMAGE_COHESION_LOSS;
        }
    }

    for event in death_events.read() {
        match event.team {
            Team::Enemy => {
                cohesion.value += ENEMY_KILL_COHESION_GAIN;
                for mut morale in &mut morale_sets.p1() {
                    morale.current =
                        (morale.current + ENEMY_KILL_MORALE_GAIN).clamp(0.0, morale.max);
                }
                for mut morale in &mut morale_sets.p2() {
                    morale.current =
                        (morale.current - ALLY_DEATH_MORALE_LOSS).clamp(0.0, morale.max);
                }
            }
            Team::Friendly => {
                if event.kind != UnitKind::Commander {
                    cohesion.value -= RETINUE_DEATH_COHESION_LOSS;
                }
                for mut morale in &mut morale_sets.p1() {
                    morale.current =
                        (morale.current - ALLY_DEATH_MORALE_LOSS).clamp(0.0, morale.max);
                }
                for mut morale in &mut morale_sets.p2() {
                    morale.current =
                        (morale.current + ENEMY_KILL_MORALE_GAIN).clamp(0.0, morale.max);
                }
            }
            Team::Neutral => {}
        }
    }

    cohesion.value = cohesion.value.clamp(0.0, 100.0);
}

fn apply_low_morale_cohesion_pressure(
    time: Res<Time>,
    buffs: Option<Res<GlobalBuffs>>,
    mut cohesion: ResMut<Cohesion>,
    retinue_morale: Query<&Morale, (With<FriendlyUnit>, Without<CommanderUnit>)>,
) {
    let ratios: Vec<f32> = retinue_morale.iter().map(|morale| morale.ratio()).collect();
    let low_ratio = low_morale_ratio(&ratios, LOW_MORALE_RATIO_THRESHOLD);
    if low_ratio >= LOW_MORALE_RATIO_THRESHOLD {
        cohesion.value -= LOW_MORALE_COHESION_DRAIN_PER_SEC * time.delta_seconds();
    } else {
        cohesion.value += STABLE_COHESION_RECOVERY_PER_SEC * time.delta_seconds();
    }

    if let Some(buff) = buffs {
        cohesion.value += buff.cohesion_bonus * 0.02 * time.delta_seconds();
    }
    cohesion.value = cohesion.value.clamp(0.0, 100.0);
}

fn refresh_cohesion_modifiers(
    cohesion: Res<Cohesion>,
    mut modifiers: ResMut<CohesionCombatModifiers>,
) {
    *modifiers = cohesion_modifiers(cohesion.value);
}

pub fn low_morale_ratio(morale_ratios: &[f32], threshold: f32) -> f32 {
    if morale_ratios.is_empty() {
        return 0.0;
    }
    let low_count = morale_ratios
        .iter()
        .filter(|ratio| **ratio < threshold)
        .count();
    low_count as f32 / morale_ratios.len() as f32
}

pub fn average_morale_ratio(morale_ratios: &[f32]) -> f32 {
    if morale_ratios.is_empty() {
        return 1.0;
    }
    morale_ratios.iter().sum::<f32>() / morale_ratios.len() as f32
}

pub fn cohesion_modifiers(value: f32) -> CohesionCombatModifiers {
    if value >= 80.0 {
        CohesionCombatModifiers {
            damage_multiplier: 1.08,
            defense_multiplier: 1.05,
            attack_speed_multiplier: 1.08,
            collapse_risk: false,
        }
    } else if value >= 60.0 {
        CohesionCombatModifiers {
            damage_multiplier: 1.0,
            defense_multiplier: 1.0,
            attack_speed_multiplier: 1.0,
            collapse_risk: false,
        }
    } else if value >= 40.0 {
        CohesionCombatModifiers {
            damage_multiplier: 0.9,
            defense_multiplier: 0.93,
            attack_speed_multiplier: 0.9,
            collapse_risk: false,
        }
    } else if value >= 20.0 {
        CohesionCombatModifiers {
            damage_multiplier: 0.8,
            defense_multiplier: 0.86,
            attack_speed_multiplier: 0.8,
            collapse_risk: false,
        }
    } else {
        CohesionCombatModifiers {
            damage_multiplier: 0.7,
            defense_multiplier: 0.8,
            attack_speed_multiplier: 0.7,
            collapse_risk: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::morale::{Cohesion, average_morale_ratio, cohesion_modifiers, low_morale_ratio};

    #[test]
    fn cohesion_starts_full() {
        assert!((Cohesion::default().value - 100.0).abs() < 0.0001);
    }

    #[test]
    fn high_cohesion_has_positive_bonus() {
        let modifiers = cohesion_modifiers(90.0);
        assert!(modifiers.damage_multiplier > 1.0);
        assert!(!modifiers.collapse_risk);
    }

    #[test]
    fn low_cohesion_triggers_collapse_risk() {
        let modifiers = cohesion_modifiers(10.0);
        assert!(modifiers.collapse_risk);
        assert!(modifiers.damage_multiplier < 1.0);
    }

    #[test]
    fn low_morale_ratio_counts_sub_threshold_members() {
        let morale = [0.9, 0.4, 0.2, 0.8];
        let ratio = low_morale_ratio(&morale, 0.5);
        assert!((ratio - 0.5).abs() < 0.0001);
    }

    #[test]
    fn average_morale_ratio_returns_mean_or_one_when_empty() {
        let morale = [0.8, 0.6, 0.4];
        assert!((average_morale_ratio(&morale) - 0.6).abs() < 0.0001);
        assert!((average_morale_ratio(&[]) - 1.0).abs() < 0.0001);
    }
}
