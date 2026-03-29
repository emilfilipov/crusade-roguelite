use std::collections::{HashMap, HashSet, VecDeque};

use bevy::prelude::*;

use crate::data::{GameData, UpgradeConfig};
use crate::enemies::WaveCompletedEvent;
use crate::formation::{ActiveFormation, FormationSkillBar};
use crate::inventory::ItemRarityRollBonus;
use crate::model::{
    BaseMaxHealth, FriendlyUnit, GainGoldEvent, GainHearTheCallTokenEvent, GameState, GlobalBuffs,
    Health, MAX_COMMANDER_LEVEL, MatchSetupSelection, StartRunEvent, UnitRoleTag,
};
use crate::random::runtime_entropy_seed_u64;
use crate::squad::RosterEconomy;

const LEVEL_UP_OPTION_COUNT: usize = 5;
const MAJOR_REWARD_LEVEL_INTERVAL: u32 = 5;
const ONE_TIME_OPTION_WEIGHT: f32 = 0.22;
const NORMAL_OPTION_WEIGHT: f32 = 1.0;
const MAX_UNIQUE_UPGRADES: usize = 5;
const AUTHORITY_ENEMY_MORALE_DRAIN_SCALE: f32 = 10.8;
const MAX_AUTHORITY_LOSS_RESISTANCE: f32 = 0.75;
const HOSPITALIER_MORALE_REGEN_SCALE: f32 = 0.1;

const MOB_FURY_DAMAGE_BONUS: f32 = 0.18;
const MOB_FURY_ATTACK_SPEED_BONUS: f32 = 0.18;
const MOB_FURY_MOVE_SPEED_BONUS: f32 = 24.0;
const MOB_FURY_LOSS_MITIGATION: f32 = 0.25;
const MOB_JUSTICE_EXECUTE_THRESHOLD: f32 = 0.10;
const MOB_MERCY_RESCUE_TIME_MULTIPLIER: f32 = 0.5;
const DOCTRINE_EXECUTION_RITES_DAMAGE_BONUS: f32 = 0.14;
const DOCTRINE_EXECUTION_RITES_EXECUTE_THRESHOLD: f32 = 0.18;
const DOCTRINE_EXECUTION_RITES_RESCUE_PENALTY: f32 = 1.25;
const DOCTRINE_COUNTERVOLLEY_DAMAGE_BONUS: f32 = 0.18;
const DOCTRINE_COUNTERVOLLEY_ATTACK_SPEED_BONUS: f32 = 0.12;
const DOCTRINE_COUNTERVOLLEY_MORALE_LOSS_MULTIPLIER: f32 = 1.12;
const DOCTRINE_PIKE_HEDGEHOG_DAMAGE_BONUS: f32 = 0.16;
const DOCTRINE_PIKE_HEDGEHOG_MOVE_SPEED_PENALTY: f32 = 14.0;
const DOCTRINE_PIKE_HEDGEHOG_MORALE_LOSS_MULTIPLIER: f32 = 0.88;
const UNIQUE_SLOT_TRADEOFF_KIND: &str = "unique_slot_tradeoff";
const MAX_SKILL_COOLDOWN_REDUCTION: f32 = 0.75;
const LUCK_TO_CRIT_CHANCE_MULTIPLIER: f32 = 0.5;
const LUCK_TO_CRIT_DAMAGE_MULTIPLIER: f32 = 2.0;

pub const fn max_unique_upgrades() -> usize {
    MAX_UNIQUE_UPGRADES
}

pub fn is_supported_upgrade_kind(kind: &str) -> bool {
    matches!(
        kind,
        "damage"
            | "attack_speed"
            | "quartermaster"
            | "luck"
            | "crit_chance"
            | "crit_damage"
            | "armor"
            | "pickup_radius"
            | "aura_radius"
            | "authority_aura"
            | "move_speed"
            | "hospitalier_aura"
            | "item_rarity"
            | "upgrade_rarity"
            | "skill_duration"
            | "cooldown_reduction"
            | "formation_breach"
            | "unlock_formation"
            | "mob_fury"
            | "mob_justice"
            | "mob_mercy"
            | "doctrine_command_net"
            | "doctrine_stalwart_oath"
            | "doctrine_forced_march"
            | "doctrine_execution_rites"
            | "doctrine_countervolley"
            | "doctrine_pike_hedgehog"
            | UNIQUE_SLOT_TRADEOFF_KIND
    )
}

pub fn effective_max_unique_upgrades(tracker: &OneTimeUpgradeTracker) -> usize {
    MAX_UNIQUE_UPGRADES.saturating_add(tracker.extra_slots)
}

pub fn consume_hear_the_call_for_hero_recruit(progression: &mut Progression) -> bool {
    if progression.hear_the_call_tokens == 0 {
        return false;
    }
    progression.hear_the_call_tokens = progression.hear_the_call_tokens.saturating_sub(1);
    true
}

#[derive(Resource, Clone, Debug)]
pub struct Progression {
    pub gold: f32,
    pub hear_the_call_tokens: u32,
    pub level: u32,
    pub pending_level_ups: u32,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct PendingRewardQueue {
    pub kinds: VecDeque<UpgradeRewardKind>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ProgressionLockFeedback {
    pub message: Option<String>,
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            gold: 0.0,
            hear_the_call_tokens: 0,
            level: 1,
            pending_level_ups: 0,
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct UpgradeDraft {
    pub active: bool,
    pub reward_kind: UpgradeRewardKind,
    pub options: Vec<UpgradeConfig>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum UpgradeRewardKind {
    #[default]
    Minor,
    Major,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UpgradeDraftLane {
    Minor,
    Major,
}

impl UpgradeRewardKind {
    const fn lane(self) -> UpgradeDraftLane {
        match self {
            UpgradeRewardKind::Minor => UpgradeDraftLane::Minor,
            UpgradeRewardKind::Major => UpgradeDraftLane::Major,
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct OneTimeUpgradeTracker {
    pub acquired_ids: HashSet<String>,
    pub extra_slots: usize,
    pub mythical_rolls_locked: bool,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct UpgradeStackTracker {
    stacks_by_id: HashMap<String, u32>,
}

impl UpgradeStackTracker {
    fn count_for(&self, id: &str) -> u32 {
        self.stacks_by_id.get(id).copied().unwrap_or(0)
    }

    fn increment(&mut self, id: &str) {
        let next = self.count_for(id).saturating_add(1);
        self.stacks_by_id.insert(id.to_string(), next);
    }
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct UpgradeRarityRollBonus {
    pub percent: f32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct SkillTimingBuffs {
    pub duration_multiplier: f32,
    pub cooldown_reduction: f32,
}

impl Default for SkillTimingBuffs {
    fn default() -> Self {
        Self {
            duration_multiplier: 1.0,
            cooldown_reduction: 0.0,
        }
    }
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
    Tier0Share {
        min_share: f32,
    },
    FormationActive {
        formation: ActiveFormation,
    },
    MapTag {
        tag: String,
    },
    HasTrait {
        trait_tag: RequirementTraitTag,
        min_count: u32,
    },
    BandAtLeast {
        stat: RequirementBandStat,
        band: RequirementBand,
    },
    BandAtMost {
        stat: RequirementBandStat,
        band: RequirementBand,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementTraitTag {
    Shielded,
    Frontline,
    AntiCavalry,
    Cavalry,
    AntiArmor,
    Skirmisher,
    Support,
}

impl RequirementTraitTag {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "shielded" => Some(Self::Shielded),
            "frontline" => Some(Self::Frontline),
            "anti_cavalry" => Some(Self::AntiCavalry),
            "cavalry" => Some(Self::Cavalry),
            "anti_armor" => Some(Self::AntiArmor),
            "skirmisher" => Some(Self::Skirmisher),
            "support" => Some(Self::Support),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Shielded => "Shielded",
            Self::Frontline => "Frontline",
            Self::AntiCavalry => "Anti-Cavalry",
            Self::Cavalry => "Cavalry",
            Self::AntiArmor => "Anti-Armor",
            Self::Skirmisher => "Skirmisher",
            Self::Support => "Support",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequirementBandStat {
    Tier0Share,
    ShieldedShare,
    FrontlineShare,
    AntiCavalryShare,
    SupportShare,
    CavalryShare,
    ArcherShare,
    AntiArmorShare,
}

impl RequirementBandStat {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "tier0_share" => Some(Self::Tier0Share),
            "shielded_share" => Some(Self::ShieldedShare),
            "frontline_share" => Some(Self::FrontlineShare),
            "anti_cavalry_share" => Some(Self::AntiCavalryShare),
            "support_share" => Some(Self::SupportShare),
            "cavalry_share" => Some(Self::CavalryShare),
            "archer_share" => Some(Self::ArcherShare),
            "anti_armor_share" => Some(Self::AntiArmorShare),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Tier0Share => "Tier-0 share",
            Self::ShieldedShare => "Shielded share",
            Self::FrontlineShare => "Frontline share",
            Self::AntiCavalryShare => "Anti-cavalry share",
            Self::SupportShare => "Support share",
            Self::CavalryShare => "Cavalry share",
            Self::ArcherShare => "Archer share",
            Self::AntiArmorShare => "Anti-armor share",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RequirementBand {
    VeryLow,
    Low,
    Moderate,
    High,
    VeryHigh,
}

impl RequirementBand {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "very_low" => Some(Self::VeryLow),
            "low" => Some(Self::Low),
            "moderate" => Some(Self::Moderate),
            "high" => Some(Self::High),
            "very_high" => Some(Self::VeryHigh),
            _ => None,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::VeryLow => "Very Low",
            Self::Low => "Low",
            Self::Moderate => "Moderate",
            Self::High => "High",
            Self::VeryHigh => "Very High",
        }
    }
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
    pub friendly_morale_loss_multiplier: f32,
    pub execute_below_health_ratio: f32,
    pub rescue_time_multiplier: f32,
}

impl Default for ConditionalUpgradeEffects {
    fn default() -> Self {
        Self {
            friendly_damage_multiplier: 1.0,
            friendly_attack_speed_multiplier: 1.0,
            friendly_move_speed_bonus: 0.0,
            friendly_morale_loss_multiplier: 1.0,
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
    Luck,
    CritChance,
    CritDamage,
    Armor,
    PickupRadius,
    AuraRadius,
    AuthorityAura,
    MoveSpeed,
    HospitalierAura,
    ItemRarity,
    UpgradeRarity,
    SkillDuration,
    CooldownReduction,
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
        self.state = runtime_entropy_seed_u64();
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
            .init_resource::<PendingRewardQueue>()
            .init_resource::<ProgressionLockFeedback>()
            .init_resource::<UpgradeDraft>()
            .init_resource::<OneTimeUpgradeTracker>()
            .init_resource::<UpgradeStackTracker>()
            .init_resource::<UpgradeRarityRollBonus>()
            .init_resource::<SkillTimingBuffs>()
            .init_resource::<SkillBookLog>()
            .init_resource::<ConditionalUpgradeOwnership>()
            .init_resource::<ConditionalUpgradeStatus>()
            .init_resource::<ConditionalUpgradeEffects>()
            .init_resource::<LevelPassiveRuntime>()
            .init_resource::<UpgradeRngState>()
            .init_resource::<GlobalBuffs>()
            .add_event::<SelectUpgradeEvent>()
            .add_systems(
                Update,
                (
                    reset_progress_on_run_start,
                    reset_upgrade_stack_tracker_on_run_start,
                    reset_pending_reward_queue_on_run_start,
                    reset_progress_lock_feedback_on_run_start,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    gain_gold,
                    gain_hear_the_call_tokens,
                    queue_level_rewards_from_wave_completions,
                    open_draft_on_pending_levels,
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
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    mut progression: ResMut<Progression>,
    mut draft: ResMut<UpgradeDraft>,
    mut one_time_tracker: ResMut<OneTimeUpgradeTracker>,
    mut skill_book: ResMut<SkillBookLog>,
    mut conditional_ownership: ResMut<ConditionalUpgradeOwnership>,
    mut conditional_status: ResMut<ConditionalUpgradeStatus>,
    mut conditional_effects: ResMut<ConditionalUpgradeEffects>,
    mut passive_runtime: ResMut<LevelPassiveRuntime>,
    mut buffs: ResMut<GlobalBuffs>,
    mut item_rarity_bonus: ResMut<ItemRarityRollBonus>,
    mut upgrade_rarity_bonus: ResMut<UpgradeRarityRollBonus>,
    mut skill_timing: ResMut<SkillTimingBuffs>,
    mut rng: ResMut<UpgradeRngState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *progression = Progression::default();
    *draft = UpgradeDraft::default();
    *one_time_tracker = OneTimeUpgradeTracker::default();
    *skill_book = SkillBookLog::default();
    *conditional_ownership = ConditionalUpgradeOwnership::default();
    *conditional_status = ConditionalUpgradeStatus::default();
    *conditional_effects = ConditionalUpgradeEffects::default();
    *passive_runtime = LevelPassiveRuntime::default();
    *buffs = GlobalBuffs::default();
    *item_rarity_bonus = ItemRarityRollBonus::default();
    *upgrade_rarity_bonus = UpgradeRarityRollBonus::default();
    *skill_timing = SkillTimingBuffs::default();
    let player_faction = setup_selection
        .as_deref()
        .map(|selection| selection.faction)
        .unwrap_or(crate::model::PlayerFaction::Christian);
    let selected_commander_id = setup_selection
        .as_deref()
        .map(|selection| selection.commander_id.as_str())
        .unwrap_or_else(|| {
            crate::data::UnitsConfigFile::default_commander_id_for_faction(player_faction)
        });
    if let Some(commander) = data
        .units
        .commander_option_for_faction_and_id(player_faction, selected_commander_id)
    {
        buffs.damage_multiplier += commander.run_bonuses.damage_multiplier_bonus;
        buffs.move_speed_bonus += commander.run_bonuses.move_speed_bonus;
        buffs.commander_aura_radius_bonus += commander.run_bonuses.aura_radius_bonus;
        buffs.pickup_radius_bonus += commander.run_bonuses.pickup_radius_bonus;
    }
    rng.reseed_from_time();
}

fn reset_upgrade_stack_tracker_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut stack_tracker: ResMut<UpgradeStackTracker>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *stack_tracker = UpgradeStackTracker::default();
}

fn reset_pending_reward_queue_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut pending_rewards: ResMut<PendingRewardQueue>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *pending_rewards = PendingRewardQueue::default();
}

fn reset_progress_lock_feedback_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut lock_feedback: ResMut<ProgressionLockFeedback>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *lock_feedback = ProgressionLockFeedback::default();
}

fn gain_gold(mut progression: ResMut<Progression>, mut gold_events: EventReader<GainGoldEvent>) {
    for event in gold_events.read() {
        progression.gold = (progression.gold + event.0).max(0.0);
    }
}

fn gain_hear_the_call_tokens(
    mut progression: ResMut<Progression>,
    mut token_events: EventReader<GainHearTheCallTokenEvent>,
) {
    for event in token_events.read() {
        progression.hear_the_call_tokens = progression
            .hear_the_call_tokens
            .saturating_add(event.0.max(1));
    }
}

fn queue_level_rewards_from_wave_completions(
    mut progression: ResMut<Progression>,
    mut pending_rewards: ResMut<PendingRewardQueue>,
    mut wave_completed_events: EventReader<WaveCompletedEvent>,
) {
    if progression.level >= MAX_COMMANDER_LEVEL {
        pending_rewards.kinds.clear();
        progression.pending_level_ups = 0;
        for _ in wave_completed_events.read() {}
        return;
    }
    for event in wave_completed_events.read() {
        enqueue_reward_kinds(
            &mut pending_rewards.kinds,
            progression.level,
            level_rewards_for_wave_completion(event.wave_number),
        );
    }
    progression.pending_level_ups = pending_reward_queue_len(&pending_rewards);
}

#[allow(clippy::too_many_arguments)]
fn open_draft_on_pending_levels(
    mut progression: ResMut<Progression>,
    mut pending_rewards: ResMut<PendingRewardQueue>,
    mut lock_feedback: ResMut<ProgressionLockFeedback>,
    mut draft: ResMut<UpgradeDraft>,
    one_time_tracker: Res<OneTimeUpgradeTracker>,
    stack_tracker: Res<UpgradeStackTracker>,
    upgrade_rarity_bonus: Res<UpgradeRarityRollBonus>,
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
        pending_rewards.kinds.clear();
        progression.pending_level_ups = 0;
        lock_feedback.message = None;
        return;
    }

    if pending_rewards.kinds.is_empty() {
        progression.pending_level_ups = 0;
        return;
    }

    let Some(reward_kind) = pending_rewards.kinds.pop_front() else {
        progression.pending_level_ups = 0;
        return;
    };
    progression.pending_level_ups = pending_reward_queue_len(&pending_rewards);
    progression.level += 1;
    progression.level = progression
        .level
        .min(allowed_max_level)
        .min(MAX_COMMANDER_LEVEL);
    draft.reward_kind = reward_kind;
    draft.options = roll_upgrade_options(
        &data.upgrades.upgrades,
        &mut rng,
        LEVEL_UP_OPTION_COUNT,
        draft.reward_kind,
        upgrade_rarity_bonus.percent,
        &one_time_tracker,
        &stack_tracker,
        &skillbar,
    );
    draft.active = !draft.options.is_empty();
    if draft.active {
        next_state.set(GameState::LevelUp);
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

pub fn level_rewards_for_wave_completion(wave_number: u32) -> u32 {
    if wave_number == 0 {
        return 0;
    }
    if wave_number == 98 { 2 } else { 1 }
}

pub fn reward_kind_for_level(level: u32) -> UpgradeRewardKind {
    if level > 0 && level.is_multiple_of(MAJOR_REWARD_LEVEL_INTERVAL) {
        UpgradeRewardKind::Major
    } else {
        UpgradeRewardKind::Minor
    }
}

pub fn major_minor_reward_counts_for_level(level: u32) -> (u32, u32) {
    let major = level / MAJOR_REWARD_LEVEL_INTERVAL;
    let minor = level.saturating_sub(major);
    (major, minor)
}

fn pending_reward_queue_len(queue: &PendingRewardQueue) -> u32 {
    queue.kinds.len().min(u32::MAX as usize) as u32
}

fn enqueue_reward_kinds(
    queue: &mut VecDeque<UpgradeRewardKind>,
    current_level: u32,
    reward_count: u32,
) -> u32 {
    let mut queued = 0;
    let queued_existing = queue.len().min(u32::MAX as usize) as u32;
    let mut projected_level = current_level.saturating_add(queued_existing);
    for _ in 0..reward_count {
        if projected_level >= MAX_COMMANDER_LEVEL {
            break;
        }
        projected_level = projected_level.saturating_add(1);
        if projected_level > MAX_COMMANDER_LEVEL {
            break;
        }
        queue.push_back(reward_kind_for_level(projected_level));
        queued += 1;
    }
    queued
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

#[allow(clippy::too_many_arguments)]
fn resolve_upgrade_selection(
    mut draft: ResMut<UpgradeDraft>,
    mut selection_events: EventReader<SelectUpgradeEvent>,
    mut buffs: ResMut<GlobalBuffs>,
    mut skill_book: ResMut<SkillBookLog>,
    mut conditional_ownership: ResMut<ConditionalUpgradeOwnership>,
    mut one_time_tracker: ResMut<OneTimeUpgradeTracker>,
    mut stack_tracker: ResMut<UpgradeStackTracker>,
    mut item_rarity_bonus: ResMut<ItemRarityRollBonus>,
    mut upgrade_rarity_bonus: ResMut<UpgradeRarityRollBonus>,
    mut skill_timing: ResMut<SkillTimingBuffs>,
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
    let current_stacks = stack_tracker.count_for(picked.id.as_str());
    if let Some(cap) = picked.stack_cap
        && current_stacks >= cap
    {
        draft.active = false;
        draft.options.clear();
        next_state.set(GameState::InRun);
        return;
    }
    let mut applied = picked.clone();
    applied.value = effective_upgrade_value(&picked, current_stacks);
    apply_upgrade(
        &applied,
        &mut buffs,
        &mut item_rarity_bonus,
        &mut upgrade_rarity_bonus,
        &mut skill_timing,
        &mut conditional_ownership,
        &mut skillbar,
    );
    apply_one_time_tracker_effects(&applied, &mut one_time_tracker);
    record_skill_book_entry(&mut skill_book, &applied);
    stack_tracker.increment(applied.id.as_str());
    if applied.one_time {
        one_time_tracker.acquired_ids.insert(applied.id.clone());
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

fn apply_one_time_tracker_effects(upgrade: &UpgradeConfig, tracker: &mut OneTimeUpgradeTracker) {
    if upgrade.kind != UNIQUE_SLOT_TRADEOFF_KIND {
        return;
    }
    let slot_bonus = upgrade.value.max(0.0).round() as usize;
    tracker.extra_slots = tracker.extra_slots.saturating_add(slot_bonus);
    tracker.mythical_rolls_locked = true;
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
        "quartermaster" => format!(
            "Total gold gain bonus from pickups: +{:.0}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "luck" => format!(
            "Total luck bonus: +{:.0}% (boosts crit chance/damage, loot rarity, and drop odds).",
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
        "item_rarity" => format!(
            "Total item rarity roll bonus: +{:.0}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "upgrade_rarity" => format!(
            "Total level-up rarity roll bonus: +{:.0}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "skill_duration" => format!(
            "Total skill duration bonus: +{:.0}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "cooldown_reduction" => format!(
            "Total skill cooldown reduction: +{:.0}%.",
            entry.total_value.max(0.0) * 100.0
        ),
        "hospitalier_aura" => format!(
            "Aura regen totals: +{:.1} HP/s and +{:.2} morale/s for friendlies in aura.",
            entry.total_value.max(0.0),
            entry.total_value.max(0.0) * HOSPITALIER_MORALE_REGEN_SCALE
        ),
        "formation_breach" => format!(
            "Enemies inside formation take +{:.0}% damage.",
            entry.total_value.max(0.0)
        ),
        "unique_slot_tradeoff" => {
            "Gain +2 unique upgrade slots. Downside: narrows late-run doctrine flexibility."
                .to_string()
        }
        "mob_fury"
        | "mob_justice"
        | "mob_mercy"
        | "doctrine_command_net"
        | "doctrine_stalwart_oath"
        | "doctrine_forced_march"
        | "doctrine_execution_rites"
        | "doctrine_countervolley"
        | "doctrine_pike_hedgehog"
        | "unlock_formation" => entry.description.clone(),
        _ => entry.description.clone(),
    }
}

#[allow(clippy::too_many_arguments)]
fn roll_upgrade_options(
    pool: &[UpgradeConfig],
    rng: &mut UpgradeRngState,
    count: usize,
    reward_kind: UpgradeRewardKind,
    upgrade_rarity_bonus_percent: f32,
    one_time_tracker: &OneTimeUpgradeTracker,
    stack_tracker: &UpgradeStackTracker,
    skillbar: &FormationSkillBar,
) -> Vec<UpgradeConfig> {
    if pool.is_empty() || count == 0 {
        return Vec::new();
    }
    let required_lane = reward_kind.lane();
    let unique_cap_reached =
        one_time_tracker.acquired_ids.len() >= effective_max_unique_upgrades(one_time_tracker);
    let candidate_indices: Vec<usize> = pool
        .iter()
        .enumerate()
        .filter(|(_, upgrade)| {
            if upgrade_draft_lane(upgrade) != required_lane {
                return false;
            }
            if upgrade.one_time && one_time_tracker.acquired_ids.contains(&upgrade.id) {
                return false;
            }
            if unique_cap_reached && upgrade.one_time {
                return false;
            }
            if let Some(stack_cap) = upgrade.stack_cap
                && stack_tracker.count_for(upgrade.id.as_str()) >= stack_cap
            {
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
    let indices = draw_unique_indices_weighted(pool, &candidate_indices, pick_count, rng);
    indices
        .into_iter()
        .map(|idx| {
            let mut rolled = pool[idx].clone();
            let current_stacks = stack_tracker.count_for(rolled.id.as_str());
            let rolled_value = roll_upgrade_value(
                &rolled,
                rng,
                !one_time_tracker.mythical_rolls_locked,
                upgrade_rarity_bonus_percent,
            );
            rolled.value = effective_upgrade_value_with_base(&rolled, current_stacks, rolled_value);
            rolled
        })
        .collect()
}

fn upgrade_draft_lane(upgrade: &UpgradeConfig) -> UpgradeDraftLane {
    match upgrade
        .reward_lane
        .as_deref()
        .map(str::trim)
        .unwrap_or("minor")
    {
        "major" => UpgradeDraftLane::Major,
        _ => UpgradeDraftLane::Minor,
    }
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

fn draw_unique_indices_weighted(
    pool: &[UpgradeConfig],
    candidate_indices: &[usize],
    count: usize,
    rng: &mut UpgradeRngState,
) -> Vec<usize> {
    let mut remaining = candidate_indices.to_vec();
    let mut selected = Vec::with_capacity(count.min(remaining.len()));
    while selected.len() < count && !remaining.is_empty() {
        let total_weight: f32 = remaining
            .iter()
            .map(|idx| option_selection_weight(&pool[*idx]))
            .sum();
        if total_weight <= f32::EPSILON {
            let fallback = draw_unique_indices(remaining.as_mut_slice(), 1, rng);
            if let Some(pick) = fallback.first().copied() {
                selected.push(pick);
                remaining.retain(|idx| *idx != pick);
            }
            continue;
        }

        let mut roll = rng.next_f32() * total_weight;
        let mut chosen = remaining[0];
        for candidate in &remaining {
            let weight = option_selection_weight(&pool[*candidate]);
            if roll <= weight {
                chosen = *candidate;
                break;
            }
            roll -= weight;
        }
        selected.push(chosen);
        remaining.retain(|idx| *idx != chosen);
    }
    selected
}

fn option_selection_weight(config: &UpgradeConfig) -> f32 {
    if config.one_time {
        ONE_TIME_OPTION_WEIGHT
    } else {
        NORMAL_OPTION_WEIGHT
    }
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

fn roll_upgrade_value(
    config: &UpgradeConfig,
    _rng: &mut UpgradeRngState,
    _mythical_rolls_enabled: bool,
    _upgrade_rarity_bonus_percent: f32,
) -> f32 {
    config.value.max(0.0)
}

fn effective_upgrade_value(upgrade: &UpgradeConfig, current_stacks: u32) -> f32 {
    effective_upgrade_value_with_base(upgrade, current_stacks, upgrade.value.max(0.0))
}

fn effective_upgrade_value_with_base(
    upgrade: &UpgradeConfig,
    current_stacks: u32,
    base_value: f32,
) -> f32 {
    let diminishing_factor = upgrade.diminishing_factor.unwrap_or(1.0).clamp(0.01, 1.0);
    let exponent = current_stacks as i32;
    let diminishing_multiplier = diminishing_factor.powi(exponent);
    (base_value.max(0.0) * diminishing_multiplier).max(0.0)
}

pub fn upgrade_value_tier(upgrade: &UpgradeConfig) -> UpgradeValueTier {
    if upgrade.one_time {
        return UpgradeValueTier::Unique;
    }
    match upgrade_draft_lane(upgrade) {
        UpgradeDraftLane::Major => UpgradeValueTier::Epic,
        UpgradeDraftLane::Minor => {
            if upgrade.value >= 10.0 {
                UpgradeValueTier::Rare
            } else if upgrade.value >= 5.0 {
                UpgradeValueTier::Uncommon
            } else {
                UpgradeValueTier::Common
            }
        }
    }
}

pub fn upgrade_card_icon(upgrade: &UpgradeConfig) -> UpgradeCardIcon {
    match upgrade.kind.as_str() {
        "damage" => UpgradeCardIcon::Damage,
        "attack_speed" => UpgradeCardIcon::AttackSpeed,
        "quartermaster" => UpgradeCardIcon::FastLearner,
        "luck" => UpgradeCardIcon::Luck,
        "crit_chance" => UpgradeCardIcon::CritChance,
        "crit_damage" => UpgradeCardIcon::CritDamage,
        "armor" => UpgradeCardIcon::Armor,
        "pickup_radius" => UpgradeCardIcon::PickupRadius,
        "aura_radius" => UpgradeCardIcon::AuraRadius,
        "authority_aura" => UpgradeCardIcon::AuthorityAura,
        "move_speed" => UpgradeCardIcon::MoveSpeed,
        "hospitalier_aura" => UpgradeCardIcon::HospitalierAura,
        "item_rarity" => UpgradeCardIcon::ItemRarity,
        "upgrade_rarity" => UpgradeCardIcon::UpgradeRarity,
        "skill_duration" => UpgradeCardIcon::SkillDuration,
        "cooldown_reduction" => UpgradeCardIcon::CooldownReduction,
        "doctrine_command_net" => UpgradeCardIcon::AuthorityAura,
        "doctrine_stalwart_oath" => UpgradeCardIcon::HospitalierAura,
        "doctrine_forced_march" => UpgradeCardIcon::MoveSpeed,
        "doctrine_execution_rites" => UpgradeCardIcon::MobJustice,
        "doctrine_countervolley" => UpgradeCardIcon::AttackSpeed,
        "doctrine_pike_hedgehog" => UpgradeCardIcon::Armor,
        "unique_slot_tradeoff" => UpgradeCardIcon::AuthorityAura,
        "formation_breach" => UpgradeCardIcon::FormationSquare,
        "mob_fury" => UpgradeCardIcon::MobFury,
        "mob_justice" => UpgradeCardIcon::MobJustice,
        "mob_mercy" => UpgradeCardIcon::MobMercy,
        "unlock_formation" => match upgrade.formation_id.as_deref() {
            Some("skean") | Some("diamond") => UpgradeCardIcon::FormationDiamond,
            _ => UpgradeCardIcon::FormationSquare,
        },
        _ => UpgradeCardIcon::Damage,
    }
}

pub fn upgrade_display_title(upgrade: &UpgradeConfig) -> &'static str {
    match upgrade.kind.as_str() {
        "damage" => "Sharpened Steel",
        "attack_speed" => "Rapid Drill",
        "quartermaster" => "Quartermaster",
        "luck" => "Fortune's Favor",
        "crit_chance" => "Killer Instinct",
        "crit_damage" => "Deadly Precision",
        "armor" => "Hardened Armor",
        "pickup_radius" => "Supply Reach",
        "aura_radius" => "Extended Command",
        "authority_aura" => "Authority Aura",
        "move_speed" => "Forced March",
        "hospitalier_aura" => "Hospitalier Aura",
        "item_rarity" => "Master Quartermaster",
        "upgrade_rarity" => "Tactical Insight",
        "skill_duration" => "Enduring Cadence",
        "cooldown_reduction" => "Swift Drills",
        "doctrine_command_net" => "Command Net",
        "doctrine_stalwart_oath" => "Stalwart Oath",
        "doctrine_forced_march" => "Forced March Doctrine",
        "doctrine_execution_rites" => "Execution Rites",
        "doctrine_countervolley" => "Countervolley Doctrine",
        "doctrine_pike_hedgehog" => "Pike Hedgehog",
        "unique_slot_tradeoff" => "War Council Edict",
        "formation_breach" => "Into the Wolf's Dev",
        "mob_fury" => "Mob's Fury",
        "mob_justice" => "Mob's Justice",
        "mob_mercy" => "Mob's Mercy",
        "unlock_formation" => match upgrade.formation_id.as_deref() {
            Some("circle") => "Circle Formation",
            Some("skean") => "Skean Formation",
            Some("diamond") => "Diamond Formation",
            Some("shield_wall") => "Shield Wall Formation",
            Some("loose") => "Loose Formation",
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
        "quartermaster" => format!(
            "Increase gold gained from all gold pickups by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "luck" => format!(
            "Increase Luck by +{:.0}%: improves crit chance, crit damage, item quality, and drop chances.",
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
            "Friendlies in aura lose {:.0}% less morale; enemies in aura lose {:.2} morale/s.",
            upgrade.value * 100.0,
            upgrade.value * AUTHORITY_ENEMY_MORALE_DRAIN_SCALE
        ),
        "move_speed" => format!("Increase army movement speed by +{:.0}.", upgrade.value),
        "item_rarity" => format!(
            "Increase equipment item rarity roll bonus by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "upgrade_rarity" => format!(
            "Increase level-up rarity roll bonus by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "skill_duration" => format!(
            "Increase duration of cooldown-based skills by +{:.0}%.",
            upgrade.value * 100.0
        ),
        "cooldown_reduction" => format!(
            "Reduce cooldown of cooldown-based skills by {:.0}%.",
            upgrade.value * 100.0
        ),
        "hospitalier_aura" => format!(
            "Friendlies in aura regen +{:.1} HP/s and +{:.2} morale/s.",
            upgrade.value,
            upgrade.value * HOSPITALIER_MORALE_REGEN_SCALE
        ),
        "doctrine_command_net" => format!(
            "Major doctrine: commander aura radius +{:.0}, stronger enemy morale pressure in aura. Downside: -8% global damage.",
            upgrade.value
        ),
        "doctrine_stalwart_oath" => format!(
            "Major doctrine: +{:.1} armor and stronger sustain aura regen. Downside: -10 movement speed.",
            upgrade.value
        ),
        "doctrine_forced_march" => format!(
            "Major doctrine: +{:.0} movement speed and faster attacks. Downside: armor is reduced.",
            upgrade.value
        ),
        "doctrine_execution_rites" => "Major doctrine: conditional execute threshold increases and damage rises. Downside: rescue channels are slower while active.".to_string(),
        "doctrine_countervolley" => "Major doctrine: archer-heavy rosters gain burst damage and attack cadence. Downside: morale loss under pressure increases.".to_string(),
        "doctrine_pike_hedgehog" => "Major doctrine: anti-cavalry lines gain damage and steadier morale under pressure. Downside: formation movement slows.".to_string(),
        "unique_slot_tradeoff" => format!(
            "Gain +{:.0} Unique upgrade slots, but narrows late-run doctrine flexibility.",
            upgrade.value.max(0.0)
        ),
        "formation_breach" => format!(
            "Enemies inside your active formation footprint take +{:.0}% damage.",
            upgrade.value
        ),
        "mob_fury" => "If frontline share is High or above, friendlies gain +25% morale loss mitigation and bonus damage, attack speed, and movement speed.".to_string(),
        "mob_justice" => "If anti-armor share is Moderate or above, hits execute enemies below 10% HP.".to_string(),
        "mob_mercy" => "If support share is High or above, rescue channel time is reduced by 50%.".to_string(),
        "unlock_formation" => match upgrade.formation_id.as_deref() {
            Some("circle") => "Major reward only: unlock Circle formation on the skill bar (higher defense, lower mobility and offense).".to_string(),
            Some("skean") => "Major reward only: unlock Skean formation on the skill bar (faster movement and stronger moving offense, lower defense).".to_string(),
            Some("diamond") => "Major reward only: unlock Diamond formation on the skill bar (offense bonus while moving, faster movement, lower defense).".to_string(),
            Some("shield_wall") => "Major reward only: unlock Shield Wall formation on the skill bar (anti-entry, shielded block bonus, and melee reflect).".to_string(),
            Some("loose") => "Major reward only: unlock Loose formation on the skill bar (wider spacing and unlimited enemy interior occupancy).".to_string(),
            Some("square") => "Major reward only: unlock Square formation on the skill bar.".to_string(),
            _ => "Major reward only: unlock a formation skill for the skill bar.".to_string(),
        },
        _ => "Apply a battlefield improvement.".to_string(),
    }
}

fn apply_upgrade(
    upgrade: &UpgradeConfig,
    buffs: &mut GlobalBuffs,
    item_rarity_bonus: &mut ItemRarityRollBonus,
    upgrade_rarity_bonus: &mut UpgradeRarityRollBonus,
    skill_timing: &mut SkillTimingBuffs,
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
        "quartermaster" => {
            buffs.gold_gain_multiplier += upgrade.value;
        }
        "luck" => {
            buffs.luck_bonus += upgrade.value;
            buffs.crit_chance_bonus = (buffs.crit_chance_bonus
                + upgrade.value * LUCK_TO_CRIT_CHANCE_MULTIPLIER)
                .clamp(0.0, 0.95);
            buffs.crit_damage_multiplier = (buffs.crit_damage_multiplier
                + upgrade.value * LUCK_TO_CRIT_DAMAGE_MULTIPLIER)
                .max(1.0);
            item_rarity_bonus.percent = (item_rarity_bonus.percent + upgrade.value).max(0.0);
            upgrade_rarity_bonus.percent = (upgrade_rarity_bonus.percent + upgrade.value).max(0.0);
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
        "item_rarity" => {
            item_rarity_bonus.percent = (item_rarity_bonus.percent + upgrade.value).max(0.0);
        }
        "upgrade_rarity" => {
            upgrade_rarity_bonus.percent = (upgrade_rarity_bonus.percent + upgrade.value).max(0.0);
        }
        "skill_duration" => {
            skill_timing.duration_multiplier =
                (skill_timing.duration_multiplier + upgrade.value).max(1.0);
        }
        "cooldown_reduction" => {
            skill_timing.cooldown_reduction = (skill_timing.cooldown_reduction + upgrade.value)
                .clamp(0.0, MAX_SKILL_COOLDOWN_REDUCTION);
        }
        "hospitalier_aura" => {
            buffs.hospitalier_hp_regen_per_sec += upgrade.value;
            buffs.hospitalier_morale_regen_per_sec +=
                upgrade.value * HOSPITALIER_MORALE_REGEN_SCALE;
        }
        "formation_breach" => {
            buffs.inside_formation_damage_multiplier = buffs
                .inside_formation_damage_multiplier
                .max(1.0 + upgrade.value * 0.01);
        }
        "doctrine_command_net" => {
            buffs.commander_aura_radius_bonus += upgrade.value;
            buffs.authority_enemy_morale_drain_per_sec += upgrade.value * 0.05;
            buffs.damage_multiplier = (buffs.damage_multiplier - 0.08).max(0.6);
        }
        "doctrine_stalwart_oath" => {
            buffs.armor_bonus += upgrade.value;
            buffs.hospitalier_hp_regen_per_sec += 0.35;
            buffs.hospitalier_morale_regen_per_sec += 0.035;
            buffs.move_speed_bonus -= 10.0;
        }
        "doctrine_forced_march" => {
            buffs.move_speed_bonus += upgrade.value;
            buffs.attack_speed_multiplier += 0.12;
            buffs.armor_bonus = (buffs.armor_bonus - 1.5).max(-8.0);
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
        "mob_fury"
        | "mob_justice"
        | "mob_mercy"
        | "doctrine_execution_rites"
        | "doctrine_countervolley"
        | "doctrine_pike_hedgehog" => {
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
        .find(|entry| entry.kind == upgrade.kind)
    {
        existing.stacks = existing.stacks.saturating_add(1);
        existing.id = upgrade.id.clone();
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
        "has_trait" => upgrade
            .requirement_trait
            .as_deref()
            .and_then(|value| RequirementTraitTag::from_id(value.trim()))
            .map(|trait_tag| UpgradeRequirement::HasTrait {
                trait_tag,
                min_count: 1,
            })
            .unwrap_or(UpgradeRequirement::None),
        "band_at_least" => {
            let stat = upgrade
                .requirement_band_stat
                .as_deref()
                .and_then(|value| RequirementBandStat::from_id(value.trim()));
            let band = upgrade
                .requirement_band_at_least
                .as_deref()
                .and_then(|value| RequirementBand::from_id(value.trim()));
            match (stat, band) {
                (Some(stat), Some(band)) => UpgradeRequirement::BandAtLeast { stat, band },
                _ => UpgradeRequirement::None,
            }
        }
        "band_at_most" => {
            let stat = upgrade
                .requirement_band_stat
                .as_deref()
                .and_then(|value| RequirementBandStat::from_id(value.trim()));
            let band = upgrade
                .requirement_band_at_most
                .as_deref()
                .and_then(|value| RequirementBand::from_id(value.trim()));
            match (stat, band) {
                (Some(stat), Some(band)) => UpgradeRequirement::BandAtMost { stat, band },
                _ => UpgradeRequirement::None,
            }
        }
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
    let eval_context = requirement_eval_context(roster.as_deref());
    let (next_effects, next_status) = conditional_effects_from_owned(
        &conditional_owned.entries,
        &eval_context,
        *active_formation,
    );
    *effects = next_effects;
    status.entries = next_status;
}

fn conditional_effects_from_owned(
    entries: &[OwnedConditionalUpgrade],
    eval_context: &RequirementEvalContext,
    active_formation: ActiveFormation,
) -> (
    ConditionalUpgradeEffects,
    Vec<ConditionalUpgradeStatusEntry>,
) {
    let mut effects = ConditionalUpgradeEffects::default();
    let mut status_entries = Vec::with_capacity(entries.len());
    let mut applied_kinds = HashSet::new();

    for entry in entries {
        let (active, detail) =
            evaluate_upgrade_requirement(&entry.requirement, eval_context, active_formation);
        status_entries.push(ConditionalUpgradeStatusEntry {
            id: entry.id.clone(),
            kind: entry.kind.clone(),
            active,
            detail,
        });
        if !active {
            continue;
        }
        if !applied_kinds.insert(entry.kind.clone()) {
            continue;
        }
        match entry.kind.as_str() {
            "mob_fury" => {
                effects.friendly_damage_multiplier *= 1.0 + MOB_FURY_DAMAGE_BONUS;
                effects.friendly_attack_speed_multiplier *= 1.0 + MOB_FURY_ATTACK_SPEED_BONUS;
                effects.friendly_move_speed_bonus += MOB_FURY_MOVE_SPEED_BONUS;
                let mitigation_multiplier = 1.0 - MOB_FURY_LOSS_MITIGATION;
                effects.friendly_morale_loss_multiplier *= mitigation_multiplier;
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
            "doctrine_execution_rites" => {
                effects.friendly_damage_multiplier *= 1.0 + DOCTRINE_EXECUTION_RITES_DAMAGE_BONUS;
                effects.execute_below_health_ratio = effects
                    .execute_below_health_ratio
                    .max(DOCTRINE_EXECUTION_RITES_EXECUTE_THRESHOLD);
                effects.rescue_time_multiplier *= DOCTRINE_EXECUTION_RITES_RESCUE_PENALTY;
            }
            "doctrine_countervolley" => {
                effects.friendly_damage_multiplier *= 1.0 + DOCTRINE_COUNTERVOLLEY_DAMAGE_BONUS;
                effects.friendly_attack_speed_multiplier *=
                    1.0 + DOCTRINE_COUNTERVOLLEY_ATTACK_SPEED_BONUS;
                effects.friendly_morale_loss_multiplier *=
                    DOCTRINE_COUNTERVOLLEY_MORALE_LOSS_MULTIPLIER;
            }
            "doctrine_pike_hedgehog" => {
                effects.friendly_damage_multiplier *= 1.0 + DOCTRINE_PIKE_HEDGEHOG_DAMAGE_BONUS;
                effects.friendly_move_speed_bonus -= DOCTRINE_PIKE_HEDGEHOG_MOVE_SPEED_PENALTY;
                effects.friendly_morale_loss_multiplier *=
                    DOCTRINE_PIKE_HEDGEHOG_MORALE_LOSS_MULTIPLIER;
            }
            _ => {}
        }
    }
    (effects, status_entries)
}

pub fn evaluate_upgrade_requirement(
    requirement: &UpgradeRequirement,
    eval_context: &RequirementEvalContext,
    active_formation: ActiveFormation,
) -> (bool, Option<String>) {
    match requirement {
        UpgradeRequirement::None => (true, None),
        UpgradeRequirement::Tier0Share { min_share } => {
            let tier0_share = eval_context.tier0_share;
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
        UpgradeRequirement::HasTrait {
            trait_tag,
            min_count,
        } => {
            let count = eval_context.count_for_trait(*trait_tag);
            if count >= *min_count {
                (true, None)
            } else {
                (
                    false,
                    Some(format!(
                        "Requires at least {min_count} {} unit(s) in roster (current {}).",
                        trait_tag.label(),
                        count
                    )),
                )
            }
        }
        UpgradeRequirement::BandAtLeast { stat, band } => {
            let current = eval_context.band_for_stat(*stat);
            if current >= *band {
                (true, None)
            } else {
                (
                    false,
                    Some(format!(
                        "Requires {} at least {} (current {}).",
                        stat.label(),
                        band.label(),
                        current.label()
                    )),
                )
            }
        }
        UpgradeRequirement::BandAtMost { stat, band } => {
            let current = eval_context.band_for_stat(*stat);
            if current <= *band {
                (true, None)
            } else {
                (
                    false,
                    Some(format!(
                        "Requires {} at most {} (current {}).",
                        stat.label(),
                        band.label(),
                        current.label()
                    )),
                )
            }
        }
    }
}

fn roster_tier0_share(roster: &RosterEconomy) -> f32 {
    if roster.total_retinue_count == 0 {
        return 0.0;
    }
    roster.tier0_retinue_count as f32 / roster.total_retinue_count as f32
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RequirementEvalContext {
    tier0_share: f32,
    shielded_share: f32,
    frontline_share: f32,
    anti_cavalry_share: f32,
    support_share: f32,
    cavalry_share: f32,
    archer_share: f32,
    anti_armor_share: f32,
    shielded_count: u32,
    frontline_count: u32,
    anti_cavalry_count: u32,
    cavalry_count: u32,
    anti_armor_count: u32,
    skirmisher_count: u32,
    support_count: u32,
}

impl RequirementEvalContext {
    fn count_for_trait(self, trait_tag: RequirementTraitTag) -> u32 {
        match trait_tag {
            RequirementTraitTag::Shielded => self.shielded_count,
            RequirementTraitTag::Frontline => self.frontline_count,
            RequirementTraitTag::AntiCavalry => self.anti_cavalry_count,
            RequirementTraitTag::Cavalry => self.cavalry_count,
            RequirementTraitTag::AntiArmor => self.anti_armor_count,
            RequirementTraitTag::Skirmisher => self.skirmisher_count,
            RequirementTraitTag::Support => self.support_count,
        }
    }

    fn share_for_stat(self, stat: RequirementBandStat) -> f32 {
        match stat {
            RequirementBandStat::Tier0Share => self.tier0_share,
            RequirementBandStat::ShieldedShare => self.shielded_share,
            RequirementBandStat::FrontlineShare => self.frontline_share,
            RequirementBandStat::AntiCavalryShare => self.anti_cavalry_share,
            RequirementBandStat::SupportShare => self.support_share,
            RequirementBandStat::CavalryShare => self.cavalry_share,
            RequirementBandStat::ArcherShare => self.archer_share,
            RequirementBandStat::AntiArmorShare => self.anti_armor_share,
        }
    }

    fn band_for_stat(self, stat: RequirementBandStat) -> RequirementBand {
        requirement_band_from_share(self.share_for_stat(stat))
    }
}

fn requirement_eval_context(roster: Option<&RosterEconomy>) -> RequirementEvalContext {
    let Some(roster) = roster else {
        return RequirementEvalContext::default();
    };
    let total = roster.total_retinue_count.max(1);
    let mut shielded = 0u32;
    let mut frontline = 0u32;
    let mut anti_cavalry = 0u32;
    let mut cavalry = 0u32;
    let mut anti_armor = 0u32;
    let mut skirmisher = 0u32;
    let mut support = 0u32;
    let mut archer = 0u32;

    for (kind, count) in &roster.kind_counts {
        let count = *count;
        if kind.has_shielded_trait() {
            shielded = shielded.saturating_add(count);
        }
        if kind.has_role_tag(UnitRoleTag::Frontline) {
            frontline = frontline.saturating_add(count);
        }
        if kind.has_role_tag(UnitRoleTag::AntiCavalry) {
            anti_cavalry = anti_cavalry.saturating_add(count);
        }
        if kind.has_role_tag(UnitRoleTag::Cavalry) {
            cavalry = cavalry.saturating_add(count);
        }
        if kind.has_role_tag(UnitRoleTag::AntiArmor) {
            anti_armor = anti_armor.saturating_add(count);
        }
        if kind.has_role_tag(UnitRoleTag::Skirmisher) {
            skirmisher = skirmisher.saturating_add(count);
        }
        if kind.has_role_tag(UnitRoleTag::Support) {
            support = support.saturating_add(count);
        }
        if kind.is_archer_line() {
            archer = archer.saturating_add(count);
        }
    }

    let to_share = |count: u32| count as f32 / total as f32;
    RequirementEvalContext {
        tier0_share: roster_tier0_share(roster).clamp(0.0, 1.0),
        shielded_share: to_share(shielded),
        frontline_share: to_share(frontline),
        anti_cavalry_share: to_share(anti_cavalry),
        support_share: to_share(support),
        cavalry_share: to_share(cavalry),
        archer_share: to_share(archer),
        anti_armor_share: to_share(anti_armor),
        shielded_count: shielded,
        frontline_count: frontline,
        anti_cavalry_count: anti_cavalry,
        cavalry_count: cavalry,
        anti_armor_count: anti_armor,
        skirmisher_count: skirmisher,
        support_count: support,
    }
}

fn requirement_band_from_share(share: f32) -> RequirementBand {
    let clamped = share.clamp(0.0, 1.0);
    if clamped < 0.10 {
        RequirementBand::VeryLow
    } else if clamped < 0.25 {
        RequirementBand::Low
    } else if clamped < 0.45 {
        RequirementBand::Moderate
    } else if clamped < 0.70 {
        RequirementBand::High
    } else {
        RequirementBand::VeryHigh
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::data::{GameData, UpgradeConfig};
    use crate::formation::{
        ActiveFormation, FormationSkillBar, SKILL_BAR_CAPACITY, SkillBarSkill, SkillBarSkillKind,
    };
    use crate::inventory::ItemRarityRollBonus;
    use crate::model::GlobalBuffs;
    use crate::upgrades::{
        OneTimeUpgradeTracker, SkillBookLog, SkillTimingBuffs, UpgradeCardIcon,
        UpgradeRarityRollBonus, UpgradeRngState, UpgradeStackTracker, UpgradeValueTier,
        commander_level_hp_bonus, consume_hear_the_call_for_hero_recruit, progression_lock_reason,
        roll_upgrade_options, roll_upgrade_value, upgrade_card_icon, upgrade_display_description,
        upgrade_display_title, upgrade_value_tier,
    };

    fn upgrade(kind: &str, id: &str) -> UpgradeConfig {
        UpgradeConfig {
            id: id.to_string(),
            kind: kind.to_string(),
            value: 1.0,
            reward_lane: Some("minor".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: None,
            major_unlock_hint: None,
            min_value: None,
            max_value: None,
            value_step: None,
            weight_exponent: None,
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
            requirement_type: None,
            requirement_min_tier0_share: None,
            requirement_active_formation: None,
            requirement_map_tag: None,
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        }
    }

    fn empty_upgrade_bonus_state() -> (
        ItemRarityRollBonus,
        UpgradeRarityRollBonus,
        SkillTimingBuffs,
    ) {
        (
            ItemRarityRollBonus::default(),
            UpgradeRarityRollBonus::default(),
            SkillTimingBuffs::default(),
        )
    }

    fn requirement_eval_context_with_tier0_share(
        tier0_share: f32,
    ) -> super::RequirementEvalContext {
        super::RequirementEvalContext {
            tier0_share,
            ..Default::default()
        }
    }

    #[test]
    fn hero_recruit_consumes_exactly_one_hear_the_call_token() {
        let mut progression = super::Progression {
            hear_the_call_tokens: 2,
            ..Default::default()
        };
        assert!(consume_hear_the_call_for_hero_recruit(&mut progression));
        assert_eq!(progression.hear_the_call_tokens, 1);
        assert!(consume_hear_the_call_for_hero_recruit(&mut progression));
        assert_eq!(progression.hear_the_call_tokens, 0);
        assert!(!consume_hear_the_call_for_hero_recruit(&mut progression));
        assert_eq!(progression.hear_the_call_tokens, 0);
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
            super::UpgradeRewardKind::Minor,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
        assert_eq!(picks.len(), 3);
        let ids: HashSet<String> = picks.iter().map(|picked| picked.id.clone()).collect();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn one_time_options_are_downweighted_in_draft_selection() {
        let mut pool = Vec::new();
        for idx in 0..8 {
            let mut unique = upgrade("mob_fury", &format!("unique_{idx}"));
            unique.one_time = true;
            unique.min_value = None;
            unique.max_value = None;
            pool.push(unique);
        }
        for idx in 0..8 {
            pool.push(upgrade("damage", &format!("normal_{idx}")));
        }

        let mut rng = UpgradeRngState {
            state: 0x1234_5678_ABCD_EF01,
        };
        let mut one_time_seen = 0u32;
        let mut total_seen = 0u32;
        for _ in 0..1_000 {
            let picks = roll_upgrade_options(
                &pool,
                &mut rng,
                5,
                super::UpgradeRewardKind::Minor,
                0.0,
                &OneTimeUpgradeTracker::default(),
                &UpgradeStackTracker::default(),
                &FormationSkillBar::default(),
            );
            total_seen += picks.len() as u32;
            one_time_seen += picks.iter().filter(|entry| entry.one_time).count() as u32;
        }
        let ratio = one_time_seen as f32 / total_seen as f32;
        assert!(ratio < 0.24, "one-time ratio too high: {ratio}");
    }

    #[test]
    fn deterministic_roll_always_returns_authored_value() {
        let mut authored = upgrade("damage", "authored");
        authored.value = 7.5;
        let mut rng = UpgradeRngState {
            state: 0xCAFE_BABE_0123_4567,
        };
        for _ in 0..64 {
            let value = roll_upgrade_value(&authored, &mut rng, true, 0.8);
            assert!((value - 7.5).abs() < 0.0001);
        }
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
    fn upgrade_value_tier_maps_lane_and_one_time_contract() {
        let mut minor = upgrade("damage", "minor_common");
        minor.value = 2.0;
        assert_eq!(upgrade_value_tier(&minor), UpgradeValueTier::Common);

        minor.value = 6.0;
        assert_eq!(upgrade_value_tier(&minor), UpgradeValueTier::Uncommon);

        minor.value = 12.0;
        assert_eq!(upgrade_value_tier(&minor), UpgradeValueTier::Rare);

        let mut major = upgrade("mob_fury", "major_lane");
        major.reward_lane = Some("major".to_string());
        major.one_time = false;
        assert_eq!(upgrade_value_tier(&major), UpgradeValueTier::Epic);

        major.one_time = true;
        assert_eq!(upgrade_value_tier(&major), UpgradeValueTier::Unique);
    }

    #[test]
    fn one_time_upgrade_is_removed_after_pick() {
        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Consumes a major doctrine pick.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        };
        let pool = vec![formation_upgrade];
        let mut tracker = OneTimeUpgradeTracker::default();
        tracker.acquired_ids.insert("unlock_diamond".to_string());
        let mut rng = UpgradeRngState {
            state: 0xBEEF_1234_9876_1111,
        };
        let picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            super::UpgradeRewardKind::Minor,
            0.0,
            &tracker,
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
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
        let picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            super::UpgradeRewardKind::Minor,
            0.0,
            &tracker,
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );

        assert!(!picks.iter().any(|upgrade| upgrade.one_time));
        assert!(picks.iter().any(|upgrade| upgrade.id == "normal_damage"));
    }

    #[test]
    fn unique_slot_tradeoff_extends_unique_cap_and_marks_tradeoff_state() {
        let mut tracker = OneTimeUpgradeTracker::default();
        let tradeoff = UpgradeConfig {
            id: "war_council_edict".to_string(),
            kind: "unique_slot_tradeoff".to_string(),
            value: 2.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Reduced doctrine flexibility.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        };
        super::apply_one_time_tracker_effects(&tradeoff, &mut tracker);
        assert_eq!(
            super::effective_max_unique_upgrades(&tracker),
            super::MAX_UNIQUE_UPGRADES + 2
        );
        assert!(tracker.mythical_rolls_locked);

        for idx in 0..(super::MAX_UNIQUE_UPGRADES + 1) {
            tracker.acquired_ids.insert(format!("picked_{idx}"));
        }
        let mut extra_unique = upgrade("mob_fury", "fresh_unique_pick");
        extra_unique.one_time = true;
        extra_unique.min_value = None;
        extra_unique.max_value = None;
        let normal = upgrade("damage", "normal_damage");
        let pool = vec![extra_unique, normal];
        let mut rng = UpgradeRngState {
            state: 0xA11C_E55E_1234_5678,
        };
        let picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            super::UpgradeRewardKind::Minor,
            0.0,
            &tracker,
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
        assert!(picks.iter().any(|upgrade| upgrade.one_time));
    }

    #[test]
    fn skillbar_bound_upgrades_are_filtered_when_hotbar_is_full() {
        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Consumes a major doctrine pick.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
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
            super::UpgradeRewardKind::Minor,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &UpgradeStackTracker::default(),
            &full_skillbar,
        );
        assert_eq!(picks.len(), 1);
        assert_eq!(picks[0].id, "damage_up");
    }

    #[test]
    fn formation_unlock_upgrades_only_roll_on_major_rewards() {
        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Consumes a major doctrine pick.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        };
        let pool = vec![formation_upgrade];
        let mut rng = UpgradeRngState {
            state: 0x00AB_CDEF_1234_5678,
        };

        let minor_picks = roll_upgrade_options(
            &pool,
            &mut rng,
            1,
            super::UpgradeRewardKind::Minor,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
        assert!(minor_picks.is_empty());

        let major_picks = roll_upgrade_options(
            &pool,
            &mut rng,
            1,
            super::UpgradeRewardKind::Major,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
        assert_eq!(major_picks.len(), 1);
        assert_eq!(major_picks[0].kind, "unlock_formation");
    }

    #[test]
    fn reward_lane_routing_keeps_major_and_minor_pools_separate() {
        let mut minor = upgrade("damage", "minor_damage");
        minor.reward_lane = Some("minor".to_string());

        let mut major = upgrade("mob_fury", "major_doctrine");
        major.reward_lane = Some("major".to_string());
        major.one_time = true;
        major.downside = Some("Doctrine lock-in.".to_string());

        let pool = vec![minor, major];
        let mut rng = UpgradeRngState {
            state: 0x0A0B_0C0D_0E0F_1011,
        };
        let minor_picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            super::UpgradeRewardKind::Minor,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
        assert_eq!(minor_picks.len(), 1);
        assert_eq!(minor_picks[0].id, "minor_damage");

        let major_picks = roll_upgrade_options(
            &pool,
            &mut rng,
            3,
            super::UpgradeRewardKind::Major,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &UpgradeStackTracker::default(),
            &FormationSkillBar::default(),
        );
        assert_eq!(major_picks.len(), 1);
        assert_eq!(major_picks[0].id, "major_doctrine");
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
    fn stack_cap_blocks_additional_draft_appearance() {
        let mut capped = upgrade("damage", "damage_capped");
        capped.stack_cap = Some(2);
        let mut tracker = UpgradeStackTracker::default();
        tracker.increment("damage_capped");
        tracker.increment("damage_capped");

        let mut rng = UpgradeRngState {
            state: 0xA5A5_5A5A_1234_9876,
        };
        let picks = roll_upgrade_options(
            &[capped],
            &mut rng,
            3,
            super::UpgradeRewardKind::Minor,
            0.0,
            &OneTimeUpgradeTracker::default(),
            &tracker,
            &FormationSkillBar::default(),
        );
        assert!(picks.is_empty());
    }

    #[test]
    fn diminishing_factor_reduces_value_per_stack() {
        let mut config = upgrade("armor", "armor_diminishing");
        config.value = 4.0;
        config.diminishing_factor = Some(0.75);
        assert!((super::effective_upgrade_value(&config, 0) - 4.0).abs() < 0.001);
        assert!((super::effective_upgrade_value(&config, 1) - 3.0).abs() < 0.001);
        assert!((super::effective_upgrade_value(&config, 2) - 2.25).abs() < 0.001);
    }

    #[test]
    fn formation_breach_upgrade_enables_inside_formation_damage_bonus() {
        let mut buffs = GlobalBuffs::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();
        let upgrade = UpgradeConfig {
            id: "encirclement_doctrine".to_string(),
            kind: "formation_breach".to_string(),
            value: 20.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Only applies when enemies breach formation.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        };

        super::apply_upgrade(
            &upgrade,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        assert!((buffs.inside_formation_damage_multiplier - 1.2).abs() < 0.001);
    }

    #[test]
    fn command_net_doctrine_applies_upside_and_downside() {
        let mut buffs = GlobalBuffs::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();

        let mut doctrine = upgrade("doctrine_command_net", "doctrine_command_net");
        doctrine.reward_lane = Some("major".to_string());
        doctrine.one_time = true;
        doctrine.value = 26.0;
        doctrine.downside = Some("Lower base damage while active.".to_string());

        super::apply_upgrade(
            &doctrine,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );

        assert!(buffs.commander_aura_radius_bonus >= 26.0);
        assert!(buffs.authority_enemy_morale_drain_per_sec > 0.0);
        assert!(buffs.damage_multiplier < 1.0);
    }

    #[test]
    fn quartermaster_upgrade_stacks_gold_gain_multiplier() {
        let mut buffs = GlobalBuffs::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();
        let upgrade = UpgradeConfig {
            id: "quartermaster_up".to_string(),
            kind: "quartermaster".to_string(),
            value: 0.08,
            reward_lane: Some("minor".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: None,
            major_unlock_hint: None,
            min_value: None,
            max_value: None,
            value_step: None,
            weight_exponent: None,
            one_time: false,
            adds_to_skillbar: false,
            formation_id: None,
            requirement_type: None,
            requirement_min_tier0_share: None,
            requirement_active_formation: None,
            requirement_map_tag: None,
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        };
        super::apply_upgrade(
            &upgrade,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        super::apply_upgrade(
            &upgrade,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        assert!((buffs.gold_gain_multiplier - 1.16).abs() < 0.001);
    }

    #[test]
    fn countervolley_doctrine_applies_conditional_upside_and_downside() {
        let mut buffs = GlobalBuffs::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();

        let mut doctrine = upgrade("doctrine_countervolley", "doctrine_countervolley");
        doctrine.reward_lane = Some("major".to_string());
        doctrine.one_time = true;
        doctrine.requirement_type = Some("band_at_least".to_string());
        doctrine.requirement_band_stat = Some("archer_share".to_string());
        doctrine.requirement_band_at_least = Some("moderate".to_string());
        doctrine.downside = Some("Higher morale loss while pressured.".to_string());

        super::apply_upgrade(
            &doctrine,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );

        let eval_context = super::RequirementEvalContext {
            archer_share: 0.45,
            ..Default::default()
        };
        let (effects, status) = super::conditional_effects_from_owned(
            &conditional.entries,
            &eval_context,
            ActiveFormation::Square,
        );

        assert!(
            status
                .iter()
                .any(|entry| entry.id == "doctrine_countervolley" && entry.active)
        );
        assert!(effects.friendly_damage_multiplier > 1.0);
        assert!(effects.friendly_attack_speed_multiplier > 1.0);
        assert!(effects.friendly_morale_loss_multiplier > 1.0);
    }

    #[test]
    fn item_and_upgrade_rarity_upgrades_update_roll_bonus_resources() {
        let mut buffs = GlobalBuffs::default();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();

        let mut item_upgrade = upgrade("item_rarity", "item_rarity_up");
        item_upgrade.value = 0.08;
        super::apply_upgrade(
            &item_upgrade,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        assert!((item_rarity.percent - 0.08).abs() < 0.001);

        let mut upgrade_upgrade = upgrade("upgrade_rarity", "upgrade_rarity_up");
        upgrade_upgrade.value = 0.09;
        super::apply_upgrade(
            &upgrade_upgrade,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        assert!((upgrade_rarity.percent - 0.09).abs() < 0.001);
    }

    #[test]
    fn luck_upgrade_distributes_into_crit_and_rarity_systems() {
        let mut buffs = GlobalBuffs::default();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();

        let mut luck = upgrade("luck", "luck_up");
        luck.value = 0.10;
        super::apply_upgrade(
            &luck,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );

        assert!((buffs.luck_bonus - 0.10).abs() < 0.001);
        assert!((buffs.crit_chance_bonus - 0.05).abs() < 0.001);
        assert!((buffs.crit_damage_multiplier - 1.4).abs() < 0.001);
        assert!((item_rarity.percent - 0.10).abs() < 0.001);
        assert!((upgrade_rarity.percent - 0.10).abs() < 0.001);
    }

    #[test]
    fn upgrade_rarity_bonus_does_not_change_authored_value() {
        let mut cfg = upgrade("damage", "rarity_shift");
        cfg.value = 12.0;
        let mut baseline_rng = UpgradeRngState {
            state: 0x89AB_CDEF_0123_4567,
        };
        let mut boosted_rng = UpgradeRngState {
            state: 0x89AB_CDEF_0123_4567,
        };
        let mut baseline_sum = 0.0;
        let mut boosted_sum = 0.0;
        for _ in 0..1024 {
            baseline_sum += roll_upgrade_value(&cfg, &mut baseline_rng, true, 0.0);
            boosted_sum += roll_upgrade_value(&cfg, &mut boosted_rng, true, 0.20);
        }
        assert!((baseline_sum - boosted_sum).abs() < 0.001);
    }

    #[test]
    fn crit_upgrades_apply_to_global_buffs_with_expected_bounds() {
        let mut buffs = GlobalBuffs::default();
        let (mut item_rarity, mut upgrade_rarity, mut skill_timing) = empty_upgrade_bonus_state();
        let mut conditional = super::ConditionalUpgradeOwnership::default();
        let mut skillbar = FormationSkillBar::default();

        let mut crit_chance = upgrade("crit_chance", "crit_chance_up");
        crit_chance.value = 0.60;
        super::apply_upgrade(
            &crit_chance,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        super::apply_upgrade(
            &crit_chance,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        assert!((buffs.crit_chance_bonus - 0.95).abs() < 0.001);

        let mut crit_damage = upgrade("crit_damage", "crit_damage_up");
        crit_damage.value = 0.20;
        super::apply_upgrade(
            &crit_damage,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        super::apply_upgrade(
            &crit_damage,
            &mut buffs,
            &mut item_rarity,
            &mut upgrade_rarity,
            &mut skill_timing,
            &mut conditional,
            &mut skillbar,
        );
        assert!((buffs.crit_damage_multiplier - 1.60).abs() < 0.001);
    }

    #[test]
    fn wave_98_grants_double_level_reward() {
        assert_eq!(super::level_rewards_for_wave_completion(0), 0);
        assert_eq!(super::level_rewards_for_wave_completion(1), 1);
        assert_eq!(super::level_rewards_for_wave_completion(97), 1);
        assert_eq!(super::level_rewards_for_wave_completion(98), 2);
        assert_eq!(super::level_rewards_for_wave_completion(99), 1);
    }

    #[test]
    fn wave_98_reward_reaches_level_100_by_end_of_wave_98() {
        let mut level = 1u32;
        for wave in 1..=98 {
            level += super::level_rewards_for_wave_completion(wave);
        }
        assert_eq!(level, 100);
    }

    #[test]
    fn reward_kind_switches_to_major_on_every_fifth_level() {
        assert_eq!(
            super::reward_kind_for_level(1),
            super::UpgradeRewardKind::Minor
        );
        assert_eq!(
            super::reward_kind_for_level(4),
            super::UpgradeRewardKind::Minor
        );
        assert_eq!(
            super::reward_kind_for_level(5),
            super::UpgradeRewardKind::Major
        );
        assert_eq!(
            super::reward_kind_for_level(10),
            super::UpgradeRewardKind::Major
        );
        assert_eq!(
            super::reward_kind_for_level(11),
            super::UpgradeRewardKind::Minor
        );
    }

    #[test]
    fn major_minor_counts_follow_shared_level_parity_formula() {
        assert_eq!(super::major_minor_reward_counts_for_level(1), (0, 1));
        assert_eq!(super::major_minor_reward_counts_for_level(5), (1, 4));
        assert_eq!(super::major_minor_reward_counts_for_level(30), (6, 24));
        assert_eq!(super::major_minor_reward_counts_for_level(100), (20, 80));
    }

    #[test]
    fn reward_queue_order_is_deterministic_for_minor_then_major_levels() {
        let mut queue = std::collections::VecDeque::new();
        let queued = super::enqueue_reward_kinds(&mut queue, 98, 2);
        assert_eq!(queued, 2);
        assert_eq!(queue.pop_front(), Some(super::UpgradeRewardKind::Minor));
        assert_eq!(queue.pop_front(), Some(super::UpgradeRewardKind::Major));
        assert!(queue.is_empty());
    }

    #[test]
    fn reward_queue_respects_level_cap_when_enqueuing() {
        let mut queue = std::collections::VecDeque::new();
        let queued = super::enqueue_reward_kinds(&mut queue, 99, 3);
        assert_eq!(queued, 1);
        assert_eq!(queue.pop_front(), Some(super::UpgradeRewardKind::Major));
        assert!(queue.is_empty());

        let mut capped_queue = std::collections::VecDeque::new();
        let queued_at_cap = super::enqueue_reward_kinds(&mut capped_queue, 100, 2);
        assert_eq!(queued_at_cap, 0);
        assert!(capped_queue.is_empty());
    }

    #[test]
    fn commander_level_hp_bonus_increases_linearly() {
        assert_eq!(commander_level_hp_bonus(1), 0.0);
        assert_eq!(commander_level_hp_bonus(6), 5.0);
    }

    #[test]
    fn progression_lock_reason_engages_and_clears_with_budget() {
        let locked = progression_lock_reason(99, 99);
        assert!(locked.is_some());

        let unlocked = progression_lock_reason(80, 99);
        assert!(unlocked.is_none());

        let hard_cap_only = progression_lock_reason(100, 100);
        assert!(hard_cap_only.is_none());
    }

    #[test]
    fn requirement_evaluator_handles_tier0_and_formation_conditions() {
        let tier_gate = super::UpgradeRequirement::Tier0Share { min_share: 0.75 };
        let (tier_inactive, tier_reason) = super::evaluate_upgrade_requirement(
            &tier_gate,
            &requirement_eval_context_with_tier0_share(0.4),
            ActiveFormation::Square,
        );
        assert!(!tier_inactive);
        assert!(
            tier_reason
                .as_deref()
                .unwrap_or_default()
                .contains("tier-0 share")
        );

        let (tier_active, tier_reason_active) = super::evaluate_upgrade_requirement(
            &tier_gate,
            &requirement_eval_context_with_tier0_share(0.9),
            ActiveFormation::Square,
        );
        assert!(tier_active);
        assert!(tier_reason_active.is_none());

        let formation_gate = super::UpgradeRequirement::FormationActive {
            formation: ActiveFormation::Diamond,
        };
        let eval_context = requirement_eval_context_with_tier0_share(1.0);
        let (formation_inactive, _) = super::evaluate_upgrade_requirement(
            &formation_gate,
            &eval_context,
            ActiveFormation::Square,
        );
        assert!(!formation_inactive);
        let (formation_active, _) = super::evaluate_upgrade_requirement(
            &formation_gate,
            &eval_context,
            ActiveFormation::Diamond,
        );
        assert!(formation_active);
    }

    #[test]
    fn requirement_evaluator_handles_trait_and_band_conditions() {
        let has_shielded = super::UpgradeRequirement::HasTrait {
            trait_tag: super::RequirementTraitTag::Shielded,
            min_count: 1,
        };
        let mut trait_context = super::RequirementEvalContext {
            shielded_count: 0,
            ..Default::default()
        };
        let (inactive, reason) = super::evaluate_upgrade_requirement(
            &has_shielded,
            &trait_context,
            ActiveFormation::Square,
        );
        assert!(!inactive);
        assert!(reason.as_deref().unwrap_or_default().contains("Shielded"));

        trait_context.shielded_count = 2;
        let (active, active_reason) = super::evaluate_upgrade_requirement(
            &has_shielded,
            &trait_context,
            ActiveFormation::Square,
        );
        assert!(active);
        assert!(active_reason.is_none());

        let band_gate = super::UpgradeRequirement::BandAtLeast {
            stat: super::RequirementBandStat::FrontlineShare,
            band: super::RequirementBand::Moderate,
        };
        let mut band_context = super::RequirementEvalContext {
            frontline_share: 0.12,
            ..Default::default()
        };
        let (band_inactive, band_reason) =
            super::evaluate_upgrade_requirement(&band_gate, &band_context, ActiveFormation::Square);
        assert!(!band_inactive);
        assert!(
            band_reason
                .as_deref()
                .unwrap_or_default()
                .contains("Frontline")
        );

        band_context.frontline_share = 0.34;
        let (band_active, _) =
            super::evaluate_upgrade_requirement(&band_gate, &band_context, ActiveFormation::Square);
        assert!(band_active);
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
        let (active_effects, active_status) = super::conditional_effects_from_owned(
            &entries,
            &requirement_eval_context_with_tier0_share(1.0),
            ActiveFormation::Square,
        );
        assert!((active_effects.friendly_morale_loss_multiplier - 0.75).abs() < 0.001);
        assert!((active_effects.friendly_damage_multiplier - 1.18).abs() < 0.001);
        assert_eq!(active_status.len(), 2);
        assert!(active_status.iter().all(|entry| entry.active));

        let (inactive_effects, inactive_status) = super::conditional_effects_from_owned(
            &entries,
            &requirement_eval_context_with_tier0_share(0.2),
            ActiveFormation::Square,
        );
        assert!((inactive_effects.friendly_morale_loss_multiplier - 1.0).abs() < 0.001);
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

        let (active_effects, active_status) = super::conditional_effects_from_owned(
            &entries,
            &requirement_eval_context_with_tier0_share(1.0),
            ActiveFormation::Square,
        );
        assert!((active_effects.rescue_time_multiplier - 0.5).abs() < 0.001);
        assert_eq!(active_effects.execute_below_health_ratio, 0.0);
        assert!(active_status.iter().all(|entry| entry.active));

        let (inactive_effects, inactive_status) = super::conditional_effects_from_owned(
            &entries,
            &requirement_eval_context_with_tier0_share(0.75),
            ActiveFormation::Square,
        );
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

        let (effects, status) = super::conditional_effects_from_owned(
            &entries,
            &requirement_eval_context_with_tier0_share(1.0),
            ActiveFormation::Square,
        );
        assert!((effects.friendly_morale_loss_multiplier - 0.75).abs() < 0.001);
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

        let mut luck_upgrade = upgrade("luck", "luck_up");
        luck_upgrade.value = 0.06;
        assert_eq!(upgrade_card_icon(&luck_upgrade), UpgradeCardIcon::Luck);
        assert_eq!(upgrade_display_title(&luck_upgrade), "Fortune's Favor");
        assert!(upgrade_display_description(&luck_upgrade).contains("Luck"));

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

        let mut quartermaster_upgrade = upgrade("quartermaster", "quartermaster_up");
        quartermaster_upgrade.value = 0.08;
        assert_eq!(
            upgrade_card_icon(&quartermaster_upgrade),
            UpgradeCardIcon::FastLearner
        );
        assert_eq!(
            upgrade_display_title(&quartermaster_upgrade),
            "Quartermaster"
        );
        assert!(
            upgrade_display_description(&quartermaster_upgrade)
                .to_lowercase()
                .contains("gold")
        );

        let unique_slots_upgrade = UpgradeConfig {
            id: "war_council_edict".to_string(),
            kind: "unique_slot_tradeoff".to_string(),
            value: 2.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Limits doctrine flexibility later in run.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
        };
        assert_eq!(
            upgrade_display_title(&unique_slots_upgrade),
            "War Council Edict"
        );
        assert!(upgrade_display_description(&unique_slots_upgrade).contains("flexibility"));

        let formation_upgrade = UpgradeConfig {
            id: "unlock_diamond".to_string(),
            kind: "unlock_formation".to_string(),
            value: 1.0,
            reward_lane: Some("major".to_string()),
            doctrine_tags: Vec::new(),
            stack_cap: None,
            diminishing_factor: None,
            downside: Some("Consumes a major doctrine pick.".to_string()),
            major_unlock_hint: None,
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
            requirement_trait: None,
            requirement_band_stat: None,
            requirement_band_at_least: None,
            requirement_band_at_most: None,
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

    #[test]
    fn configured_upgrade_kinds_are_all_supported() {
        let data =
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("load game data");
        for upgrade in &data.upgrades.upgrades {
            assert!(
                super::is_supported_upgrade_kind(&upgrade.kind),
                "unsupported upgrade kind in data: {}",
                upgrade.kind
            );
        }
    }
}
