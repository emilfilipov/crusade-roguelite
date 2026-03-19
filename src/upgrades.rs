use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

use crate::data::{GameData, UpgradeConfig};
use crate::model::{
    BaseMaxHealth, FriendlyUnit, GainXpEvent, GameState, GlobalBuffs, Health, StartRunEvent,
};

const LEVEL_UP_OPTION_COUNT: usize = 3;
const DEFAULT_UPGRADE_WEIGHT_EXPONENT: f32 = 2.0;
const AUTHORITY_ENEMY_MORALE_DRAIN_SCALE: f32 = 6.0;
const HOSPITALIER_COHESION_REGEN_SCALE: f32 = 0.35;
const HOSPITALIER_MORALE_REGEN_SCALE: f32 = 0.18;

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
    Damage,
    AttackSpeed,
    Armor,
    PickupRadius,
    AuraRadius,
    AuthorityAura,
    MoveSpeed,
    HospitalierAura,
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

#[derive(Resource, Clone, Copy, Debug)]
struct UpgradeRngState {
    state: u64,
}

impl Default for UpgradeRngState {
    fn default() -> Self {
        Self {
            state: 0xC57A_5EED_5EED_u64,
        }
    }
}

impl UpgradeRngState {
    fn reseed_from_time(&mut self) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0xBADC_0FFEE_u64);
        let mixed = nanos ^ 0x9E37_79B9_7F4A_7C15_u64;
        self.state = if mixed == 0 {
            0xC57A_5EED_5EED_u64
        } else {
            mixed
        };
    }

    fn next_u32(&mut self) -> u32 {
        // LCG parameters from Numerical Recipes with 64-bit state.
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

pub struct UpgradePlugin;

impl Plugin for UpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Progression>()
            .init_resource::<UpgradeDraft>()
            .init_resource::<LevelPassiveRuntime>()
            .init_resource::<UpgradeRngState>()
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
    mut rng: ResMut<UpgradeRngState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *progression = Progression::default();
    *draft = UpgradeDraft::default();
    *passive_runtime = LevelPassiveRuntime::default();
    *buffs = GlobalBuffs::default();
    rng.reseed_from_time();
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
    mut rng: ResMut<UpgradeRngState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if draft.active {
        return;
    }

    if progression.xp >= progression.next_level_xp {
        progression.level += 1;
        progression.xp -= progression.next_level_xp;
        progression.next_level_xp = xp_required_for_level(progression.level);
        draft.options =
            roll_upgrade_options(&data.upgrades.upgrades, &mut rng, LEVEL_UP_OPTION_COUNT);
        draft.active = !draft.options.is_empty();
        if draft.active {
            next_state.set(GameState::LevelUp);
        }
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
    }
    if let Some(option_index) = selected_idx {
        selection_events.send(SelectUpgradeEvent { option_index });
    }
}

fn resolve_upgrade_selection(
    mut draft: ResMut<UpgradeDraft>,
    mut selection_events: EventReader<SelectUpgradeEvent>,
    mut buffs: ResMut<GlobalBuffs>,
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
    apply_upgrade(&picked, &mut buffs);
    draft.active = false;
    draft.options.clear();
    next_state.set(GameState::InRun);
}

fn roll_upgrade_options(
    pool: &[UpgradeConfig],
    rng: &mut UpgradeRngState,
    count: usize,
) -> Vec<UpgradeConfig> {
    if pool.is_empty() || count == 0 {
        return Vec::new();
    }
    let pick_count = count.min(pool.len());
    let indices = draw_unique_indices(pool.len(), pick_count, rng);
    indices
        .into_iter()
        .map(|idx| {
            let mut rolled = pool[idx].clone();
            rolled.value = roll_upgrade_value(&rolled, rng);
            rolled
        })
        .collect()
}

fn draw_unique_indices(pool_len: usize, count: usize, rng: &mut UpgradeRngState) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..pool_len).collect();
    for i in 0..count {
        let remaining = pool_len - i;
        let offset = (rng.next_u32() as usize) % remaining;
        let j = i + offset;
        indices.swap(i, j);
    }
    indices.truncate(count);
    indices
}

fn roll_upgrade_value(config: &UpgradeConfig, rng: &mut UpgradeRngState) -> f32 {
    let min_value = config.min_value.unwrap_or(config.value);
    let max_value = config.max_value.unwrap_or(min_value);
    if (max_value - min_value).abs() <= f32::EPSILON {
        return min_value.max(0.0);
    }

    let exponent = config
        .weight_exponent
        .unwrap_or(DEFAULT_UPGRADE_WEIGHT_EXPONENT)
        .max(0.01);
    let roll = rng.next_f32().powf(exponent);
    let value = min_value + (max_value - min_value) * roll;
    quantize_to_step(value, min_value, max_value, config.value_step)
}

fn quantize_to_step(value: f32, min_value: f32, max_value: f32, step: Option<f32>) -> f32 {
    let Some(step) = step else {
        return value.clamp(min_value, max_value);
    };
    if step <= 0.0 {
        return value.clamp(min_value, max_value);
    }
    let steps = ((value - min_value) / step).round();
    (min_value + steps * step).clamp(min_value, max_value)
}

pub fn upgrade_card_icon(upgrade: &UpgradeConfig) -> UpgradeCardIcon {
    match upgrade.kind.as_str() {
        "damage" => UpgradeCardIcon::Damage,
        "attack_speed" => UpgradeCardIcon::AttackSpeed,
        "armor" => UpgradeCardIcon::Armor,
        "pickup_radius" => UpgradeCardIcon::PickupRadius,
        "aura_radius" => UpgradeCardIcon::AuraRadius,
        "authority_aura" => UpgradeCardIcon::AuthorityAura,
        "move_speed" => UpgradeCardIcon::MoveSpeed,
        "hospitalier_aura" => UpgradeCardIcon::HospitalierAura,
        _ => UpgradeCardIcon::Damage,
    }
}

pub fn upgrade_display_title(upgrade: &UpgradeConfig) -> &'static str {
    match upgrade.kind.as_str() {
        "damage" => "Sharpened Steel",
        "attack_speed" => "Rapid Drill",
        "armor" => "Hardened Armor",
        "pickup_radius" => "Supply Reach",
        "aura_radius" => "Extended Command",
        "authority_aura" => "Authority Aura",
        "move_speed" => "Forced March",
        "hospitalier_aura" => "Hospitalier Aura",
        _ => "Field Upgrade",
    }
}

pub fn upgrade_display_description(upgrade: &UpgradeConfig) -> String {
    match upgrade.kind.as_str() {
        "damage" => format!("Increase army damage by +{:.1}%.", upgrade.value),
        "attack_speed" => format!(
            "Increase army attack speed by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "armor" => format!("Add +{:.1} armor to friendlies.", upgrade.value),
        "pickup_radius" => format!("Increase pickup radius by +{:.0}.", upgrade.value),
        "aura_radius" => format!("Increase commander aura radius by +{:.0}.", upgrade.value),
        "authority_aura" => format!(
            "Friendlies in aura lose {:.0}% less morale/cohesion; enemies in aura lose {:.2} morale/s.",
            upgrade.value * 100.0,
            upgrade.value * AUTHORITY_ENEMY_MORALE_DRAIN_SCALE
        ),
        "move_speed" => format!("Increase army movement speed by +{:.0}.", upgrade.value),
        "hospitalier_aura" => format!(
            "Friendlies in aura regen +{:.1} HP/s, +{:.2} cohesion/s, +{:.2} morale/s.",
            upgrade.value,
            upgrade.value * HOSPITALIER_COHESION_REGEN_SCALE,
            upgrade.value * HOSPITALIER_MORALE_REGEN_SCALE
        ),
        _ => "Apply a battlefield improvement.".to_string(),
    }
}

fn apply_upgrade(upgrade: &UpgradeConfig, buffs: &mut GlobalBuffs) {
    match upgrade.kind.as_str() {
        "damage" => {
            buffs.damage_multiplier += upgrade.value * 0.01;
        }
        "attack_speed" => {
            buffs.attack_speed_multiplier += upgrade.value;
        }
        "armor" => {
            buffs.armor_bonus += upgrade.value;
        }
        "pickup_radius" => {
            buffs.pickup_radius_bonus += upgrade.value;
        }
        "aura_radius" => {
            buffs.commander_aura_radius_bonus += upgrade.value;
        }
        "authority_aura" => {
            buffs.authority_friendly_loss_resistance += upgrade.value;
            buffs.authority_enemy_morale_drain_per_sec +=
                upgrade.value * AUTHORITY_ENEMY_MORALE_DRAIN_SCALE;
        }
        "move_speed" => {
            buffs.move_speed_bonus += upgrade.value;
        }
        "hospitalier_aura" => {
            buffs.hospitalier_hp_regen_per_sec += upgrade.value;
            buffs.hospitalier_cohesion_regen_per_sec +=
                upgrade.value * HOSPITALIER_COHESION_REGEN_SCALE;
            buffs.hospitalier_morale_regen_per_sec +=
                upgrade.value * HOSPITALIER_MORALE_REGEN_SCALE;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::UpgradeConfig;
    use crate::model::GlobalBuffs;
    use crate::upgrades::{
        UpgradeCardIcon, UpgradeRngState, commander_level_hp_bonus, roll_upgrade_options,
        roll_upgrade_value, upgrade_card_icon, upgrade_display_description, upgrade_display_title,
        xp_required_for_level,
    };

    #[test]
    fn rolls_three_unique_options() {
        let pool = vec![
            UpgradeConfig {
                id: "a".to_string(),
                kind: "damage".to_string(),
                value: 1.0,
                min_value: Some(1.0),
                max_value: Some(4.0),
                value_step: Some(0.5),
                weight_exponent: Some(2.0),
            },
            UpgradeConfig {
                id: "b".to_string(),
                kind: "armor".to_string(),
                value: 1.0,
                min_value: Some(1.0),
                max_value: Some(4.0),
                value_step: Some(0.5),
                weight_exponent: Some(2.0),
            },
            UpgradeConfig {
                id: "c".to_string(),
                kind: "attack_speed".to_string(),
                value: 0.1,
                min_value: Some(0.01),
                max_value: Some(0.08),
                value_step: Some(0.01),
                weight_exponent: Some(2.0),
            },
            UpgradeConfig {
                id: "d".to_string(),
                kind: "pickup_radius".to_string(),
                value: 3.0,
                min_value: Some(2.0),
                max_value: Some(8.0),
                value_step: Some(1.0),
                weight_exponent: Some(2.0),
            },
        ];
        let mut rng = UpgradeRngState {
            state: 0x1234_5678_9ABC_DEF0,
        };
        let picks = roll_upgrade_options(&pool, &mut rng, 3);
        assert_eq!(picks.len(), 3);
        let ids: HashSet<String> = picks.iter().map(|upgrade| upgrade.id.clone()).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn weighted_roll_stays_in_min_max_bounds() {
        let upgrade = UpgradeConfig {
            id: "damage".to_string(),
            kind: "damage".to_string(),
            value: 1.0,
            min_value: Some(1.0),
            max_value: Some(4.0),
            value_step: Some(0.5),
            weight_exponent: Some(2.2),
        };
        let mut rng = UpgradeRngState {
            state: 0xCAFE_BABE_0123_4567,
        };
        for _ in 0..50 {
            let value = roll_upgrade_value(&upgrade, &mut rng);
            assert!((1.0..=4.0).contains(&value));
        }
    }

    #[test]
    fn buffs_stack_for_repeat_upgrades() {
        let mut buffs = GlobalBuffs::default();
        buffs.damage_multiplier += 0.05;
        buffs.damage_multiplier += 0.05;
        assert!((buffs.damage_multiplier - 1.10).abs() < 0.001);

        buffs.move_speed_bonus += 5.0;
        buffs.move_speed_bonus += 3.0;
        assert!((buffs.move_speed_bonus - 8.0).abs() < 0.001);
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
            min_value: Some(1.0),
            max_value: Some(4.0),
            value_step: Some(0.5),
            weight_exponent: Some(2.0),
        };
        assert_eq!(upgrade_card_icon(&upgrade), UpgradeCardIcon::Damage);
        assert_eq!(upgrade_display_title(&upgrade), "Sharpened Steel");
        assert!(upgrade_display_description(&upgrade).contains("damage"));
    }
}
