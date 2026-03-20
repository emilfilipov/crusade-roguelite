use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;

use crate::data::{GameData, UpgradeConfig};
use crate::formation::{ActiveFormation, FormationSkillBar};
use crate::model::{
    BaseMaxHealth, FriendlyUnit, GainXpEvent, GameState, GlobalBuffs, Health, MAX_COMMANDER_LEVEL,
    StartRunEvent,
};
use crate::squad::RosterEconomy;

const LEVEL_UP_OPTION_COUNT: usize = 3;
const DEFAULT_UPGRADE_WEIGHT_EXPONENT: f32 = 2.0;
const AUTHORITY_ENEMY_MORALE_DRAIN_SCALE: f32 = 6.0;
const HOSPITALIER_COHESION_REGEN_SCALE: f32 = 0.35;
const HOSPITALIER_MORALE_REGEN_SCALE: f32 = 0.18;

const MOB_FURY_DAMAGE_BONUS: f32 = 0.18;
const MOB_FURY_ATTACK_SPEED_BONUS: f32 = 0.18;
const MOB_FURY_MOVE_SPEED_BONUS: f32 = 24.0;
const MOB_JUSTICE_EXECUTE_THRESHOLD: f32 = 0.10;
const MOB_MERCY_RESCUE_TIME_MULTIPLIER: f32 = 0.5;

#[derive(Resource, Clone, Debug)]
pub struct Progression {
    pub xp: f32,
    pub level: u32,
    pub next_level_xp: f32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ProgressionLockFeedback {
    pub message: Option<String>,
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

#[derive(Resource, Clone, Debug, Default)]
pub struct OneTimeUpgradeTracker {
    pub acquired_ids: HashSet<String>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct SkillBookLog {
    pub entries: Vec<SkillBookEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SkillBookEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub icon: UpgradeCardIcon,
    pub stacks: u32,
    pub one_time: bool,
    pub adds_to_skillbar: bool,
    pub formation_id: Option<String>,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct MobUpgradeOwnership {
    pub fury_owned: bool,
    pub fury_required_tier0_share: f32,
    pub justice_owned: bool,
    pub justice_required_tier0_share: f32,
    pub mercy_owned: bool,
    pub mercy_required_tier0_share: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct ConditionalUpgradeEffects {
    pub friendly_damage_multiplier: f32,
    pub friendly_attack_speed_multiplier: f32,
    pub friendly_move_speed_bonus: f32,
    pub friendly_loss_immunity: bool,
    pub execute_below_health_ratio: f32,
    pub rescue_time_multiplier: f32,
}

impl Default for ConditionalUpgradeEffects {
    fn default() -> Self {
        Self {
            friendly_damage_multiplier: 1.0,
            friendly_attack_speed_multiplier: 1.0,
            friendly_move_speed_bonus: 0.0,
            friendly_loss_immunity: false,
            execute_below_health_ratio: 0.0,
            rescue_time_multiplier: 1.0,
        }
    }
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
    FormationSquare,
    FormationDiamond,
    MobFury,
    MobJustice,
    MobMercy,
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
            .init_resource::<ProgressionLockFeedback>()
            .init_resource::<UpgradeDraft>()
            .init_resource::<OneTimeUpgradeTracker>()
            .init_resource::<SkillBookLog>()
            .init_resource::<MobUpgradeOwnership>()
            .init_resource::<ConditionalUpgradeEffects>()
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
                    refresh_conditional_upgrade_effects,
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

#[allow(clippy::too_many_arguments)]
fn reset_progress_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut progression: ResMut<Progression>,
    mut lock_feedback: ResMut<ProgressionLockFeedback>,
    mut draft: ResMut<UpgradeDraft>,
    mut one_time_tracker: ResMut<OneTimeUpgradeTracker>,
    mut skill_book: ResMut<SkillBookLog>,
    mut mob_owned: ResMut<MobUpgradeOwnership>,
    mut conditional_effects: ResMut<ConditionalUpgradeEffects>,
    mut passive_runtime: ResMut<LevelPassiveRuntime>,
    mut buffs: ResMut<GlobalBuffs>,
    mut rng: ResMut<UpgradeRngState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *progression = Progression::default();
    *lock_feedback = ProgressionLockFeedback::default();
    *draft = UpgradeDraft::default();
    *one_time_tracker = OneTimeUpgradeTracker::default();
    *skill_book = SkillBookLog::default();
    *mob_owned = MobUpgradeOwnership::default();
    *conditional_effects = ConditionalUpgradeEffects::default();
    *passive_runtime = LevelPassiveRuntime::default();
    *buffs = GlobalBuffs::default();
    rng.reseed_from_time();
}

fn gain_xp(mut progression: ResMut<Progression>, mut xp_events: EventReader<GainXpEvent>) {
    for event in xp_events.read() {
        progression.xp += event.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn open_draft_on_level_up(
    mut progression: ResMut<Progression>,
    mut lock_feedback: ResMut<ProgressionLockFeedback>,
    mut draft: ResMut<UpgradeDraft>,
    one_time_tracker: Res<OneTimeUpgradeTracker>,
    skillbar: Res<FormationSkillBar>,
    roster_economy: Option<Res<RosterEconomy>>,
    data: Res<GameData>,
    mut rng: ResMut<UpgradeRngState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if draft.active {
        return;
    }

    let allowed_max_level = roster_economy
        .as_deref()
        .map(|value| value.allowed_max_level)
        .unwrap_or(MAX_COMMANDER_LEVEL)
        .min(MAX_COMMANDER_LEVEL);
    lock_feedback.message = progression_lock_reason(progression.level, allowed_max_level);
    if progression.level >= allowed_max_level {
        progression.level = allowed_max_level;
        return;
    }

    if progression.level >= MAX_COMMANDER_LEVEL {
        progression.level = MAX_COMMANDER_LEVEL;
        lock_feedback.message = None;
        return;
    }

    if progression.xp >= progression.next_level_xp {
        progression.level += 1;
        progression.level = progression
            .level
            .min(allowed_max_level)
            .min(MAX_COMMANDER_LEVEL);
        progression.xp -= progression.next_level_xp;
        progression.next_level_xp = xp_required_for_level(progression.level);
        draft.options = roll_upgrade_options(
            &data.upgrades.upgrades,
            &mut rng,
            LEVEL_UP_OPTION_COUNT,
            &one_time_tracker,
            &skillbar,
        );
        draft.active = !draft.options.is_empty();
        if draft.active {
            next_state.set(GameState::LevelUp);
        }
    }
}

pub fn progression_lock_reason(level: u32, allowed_max_level: u32) -> Option<String> {
    if allowed_max_level < MAX_COMMANDER_LEVEL && level >= allowed_max_level {
        Some(format!(
            "Level progression locked at {}/{} because roster unit costs are consuming level budget.",
            allowed_max_level, MAX_COMMANDER_LEVEL
        ))
    } else {
        None
    }
}

pub fn xp_required_for_level(level: u32) -> f32 {
    if level >= MAX_COMMANDER_LEVEL {
        return f32::INFINITY;
    }

    const BASE_REQUIREMENT: f32 = 30.0;
    const BRACKET_SIZE: u32 = 10;
    const BRACKET_GROWTH: f32 = 6.0;
    const INTRA_BRACKET_GROWTH: f32 = 1.2;

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

#[allow(clippy::too_many_arguments)]
fn resolve_upgrade_selection(
    mut draft: ResMut<UpgradeDraft>,
    mut selection_events: EventReader<SelectUpgradeEvent>,
    mut buffs: ResMut<GlobalBuffs>,
    mut skill_book: ResMut<SkillBookLog>,
    mut mob_owned: ResMut<MobUpgradeOwnership>,
    mut one_time_tracker: ResMut<OneTimeUpgradeTracker>,
    mut skillbar: ResMut<FormationSkillBar>,
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
    apply_upgrade(&picked, &mut buffs, &mut mob_owned, &mut skillbar);
    record_skill_book_entry(&mut skill_book, &picked);
    if picked.one_time {
        one_time_tracker.acquired_ids.insert(picked.id.clone());
    }
    draft.active = false;
    draft.options.clear();
    next_state.set(GameState::InRun);
}

fn record_skill_book_entry(skill_book: &mut SkillBookLog, picked: &UpgradeConfig) {
    if let Some(entry) = skill_book
        .entries
        .iter_mut()
        .find(|entry| entry.id == picked.id)
    {
        entry.stacks += 1;
        entry.description = upgrade_display_description(picked);
        return;
    }
    skill_book.entries.push(SkillBookEntry {
        id: picked.id.clone(),
        kind: picked.kind.clone(),
        title: upgrade_display_title(picked).to_string(),
        description: upgrade_display_description(picked),
        icon: upgrade_card_icon(picked),
        stacks: 1,
        one_time: picked.one_time,
        adds_to_skillbar: picked.adds_to_skillbar,
        formation_id: picked.formation_id.clone(),
    });
}

fn roll_upgrade_options(
    pool: &[UpgradeConfig],
    rng: &mut UpgradeRngState,
    count: usize,
    one_time_tracker: &OneTimeUpgradeTracker,
    skillbar: &FormationSkillBar,
) -> Vec<UpgradeConfig> {
    if pool.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut candidate_indices: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, upgrade)| {
            if upgrade.one_time && one_time_tracker.acquired_ids.contains(&upgrade.id) {
                return false;
            }
            if upgrade.adds_to_skillbar
                && (skillbar.is_full() || skillbar_contains_upgrade(skillbar, upgrade))
            {
                return false;
            }
            true
        })
        .map(|(index, _)| index)
        .collect();
    if candidate_indices.is_empty() {
        return Vec::new();
    }
    let pick_count = count.min(candidate_indices.len());
    let indices = draw_unique_indices(&mut candidate_indices, pick_count, rng);
    indices
        .into_iter()
        .map(|idx| {
            let mut rolled = pool[idx].clone();
            rolled.value = roll_upgrade_value(&rolled, rng);
            rolled
        })
        .collect()
}

fn draw_unique_indices(
    candidate_indices: &mut [usize],
    count: usize,
    rng: &mut UpgradeRngState,
) -> Vec<usize> {
    let pool_len = candidate_indices.len();
    for i in 0..count {
        let remaining = pool_len - i;
        let offset = (rng.next_u32() as usize) % remaining;
        let j = i + offset;
        candidate_indices.swap(i, j);
    }
    candidate_indices[..count].to_vec()
}

fn skillbar_contains_upgrade(skillbar: &FormationSkillBar, upgrade: &UpgradeConfig) -> bool {
    if upgrade.kind != "unlock_formation" {
        return false;
    }
    let Some(formation_id) = upgrade.formation_id.as_deref() else {
        return false;
    };
    let Some(formation) = ActiveFormation::from_id(formation_id) else {
        return false;
    };
    skillbar.has_formation(formation)
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
        "mob_fury" => UpgradeCardIcon::MobFury,
        "mob_justice" => UpgradeCardIcon::MobJustice,
        "mob_mercy" => UpgradeCardIcon::MobMercy,
        "unlock_formation" => match upgrade.formation_id.as_deref() {
            Some("diamond") => UpgradeCardIcon::FormationDiamond,
            _ => UpgradeCardIcon::FormationSquare,
        },
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
        "mob_fury" => "Mob's Fury",
        "mob_justice" => "Mob's Justice",
        "mob_mercy" => "Mob's Mercy",
        "unlock_formation" => match upgrade.formation_id.as_deref() {
            Some("diamond") => "Diamond Formation",
            Some("square") => "Square Formation",
            _ => "Formation Unlock",
        },
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
        "mob_fury" => "If tier-0 share requirement is met, friendlies become immune to morale/cohesion loss and gain bonus damage, attack speed, and movement speed.".to_string(),
        "mob_justice" => "If tier-0 share requirement is met, hits execute enemies below 10% HP.".to_string(),
        "mob_mercy" => "If tier-0 share requirement is met, rescue channel time is reduced by 50%.".to_string(),
        "unlock_formation" => match upgrade.formation_id.as_deref() {
            Some("diamond") => "Unlock Diamond formation on the skill bar: offense bonus while moving, faster movement, lower defense.".to_string(),
            Some("square") => "Unlock Square formation on the skill bar.".to_string(),
            _ => "Unlock a formation skill for the skill bar.".to_string(),
        },
        _ => "Apply a battlefield improvement.".to_string(),
    }
}

fn apply_upgrade(
    upgrade: &UpgradeConfig,
    buffs: &mut GlobalBuffs,
    mob_owned: &mut MobUpgradeOwnership,
    skillbar: &mut FormationSkillBar,
) {
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
        "unlock_formation" => {
            if let Some(formation) = upgrade
                .formation_id
                .as_deref()
                .and_then(ActiveFormation::from_id)
            {
                skillbar.try_add_formation(formation);
            }
        }
        "mob_fury" => {
            mob_owned.fury_owned = true;
            mob_owned.fury_required_tier0_share = mob_tier0_requirement(upgrade);
        }
        "mob_justice" => {
            mob_owned.justice_owned = true;
            mob_owned.justice_required_tier0_share = mob_tier0_requirement(upgrade);
        }
        "mob_mercy" => {
            mob_owned.mercy_owned = true;
            mob_owned.mercy_required_tier0_share = mob_tier0_requirement(upgrade);
        }
        _ => {}
    }
}

fn refresh_conditional_upgrade_effects(
    mob_owned: Res<MobUpgradeOwnership>,
    roster: Option<Res<RosterEconomy>>,
    mut effects: ResMut<ConditionalUpgradeEffects>,
) {
    let tier0_share = roster
        .as_deref()
        .map(roster_tier0_share)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let fury_active = mob_owned.fury_owned && tier0_share >= mob_owned.fury_required_tier0_share;
    let justice_active =
        mob_owned.justice_owned && tier0_share >= mob_owned.justice_required_tier0_share;
    let mercy_active = mob_owned.mercy_owned && tier0_share >= mob_owned.mercy_required_tier0_share;

    effects.friendly_damage_multiplier = if fury_active {
        1.0 + MOB_FURY_DAMAGE_BONUS
    } else {
        1.0
    };
    effects.friendly_attack_speed_multiplier = if fury_active {
        1.0 + MOB_FURY_ATTACK_SPEED_BONUS
    } else {
        1.0
    };
    effects.friendly_move_speed_bonus = if fury_active {
        MOB_FURY_MOVE_SPEED_BONUS
    } else {
        0.0
    };
    effects.friendly_loss_immunity = fury_active;
    effects.execute_below_health_ratio = if justice_active {
        MOB_JUSTICE_EXECUTE_THRESHOLD
    } else {
        0.0
    };
    effects.rescue_time_multiplier = if mercy_active {
        MOB_MERCY_RESCUE_TIME_MULTIPLIER
    } else {
        1.0
    };
}

fn roster_tier0_share(roster: &RosterEconomy) -> f32 {
    if roster.total_retinue_count == 0 {
        return 0.0;
    }
    roster.tier0_retinue_count as f32 / roster.total_retinue_count as f32
}

fn mob_tier0_requirement(upgrade: &UpgradeConfig) -> f32 {
    if upgrade.requirement_type.as_deref() == Some("tier0_share") {
        upgrade
            .requirement_min_tier0_share
            .unwrap_or(1.0)
            .clamp(0.0, 1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::UpgradeConfig;
    use crate::formation::{
        ActiveFormation, FormationSkillBar, SKILL_BAR_CAPACITY, SkillBarSkill, SkillBarSkillKind,
    };
    use crate::model::GlobalBuffs;
    use crate::upgrades::{
        OneTimeUpgradeTracker, SkillBookLog, UpgradeCardIcon, UpgradeRngState,
        commander_level_hp_bonus, progression_lock_reason, roll_upgrade_options,
        roll_upgrade_value, upgrade_card_icon, upgrade_display_description, upgrade_display_title,
        xp_required_for_level,
    };

    fn upgrade(kind: &str, id: &str) -> UpgradeConfig {
        UpgradeConfig {
            id: id.to_string(),
            kind: kind.to_string(),
            value: 1.0,
            min_value: Some(1.0),
            max_value: Some(4.0),
            value_step: Some(0.5),
            weight_exponent: Some(2.0),
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
            requirement_type: None,
            requirement_min_tier0_share: None,
        }
    }

    #[test]
    fn rolls_three_unique_options() {
        let pool = vec![
            upgrade("damage", "a"),
            upgrade("armor", "b"),
            upgrade("attack_speed", "c"),
            upgrade("pickup_radius", "d"),
        ];
        let mut rng = UpgradeRngState {
            state: 0x1234_5678_9ABC_DEF0,
        };
        let picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            &OneTimeUpgradeTracker::default(),
            &FormationSkillBar::default(),
        );
        assert_eq!(picks.len(), 3);
        let ids: HashSet<String> = picks.iter().map(|picked| picked.id.clone()).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn weighted_roll_stays_in_min_max_bounds() {
        let upgrade = upgrade("damage", "damage");
        let mut rng = UpgradeRngState {
            state: 0xCAFE_BABE_0123_4567,
        };
        for _ in 0..50 {
            let value = roll_upgrade_value(&upgrade, &mut rng);
            assert!((1.0..=4.0).contains(&value));
        }
    }

    #[test]
    fn one_time_upgrade_is_removed_after_pick() {
        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            min_value: None,
            max_value: None,
            value_step: None,
            weight_exponent: None,
            one_time: true,
            adds_to_skillbar: true,
            formation_id: Some("diamond".to_string()),
            requirement_type: None,
            requirement_min_tier0_share: None,
        };
        let pool = vec![formation_upgrade];
        let mut tracker = OneTimeUpgradeTracker::default();
        tracker.acquired_ids.insert("unlock_diamond".to_string());
        let mut rng = UpgradeRngState {
            state: 0xBEEF_1234_9876_1111,
        };
        let picks =
            roll_upgrade_options(&pool, &mut rng, 3, &tracker, &FormationSkillBar::default());
        assert!(picks.is_empty());
    }

    #[test]
    fn skillbar_bound_upgrades_are_filtered_when_hotbar_is_full() {
        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            min_value: None,
            max_value: None,
            value_step: None,
            weight_exponent: None,
            one_time: true,
            adds_to_skillbar: true,
            formation_id: Some("diamond".to_string()),
            requirement_type: None,
            requirement_min_tier0_share: None,
        };
        let normal_upgrade = upgrade("damage", "damage_up");
        let pool = vec![formation_upgrade, normal_upgrade];

        let mut full_skillbar = FormationSkillBar {
            slots: Vec::new(),
            active_slot: Some(0),
        };
        full_skillbar.slots = (0..SKILL_BAR_CAPACITY)
            .map(|idx| SkillBarSkill {
                id: format!("slot_{idx}"),
                label: format!("Slot {idx}"),
                kind: SkillBarSkillKind::Formation(ActiveFormation::Square),
            })
            .collect();
        assert!(full_skillbar.is_full());

        let mut rng = UpgradeRngState {
            state: 0x1357_9BDF_2468_ACE0,
        };
        let picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            &OneTimeUpgradeTracker::default(),
            &full_skillbar,
        );
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].id, "damage_up");
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
    fn progression_lock_reason_engages_and_clears_with_budget() {
        let locked = progression_lock_reason(199, 199);
        assert!(locked.is_some());

        let unlocked = progression_lock_reason(120, 199);
        assert!(unlocked.is_none());

        let hard_cap_only = progression_lock_reason(200, 200);
        assert!(hard_cap_only.is_none());
    }

    #[test]
    fn upgrade_display_metadata_maps_known_kinds() {
        let damage_upgrade = upgrade("damage", "damage_up");
        assert_eq!(upgrade_card_icon(&damage_upgrade), UpgradeCardIcon::Damage);
        assert_eq!(upgrade_display_title(&damage_upgrade), "Sharpened Steel");
        assert!(upgrade_display_description(&damage_upgrade).contains("damage"));

        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            min_value: None,
            max_value: None,
            value_step: None,
            weight_exponent: None,
            one_time: true,
            adds_to_skillbar: true,
            formation_id: Some("diamond".to_string()),
            requirement_type: None,
            requirement_min_tier0_share: None,
        };
        assert_eq!(
            upgrade_card_icon(&formation_upgrade),
            UpgradeCardIcon::FormationDiamond
        );
        assert_eq!(
            upgrade_display_title(&formation_upgrade),
            "Diamond Formation"
        );
        assert!(upgrade_display_description(&formation_upgrade).contains("Diamond"));
    }

    #[test]
    fn skill_book_records_upgrade_stacks_once_per_upgrade_id() {
        let mut skill_book = SkillBookLog::default();
        let picked = upgrade("damage", "damage_alpha");
        super::record_skill_book_entry(&mut skill_book, &picked);
        super::record_skill_book_entry(&mut skill_book, &picked);

        assert_eq!(skill_book.entries.len(), 1);
        assert_eq!(skill_book.entries[0].id, "damage_alpha");
        assert_eq!(skill_book.entries[0].stacks, 2);
    }
}
