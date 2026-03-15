use bevy::prelude::*;

use crate::data::{GameData, UpgradeConfig};
use crate::model::{
    CommanderUnit, GainXpEvent, GameState, GlobalBuffs, RecruitEvent, StartRunEvent,
};

#[derive(Resource, Clone, Debug)]
pub struct Progression {
    pub xp: f32,
    pub level: u32,
    pub next_level_xp: f32,
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            xp: 0.0,
            level: 1,
            next_level_xp: 30.0,
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct UpgradeDraft {
    pub active: bool,
    pub options: Vec<UpgradeConfig>,
    pub autopick_timer: f32,
}

pub struct UpgradePlugin;

impl Plugin for UpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progression>()
            .init_resource::<UpgradeDraft>()
            .init_resource::<GlobalBuffs>()
            .add_systems(Update, reset_progress_on_run_start)
            .add_systems(
                Update,
                (gain_xp, open_draft_on_level_up).run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                resolve_upgrade_draft.run_if(in_state(GameState::Paused)),
            );
    }
}

fn reset_progress_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut progression: ResMut<Progression>,
    mut draft: ResMut<UpgradeDraft>,
    mut buffs: ResMut<GlobalBuffs>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *progression = Progression::default();
    *draft = UpgradeDraft::default();
    *buffs = GlobalBuffs::default();
}

fn gain_xp(mut progression: ResMut<Progression>, mut xp_events: EventReader<GainXpEvent>) {
    for event in xp_events.read() {
        progression.xp += event.0;
    }
}

fn open_draft_on_level_up(
    mut progression: ResMut<Progression>,
    mut draft: ResMut<UpgradeDraft>,
    data: Res<GameData>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if draft.active {
        return;
    }

    if progression.xp >= progression.next_level_xp {
        progression.level += 1;
        progression.xp -= progression.next_level_xp;
        progression.next_level_xp *= 1.25;
        draft.options = roll_upgrade_options(&data.upgrades.upgrades, progression.level);
        draft.active = true;
        draft.autopick_timer = 0.0;
        next_state.set(GameState::Paused);
    }
}

fn resolve_upgrade_draft(
    time: Res<Time>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut draft: ResMut<UpgradeDraft>,
    mut buffs: ResMut<GlobalBuffs>,
    mut recruit_events: EventWriter<RecruitEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !draft.active || draft.options.is_empty() {
        return;
    }

    draft.autopick_timer += time.delta_seconds();
    let mut selected_idx = None;
    if let Some(keys) = keyboard {
        if keys.just_pressed(KeyCode::Digit1) {
            selected_idx = Some(0);
        } else if keys.just_pressed(KeyCode::Digit2) {
            selected_idx = Some(1);
        } else if keys.just_pressed(KeyCode::Digit3) {
            selected_idx = Some(2);
        }
    }

    if selected_idx.is_none() && draft.autopick_timer > 0.2 {
        selected_idx = Some(0);
    }

    if let Some(index) = selected_idx {
        let picked = draft.options[index.min(draft.options.len() - 1)].clone();
        apply_upgrade(&picked, &mut buffs, &commanders, &mut recruit_events);
        draft.active = false;
        draft.options.clear();
        draft.autopick_timer = 0.0;
        next_state.set(GameState::InRun);
    }
}

pub fn roll_upgrade_options(pool: &[UpgradeConfig], level: u32) -> Vec<UpgradeConfig> {
    if pool.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(3);
    let offset = (level as usize) % pool.len();
    for idx in 0..3 {
        result.push(pool[(offset + idx) % pool.len()].clone());
    }
    result
}

fn apply_upgrade(
    upgrade: &UpgradeConfig,
    buffs: &mut GlobalBuffs,
    commanders: &Query<&Transform, With<CommanderUnit>>,
    recruit_events: &mut EventWriter<RecruitEvent>,
) {
    match upgrade.kind.as_str() {
        "add_units" => {
            let commander_pos = commanders
                .get_single()
                .map(|transform| transform.translation.truncate())
                .unwrap_or(Vec2::ZERO);
            recruit_events.send(RecruitEvent {
                world_position: commander_pos + Vec2::new(30.0, 0.0),
            });
        }
        "armor" => {
            buffs.armor_bonus += upgrade.value;
        }
        "damage" => {
            buffs.damage_multiplier += upgrade.value * 0.01;
        }
        "attack_speed" => {
            buffs.attack_speed_multiplier += upgrade.value;
        }
        "cohesion" => {
            buffs.cohesion_bonus += upgrade.value;
        }
        "commander_aura" => {
            buffs.commander_aura_bonus += upgrade.value;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::data::UpgradeConfig;
    use crate::model::GlobalBuffs;
    use crate::upgrades::roll_upgrade_options;

    #[test]
    fn rolls_three_options() {
        let pool = vec![
            UpgradeConfig {
                id: "a".to_string(),
                kind: "damage".to_string(),
                value: 1.0,
            },
            UpgradeConfig {
                id: "b".to_string(),
                kind: "armor".to_string(),
                value: 1.0,
            },
            UpgradeConfig {
                id: "c".to_string(),
                kind: "cohesion".to_string(),
                value: 1.0,
            },
            UpgradeConfig {
                id: "d".to_string(),
                kind: "attack_speed".to_string(),
                value: 0.1,
            },
        ];
        let picks = roll_upgrade_options(&pool, 2);
        assert_eq!(picks.len(), 3);
    }

    #[test]
    fn buffs_stack_for_repeat_upgrades() {
        let mut buffs = GlobalBuffs::default();
        buffs.damage_multiplier += 0.05;
        buffs.damage_multiplier += 0.05;
        assert!((buffs.damage_multiplier - 1.10).abs() < 0.001);
    }
}
