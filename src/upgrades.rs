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
const UPGRADE_VALUE_TIER_COUNT: u32 = 5;
const UPGRADE_HIGH_TIER_DROP_OFF_MULTIPLIER: f32 = 1.35;
const MAX_UNIQUE_UPGRADES: usize = 5;
const AUTHORITY_ENEMY_MORALE_DRAIN_SCALE: f32 = 10.8;
const MAX_AUTHORITY_LOSS_RESISTANCE: f32 = 0.75;
const HOSPITALIER_COHESION_REGEN_SCALE: f32 = 0.35;
const HOSPITALIER_MORALE_REGEN_SCALE: f32 = 0.18;
const XP_BASE_REQUIREMENT: f32 = 300.0;
const XP_GROWTH_PER_LEVEL: f32 = 1.061;

const MOB_FURY_DAMAGE_BONUS: f32 = 0.18;
const MOB_FURY_ATTACK_SPEED_BONUS: f32 = 0.18;
const MOB_FURY_MOVE_SPEED_BONUS: f32 = 24.0;
const MOB_JUSTICE_EXECUTE_THRESHOLD: f32 = 0.10;
const MOB_MERCY_RESCUE_TIME_MULTIPLIER: f32 = 0.5;

pub const fn max_unique_upgrades() -> usize {
    MAX_UNIQUE_UPGRADES
}

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

#[derive(Clone, Debug, PartialEq)]
pub struct SkillBookEntry {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub description: String,
    pub total_value: f32,
    pub icon: UpgradeCardIcon,
    pub stacks: u32,
    pub one_time: bool,
    pub adds_to_skillbar: bool,
    pub formation_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpgradeRequirement {
    None,
    Tier0Share { min_share: f32 },
    FormationActive { formation: ActiveFormation },
    MapTag { tag: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OwnedConditionalUpgrade {
    pub id: String,
    pub kind: String,
    pub requirement: UpgradeRequirement,
    pub stacks: u32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ConditionalUpgradeOwnership {
    pub entries: Vec<OwnedConditionalUpgrade>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalUpgradeStatusEntry {
    pub id: String,
    pub kind: String,
    pub active: bool,
    pub detail: Option<String>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ConditionalUpgradeStatus {
    pub entries: Vec<ConditionalUpgradeStatusEntry>,
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
    FastLearner,
    CritChance,
    CritDamage,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpgradeValueTier {
    Common,
    Uncommon,
    Rare,
    Epic,
    Mythical,
    Unique,
}

impl UpgradeValueTier {
    const fn from_weighted_index(index: u32) -> Self {
        match index {
            0 => Self::Common,
            1 => Self::Uncommon,
            2 => Self::Rare,
            3 => Self::Epic,
            _ => Self::Mythical,
        }
    }
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
            .init_resource::<ConditionalUpgradeOwnership>()
            .init_resource::<ConditionalUpgradeStatus>()
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
    mut conditional_ownership: ResMut<ConditionalUpgradeOwnership>,
    mut conditional_status: ResMut<ConditionalUpgradeStatus>,
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
    *conditional_ownership = ConditionalUpgradeOwnership::default();
    *conditional_status = ConditionalUpgradeStatus::default();
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
    let safe_level = level.max(1);
    let index = safe_level.saturating_sub(1);
    XP_BASE_REQUIREMENT * XP_GROWTH_PER_LEVEL.powf(index as f32)
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
    mut conditional_ownership: ResMut<ConditionalUpgradeOwnership>,
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
    apply_upgrade(
        &picked,
        &mut buffs,
        &mut conditional_ownership,
        &mut skillbar,
    );
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
        entry.total_value += picked.value;
        entry.description = upgrade_display_description(picked);
        return;
    }
    skill_book.entries.push(SkillBookEntry {
        id: picked.id.clone(),
        kind: picked.kind.clone(),
        title: upgrade_display_title(picked).to_string(),
        description: upgrade_display_description(picked),
        total_value: picked.value,
        icon: upgrade_card_icon(picked),
        stacks: 1,
        one_time: picked.one_time,
        adds_to_skillbar: picked.adds_to_skillbar,
        formation_id: picked.formation_id.clone(),
    });
}

pub fn skill_book_entry_cumulative_description(entry: &SkillBookEntry) -> String {
    match entry.kind.as_str() {
        "damage" => format!(
            "Total army damage bonus: +{:.1}%.",
            entry.total_value.max(0.0)
        ),
        "attack_speed" => format!(
            "Total army attack speed bonus: +{:.0}%.",
            (entry.total_value.max(0.0) * 100.0)
        ),
        "fast_learner" => format!(
            "Total experience gain bonus from XP packs: +{:.0}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "crit_chance" => format!(
            "Total critical hit chance: +{:.1}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "crit_damage" => format!(
            "Total critical hit damage bonus: +{:.0}% (base crit x1.20).",
            entry.total_value.max(0.0) * 100.0
        ),
        "armor" => format!(
            "Total armor bonus: +{:.1} for friendlies.",
            entry.total_value.max(0.0)
        ),
        "pickup_radius" => format!(
            "Total pickup radius bonus: +{:.0}.",
            entry.total_value.max(0.0)
        ),
        "aura_radius" => format!(
            "Total commander aura radius bonus: +{:.0}.",
            entry.total_value.max(0.0)
        ),
        "authority_aura" => format!(
            "Aura effect total: {:.0}% loss resistance for friendlies in aura and {:.2} morale/s drain for enemies in aura.",
            entry.total_value.max(0.0) * 100.0,
            entry.total_value.max(0.0) * AUTHORITY_ENEMY_MORALE_DRAIN_SCALE
        ),
        "move_speed" => format!(
            "Total army movement speed bonus: +{:.0}.",
            entry.total_value.max(0.0)
        ),
        "hospitalier_aura" => format!(
            "Aura regen totals: +{:.1} HP/s, +{:.2} cohesion/s, +{:.2} morale/s for friendlies in aura.",
            entry.total_value.max(0.0),
            entry.total_value.max(0.0) * HOSPITALIER_COHESION_REGEN_SCALE,
            entry.total_value.max(0.0) * HOSPITALIER_MORALE_REGEN_SCALE
        ),
        "formation_breach" => format!(
            "Enemies inside formation take +{:.0}% damage.",
            entry.total_value.max(0.0)
        ),
        "mob_fury" | "mob_justice" | "mob_mercy" | "unlock_formation" => entry.description.clone(),
        _ => entry.description.clone(),
    }
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
    let unique_cap_reached = one_time_tracker.acquired_ids.len() >= MAX_UNIQUE_UPGRADES;
    let mut candidate_indices: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, upgrade)| {
            if upgrade.one_time && one_time_tracker.acquired_ids.contains(&upgrade.id) {
                return false;
            }
            if unique_cap_reached && upgrade.one_time {
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
        .max(1.01)
        * UPGRADE_HIGH_TIER_DROP_OFF_MULTIPLIER;
    let tier_index = weighted_roll_tier_index(exponent, rng);
    let value = tier_value(min_value, max_value, tier_index);
    quantize_to_step(value, min_value, max_value, config.value_step)
}

fn weighted_roll_tier_index(exponent: f32, rng: &mut UpgradeRngState) -> u32 {
    let roll = rng.next_f32().powf(exponent);
    let bucket_count = UPGRADE_VALUE_TIER_COUNT as f32;
    ((roll * bucket_count).floor() as u32).min(UPGRADE_VALUE_TIER_COUNT.saturating_sub(1))
}

fn tier_value(min_value: f32, max_value: f32, tier_index: u32) -> f32 {
    if UPGRADE_VALUE_TIER_COUNT <= 1 {
        return min_value;
    }
    let clamped_index = tier_index.min(UPGRADE_VALUE_TIER_COUNT - 1);
    let normalized = clamped_index as f32 / (UPGRADE_VALUE_TIER_COUNT - 1) as f32;
    min_value + (max_value - min_value) * normalized
}

pub fn upgrade_value_tier(upgrade: &UpgradeConfig) -> UpgradeValueTier {
    if upgrade.one_time {
        return UpgradeValueTier::Unique;
    }

    let min_value = upgrade.min_value.unwrap_or(upgrade.value);
    let max_value = upgrade.max_value.unwrap_or(min_value);
    if (max_value - min_value).abs() <= f32::EPSILON {
        return UpgradeValueTier::Common;
    }

    let normalized = ((upgrade.value - min_value) / (max_value - min_value)).clamp(0.0, 1.0);
    let weighted_index =
        (normalized * (UPGRADE_VALUE_TIER_COUNT.saturating_sub(1)) as f32).round() as u32;
    UpgradeValueTier::from_weighted_index(weighted_index)
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
        "fast_learner" => UpgradeCardIcon::FastLearner,
        "crit_chance" => UpgradeCardIcon::CritChance,
        "crit_damage" => UpgradeCardIcon::CritDamage,
        "armor" => UpgradeCardIcon::Armor,
        "pickup_radius" => UpgradeCardIcon::PickupRadius,
        "aura_radius" => UpgradeCardIcon::AuraRadius,
        "authority_aura" => UpgradeCardIcon::AuthorityAura,
        "move_speed" => UpgradeCardIcon::MoveSpeed,
        "hospitalier_aura" => UpgradeCardIcon::HospitalierAura,
        "formation_breach" => UpgradeCardIcon::FormationSquare,
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
        "fast_learner" => "Fast Learner",
        "crit_chance" => "Killer Instinct",
        "crit_damage" => "Deadly Precision",
        "armor" => "Hardened Armor",
        "pickup_radius" => "Supply Reach",
        "aura_radius" => "Extended Command",
        "authority_aura" => "Authority Aura",
        "move_speed" => "Forced March",
        "hospitalier_aura" => "Hospitalier Aura",
        "formation_breach" => "Into the Wolf's Dev",
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
        "fast_learner" => format!(
            "Increase XP gained from all XP packs by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "crit_chance" => format!(
            "Increase critical hit chance by +{:.1}%.",
            upgrade.value * 100.0
        ),
        "crit_damage" => format!(
            "Increase critical hit damage by +{:.0}%.",
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
        "formation_breach" => format!(
            "Enemies inside your active formation footprint take +{:.0}% damage.",
            upgrade.value
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
    conditional_owned: &mut ConditionalUpgradeOwnership,
    skillbar: &mut FormationSkillBar,
) {
    match upgrade.kind.as_str() {
        "damage" => {
            buffs.damage_multiplier += upgrade.value * 0.01;
        }
        "attack_speed" => {
            buffs.attack_speed_multiplier += upgrade.value;
        }
        "fast_learner" => {
            buffs.xp_gain_multiplier += upgrade.value;
        }
        "crit_chance" => {
            buffs.crit_chance_bonus = (buffs.crit_chance_bonus + upgrade.value).clamp(0.0, 0.95);
        }
        "crit_damage" => {
            buffs.crit_damage_multiplier = (buffs.crit_damage_multiplier + upgrade.value).max(1.0);
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
            buffs.authority_friendly_loss_resistance = (buffs.authority_friendly_loss_resistance
                + upgrade.value)
                .clamp(0.0, MAX_AUTHORITY_LOSS_RESISTANCE);
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
        "formation_breach" => {
            buffs.inside_formation_damage_multiplier = buffs
                .inside_formation_damage_multiplier
                .max(1.0 + upgrade.value * 0.01);
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
        "mob_fury" | "mob_justice" | "mob_mercy" => {
            register_conditional_upgrade(conditional_owned, upgrade);
        }
        _ => {}
    }
}

fn register_conditional_upgrade(
    conditional_owned: &mut ConditionalUpgradeOwnership,
    upgrade: &UpgradeConfig,
) {
    let requirement = parse_upgrade_requirement(upgrade);
    if let Some(existing) = conditional_owned
        .entries
        .iter_mut()
        .find(|entry| entry.id == upgrade.id)
    {
        existing.stacks = existing.stacks.saturating_add(1);
        existing.requirement = requirement;
        return;
    }
    conditional_owned.entries.push(OwnedConditionalUpgrade {
        id: upgrade.id.clone(),
        kind: upgrade.kind.clone(),
        requirement,
        stacks: 1,
    });
}

pub fn parse_upgrade_requirement(upgrade: &UpgradeConfig) -> UpgradeRequirement {
    let Some(requirement_type) = upgrade
        .requirement_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return UpgradeRequirement::None;
    };
    match requirement_type {
        "tier0_share" => UpgradeRequirement::Tier0Share {
            min_share: upgrade
                .requirement_min_tier0_share
                .unwrap_or(1.0)
                .clamp(0.0, 1.0),
        },
        "formation_active" => upgrade
            .requirement_active_formation
            .as_deref()
            .and_then(ActiveFormation::from_id)
            .map(|formation| UpgradeRequirement::FormationActive { formation })
            .unwrap_or(UpgradeRequirement::None),
        "map_tag" => UpgradeRequirement::MapTag {
            tag: upgrade.requirement_map_tag.clone().unwrap_or_default(),
        },
        _ => UpgradeRequirement::None,
    }
}

fn refresh_conditional_upgrade_effects(
    conditional_owned: Res<ConditionalUpgradeOwnership>,
    roster: Option<Res<RosterEconomy>>,
    active_formation: Res<ActiveFormation>,
    mut effects: ResMut<ConditionalUpgradeEffects>,
    mut status: ResMut<ConditionalUpgradeStatus>,
) {
    let tier0_share = roster
        .as_deref()
        .map(roster_tier0_share)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let (next_effects, next_status) =
        conditional_effects_from_owned(&conditional_owned.entries, tier0_share, *active_formation);
    *effects = next_effects;
    status.entries = next_status;
}

fn conditional_effects_from_owned(
    entries: &[OwnedConditionalUpgrade],
    tier0_share: f32,
    active_formation: ActiveFormation,
) -> (
    ConditionalUpgradeEffects,
    Vec<ConditionalUpgradeStatusEntry>,
) {
    let mut effects = ConditionalUpgradeEffects::default();
    let mut status_entries = Vec::with_capacity(entries.len());
    let mut applied_ids = HashSet::new();

    for entry in entries {
        let (active, detail) =
            evaluate_upgrade_requirement(&entry.requirement, tier0_share, active_formation);
        status_entries.push(ConditionalUpgradeStatusEntry {
            id: entry.id.clone(),
            kind: entry.kind.clone(),
            active,
            detail,
        });
        if !active {
            continue;
        }
        if !applied_ids.insert(entry.id.clone()) {
            continue;
        }
        match entry.kind.as_str() {
            "mob_fury" => {
                effects.friendly_damage_multiplier *= 1.0 + MOB_FURY_DAMAGE_BONUS;
                effects.friendly_attack_speed_multiplier *= 1.0 + MOB_FURY_ATTACK_SPEED_BONUS;
                effects.friendly_move_speed_bonus += MOB_FURY_MOVE_SPEED_BONUS;
                effects.friendly_loss_immunity = true;
            }
            "mob_justice" => {
                effects.execute_below_health_ratio = effects
                    .execute_below_health_ratio
                    .max(MOB_JUSTICE_EXECUTE_THRESHOLD);
            }
            "mob_mercy" => {
                effects.rescue_time_multiplier = effects
                    .rescue_time_multiplier
                    .min(MOB_MERCY_RESCUE_TIME_MULTIPLIER);
            }
            _ => {}
        }
    }
    (effects, status_entries)
}

pub fn evaluate_upgrade_requirement(
    requirement: &UpgradeRequirement,
    tier0_share: f32,
    active_formation: ActiveFormation,
) -> (bool, Option<String>) {
    match requirement {
        UpgradeRequirement::None => (true, None),
        UpgradeRequirement::Tier0Share { min_share } => {
            if tier0_share >= *min_share {
                (true, None)
            } else {
                (
                    false,
                    Some(format!(
                        "Requires tier-0 share >= {:.0}% (current {:.0}%).",
                        min_share * 100.0,
                        tier0_share * 100.0
                    )),
                )
            }
        }
        UpgradeRequirement::FormationActive { formation } => {
            if *formation == active_formation {
                (true, None)
            } else {
                (
                    false,
                    Some(format!("Requires active formation '{}'.", formation.id())),
                )
            }
        }
        UpgradeRequirement::MapTag { tag } => (
            false,
            Some(format!(
                "Requires map tag '{}', which is not available in this build yet.",
                tag
            )),
        ),
    }
}

fn roster_tier0_share(roster: &RosterEconomy) -> f32 {
    if roster.total_retinue_count == 0 {
        return 0.0;
    }
    roster.tier0_retinue_count as f32 / roster.total_retinue_count as f32
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
        OneTimeUpgradeTracker, SkillBookLog, UpgradeCardIcon, UpgradeRngState, UpgradeValueTier,
        commander_level_hp_bonus, progression_lock_reason, roll_upgrade_options,
        roll_upgrade_value, upgrade_card_icon, upgrade_display_description, upgrade_display_title,
        upgrade_value_tier, xp_required_for_level,
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
            requirement_active_formation: None,
            requirement_map_tag: None,
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
    fn weighted_roll_uses_fixed_five_value_buckets() {
        let mut upgrade = upgrade("damage", "bucketed");
        upgrade.min_value = Some(10.0);
        upgrade.max_value = Some(30.0);
        upgrade.value_step = None;
        let mut rng = UpgradeRngState {
            state: 0x1234_ABCD_9876_EF01,
        };

        let mut distinct_scaled = HashSet::new();
        for _ in 0..300 {
            let value = roll_upgrade_value(&upgrade, &mut rng);
            distinct_scaled.insert((value * 1000.0).round() as i32);
        }
        assert!(distinct_scaled.len() <= super::UPGRADE_VALUE_TIER_COUNT as usize);

        let expected = [10.0, 15.0, 20.0, 25.0, 30.0];
        for scaled in distinct_scaled {
            let value = scaled as f32 / 1000.0;
            assert!(
                expected
                    .iter()
                    .any(|candidate| (value - *candidate).abs() < 0.001)
            );
        }
    }

    #[test]
    fn weighted_roll_distribution_descends_by_tier_rarity() {
        let mut upgrade = upgrade("damage", "rarity_distribution");
        upgrade.min_value = Some(10.0);
        upgrade.max_value = Some(30.0);
        upgrade.value_step = None;
        upgrade.weight_exponent = Some(2.0);

        let mut counts = [0u32; 5];
        let mut rng = UpgradeRngState {
            state: 0x9ABC_DEF0_1234_5678,
        };

        for _ in 0..20_000 {
            let value = roll_upgrade_value(&upgrade, &mut rng);
            upgrade.value = value;
            match upgrade_value_tier(&upgrade) {
                UpgradeValueTier::Common => counts[0] += 1,
                UpgradeValueTier::Uncommon => counts[1] += 1,
                UpgradeValueTier::Rare => counts[2] += 1,
                UpgradeValueTier::Epic => counts[3] += 1,
                UpgradeValueTier::Mythical => counts[4] += 1,
                UpgradeValueTier::Unique => unreachable!("weighted non-one-time roll"),
            }
        }

        assert!(counts[0] > counts[1]);
        assert!(counts[1] > counts[2]);
        assert!(counts[2] > counts[3]);
        assert!(counts[3] > counts[4]);
    }

    #[test]
    fn one_time_upgrades_are_classified_as_unique_tier() {
        let mut one_time = upgrade("mob_fury", "mob_fury");
        one_time.one_time = true;
        one_time.min_value = None;
        one_time.max_value = None;
        one_time.value = 1.0;
        assert_eq!(upgrade_value_tier(&one_time), UpgradeValueTier::Unique);
    }

    #[test]
    fn weighted_upgrade_values_map_to_named_tiers() {
        let mut weighted = upgrade("damage", "damage_tiered");
        weighted.min_value = Some(10.0);
        weighted.max_value = Some(30.0);

        weighted.value = 10.0;
        assert_eq!(upgrade_value_tier(&weighted), UpgradeValueTier::Common);
        weighted.value = 15.0;
        assert_eq!(upgrade_value_tier(&weighted), UpgradeValueTier::Uncommon);
        weighted.value = 20.0;
        assert_eq!(upgrade_value_tier(&weighted), UpgradeValueTier::Rare);
        weighted.value = 25.0;
        assert_eq!(upgrade_value_tier(&weighted), UpgradeValueTier::Epic);
        weighted.value = 30.0;
        assert_eq!(upgrade_value_tier(&weighted), UpgradeValueTier::Mythical);
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
            requirement_active_formation: None,
            requirement_map_tag: None,
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
    fn unique_upgrades_are_filtered_after_unique_cap() {
        let mut unique_a = upgrade("mob_fury", "unique_a");
        unique_a.one_time = true;
        unique_a.min_value = None;
        unique_a.max_value = None;
        unique_a.value = 1.0;
        let mut unique_b = upgrade("mob_justice", "unique_b");
        unique_b.one_time = true;
        unique_b.min_value = None;
        unique_b.max_value = None;
        unique_b.value = 1.0;
        let normal = upgrade("damage", "normal_damage");
        let pool = vec![unique_a, unique_b, normal];

        let mut tracker = OneTimeUpgradeTracker::default();
        for idx in 0..super::MAX_UNIQUE_UPGRADES {
            tracker.acquired_ids.insert(format!("picked_{idx}"));
        }

        let mut rng = UpgradeRngState {
            state: 0xBEEF_CAFE_1020_3040,
        };
        let picks =
            roll_upgrade_options(&pool, &mut rng, 3, &tracker, &FormationSkillBar::default());

        assert!(!picks.iter().any(|upgrade| upgrade.one_time));
        assert!(picks.iter().any(|upgrade| upgrade.id == "normal_damage"));
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
            requirement_active_formation: None,
            requirement_map_tag: None,
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
    fn formation_breach_upgrade_enables_inside_formation_damage_bonus() {
        let mut buffs = GlobalBuffs::default();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();
        let upgrade = UpgradeConfig {
            id: "encirclement_doctrine".to_string(),
            kind: "formation_breach".to_string(),
            value: 20.0,
            min_value: None,
            max_value: None,
            value_step: None,
            weight_exponent: None,
            one_time: true,
            adds_to_skillbar: false,
            formation_id: None,
            requirement_type: None,
            requirement_min_tier0_share: None,
            requirement_active_formation: None,
            requirement_map_tag: None,
        };

        super::apply_upgrade(&upgrade, &mut buffs, &mut conditional, &mut skillbar);
        assert!((buffs.inside_formation_damage_multiplier - 1.2).abs() < 0.001);
    }

    #[test]
    fn fast_learner_upgrade_stacks_xp_gain_multiplier() {
        let mut buffs = GlobalBuffs::default();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();
        let upgrade = UpgradeConfig {
            id: "fast_learner_up".to_string(),
            kind: "fast_learner".to_string(),
            value: 0.08,
            min_value: Some(0.04),
            max_value: Some(0.16),
            value_step: Some(0.02),
            weight_exponent: Some(2.0),
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
            requirement_type: None,
            requirement_min_tier0_share: None,
            requirement_active_formation: None,
            requirement_map_tag: None,
        };
        super::apply_upgrade(&upgrade, &mut buffs, &mut conditional, &mut skillbar);
        super::apply_upgrade(&upgrade, &mut buffs, &mut conditional, &mut skillbar);
        assert!((buffs.xp_gain_multiplier - 1.16).abs() < 0.001);
    }

    #[test]
    fn crit_upgrades_apply_to_global_buffs_with_expected_bounds() {
        let mut buffs = GlobalBuffs::default();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();

        let mut crit_chance = upgrade("crit_chance", "crit_chance_up");
        crit_chance.value = 0.60;
        super::apply_upgrade(&crit_chance, &mut buffs, &mut conditional, &mut skillbar);
        super::apply_upgrade(&crit_chance, &mut buffs, &mut conditional, &mut skillbar);
        assert!((buffs.crit_chance_bonus - 0.95).abs() < 0.001);

        let mut crit_damage = upgrade("crit_damage", "crit_damage_up");
        crit_damage.value = 0.20;
        super::apply_upgrade(&crit_damage, &mut buffs, &mut conditional, &mut skillbar);
        super::apply_upgrade(&crit_damage, &mut buffs, &mut conditional, &mut skillbar);
        assert!((buffs.crit_damage_multiplier - 1.60).abs() < 0.001);
    }

    #[test]
    fn xp_requirements_increase_each_level() {
        assert!((xp_required_for_level(1) - 300.0).abs() < 0.001);
        assert!(xp_required_for_level(2) > xp_required_for_level(1));
        assert!(xp_required_for_level(5) > xp_required_for_level(4));
        assert!(xp_required_for_level(11) > xp_required_for_level(10));
        assert!(xp_required_for_level(21) > xp_required_for_level(20));
    }

    #[test]
    fn xp_requirements_use_uniform_per_level_growth() {
        let early_ratio = xp_required_for_level(11) / xp_required_for_level(10);
        let late_ratio = xp_required_for_level(121) / xp_required_for_level(120);
        assert!((early_ratio - super::XP_GROWTH_PER_LEVEL).abs() < 0.0001);
        assert!((late_ratio - super::XP_GROWTH_PER_LEVEL).abs() < 0.0001);
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
    fn requirement_evaluator_handles_tier0_and_formation_conditions() {
        let tier_gate = super::UpgradeRequirement::Tier0Share { min_share: 0.75 };
        let (tier_inactive, tier_reason) =
            super::evaluate_upgrade_requirement(&tier_gate, 0.4, ActiveFormation::Square);
        assert!(!tier_inactive);
        assert!(
            tier_reason
                .as_deref()
                .unwrap_or_default()
                .contains("tier-0 share")
        );

        let (tier_active, tier_reason_active) =
            super::evaluate_upgrade_requirement(&tier_gate, 0.9, ActiveFormation::Square);
        assert!(tier_active);
        assert!(tier_reason_active.is_none());

        let formation_gate = super::UpgradeRequirement::FormationActive {
            formation: ActiveFormation::Diamond,
        };
        let (formation_inactive, _) =
            super::evaluate_upgrade_requirement(&formation_gate, 1.0, ActiveFormation::Square);
        assert!(!formation_inactive);
        let (formation_active, _) =
            super::evaluate_upgrade_requirement(&formation_gate, 1.0, ActiveFormation::Diamond);
        assert!(formation_active);
    }

    #[test]
    fn conditional_effects_apply_and_revoke_without_duplicate_stack_bugs() {
        let entries = vec![
            super::OwnedConditionalUpgrade {
                id: "mob_fury".to_string(),
                kind: "mob_fury".to_string(),
                requirement: super::UpgradeRequirement::Tier0Share { min_share: 0.8 },
                stacks: 1,
            },
            super::OwnedConditionalUpgrade {
                id: "mob_fury".to_string(),
                kind: "mob_fury".to_string(),
                requirement: super::UpgradeRequirement::Tier0Share { min_share: 0.8 },
                stacks: 2,
            },
        ];
        let (active_effects, active_status) =
            super::conditional_effects_from_owned(&entries, 1.0, ActiveFormation::Square);
        assert!(active_effects.friendly_loss_immunity);
        assert!((active_effects.friendly_damage_multiplier - 1.18).abs() < 0.001);
        assert_eq!(active_status.len(), 2);
        assert!(active_status.iter().all(|entry| entry.active));

        let (inactive_effects, inactive_status) =
            super::conditional_effects_from_owned(&entries, 0.2, ActiveFormation::Square);
        assert!(!inactive_effects.friendly_loss_immunity);
        assert!((inactive_effects.friendly_damage_multiplier - 1.0).abs() < 0.001);
        assert!(inactive_status.iter().all(|entry| !entry.active));
    }

    #[test]
    fn mob_mercy_rescue_multiplier_toggles_with_requirement_state() {
        let entries = vec![super::OwnedConditionalUpgrade {
            id: "mob_mercy".to_string(),
            kind: "mob_mercy".to_string(),
            requirement: super::UpgradeRequirement::Tier0Share { min_share: 1.0 },
            stacks: 1,
        }];

        let (active_effects, active_status) =
            super::conditional_effects_from_owned(&entries, 1.0, ActiveFormation::Square);
        assert!((active_effects.rescue_time_multiplier - 0.5).abs() < 0.001);
        assert_eq!(active_effects.execute_below_health_ratio, 0.0);
        assert!(active_status.iter().all(|entry| entry.active));

        let (inactive_effects, inactive_status) =
            super::conditional_effects_from_owned(&entries, 0.75, ActiveFormation::Square);
        assert!((inactive_effects.rescue_time_multiplier - 1.0).abs() < 0.001);
        assert!(inactive_status.iter().all(|entry| !entry.active));
    }

    #[test]
    fn mob_trio_effects_do_not_cross_wire_when_requirements_diverge() {
        let entries = vec![
            super::OwnedConditionalUpgrade {
                id: "mob_fury".to_string(),
                kind: "mob_fury".to_string(),
                requirement: super::UpgradeRequirement::Tier0Share { min_share: 1.0 },
                stacks: 1,
            },
            super::OwnedConditionalUpgrade {
                id: "mob_justice".to_string(),
                kind: "mob_justice".to_string(),
                requirement: super::UpgradeRequirement::FormationActive {
                    formation: ActiveFormation::Diamond,
                },
                stacks: 1,
            },
            super::OwnedConditionalUpgrade {
                id: "mob_mercy".to_string(),
                kind: "mob_mercy".to_string(),
                requirement: super::UpgradeRequirement::Tier0Share { min_share: 1.0 },
                stacks: 1,
            },
        ];

        let (effects, status) =
            super::conditional_effects_from_owned(&entries, 1.0, ActiveFormation::Square);
        assert!(effects.friendly_loss_immunity);
        assert!((effects.friendly_damage_multiplier - 1.18).abs() < 0.001);
        assert!((effects.rescue_time_multiplier - 0.5).abs() < 0.001);
        assert_eq!(effects.execute_below_health_ratio, 0.0);

        let justice_status = status
            .iter()
            .find(|entry| entry.kind == "mob_justice")
            .expect("justice status");
        assert!(!justice_status.active);
    }

    #[test]
    fn upgrade_display_metadata_maps_known_kinds() {
        let damage_upgrade = upgrade("damage", "damage_up");
        assert_eq!(upgrade_card_icon(&damage_upgrade), UpgradeCardIcon::Damage);
        assert_eq!(upgrade_display_title(&damage_upgrade), "Sharpened Steel");
        assert!(upgrade_display_description(&damage_upgrade).contains("damage"));

        let mut crit_chance_upgrade = upgrade("crit_chance", "crit_chance_up");
        crit_chance_upgrade.value = 0.03;
        assert_eq!(
            upgrade_card_icon(&crit_chance_upgrade),
            UpgradeCardIcon::CritChance
        );
        assert_eq!(
            upgrade_display_title(&crit_chance_upgrade),
            "Killer Instinct"
        );
        assert!(upgrade_display_description(&crit_chance_upgrade).contains("critical hit chance"));

        let mut crit_damage_upgrade = upgrade("crit_damage", "crit_damage_up");
        crit_damage_upgrade.value = 0.20;
        assert_eq!(
            upgrade_card_icon(&crit_damage_upgrade),
            UpgradeCardIcon::CritDamage
        );
        assert_eq!(
            upgrade_display_title(&crit_damage_upgrade),
            "Deadly Precision"
        );
        assert!(upgrade_display_description(&crit_damage_upgrade).contains("critical hit damage"));

        let mut fast_learner_upgrade = upgrade("fast_learner", "fast_learner_up");
        fast_learner_upgrade.value = 0.08;
        assert_eq!(
            upgrade_card_icon(&fast_learner_upgrade),
            UpgradeCardIcon::FastLearner
        );
        assert_eq!(upgrade_display_title(&fast_learner_upgrade), "Fast Learner");
        assert!(upgrade_display_description(&fast_learner_upgrade).contains("XP"));

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
            requirement_active_formation: None,
            requirement_map_tag: None,
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
        assert!((skill_book.entries[0].total_value - picked.value * 2.0).abs() < 0.001);
    }
}
