use bevy::prelude::*;

use crate::data::{GameData, UpgradeConfig};
use crate::model::{
    BaseMaxHealth, CommanderUnit, FriendlyUnit, GainXpEvent, GameState, GlobalBuffs, Health,
    RecruitEvent, StartRunEvent,
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
            next_level_xp: xp_required_for_level(1),
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct UpgradeDraft {
    pub active: bool,
    pub options: Vec<UpgradeConfig>,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct SelectUpgradeEvent {
    pub option_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpgradeCardIcon {
    Recruit,
    Armor,
    Damage,
    AttackSpeed,
    Cohesion,
    Aura,
}

#[derive(Resource, Clone, Copy, Debug)]
struct LevelPassiveRuntime {
    applied_level: u32,
}

impl Default for LevelPassiveRuntime {
    fn default() -> Self {
        Self { applied_level: 1 }
    }
}

pub struct UpgradePlugin;

impl Plugin for UpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progression>()
            .init_resource::<UpgradeDraft>()
            .init_resource::<LevelPassiveRuntime>()
            .init_resource::<GlobalBuffs>()
            .add_event::<SelectUpgradeEvent>()
            .add_systems(Update, reset_progress_on_run_start)
            .add_systems(
                Update,
                (
                    gain_xp,
                    open_draft_on_level_up,
                    sync_friendly_level_health_caps,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (
                    queue_upgrade_selection_from_keyboard,
                    resolve_upgrade_selection,
                )
                    .chain()
                    .run_if(in_state(GameState::LevelUp)),
            );
    }
}

fn reset_progress_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut progression: ResMut<Progression>,
    mut draft: ResMut<UpgradeDraft>,
    mut passive_runtime: ResMut<LevelPassiveRuntime>,
    mut buffs: ResMut<GlobalBuffs>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *progression = Progression::default();
    *draft = UpgradeDraft::default();
    *passive_runtime = LevelPassiveRuntime::default();
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
        progression.next_level_xp = xp_required_for_level(progression.level);
        draft.options = roll_upgrade_options(&data.upgrades.upgrades, progression.level);
        draft.active = true;
        next_state.set(GameState::LevelUp);
    }
}

pub fn xp_required_for_level(level: u32) -> f32 {
    const BASE_REQUIREMENT: f32 = 30.0;
    const BRACKET_SIZE: u32 = 10;
    const BRACKET_GROWTH: f32 = 5.5;
    const INTRA_BRACKET_GROWTH: f32 = 1.18;

    let safe_level = level.max(1);
    let index = safe_level - 1;
    let bracket = index / BRACKET_SIZE;
    let within_bracket = index % BRACKET_SIZE;

    BASE_REQUIREMENT
        * BRACKET_GROWTH.powf(bracket as f32)
        * INTRA_BRACKET_GROWTH.powf(within_bracket as f32)
}

pub fn commander_level_hp_bonus(level: u32) -> f32 {
    level.saturating_sub(1) as f32
}

fn sync_friendly_level_health_caps(
    progression: Res<Progression>,
    mut passive_runtime: ResMut<LevelPassiveRuntime>,
    mut friendlies: Query<(&mut Health, &BaseMaxHealth), With<FriendlyUnit>>,
) {
    let level = progression.level.max(1);
    let hp_bonus = commander_level_hp_bonus(level);
    let leveled_up = level > passive_runtime.applied_level;
    for (mut health, base_max) in &mut friendlies {
        let expected_max = base_max.0 + hp_bonus;
        let previous_max = health.max;
        if (health.max - expected_max).abs() > 0.001 {
            health.max = expected_max;
        }
        if leveled_up || health.max > previous_max {
            health.current = health.max;
        }
        if health.current > health.max {
            health.current = health.max;
        }
    }
    passive_runtime.applied_level = passive_runtime.applied_level.max(level);
}

fn queue_upgrade_selection_from_keyboard(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    draft: Res<UpgradeDraft>,
    mut selection_events: EventWriter<SelectUpgradeEvent>,
) {
    if !draft.active || draft.options.is_empty() {
        return;
    }
    let Some(keys) = keyboard else {
        return;
    };
    let mut selected_idx = None;
    if keys.just_pressed(KeyCode::Digit1) {
        selected_idx = Some(0);
    } else if keys.just_pressed(KeyCode::Digit2) {
        selected_idx = Some(1);
    } else if keys.just_pressed(KeyCode::Digit3) {
        selected_idx = Some(2);
    } else if keys.just_pressed(KeyCode::Digit4) {
        selected_idx = Some(3);
    } else if keys.just_pressed(KeyCode::Digit5) {
        selected_idx = Some(4);
    }
    if let Some(option_index) = selected_idx {
        selection_events.send(SelectUpgradeEvent { option_index });
    }
}

fn resolve_upgrade_selection(
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut draft: ResMut<UpgradeDraft>,
    mut selection_events: EventReader<SelectUpgradeEvent>,
    mut buffs: ResMut<GlobalBuffs>,
    mut recruit_events: EventWriter<RecruitEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !draft.active || draft.options.is_empty() {
        return;
    }

    let Some(selection) = selection_events.read().last().copied() else {
        return;
    };
    let index = selection.option_index.min(draft.options.len() - 1);
    let picked = draft.options[index].clone();
    apply_upgrade(&picked, &mut buffs, &commanders, &mut recruit_events);
    draft.active = false;
    draft.options.clear();
    next_state.set(GameState::InRun);
}

pub fn roll_upgrade_options(pool: &[UpgradeConfig], level: u32) -> Vec<UpgradeConfig> {
    if pool.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::with_capacity(5);
    let offset = (level as usize) % pool.len();
    for idx in 0..5 {
        result.push(pool[(offset + idx) % pool.len()].clone());
    }
    result
}

pub fn upgrade_card_icon(upgrade: &UpgradeConfig) -> UpgradeCardIcon {
    match upgrade.kind.as_str() {
        "add_units" => UpgradeCardIcon::Recruit,
        "armor" => UpgradeCardIcon::Armor,
        "damage" => UpgradeCardIcon::Damage,
        "attack_speed" => UpgradeCardIcon::AttackSpeed,
        "cohesion" => UpgradeCardIcon::Cohesion,
        "commander_aura" => UpgradeCardIcon::Aura,
        _ => UpgradeCardIcon::Recruit,
    }
}

pub fn upgrade_display_title(upgrade: &UpgradeConfig) -> &'static str {
    match upgrade.kind.as_str() {
        "add_units" => "Reinforcements",
        "armor" => "Hardened Armor",
        "damage" => "Sharpened Steel",
        "attack_speed" => "Rapid Drill",
        "cohesion" => "Tight Formation",
        "commander_aura" => "Command Presence",
        _ => "Field Upgrade",
    }
}

pub fn upgrade_display_description(upgrade: &UpgradeConfig) -> String {
    match upgrade.kind.as_str() {
        "add_units" => "Recruit 1 infantry knight into your retinue.".to_string(),
        "armor" => format!("Add +{:.0} armor to the army.", upgrade.value),
        "damage" => format!("Increase army damage by +{:.1}%.", upgrade.value),
        "attack_speed" => format!(
            "Increase army attack speed by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "cohesion" => format!("Increase cohesion reserve by +{:.0}.", upgrade.value),
        "commander_aura" => format!("Increase commander aura strength by +{:.0}.", upgrade.value),
        _ => "Apply a battlefield improvement.".to_string(),
    }
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
    use crate::upgrades::{
        UpgradeCardIcon, commander_level_hp_bonus, roll_upgrade_options, upgrade_card_icon,
        upgrade_display_description, upgrade_display_title, xp_required_for_level,
    };

    #[test]
    fn rolls_five_options() {
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
        assert_eq!(picks.len(), 5);
    }

    #[test]
    fn buffs_stack_for_repeat_upgrades() {
        let mut buffs = GlobalBuffs::default();
        buffs.damage_multiplier += 0.05;
        buffs.damage_multiplier += 0.05;
        assert!((buffs.damage_multiplier - 1.10).abs() < 0.001);
    }

    #[test]
    fn xp_requirements_increase_each_level() {
        assert!((xp_required_for_level(1) - 30.0).abs() < 0.001);
        assert!(xp_required_for_level(2) > xp_required_for_level(1));
        assert!(xp_required_for_level(5) > xp_required_for_level(4));
        assert!(xp_required_for_level(11) > xp_required_for_level(10));
        assert!(xp_required_for_level(21) > xp_required_for_level(20));
    }

    #[test]
    fn commander_level_hp_bonus_increases_linearly() {
        assert_eq!(commander_level_hp_bonus(1), 0.0);
        assert_eq!(commander_level_hp_bonus(6), 5.0);
    }

    #[test]
    fn upgrade_display_metadata_maps_known_kinds() {
        let upgrade = UpgradeConfig {
            id: "damage_up".to_string(),
            kind: "damage".to_string(),
            value: 1.5,
        };
        assert_eq!(upgrade_card_icon(&upgrade), UpgradeCardIcon::Damage);
        assert_eq!(upgrade_display_title(&upgrade), "Sharpened Steel");
        assert!(upgrade_display_description(&upgrade).contains("damage"));
    }
}
