use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::banner::BannerState;
use crate::model::{GameState, PlayerFaction, StartRunEvent, UnitKind};
use crate::upgrades::{Progression, SkillTimingBuffs, commander_level_hp_bonus};

pub const BACKPACK_ROWS: usize = 5;
pub const BACKPACK_COLS: usize = 6;
pub const BACKPACK_SLOT_CAPACITY: usize = BACKPACK_ROWS * BACKPACK_COLS;
pub const CHEST_SLOT_CAPACITY: usize = 3;

const ITEM_ROLL_BUDGET: u8 = 6;
const DRUM_EFFECT_DURATION_SECS: f32 = 5.0;
const DRUM_BASE_COOLDOWN_SECS: f32 = 20.0;
const CHANT_EFFECT_DURATION_SECS: f32 = 5.0;
const CHANT_BASE_COOLDOWN_SECS: f32 = 20.0;
const MIN_ARMY_SKILL_COOLDOWN_SECS: f32 = 3.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EquipmentUnitType {
    Commander,
    Tier0,
    Tier1,
    Tier2,
    Tier3,
    Tier4,
    Tier5,
    Hero,
}

impl EquipmentUnitType {
    pub const fn all() -> [Self; 8] {
        [
            Self::Commander,
            Self::Tier0,
            Self::Tier1,
            Self::Tier2,
            Self::Tier3,
            Self::Tier4,
            Self::Tier5,
            Self::Hero,
        ]
    }

    pub const fn from_tier(tier: u8) -> Option<Self> {
        match tier {
            0 => Some(Self::Tier0),
            1 => Some(Self::Tier1),
            2 => Some(Self::Tier2),
            3 => Some(Self::Tier3),
            4 => Some(Self::Tier4),
            5 => Some(Self::Tier5),
            6 => Some(Self::Hero),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GearRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Mythical,
    Unique,
}

impl GearRarity {
    pub const fn points(self) -> u8 {
        match self {
            Self::Common => 1,
            Self::Uncommon => 2,
            Self::Rare => 3,
            Self::Epic => 4,
            Self::Mythical => 5,
            Self::Unique => 6,
        }
    }

    pub const fn from_points(points: u8) -> Self {
        match points {
            1 => Self::Common,
            2 => Self::Uncommon,
            3 => Self::Rare,
            4 => Self::Epic,
            5 => Self::Mythical,
            _ => Self::Unique,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Uncommon => "Uncommon",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Mythical => "Mythical",
            Self::Unique => "Unique",
        }
    }

    pub const fn scalar(self) -> f32 {
        match self {
            Self::Common => 0.06,
            Self::Uncommon => 0.12,
            Self::Rare => 0.216,
            Self::Epic => 0.408,
            Self::Mythical => 0.624,
            Self::Unique => 0.90,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GearItemType {
    Banner,
    Instrument,
    Chant,
    Squire,
    Symbol,
    MeleeWeapon,
    RangedWeapon,
    Armor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GearStatKind {
    DamagePercent,
    AttackSpeedPercent,
    ArmorFlat,
    RangedRangeFlat,
    MoveSpeedFlat,
    CommanderMaxHealthFlat,
    MoraleRegenPerSec,
    CohesionRegenPerSec,
    MoraleLossResistancePercent,
    CohesionLossResistancePercent,
    AuraRangeFlat,
    AuraEnemyEffectPercent,
    CooldownReductionSecs,
    ArmorPulseFlat,
    SquirePrimary,
    SquireSecondary,
}

impl GearStatKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DamagePercent => "Damage",
            Self::AttackSpeedPercent => "Attack Speed",
            Self::ArmorFlat => "Armor",
            Self::RangedRangeFlat => "Ranged Range",
            Self::MoveSpeedFlat => "Move Speed",
            Self::CommanderMaxHealthFlat => "Unit Health",
            Self::MoraleRegenPerSec => "Morale Regen/s",
            Self::CohesionRegenPerSec => "Cohesion Regen/s",
            Self::MoraleLossResistancePercent => "Morale Loss Resist",
            Self::CohesionLossResistancePercent => "Cohesion Loss Resist",
            Self::AuraRangeFlat => "Aura Range",
            Self::AuraEnemyEffectPercent => "Aura Enemy Effect",
            Self::CooldownReductionSecs => "Cooldown Reduction",
            Self::ArmorPulseFlat => "Armor Pulse",
            Self::SquirePrimary => "Squire Primary",
            Self::SquireSecondary => "Squire Secondary",
        }
    }

    pub const fn base_magnitude(self) -> f32 {
        match self {
            Self::DamagePercent => 0.08,
            Self::AttackSpeedPercent => 0.07,
            Self::ArmorFlat => 4.0,
            Self::RangedRangeFlat => 22.0,
            Self::MoveSpeedFlat => 14.0,
            Self::CommanderMaxHealthFlat => 18.0,
            Self::MoraleRegenPerSec => 0.35,
            Self::CohesionRegenPerSec => 0.30,
            Self::MoraleLossResistancePercent => 0.08,
            Self::CohesionLossResistancePercent => 0.08,
            Self::AuraRangeFlat => 24.0,
            Self::AuraEnemyEffectPercent => 0.12,
            Self::CooldownReductionSecs => 1.2,
            Self::ArmorPulseFlat => 5.0,
            Self::SquirePrimary => 0.09,
            Self::SquireSecondary => 0.07,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum GearSpecialEffectKind {
    DrumArmorPulse,
    BattleChantMoraleImmunity,
}

impl GearSpecialEffectKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DrumArmorPulse => "Drum: +Armor pulse (5s / 20s)",
            Self::BattleChantMoraleImmunity => "Battle Song: Morale-loss immunity (5s / 20s)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GearRolledStat {
    pub kind: GearStatKind,
    pub rarity: GearRarity,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GearItemEntry {
    pub instance_id: String,
    pub template_id: String,
    pub name: String,
    pub description: String,
    pub icon_key: String,
    pub item_type: GearItemType,
    pub faction: Option<PlayerFaction>,
    pub stats: Vec<GearRolledStat>,
    pub special_effect: Option<GearSpecialEffectKind>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquippedSlot {
    pub slot_id: String,
    pub display_name: String,
    pub accepted_item_types: Vec<GearItemType>,
    pub item: Option<GearItemEntry>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitEquipmentSetup {
    pub unit_type: EquipmentUnitType,
    pub slots: Vec<EquippedSlot>,
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    pub bag_slots: Vec<Option<GearItemEntry>>,
    pub setups: Vec<UnitEquipmentSetup>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct EquipmentChestState {
    pub slots: Vec<Option<GearItemEntry>>,
}

impl EquipmentChestState {
    pub fn clear(&mut self) {
        self.slots = vec![None; CHEST_SLOT_CAPACITY];
    }

    pub fn ensure_capacity(&mut self) {
        if self.slots.len() != CHEST_SLOT_CAPACITY {
            self.clear();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(Option::is_none)
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct InventoryRngState {
    state: u64,
    next_item_serial: u64,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ItemRarityRollBonus {
    pub percent: f32,
}

impl Default for InventoryRngState {
    fn default() -> Self {
        Self {
            state: 0xBADC_0FFE_FEED_F00D,
            next_item_serial: 1,
        }
    }
}

impl InventoryRngState {
    fn reseed_from_time(&mut self) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0xFACE_FEED_AA55_11AA);
        self.state = nanos ^ 0x9E37_79B9_7F4A_7C15;
        if self.state == 0 {
            self.state = 0xBADC_0FFE_FEED_F00D;
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 32) as u32
    }

    pub fn next_u32_roll(&mut self) -> u32 {
        self.next_u32()
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn next_item_instance_id(&mut self, template_id: &str) -> String {
        let value = self.next_item_serial;
        self.next_item_serial = self.next_item_serial.saturating_add(1);
        format!("{template_id}_{value:08}")
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct EquipmentArmyEffects {
    pub temporary_armor_bonus: f32,
    pub morale_loss_immunity: bool,
    pub drum_active_secs: f32,
    pub drum_cooldown_secs: f32,
    pub chant_active_secs: f32,
    pub chant_cooldown_secs: f32,
}

impl Default for EquipmentArmyEffects {
    fn default() -> Self {
        Self {
            temporary_armor_bonus: 0.0,
            morale_loss_immunity: false,
            drum_active_secs: 0.0,
            drum_cooldown_secs: DRUM_BASE_COOLDOWN_SECS,
            chant_active_secs: 0.0,
            chant_cooldown_secs: CHANT_BASE_COOLDOWN_SECS,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventorySlotRef {
    Backpack(usize),
    Equipment {
        unit_type: EquipmentUnitType,
        slot_index: usize,
    },
    Chest(usize),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryPlaceError {
    InvalidSlot,
    ItemTypeMismatch,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UnitEquipmentBonuses {
    pub health_bonus: f32,
    pub melee_damage_multiplier: f32,
    pub ranged_damage_multiplier: f32,
    pub attack_speed_multiplier: f32,
    pub armor_bonus: f32,
    pub ranged_range_bonus: f32,
    pub move_speed_bonus: f32,
    pub aura_radius_bonus: f32,
    pub aura_enemy_effect_bonus_multiplier: f32,
    pub cooldown_reduction_secs: f32,
    pub morale_regen_per_sec: f32,
    pub cohesion_regen_per_sec: f32,
    pub morale_loss_resistance: f32,
    pub cohesion_loss_resistance: f32,
    pub drum_armor_pulse_flat: f32,
}

impl UnitEquipmentBonuses {
    pub const fn zero() -> Self {
        Self {
            health_bonus: 0.0,
            melee_damage_multiplier: 0.0,
            ranged_damage_multiplier: 0.0,
            attack_speed_multiplier: 0.0,
            armor_bonus: 0.0,
            ranged_range_bonus: 0.0,
            move_speed_bonus: 0.0,
            aura_radius_bonus: 0.0,
            aura_enemy_effect_bonus_multiplier: 0.0,
            cooldown_reduction_secs: 0.0,
            morale_regen_per_sec: 0.0,
            cohesion_regen_per_sec: 0.0,
            morale_loss_resistance: 0.0,
            cohesion_loss_resistance: 0.0,
            drum_armor_pulse_flat: 0.0,
        }
    }

    fn merge_assign(&mut self, other: Self) {
        self.health_bonus += other.health_bonus;
        self.melee_damage_multiplier += other.melee_damage_multiplier;
        self.ranged_damage_multiplier += other.ranged_damage_multiplier;
        self.attack_speed_multiplier += other.attack_speed_multiplier;
        self.armor_bonus += other.armor_bonus;
        self.ranged_range_bonus += other.ranged_range_bonus;
        self.move_speed_bonus += other.move_speed_bonus;
        self.aura_radius_bonus += other.aura_radius_bonus;
        self.aura_enemy_effect_bonus_multiplier += other.aura_enemy_effect_bonus_multiplier;
        self.cooldown_reduction_secs += other.cooldown_reduction_secs;
        self.morale_regen_per_sec += other.morale_regen_per_sec;
        self.cohesion_regen_per_sec += other.cohesion_regen_per_sec;
        self.morale_loss_resistance += other.morale_loss_resistance;
        self.cohesion_loss_resistance += other.cohesion_loss_resistance;
        self.drum_armor_pulse_flat += other.drum_armor_pulse_flat;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitCombatRole {
    Commander,
    Melee,
    Ranged,
    Support,
}

#[derive(Clone, Copy, Debug)]
struct GearTemplate {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon_key: &'static str,
    item_type: GearItemType,
    stats: [GearStatKind; 2],
    special_effect: Option<GearSpecialEffectKind>,
    faction: Option<PlayerFaction>,
}

const BASE_RARITY_WEIGHTS: [(GearRarity, f32); 6] = [
    (GearRarity::Common, 0.45),
    (GearRarity::Uncommon, 0.28),
    (GearRarity::Rare, 0.16),
    (GearRarity::Epic, 0.07),
    (GearRarity::Mythical, 0.03),
    (GearRarity::Unique, 0.01),
];

const TEMPLATES: [GearTemplate; 9] = [
    GearTemplate {
        id: "banner_sturdy_pole",
        name: "Banner on Sturdy Pole",
        description: "Army banner balancing sustain versus loss resistance.",
        icon_key: "item_banner",
        item_type: GearItemType::Banner,
        stats: [
            GearStatKind::MoraleRegenPerSec,
            GearStatKind::CohesionLossResistancePercent,
        ],
        special_effect: None,
        faction: None,
    },
    GearTemplate {
        id: "instrument_drum",
        name: "War Drum",
        description: "Triggers a periodic armor pulse for the army.",
        icon_key: "item_instrument",
        item_type: GearItemType::Instrument,
        stats: [
            GearStatKind::ArmorPulseFlat,
            GearStatKind::CommanderMaxHealthFlat,
        ],
        special_effect: Some(GearSpecialEffectKind::DrumArmorPulse),
        faction: None,
    },
    GearTemplate {
        id: "chant_battle_song",
        name: "Battle Song",
        description: "Periodically grants temporary morale-loss immunity.",
        icon_key: "item_chant",
        item_type: GearItemType::Chant,
        stats: [
            GearStatKind::CooldownReductionSecs,
            GearStatKind::CommanderMaxHealthFlat,
        ],
        special_effect: Some(GearSpecialEffectKind::BattleChantMoraleImmunity),
        faction: None,
    },
    GearTemplate {
        id: "squire_young",
        name: "Young Squire",
        description: "Role-based support stats for the assigned setup.",
        icon_key: "item_squire",
        item_type: GearItemType::Squire,
        stats: [GearStatKind::SquirePrimary, GearStatKind::SquireSecondary],
        special_effect: None,
        faction: None,
    },
    GearTemplate {
        id: "symbol_cross",
        name: "Holy Cross",
        description: "Empowers aura effects against enemies and extends aura reach.",
        icon_key: "item_symbol_cross",
        item_type: GearItemType::Symbol,
        stats: [
            GearStatKind::AuraEnemyEffectPercent,
            GearStatKind::AuraRangeFlat,
        ],
        special_effect: None,
        faction: Some(PlayerFaction::Christian),
    },
    GearTemplate {
        id: "symbol_crescent",
        name: "Crescent Emblem",
        description: "Empowers aura effects against enemies and extends aura reach.",
        icon_key: "item_symbol_crescent",
        item_type: GearItemType::Symbol,
        stats: [
            GearStatKind::AuraEnemyEffectPercent,
            GearStatKind::AuraRangeFlat,
        ],
        special_effect: None,
        faction: Some(PlayerFaction::Muslim),
    },
    GearTemplate {
        id: "sword_simple",
        name: "Simple Sword",
        description: "Tradeoff roll between damage and attack speed.",
        icon_key: "item_sword",
        item_type: GearItemType::MeleeWeapon,
        stats: [
            GearStatKind::DamagePercent,
            GearStatKind::AttackSpeedPercent,
        ],
        special_effect: None,
        faction: None,
    },
    GearTemplate {
        id: "bow_simple",
        name: "Simple Bow",
        description: "Tradeoff roll between ranged damage and range.",
        icon_key: "item_bow",
        item_type: GearItemType::RangedWeapon,
        stats: [GearStatKind::DamagePercent, GearStatKind::RangedRangeFlat],
        special_effect: None,
        faction: None,
    },
    GearTemplate {
        id: "armor_simple",
        name: "Simple Chestpiece",
        description: "Tradeoff roll between armor and movement speed.",
        icon_key: "item_armor",
        item_type: GearItemType::Armor,
        stats: [GearStatKind::ArmorFlat, GearStatKind::MoveSpeedFlat],
        special_effect: None,
        faction: None,
    },
];

impl InventoryState {
    pub fn setup_for(&self, unit_type: EquipmentUnitType) -> Option<&UnitEquipmentSetup> {
        self.setups
            .iter()
            .find(|setup| setup.unit_type == unit_type)
    }

    pub fn setup_for_mut(
        &mut self,
        unit_type: EquipmentUnitType,
    ) -> Option<&mut UnitEquipmentSetup> {
        self.setups
            .iter_mut()
            .find(|setup| setup.unit_type == unit_type)
    }

    pub fn ensure_bag_capacity(&mut self) {
        if self.bag_slots.len() != BACKPACK_SLOT_CAPACITY {
            self.bag_slots = vec![None; BACKPACK_SLOT_CAPACITY];
        }
    }

    pub fn first_free_bag_slot(&self) -> Option<usize> {
        self.bag_slots.iter().position(Option::is_none)
    }

    pub fn bag_item(&self, index: usize) -> Option<&GearItemEntry> {
        self.bag_slots.get(index).and_then(Option::as_ref)
    }
}

pub fn equipment_unit_type_for_unit(kind: UnitKind, tier: Option<u8>) -> Option<EquipmentUnitType> {
    match kind {
        UnitKind::Commander => Some(EquipmentUnitType::Commander),
        UnitKind::ChristianPeasantInfantry
        | UnitKind::ChristianPeasantArcher
        | UnitKind::ChristianPeasantPriest
        | UnitKind::MuslimPeasantInfantry
        | UnitKind::MuslimPeasantArcher
        | UnitKind::MuslimPeasantPriest => EquipmentUnitType::from_tier(tier.unwrap_or(0)),
        _ => None,
    }
}

fn role_for_unit(kind: UnitKind) -> UnitCombatRole {
    match kind {
        UnitKind::Commander => UnitCombatRole::Commander,
        UnitKind::ChristianPeasantInfantry | UnitKind::MuslimPeasantInfantry => {
            UnitCombatRole::Melee
        }
        UnitKind::ChristianPeasantArcher | UnitKind::MuslimPeasantArcher => UnitCombatRole::Ranged,
        UnitKind::ChristianPeasantPriest | UnitKind::MuslimPeasantPriest => UnitCombatRole::Support,
        _ => UnitCombatRole::Melee,
    }
}

fn apply_stat_to_bonuses(
    bonuses: &mut UnitEquipmentBonuses,
    stat: &GearRolledStat,
    role: UnitCombatRole,
) {
    match stat.kind {
        GearStatKind::DamagePercent => {
            if matches!(role, UnitCombatRole::Ranged) {
                bonuses.ranged_damage_multiplier += stat.value;
            } else {
                bonuses.melee_damage_multiplier += stat.value;
            }
        }
        GearStatKind::AttackSpeedPercent => bonuses.attack_speed_multiplier += stat.value,
        GearStatKind::ArmorFlat => bonuses.armor_bonus += stat.value,
        GearStatKind::RangedRangeFlat => bonuses.ranged_range_bonus += stat.value,
        GearStatKind::MoveSpeedFlat => bonuses.move_speed_bonus += stat.value,
        GearStatKind::CommanderMaxHealthFlat => bonuses.health_bonus += stat.value,
        GearStatKind::MoraleRegenPerSec => bonuses.morale_regen_per_sec += stat.value,
        GearStatKind::CohesionRegenPerSec => bonuses.cohesion_regen_per_sec += stat.value,
        GearStatKind::MoraleLossResistancePercent => bonuses.morale_loss_resistance += stat.value,
        GearStatKind::CohesionLossResistancePercent => {
            bonuses.cohesion_loss_resistance += stat.value
        }
        GearStatKind::AuraRangeFlat => bonuses.aura_radius_bonus += stat.value,
        GearStatKind::AuraEnemyEffectPercent => {
            bonuses.aura_enemy_effect_bonus_multiplier += stat.value
        }
        GearStatKind::CooldownReductionSecs => bonuses.cooldown_reduction_secs += stat.value,
        GearStatKind::ArmorPulseFlat => bonuses.drum_armor_pulse_flat += stat.value,
        GearStatKind::SquirePrimary => match role {
            UnitCombatRole::Melee => bonuses.melee_damage_multiplier += stat.value,
            UnitCombatRole::Ranged => bonuses.attack_speed_multiplier += stat.value,
            UnitCombatRole::Support | UnitCombatRole::Commander => {
                bonuses.cooldown_reduction_secs += stat.value
            }
        },
        GearStatKind::SquireSecondary => match role {
            UnitCombatRole::Melee => bonuses.attack_speed_multiplier += stat.value,
            UnitCombatRole::Ranged => bonuses.ranged_damage_multiplier += stat.value,
            UnitCombatRole::Support | UnitCombatRole::Commander => {
                bonuses.health_bonus += stat.value * 20.0
            }
        },
    }
}

fn item_bonuses_for_role(item: &GearItemEntry, role: UnitCombatRole) -> UnitEquipmentBonuses {
    let mut bonuses = UnitEquipmentBonuses::zero();
    for stat in &item.stats {
        apply_stat_to_bonuses(&mut bonuses, stat, role);
    }
    bonuses
}

fn setup_bonuses_for_role(
    setup: &UnitEquipmentSetup,
    role: UnitCombatRole,
    banner_item_active: bool,
) -> UnitEquipmentBonuses {
    let mut bonuses = UnitEquipmentBonuses::zero();
    for slot in &setup.slots {
        if let Some(item) = slot.item.as_ref() {
            if !banner_item_active && item.item_type == GearItemType::Banner {
                continue;
            }
            bonuses.merge_assign(item_bonuses_for_role(item, role));
        }
    }
    bonuses
}

pub fn commander_armywide_bonuses(
    inventory: &InventoryState,
    role: UnitCombatRole,
) -> UnitEquipmentBonuses {
    commander_armywide_bonuses_with_banner_state(inventory, role, true)
}

pub fn commander_armywide_bonuses_with_banner_state(
    inventory: &InventoryState,
    role: UnitCombatRole,
    banner_item_active: bool,
) -> UnitEquipmentBonuses {
    let Some(commander_setup) = inventory.setup_for(EquipmentUnitType::Commander) else {
        return UnitEquipmentBonuses::zero();
    };
    setup_bonuses_for_role(commander_setup, role, banner_item_active)
}

fn armywide_move_speed_bonus(
    inventory: &InventoryState,
    banner_item_active: bool,
) -> f32 {
    let mut total = 0.0;
    for setup in &inventory.setups {
        for slot in &setup.slots {
            let Some(item) = slot.item.as_ref() else {
                continue;
            };
            if !banner_item_active && item.item_type == GearItemType::Banner {
                continue;
            }
            for stat in &item.stats {
                if stat.kind == GearStatKind::MoveSpeedFlat {
                    total += stat.value;
                }
            }
        }
    }
    total
}

pub fn gear_bonuses_for_unit(
    inventory: &InventoryState,
    kind: UnitKind,
    tier: Option<u8>,
) -> UnitEquipmentBonuses {
    gear_bonuses_for_unit_with_banner_state(inventory, kind, tier, true)
}

pub fn gear_bonuses_for_unit_with_banner_state(
    inventory: &InventoryState,
    kind: UnitKind,
    tier: Option<u8>,
    commander_banner_item_active: bool,
) -> UnitEquipmentBonuses {
    let Some(unit_type) = equipment_unit_type_for_unit(kind, tier) else {
        return UnitEquipmentBonuses::zero();
    };
    let role = role_for_unit(kind);
    let mut bonuses = inventory
        .setup_for(unit_type)
        .map(|setup| setup_bonuses_for_role(setup, role, true))
        .unwrap_or_else(UnitEquipmentBonuses::zero);

    if kind == UnitKind::Commander || kind.is_friendly_recruit() {
        let commander_bonuses = commander_armywide_bonuses_with_banner_state(
            inventory,
            role,
            commander_banner_item_active,
        );
        if unit_type != EquipmentUnitType::Commander {
            bonuses.merge_assign(commander_bonuses);
        }

        bonuses.move_speed_bonus = armywide_move_speed_bonus(inventory, commander_banner_item_active);
    }

    bonuses
}

pub fn item_rarity_tier_for_display(item: &GearItemEntry) -> GearRarity {
    item.stats
        .iter()
        .map(|stat| stat.rarity)
        .max_by_key(|rarity| rarity.points())
        .unwrap_or(GearRarity::Common)
}

pub fn format_stat_value_for_tooltip(stat: &GearRolledStat) -> String {
    match stat.kind {
        GearStatKind::DamagePercent
        | GearStatKind::AttackSpeedPercent
        | GearStatKind::MoraleLossResistancePercent
        | GearStatKind::CohesionLossResistancePercent
        | GearStatKind::AuraEnemyEffectPercent
        | GearStatKind::SquirePrimary
        | GearStatKind::SquireSecondary => format!("{:+.1}%", stat.value * 100.0),
        _ => format!("{:+.2}", stat.value),
    }
}

pub fn gear_item_type_label(item_type: GearItemType) -> &'static str {
    match item_type {
        GearItemType::Banner => "Banner",
        GearItemType::Instrument => "Instrument",
        GearItemType::Chant => "Chant",
        GearItemType::Squire => "Squire",
        GearItemType::Symbol => "Symbol",
        GearItemType::MeleeWeapon => "Melee Weapon",
        GearItemType::RangedWeapon => "Ranged Weapon",
        GearItemType::Armor => "Armor",
    }
}

pub fn gear_item_tooltip(item: &GearItemEntry) -> String {
    let mut lines = vec![
        format!(
            "{} [{}]",
            item.name,
            item_rarity_tier_for_display(item).label()
        ),
        format!("Type: {}", gear_item_type_label(item.item_type)),
        item.description.clone(),
    ];
    for stat in &item.stats {
        lines.push(format!(
            "- {} ({}) {}",
            stat.kind.label(),
            stat.rarity.label(),
            format_stat_value_for_tooltip(stat)
        ));
    }
    if let Some(effect) = item.special_effect {
        lines.push(format!("- Effect: {}", effect.label()));
    }
    lines.join("\n")
}

fn slot_accepts_item(slot: &EquippedSlot, item: &GearItemEntry) -> bool {
    slot.accepted_item_types.contains(&item.item_type)
}

pub fn get_item_from_slot<'a>(
    inventory: &'a InventoryState,
    chest: &'a EquipmentChestState,
    slot: InventorySlotRef,
) -> Option<&'a GearItemEntry> {
    match slot {
        InventorySlotRef::Backpack(index) => inventory.bag_item(index),
        InventorySlotRef::Equipment {
            unit_type,
            slot_index,
        } => inventory
            .setup_for(unit_type)
            .and_then(|setup| setup.slots.get(slot_index))
            .and_then(|slot| slot.item.as_ref()),
        InventorySlotRef::Chest(index) => chest.slots.get(index).and_then(Option::as_ref),
    }
}

pub fn take_item_from_slot(
    inventory: &mut InventoryState,
    chest: &mut EquipmentChestState,
    slot: InventorySlotRef,
) -> Option<GearItemEntry> {
    match slot {
        InventorySlotRef::Backpack(index) => {
            inventory.bag_slots.get_mut(index).and_then(Option::take)
        }
        InventorySlotRef::Equipment {
            unit_type,
            slot_index,
        } => inventory
            .setup_for_mut(unit_type)
            .and_then(|setup| setup.slots.get_mut(slot_index))
            .and_then(|slot| slot.item.take()),
        InventorySlotRef::Chest(index) => chest.slots.get_mut(index).and_then(Option::take),
    }
}

pub fn place_item_into_slot(
    inventory: &mut InventoryState,
    chest: &mut EquipmentChestState,
    slot: InventorySlotRef,
    item: GearItemEntry,
) -> Result<Option<GearItemEntry>, InventoryPlaceError> {
    match slot {
        InventorySlotRef::Backpack(index) => {
            let Some(target_slot) = inventory.bag_slots.get_mut(index) else {
                return Err(InventoryPlaceError::InvalidSlot);
            };
            Ok(target_slot.replace(item))
        }
        InventorySlotRef::Equipment {
            unit_type,
            slot_index,
        } => {
            let Some(setup) = inventory.setup_for_mut(unit_type) else {
                return Err(InventoryPlaceError::InvalidSlot);
            };
            let Some(target_slot) = setup.slots.get_mut(slot_index) else {
                return Err(InventoryPlaceError::InvalidSlot);
            };
            if !slot_accepts_item(target_slot, &item) {
                return Err(InventoryPlaceError::ItemTypeMismatch);
            }
            Ok(target_slot.item.replace(item))
        }
        InventorySlotRef::Chest(index) => {
            let Some(target_slot) = chest.slots.get_mut(index) else {
                return Err(InventoryPlaceError::InvalidSlot);
            };
            Ok(target_slot.replace(item))
        }
    }
}

fn rarity_weight_for(rarity: GearRarity, rarity_roll_bonus_pct: f32) -> f32 {
    let base = BASE_RARITY_WEIGHTS
        .iter()
        .find_map(|(tier, weight)| (*tier == rarity).then_some(*weight))
        .unwrap_or(0.0);
    let bonus = rarity_roll_bonus_pct.max(0.0);
    if bonus <= 0.0 {
        return base;
    }
    match rarity {
        GearRarity::Common | GearRarity::Uncommon => (base / (1.0 + bonus)).max(0.0001),
        _ => base * (1.0 + bonus),
    }
}

fn roll_rarity_from_pool(
    rng: &mut InventoryRngState,
    pool: &[GearRarity],
    rarity_roll_bonus_pct: f32,
) -> GearRarity {
    let mut total = 0.0;
    let mut weighted = Vec::with_capacity(pool.len());
    for rarity in pool {
        let weight = rarity_weight_for(*rarity, rarity_roll_bonus_pct);
        total += weight;
        weighted.push((*rarity, weight));
    }
    if total <= 0.0 {
        return pool.first().copied().unwrap_or(GearRarity::Common);
    }
    let mut roll = rng.next_f32() * total;
    for (rarity, weight) in weighted {
        if roll <= weight {
            return rarity;
        }
        roll -= weight;
    }
    pool.last().copied().unwrap_or(GearRarity::Common)
}

fn roll_item_from_template(
    rng: &mut InventoryRngState,
    template: GearTemplate,
    rarity_roll_bonus_pct: f32,
) -> GearItemEntry {
    let (stat_a_kind, stat_b_kind) = if rng.next_f32() < 0.5 {
        (template.stats[0], template.stats[1])
    } else {
        (template.stats[1], template.stats[0])
    };

    let first_pool = [
        GearRarity::Common,
        GearRarity::Uncommon,
        GearRarity::Rare,
        GearRarity::Epic,
        GearRarity::Mythical,
        GearRarity::Unique,
    ];
    let first_roll = roll_rarity_from_pool(rng, &first_pool, rarity_roll_bonus_pct);
    let second_roll = if first_roll == GearRarity::Unique {
        let second_pool = [
            GearRarity::Rare,
            GearRarity::Epic,
            GearRarity::Mythical,
            GearRarity::Unique,
        ];
        roll_rarity_from_pool(rng, &second_pool, rarity_roll_bonus_pct)
    } else {
        let remaining_budget = ITEM_ROLL_BUDGET.saturating_sub(first_roll.points()).max(1);
        let max_points = remaining_budget.min(5);
        let mut second_pool = Vec::new();
        for points in 1..=max_points {
            second_pool.push(GearRarity::from_points(points));
        }
        roll_rarity_from_pool(rng, &second_pool, rarity_roll_bonus_pct)
    };

    let stat_a = GearRolledStat {
        kind: stat_a_kind,
        rarity: first_roll,
        value: stat_a_kind.base_magnitude() * first_roll.scalar(),
    };
    let stat_b = GearRolledStat {
        kind: stat_b_kind,
        rarity: second_roll,
        value: stat_b_kind.base_magnitude() * second_roll.scalar(),
    };

    GearItemEntry {
        instance_id: rng.next_item_instance_id(template.id),
        template_id: template.id.to_string(),
        name: template.name.to_string(),
        description: template.description.to_string(),
        icon_key: template.icon_key.to_string(),
        item_type: template.item_type,
        faction: template.faction,
        stats: vec![stat_a, stat_b],
        special_effect: template.special_effect,
    }
}

pub fn roll_chest_items(
    rng: &mut InventoryRngState,
    player_faction: PlayerFaction,
    count: usize,
    rarity_roll_bonus_pct: f32,
) -> Vec<GearItemEntry> {
    let templates: Vec<GearTemplate> = TEMPLATES
        .iter()
        .copied()
        .filter(|template| template.faction.is_none() || template.faction == Some(player_faction))
        .collect();
    if templates.is_empty() || count == 0 {
        return Vec::new();
    }

    let mut items = Vec::with_capacity(count);
    for _ in 0..count {
        let index = (rng.next_u32() as usize) % templates.len();
        items.push(roll_item_from_template(
            rng,
            templates[index],
            rarity_roll_bonus_pct,
        ));
    }
    items
}

fn make_slot(slot_id: &str, display_name: &str, accepted: &[GearItemType]) -> EquippedSlot {
    EquippedSlot {
        slot_id: slot_id.to_string(),
        display_name: display_name.to_string(),
        accepted_item_types: accepted.to_vec(),
        item: None,
    }
}

fn default_equipment_slots(unit_type: EquipmentUnitType) -> Vec<EquippedSlot> {
    match unit_type {
        EquipmentUnitType::Commander => vec![
            make_slot("banner", "Banner", &[GearItemType::Banner]),
            make_slot("instrument", "Instrument", &[GearItemType::Instrument]),
            make_slot("chant", "Chant", &[GearItemType::Chant]),
            make_slot("squire", "Squire", &[GearItemType::Squire]),
            make_slot("symbol", "Symbol", &[GearItemType::Symbol]),
        ],
        _ => vec![
            make_slot("melee_weapon", "Melee", &[GearItemType::MeleeWeapon]),
            make_slot("ranged_weapon", "Ranged", &[GearItemType::RangedWeapon]),
            make_slot("armor", "Armor", &[GearItemType::Armor]),
            make_slot("banner", "Banner", &[GearItemType::Banner]),
            make_slot("squire", "Squire", &[GearItemType::Squire]),
        ],
    }
}

impl Default for InventoryState {
    fn default() -> Self {
        let setups = EquipmentUnitType::all()
            .into_iter()
            .map(|unit_type| UnitEquipmentSetup {
                unit_type,
                slots: default_equipment_slots(unit_type),
            })
            .collect();
        Self {
            bag_slots: vec![None; BACKPACK_SLOT_CAPACITY],
            setups,
        }
    }
}

fn reset_inventory_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut inventory: ResMut<InventoryState>,
    mut chest: ResMut<EquipmentChestState>,
    mut effects: ResMut<EquipmentArmyEffects>,
    mut rng: ResMut<InventoryRngState>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    *inventory = InventoryState::default();
    chest.clear();
    *effects = EquipmentArmyEffects::default();
    rng.reseed_from_time();
}

fn extract_special_scalar(item: &GearItemEntry, stat_kind: GearStatKind) -> f32 {
    item.stats
        .iter()
        .find_map(|stat| (stat.kind == stat_kind).then_some(stat.value))
        .unwrap_or(0.0)
}

fn update_army_equipment_effects(
    time: Res<Time>,
    banner_state: Option<Res<BannerState>>,
    inventory: Res<InventoryState>,
    skill_timing: Option<Res<SkillTimingBuffs>>,
    mut effects: ResMut<EquipmentArmyEffects>,
) {
    let Some(commander_setup) = inventory.setup_for(EquipmentUnitType::Commander) else {
        *effects = EquipmentArmyEffects::default();
        return;
    };
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    let cooldown_reduction = commander_armywide_bonuses_with_banner_state(
        &inventory,
        UnitCombatRole::Commander,
        banner_item_active,
    )
    .cooldown_reduction_secs
    .max(0.0);
    let skill_duration_multiplier = skill_timing
        .as_deref()
        .map(|value| value.duration_multiplier.max(1.0))
        .unwrap_or(1.0);
    let upgrade_cooldown_reduction = skill_timing
        .as_deref()
        .map(|value| value.cooldown_reduction.clamp(0.0, 0.9))
        .unwrap_or(0.0);

    let mut drum_power = 0.0;
    let mut has_drum = false;
    let mut has_chant = false;
    for slot in &commander_setup.slots {
        let Some(item) = slot.item.as_ref() else {
            continue;
        };
        match item.special_effect {
            Some(GearSpecialEffectKind::DrumArmorPulse) => {
                has_drum = true;
                drum_power = extract_special_scalar(item, GearStatKind::ArmorPulseFlat).max(0.0);
            }
            Some(GearSpecialEffectKind::BattleChantMoraleImmunity) => {
                has_chant = true;
            }
            None => {}
        }
    }

    let dt = time.delta_seconds().max(0.0);
    if has_drum {
        effects.drum_active_secs = (effects.drum_active_secs - dt).max(0.0);
        if effects.drum_active_secs <= 0.0 {
            effects.drum_cooldown_secs = (effects.drum_cooldown_secs - dt).max(0.0);
            if effects.drum_cooldown_secs <= 0.0 {
                effects.drum_active_secs =
                    scaled_skill_duration(DRUM_EFFECT_DURATION_SECS, skill_duration_multiplier);
                effects.drum_cooldown_secs = scaled_skill_cooldown(
                    DRUM_BASE_COOLDOWN_SECS,
                    cooldown_reduction,
                    upgrade_cooldown_reduction,
                    MIN_ARMY_SKILL_COOLDOWN_SECS,
                );
            }
        }
    } else {
        effects.drum_active_secs = 0.0;
        effects.drum_cooldown_secs = DRUM_BASE_COOLDOWN_SECS;
    }

    if has_chant {
        effects.chant_active_secs = (effects.chant_active_secs - dt).max(0.0);
        if effects.chant_active_secs <= 0.0 {
            effects.chant_cooldown_secs = (effects.chant_cooldown_secs - dt).max(0.0);
            if effects.chant_cooldown_secs <= 0.0 {
                effects.chant_active_secs =
                    scaled_skill_duration(CHANT_EFFECT_DURATION_SECS, skill_duration_multiplier);
                effects.chant_cooldown_secs = scaled_skill_cooldown(
                    CHANT_BASE_COOLDOWN_SECS,
                    cooldown_reduction,
                    upgrade_cooldown_reduction,
                    MIN_ARMY_SKILL_COOLDOWN_SECS,
                );
            }
        }
    } else {
        effects.chant_active_secs = 0.0;
        effects.chant_cooldown_secs = CHANT_BASE_COOLDOWN_SECS;
    }

    effects.temporary_armor_bonus = if effects.drum_active_secs > 0.0 {
        drum_power
    } else {
        0.0
    };
    effects.morale_loss_immunity = effects.chant_active_secs > 0.0;
}

pub fn scaled_skill_duration(base_duration: f32, duration_multiplier: f32) -> f32 {
    (base_duration.max(0.0) * duration_multiplier.max(1.0)).max(0.0)
}

pub fn scaled_skill_cooldown(
    base_cooldown: f32,
    additive_reduction_secs: f32,
    percentage_reduction: f32,
    min_cooldown: f32,
) -> f32 {
    let base_plus_additive =
        (base_cooldown.max(0.0) - additive_reduction_secs.max(0.0)).max(min_cooldown);
    let percent = percentage_reduction.clamp(0.0, 0.9);
    (base_plus_additive * (1.0 - percent)).max(min_cooldown)
}

#[allow(clippy::type_complexity)]
fn apply_equipment_health_bonus(
    banner_state: Option<Res<BannerState>>,
    inventory: Res<InventoryState>,
    progression: Option<Res<Progression>>,
    mut friendlies: Query<
        (
            &crate::model::Unit,
            Option<&crate::model::UnitTier>,
            &crate::model::BaseMaxHealth,
            &mut crate::model::Health,
        ),
        With<crate::model::FriendlyUnit>,
    >,
) {
    let level = progression.as_ref().map(|value| value.level).unwrap_or(1);
    let level_hp_bonus = commander_level_hp_bonus(level);
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    for (unit, tier, base_max, mut health) in &mut friendlies {
        let gear = gear_bonuses_for_unit_with_banner_state(
            &inventory,
            unit.kind,
            tier.map(|value| value.0),
            banner_item_active,
        );
        let expected_max = (base_max.0 + level_hp_bonus + gear.health_bonus).max(1.0);
        if (health.max - expected_max).abs() > 0.001 {
            let ratio = if health.max > 0.0 {
                (health.current / health.max).clamp(0.0, 1.0)
            } else {
                1.0
            };
            health.max = expected_max;
            health.current = (health.max * ratio).clamp(0.0, health.max);
        }
    }
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryState>()
            .init_resource::<EquipmentChestState>()
            .init_resource::<EquipmentArmyEffects>()
            .init_resource::<InventoryRngState>()
            .init_resource::<ItemRarityRollBonus>()
            .add_systems(Update, reset_inventory_on_run_start)
            .add_systems(
                Update,
                update_army_equipment_effects.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                PostUpdate,
                apply_equipment_health_bonus.run_if(in_state(GameState::InRun)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BACKPACK_SLOT_CAPACITY, CHEST_SLOT_CAPACITY, EquipmentChestState, EquipmentUnitType,
        GearItemEntry, GearItemType, GearRarity, GearRolledStat, GearStatKind, InventoryPlaceError,
        InventoryRngState, InventorySlotRef, InventoryState, commander_armywide_bonuses,
        gear_bonuses_for_unit, place_item_into_slot, roll_chest_items, take_item_from_slot,
    };
    use crate::inventory::UnitCombatRole;
    use crate::model::{PlayerFaction, UnitKind};

    fn make_test_item(item_type: GearItemType, stat: GearStatKind, value: f32) -> GearItemEntry {
        GearItemEntry {
            instance_id: "test_item_1".to_string(),
            template_id: "test_template".to_string(),
            name: "Test Item".to_string(),
            description: "Test".to_string(),
            icon_key: "test_icon".to_string(),
            item_type,
            faction: None,
            stats: vec![GearRolledStat {
                kind: stat,
                rarity: GearRarity::Rare,
                value,
            }],
            special_effect: None,
        }
    }

    #[test]
    fn default_inventory_has_fixed_bag_capacity() {
        let inventory = InventoryState::default();
        assert_eq!(inventory.bag_slots.len(), BACKPACK_SLOT_CAPACITY);
    }

    #[test]
    fn chest_capacity_is_fixed() {
        let mut chest = EquipmentChestState {
            slots: vec![None; 1],
        };
        chest.ensure_capacity();
        assert_eq!(chest.slots.len(), CHEST_SLOT_CAPACITY);
    }

    #[test]
    fn equipment_slot_rejects_wrong_item_type() {
        let mut inventory = InventoryState::default();
        let mut chest = EquipmentChestState::default();
        chest.ensure_capacity();
        let wrong_item = make_test_item(GearItemType::Armor, GearStatKind::ArmorFlat, 2.0);
        let result = place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Tier0,
                slot_index: 0,
            },
            wrong_item,
        );
        assert_eq!(result, Err(InventoryPlaceError::ItemTypeMismatch));
    }

    #[test]
    fn can_move_item_between_backpack_and_equipment() {
        let mut inventory = InventoryState::default();
        let mut chest = EquipmentChestState::default();
        chest.ensure_capacity();

        let sword = make_test_item(GearItemType::MeleeWeapon, GearStatKind::DamagePercent, 0.12);
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Backpack(0),
            sword,
        )
        .expect("place in bag");

        let grabbed =
            take_item_from_slot(&mut inventory, &mut chest, InventorySlotRef::Backpack(0))
                .expect("take from bag");
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Tier0,
                slot_index: 0,
            },
            grabbed,
        )
        .expect("equip item");

        let bonuses =
            gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        assert!(bonuses.melee_damage_multiplier > 0.0);
    }

    #[test]
    fn chest_roll_budget_rule_is_applied() {
        let mut rng = InventoryRngState {
            state: 0x1234_5678_9ABC_DEF0,
            next_item_serial: 1,
        };
        let items = roll_chest_items(&mut rng, PlayerFaction::Christian, 64, 0.0);
        assert!(!items.is_empty());
        for item in items {
            if item.stats.len() != 2 {
                continue;
            }
            let first = item.stats[0].rarity;
            let second = item.stats[1].rarity;
            if first != GearRarity::Unique {
                assert!(second.points() <= (6 - first.points()).clamp(1, 5));
            } else {
                assert!(second.points() >= 3);
            }
        }
    }

    #[test]
    fn commander_items_apply_armywide_but_not_to_non_matching_item_slots() {
        let mut inventory = InventoryState::default();
        let mut chest = EquipmentChestState::default();
        chest.ensure_capacity();

        let banner_damage = make_test_item(GearItemType::Banner, GearStatKind::DamagePercent, 0.2);
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Commander,
                slot_index: 0,
            },
            banner_damage,
        )
        .expect("equip commander banner");

        let commander = gear_bonuses_for_unit(&inventory, UnitKind::Commander, Some(0));
        assert!(commander.melee_damage_multiplier > 0.0);

        let tier0 = gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        assert!(tier0.melee_damage_multiplier > 0.0);

        let enemy = gear_bonuses_for_unit(
            &inventory,
            UnitKind::RescuableChristianPeasantInfantry,
            None,
        );
        assert_eq!(enemy.melee_damage_multiplier, 0.0);
    }

    #[test]
    fn tier_and_hero_equipment_stays_scoped_to_matching_setup() {
        let mut inventory = InventoryState::default();
        let mut chest = EquipmentChestState::default();
        chest.ensure_capacity();

        let tier0_sword =
            make_test_item(GearItemType::MeleeWeapon, GearStatKind::DamagePercent, 0.15);
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Tier0,
                slot_index: 0,
            },
            tier0_sword,
        )
        .expect("equip tier0 sword");

        let hero_sword =
            make_test_item(GearItemType::MeleeWeapon, GearStatKind::DamagePercent, 0.3);
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Hero,
                slot_index: 0,
            },
            hero_sword,
        )
        .expect("equip hero sword");

        let tier0 = gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        let tier1 = gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(1));
        let hero = gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(6));
        assert!(tier0.melee_damage_multiplier > 0.0);
        assert_eq!(tier1.melee_damage_multiplier, 0.0);
        assert!(hero.melee_damage_multiplier > tier0.melee_damage_multiplier);
    }

    #[test]
    fn commander_cooldown_reduction_is_projected_armywide_for_support_roles() {
        let mut inventory = InventoryState::default();
        let mut chest = EquipmentChestState::default();
        chest.ensure_capacity();

        let chant = make_test_item(
            GearItemType::Chant,
            GearStatKind::CooldownReductionSecs,
            1.5,
        );
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Commander,
                slot_index: 2,
            },
            chant,
        )
        .expect("equip commander chant");

        let support_bonuses = commander_armywide_bonuses(&inventory, UnitCombatRole::Support);
        assert!(support_bonuses.cooldown_reduction_secs > 0.0);
    }

    #[test]
    fn rarity_scalars_are_boosted_by_twenty_percent() {
        assert!((GearRarity::Common.scalar() - 0.06).abs() < 0.0001);
        assert!((GearRarity::Uncommon.scalar() - 0.12).abs() < 0.0001);
        assert!((GearRarity::Rare.scalar() - 0.216).abs() < 0.0001);
        assert!((GearRarity::Epic.scalar() - 0.408).abs() < 0.0001);
        assert!((GearRarity::Mythical.scalar() - 0.624).abs() < 0.0001);
        assert!((GearRarity::Unique.scalar() - 0.90).abs() < 0.0001);
    }

    #[test]
    fn move_speed_from_any_tier_item_applies_armywide() {
        let mut inventory = InventoryState::default();
        let mut chest = EquipmentChestState::default();
        chest.ensure_capacity();

        let tier5_armor_speed = make_test_item(GearItemType::Armor, GearStatKind::MoveSpeedFlat, 10.0);
        place_item_into_slot(
            &mut inventory,
            &mut chest,
            InventorySlotRef::Equipment {
                unit_type: EquipmentUnitType::Tier5,
                slot_index: 2,
            },
            tier5_armor_speed,
        )
        .expect("equip tier5 armor speed");

        let tier0_bonuses =
            gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        let commander_bonuses = gear_bonuses_for_unit(&inventory, UnitKind::Commander, Some(0));

        assert!((tier0_bonuses.move_speed_bonus - 10.0).abs() < 0.0001);
        assert!((commander_bonuses.move_speed_bonus - 10.0).abs() < 0.0001);
    }
}
