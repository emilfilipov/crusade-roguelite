use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use bevy::prelude::*;
use serde::{Deserialize, Deserializer};

use crate::model::{
    GameDifficulty, GameState, PlayerFaction, RecruitArchetype, RecruitUnitKind, UnitKind,
};

const TIER0_INFANTRY_UNIT_ID: &str = "peasant_infantry";
const TIER0_ARCHER_UNIT_ID: &str = "peasant_archer";
const TIER0_PRIEST_UNIT_ID: &str = "peasant_priest";
const HERO_SUBTYPE_SWORD_SHIELD: &str = "sword_shield";
const HERO_SUBTYPE_SPEAR: &str = "spear";
const HERO_SUBTYPE_TWO_HANDED_SWORD: &str = "two_handed_sword";
const HERO_SUBTYPE_BOW: &str = "bow";
const HERO_SUBTYPE_JAVELIN: &str = "javelin";
const HERO_SUBTYPE_BEAST_MASTER: &str = "beast_master";
const HERO_SUBTYPE_SUPER_PRIEST: &str = "super_priest";
const HERO_SUBTYPE_SUPER_FANATIC: &str = "super_fanatic";
const HERO_SUBTYPE_SUPER_KNIGHT: &str = "super_knight";
const REQUIRED_HERO_SUBTYPE_IDS: [&str; 9] = [
    HERO_SUBTYPE_SWORD_SHIELD,
    HERO_SUBTYPE_SPEAR,
    HERO_SUBTYPE_TWO_HANDED_SWORD,
    HERO_SUBTYPE_BOW,
    HERO_SUBTYPE_JAVELIN,
    HERO_SUBTYPE_BEAST_MASTER,
    HERO_SUBTYPE_SUPER_PRIEST,
    HERO_SUBTYPE_SUPER_FANATIC,
    HERO_SUBTYPE_SUPER_KNIGHT,
];
const HERO_ENTRIES_PER_SUBTYPE_PER_FACTION: usize = 10;
const DEPRECATED_UPGRADE_ID_REPLACEMENTS: [(&str, &str); 6] = [
    ("fast_learner_up", "quartermaster_up"),
    ("fast_learner_up_10", "quartermaster_up"),
    ("fast_learner_up_15", "quartermaster_up"),
    ("mob_fury_shielded_host", "mob_fury"),
    ("mob_justice_frontline_bias", "mob_justice"),
    ("mob_mercy_support_ceiling", "mob_mercy"),
];
const REQUIRED_TIER2_UNIT_IDS: [&str; 10] = [
    "shield_infantry",
    "spearman",
    "unmounted_knight",
    "squire",
    "experienced_bowman",
    "crossbowman",
    "tracker",
    "scout",
    "devoted_one",
    "fanatic",
];

#[derive(Clone, Debug, Deserialize)]
pub struct UnitStatsConfig {
    pub id: String,
    pub max_hp: f32,
    pub armor: f32,
    pub damage: f32,
    pub attack_cooldown_secs: f32,
    pub attack_range: f32,
    pub move_speed: f32,
    pub morale: f32,
    #[serde(default)]
    pub aura_radius: f32,
    #[serde(default)]
    pub ranged_attack_damage: f32,
    #[serde(default)]
    pub ranged_attack_cooldown_secs: f32,
    #[serde(default)]
    pub ranged_attack_range: f32,
    #[serde(default)]
    pub ranged_projectile_speed: f32,
    #[serde(default)]
    pub ranged_projectile_max_distance: f32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CommanderRunBonuses {
    pub damage_multiplier_bonus: f32,
    pub move_speed_bonus: f32,
    pub aura_radius_bonus: f32,
    pub pickup_radius_bonus: f32,
}

#[derive(Clone, Debug)]
pub struct CommanderOptionConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub abilities: Vec<String>,
    pub stats: UnitStatsConfig,
    pub run_bonuses: CommanderRunBonuses,
}

#[derive(Clone, Debug)]
pub struct UnitsConfigFile {
    pub commander_christian: UnitStatsConfig,
    pub commander_muslim: UnitStatsConfig,
    pub recruit_christian_peasant_infantry: UnitStatsConfig,
    pub recruit_christian_peasant_archer: UnitStatsConfig,
    pub recruit_christian_peasant_priest: UnitStatsConfig,
    pub recruit_muslim_peasant_infantry: UnitStatsConfig,
    pub recruit_muslim_peasant_archer: UnitStatsConfig,
    pub recruit_muslim_peasant_priest: UnitStatsConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitsConfigFileRaw {
    base: UnitsBaseCatalogRaw,
    #[serde(default)]
    overrides: HashMap<String, UnitsFactionOverrideRaw>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitsBaseCatalogRaw {
    commander: UnitStatsConfig,
    recruits: HashMap<String, UnitStatsConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitsFactionOverrideRaw {
    #[serde(default)]
    commander: Option<UnitStatsConfig>,
    #[serde(default)]
    recruits: HashMap<String, UnitStatsConfig>,
}

impl<'de> Deserialize<'de> for UnitsConfigFile {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UnitsConfigFileRaw::deserialize(deserializer)?;
        let base_infantry = raw
            .base
            .recruits
            .get(TIER0_INFANTRY_UNIT_ID)
            .cloned()
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "units.base.recruits is missing required key 'peasant_infantry'",
                )
            })?;
        let base_archer = raw
            .base
            .recruits
            .get(TIER0_ARCHER_UNIT_ID)
            .cloned()
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "units.base.recruits is missing required key 'peasant_archer'",
                )
            })?;
        let base_priest = raw
            .base
            .recruits
            .get(TIER0_PRIEST_UNIT_ID)
            .cloned()
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "units.base.recruits is missing required key 'peasant_priest'",
                )
            })?;

        let mut config = UnitsConfigFile {
            commander_christian: raw.base.commander.clone(),
            commander_muslim: raw.base.commander,
            recruit_christian_peasant_infantry: base_infantry.clone(),
            recruit_christian_peasant_archer: base_archer.clone(),
            recruit_christian_peasant_priest: base_priest.clone(),
            recruit_muslim_peasant_infantry: base_infantry,
            recruit_muslim_peasant_archer: base_archer,
            recruit_muslim_peasant_priest: base_priest,
        };

        for (faction_key, override_config) in raw.overrides {
            let faction = parse_faction_override_key(&faction_key, "units.overrides")?;
            if let Some(commander) = override_config.commander {
                match faction {
                    PlayerFaction::Christian => config.commander_christian = commander,
                    PlayerFaction::Muslim => config.commander_muslim = commander,
                }
            }
            for (unit_id, stats) in override_config.recruits {
                let archetype =
                    parse_recruit_archetype_unit_id(&unit_id, "units.overrides.*.recruits")?;
                match (faction, archetype) {
                    (PlayerFaction::Christian, RecruitArchetype::Infantry) => {
                        config.recruit_christian_peasant_infantry = stats;
                    }
                    (PlayerFaction::Christian, RecruitArchetype::Archer) => {
                        config.recruit_christian_peasant_archer = stats;
                    }
                    (PlayerFaction::Christian, RecruitArchetype::Priest) => {
                        config.recruit_christian_peasant_priest = stats;
                    }
                    (PlayerFaction::Muslim, RecruitArchetype::Infantry) => {
                        config.recruit_muslim_peasant_infantry = stats;
                    }
                    (PlayerFaction::Muslim, RecruitArchetype::Archer) => {
                        config.recruit_muslim_peasant_archer = stats;
                    }
                    (PlayerFaction::Muslim, RecruitArchetype::Priest) => {
                        config.recruit_muslim_peasant_priest = stats;
                    }
                }
            }
        }

        Ok(config)
    }
}

impl UnitsConfigFile {
    pub fn commander_for_faction(&self, faction: PlayerFaction) -> &UnitStatsConfig {
        match faction {
            PlayerFaction::Christian => &self.commander_christian,
            PlayerFaction::Muslim => &self.commander_muslim,
        }
    }

    pub fn recruit_for_kind(&self, kind: RecruitUnitKind) -> &UnitStatsConfig {
        self.recruit_for_faction_and_archetype(kind.faction(), kind.archetype())
    }

    pub fn recruit_for_faction_and_archetype(
        &self,
        faction: PlayerFaction,
        archetype: RecruitArchetype,
    ) -> &UnitStatsConfig {
        match (faction, archetype) {
            (PlayerFaction::Christian, RecruitArchetype::Infantry) => {
                &self.recruit_christian_peasant_infantry
            }
            (PlayerFaction::Christian, RecruitArchetype::Archer) => {
                &self.recruit_christian_peasant_archer
            }
            (PlayerFaction::Christian, RecruitArchetype::Priest) => {
                &self.recruit_christian_peasant_priest
            }
            (PlayerFaction::Muslim, RecruitArchetype::Infantry) => {
                &self.recruit_muslim_peasant_infantry
            }
            (PlayerFaction::Muslim, RecruitArchetype::Archer) => {
                &self.recruit_muslim_peasant_archer
            }
            (PlayerFaction::Muslim, RecruitArchetype::Priest) => {
                &self.recruit_muslim_peasant_priest
            }
        }
    }

    pub const fn default_commander_id_for_faction(faction: PlayerFaction) -> &'static str {
        match faction {
            PlayerFaction::Christian => "baldiun",
            PlayerFaction::Muslim => "saladin",
        }
    }

    pub fn commander_options_for_faction(
        &self,
        faction: PlayerFaction,
    ) -> Vec<CommanderOptionConfig> {
        let base = self.commander_for_faction(faction);
        match faction {
            PlayerFaction::Christian => {
                let mut marshal_stats = base.clone();
                marshal_stats.id = "baldiun".to_string();
                marshal_stats.aura_radius += 8.0;

                let mut rider_stats = base.clone();
                rider_stats.id = "raymond_the_rider".to_string();
                rider_stats.max_hp = (rider_stats.max_hp - 10.0).max(1.0);
                rider_stats.armor = (rider_stats.armor - 1.0).max(0.0);
                rider_stats.damage += 4.0;
                rider_stats.move_speed += 28.0;
                rider_stats.aura_radius = (rider_stats.aura_radius - 14.0).max(20.0);

                let mut sentinel_stats = base.clone();
                sentinel_stats.id = "templar_sentinel".to_string();
                sentinel_stats.max_hp += 24.0;
                sentinel_stats.armor += 2.0;
                sentinel_stats.damage += 1.0;
                sentinel_stats.move_speed = (sentinel_stats.move_speed - 10.0).max(1.0);
                sentinel_stats.aura_radius += 22.0;

                vec![
                    CommanderOptionConfig {
                        id: "baldiun".to_string(),
                        name: "Baldiun, Banner Marshal".to_string(),
                        description: "Balanced command profile with strong aura coverage and disciplined pacing.".to_string(),
                        abilities: vec![
                            "Disciplined March: +6 army movement speed.".to_string(),
                            "Banner Reach: +8 commander aura radius.".to_string(),
                        ],
                        stats: marshal_stats,
                        run_bonuses: CommanderRunBonuses {
                            move_speed_bonus: 6.0,
                            aura_radius_bonus: 8.0,
                            ..default()
                        },
                    },
                    CommanderOptionConfig {
                        id: "raymond_the_rider".to_string(),
                        name: "Raymond the Rider".to_string(),
                        description: "Fast aggressive doctrine focused on repositioning, pickup tempo, and pressure.".to_string(),
                        abilities: vec![
                            "Rider's Tempo: +14 army movement speed.".to_string(),
                            "Foraging Trains: +12 pickup radius.".to_string(),
                        ],
                        stats: rider_stats,
                        run_bonuses: CommanderRunBonuses {
                            move_speed_bonus: 14.0,
                            pickup_radius_bonus: 12.0,
                            ..default()
                        },
                    },
                    CommanderOptionConfig {
                        id: "templar_sentinel".to_string(),
                        name: "Templar Sentinel".to_string(),
                        description: "Defensive command profile with broader aura pressure and steadier frontline output.".to_string(),
                        abilities: vec![
                            "Line Discipline: +8% army damage.".to_string(),
                            "Sanctified Banner: +16 commander aura radius.".to_string(),
                        ],
                        stats: sentinel_stats,
                        run_bonuses: CommanderRunBonuses {
                            damage_multiplier_bonus: 0.08,
                            aura_radius_bonus: 16.0,
                            ..default()
                        },
                    },
                ]
            }
            PlayerFaction::Muslim => {
                let mut sultan_stats = base.clone();
                sultan_stats.id = "saladin".to_string();
                sultan_stats.aura_radius += 8.0;

                let mut vanguard_stats = base.clone();
                vanguard_stats.id = "faris_vanguard".to_string();
                vanguard_stats.max_hp = (vanguard_stats.max_hp - 8.0).max(1.0);
                vanguard_stats.armor = (vanguard_stats.armor - 0.8).max(0.0);
                vanguard_stats.damage += 4.0;
                vanguard_stats.move_speed += 26.0;
                vanguard_stats.aura_radius = (vanguard_stats.aura_radius - 12.0).max(20.0);

                let mut warden_stats = base.clone();
                warden_stats.id = "mamluk_warden".to_string();
                warden_stats.max_hp += 22.0;
                warden_stats.armor += 2.2;
                warden_stats.damage += 1.0;
                warden_stats.move_speed = (warden_stats.move_speed - 8.0).max(1.0);
                warden_stats.aura_radius += 20.0;

                vec![
                    CommanderOptionConfig {
                        id: "saladin".to_string(),
                        name: "Saladin, Sultan's Standard".to_string(),
                        description: "Balanced command profile with reliable aura projection and run-wide stability.".to_string(),
                        abilities: vec![
                            "Ordered Advance: +6 army movement speed.".to_string(),
                            "Standard Reach: +8 commander aura radius.".to_string(),
                        ],
                        stats: sultan_stats,
                        run_bonuses: CommanderRunBonuses {
                            move_speed_bonus: 6.0,
                            aura_radius_bonus: 8.0,
                            ..default()
                        },
                    },
                    CommanderOptionConfig {
                        id: "faris_vanguard".to_string(),
                        name: "Faris Vanguard".to_string(),
                        description: "Mobile strike doctrine focused on rotation speed and resource control.".to_string(),
                        abilities: vec![
                            "Vanguard Pace: +14 army movement speed.".to_string(),
                            "Spoils Control: +12 pickup radius.".to_string(),
                        ],
                        stats: vanguard_stats,
                        run_bonuses: CommanderRunBonuses {
                            move_speed_bonus: 14.0,
                            pickup_radius_bonus: 12.0,
                            ..default()
                        },
                    },
                    CommanderOptionConfig {
                        id: "mamluk_warden".to_string(),
                        name: "Mamluk Warden".to_string(),
                        description: "Durable command profile with stronger sustained formation pressure.".to_string(),
                        abilities: vec![
                            "Disciplined Blades: +8% army damage.".to_string(),
                            "Ward Line: +16 commander aura radius.".to_string(),
                        ],
                        stats: warden_stats,
                        run_bonuses: CommanderRunBonuses {
                            damage_multiplier_bonus: 0.08,
                            aura_radius_bonus: 16.0,
                            ..default()
                        },
                    },
                ]
            }
        }
    }

    pub fn commander_option_for_faction_and_id(
        &self,
        faction: PlayerFaction,
        commander_id: &str,
    ) -> Option<CommanderOptionConfig> {
        self.commander_options_for_faction(faction)
            .into_iter()
            .find(|entry| entry.id == commander_id)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemyStatsConfig {
    pub id: String,
    pub max_hp: f32,
    pub armor: f32,
    pub damage: f32,
    pub attack_cooldown_secs: f32,
    pub attack_range: f32,
    #[serde(default)]
    pub ranged_attack_damage: f32,
    #[serde(default)]
    pub ranged_attack_cooldown_secs: f32,
    #[serde(default)]
    pub ranged_attack_range: f32,
    #[serde(default)]
    pub ranged_projectile_speed: f32,
    #[serde(default)]
    pub ranged_projectile_max_distance: f32,
    pub move_speed: f32,
    pub morale: f32,
    #[serde(default = "default_enemy_collision_radius")]
    pub collision_radius: f32,
}

#[derive(Clone, Debug)]
pub struct EnemiesConfigFile {
    pub enemy_christian_peasant_infantry: EnemyStatsConfig,
    pub enemy_christian_peasant_archer: EnemyStatsConfig,
    pub enemy_christian_peasant_priest: EnemyStatsConfig,
    pub enemy_muslim_peasant_infantry: EnemyStatsConfig,
    pub enemy_muslim_peasant_archer: EnemyStatsConfig,
    pub enemy_muslim_peasant_priest: EnemyStatsConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnemiesConfigFileRaw {
    base: EnemiesBaseCatalogRaw,
    #[serde(default)]
    overrides: HashMap<String, EnemiesFactionOverrideRaw>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnemiesBaseCatalogRaw {
    profiles: HashMap<String, EnemyStatsConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnemiesFactionOverrideRaw {
    #[serde(default)]
    profiles: HashMap<String, EnemyStatsConfig>,
}

impl<'de> Deserialize<'de> for EnemiesConfigFile {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = EnemiesConfigFileRaw::deserialize(deserializer)?;
        let base_infantry = raw
            .base
            .profiles
            .get(TIER0_INFANTRY_UNIT_ID)
            .cloned()
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "enemies.base.profiles is missing required key 'peasant_infantry'",
                )
            })?;
        let base_archer = raw
            .base
            .profiles
            .get(TIER0_ARCHER_UNIT_ID)
            .cloned()
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "enemies.base.profiles is missing required key 'peasant_archer'",
                )
            })?;
        let base_priest = raw
            .base
            .profiles
            .get(TIER0_PRIEST_UNIT_ID)
            .cloned()
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "enemies.base.profiles is missing required key 'peasant_priest'",
                )
            })?;
        let mut config = EnemiesConfigFile {
            enemy_christian_peasant_infantry: base_infantry.clone(),
            enemy_christian_peasant_archer: base_archer.clone(),
            enemy_christian_peasant_priest: base_priest.clone(),
            enemy_muslim_peasant_infantry: base_infantry,
            enemy_muslim_peasant_archer: base_archer,
            enemy_muslim_peasant_priest: base_priest,
        };

        for (faction_key, override_config) in raw.overrides {
            let faction = parse_faction_override_key(&faction_key, "enemies.overrides")?;
            for (unit_id, stats) in override_config.profiles {
                let archetype =
                    parse_recruit_archetype_unit_id(&unit_id, "enemies.overrides.*.profiles")?;
                match (faction, archetype) {
                    (PlayerFaction::Christian, RecruitArchetype::Infantry) => {
                        config.enemy_christian_peasant_infantry = stats;
                    }
                    (PlayerFaction::Christian, RecruitArchetype::Archer) => {
                        config.enemy_christian_peasant_archer = stats;
                    }
                    (PlayerFaction::Christian, RecruitArchetype::Priest) => {
                        config.enemy_christian_peasant_priest = stats;
                    }
                    (PlayerFaction::Muslim, RecruitArchetype::Infantry) => {
                        config.enemy_muslim_peasant_infantry = stats;
                    }
                    (PlayerFaction::Muslim, RecruitArchetype::Archer) => {
                        config.enemy_muslim_peasant_archer = stats;
                    }
                    (PlayerFaction::Muslim, RecruitArchetype::Priest) => {
                        config.enemy_muslim_peasant_priest = stats;
                    }
                }
            }
        }

        Ok(config)
    }
}

impl EnemiesConfigFile {
    pub fn enemy_profile_for_kind(&self, kind: UnitKind) -> Option<&EnemyStatsConfig> {
        if kind.is_rescuable_variant() {
            return None;
        }
        let faction = kind.faction()?;
        let archetype = kind.recruit_archetype().or_else(|| {
            if kind.is_support_priest_line() || kind.is_fanatic_line() {
                Some(RecruitArchetype::Priest)
            } else if kind.is_archer_line() {
                Some(RecruitArchetype::Archer)
            } else if kind.is_block_infantry_line() {
                Some(RecruitArchetype::Infantry)
            } else {
                None
            }
        })?;
        self.enemy_profile_for_faction_and_archetype(faction, archetype)
    }

    pub fn enemy_profile_for_faction_and_archetype(
        &self,
        faction: PlayerFaction,
        archetype: RecruitArchetype,
    ) -> Option<&EnemyStatsConfig> {
        match (faction, archetype) {
            (PlayerFaction::Christian, RecruitArchetype::Infantry) => {
                Some(&self.enemy_christian_peasant_infantry)
            }
            (PlayerFaction::Christian, RecruitArchetype::Archer) => {
                Some(&self.enemy_christian_peasant_archer)
            }
            (PlayerFaction::Christian, RecruitArchetype::Priest) => {
                Some(&self.enemy_christian_peasant_priest)
            }
            (PlayerFaction::Muslim, RecruitArchetype::Infantry) => {
                Some(&self.enemy_muslim_peasant_infantry)
            }
            (PlayerFaction::Muslim, RecruitArchetype::Archer) => {
                Some(&self.enemy_muslim_peasant_archer)
            }
            (PlayerFaction::Muslim, RecruitArchetype::Priest) => {
                Some(&self.enemy_muslim_peasant_priest)
            }
        }
    }

    pub fn opposing_enemy_pool(&self, player_faction: PlayerFaction) -> [UnitKind; 3] {
        let kinds = RecruitUnitKind::all_for_faction(player_faction.opposing());
        [
            kinds[0].as_unit_kind(),
            kinds[1].as_unit_kind(),
            kinds[2].as_unit_kind(),
        ]
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemyTierPoolTierConfig {
    pub tier: u8,
    pub melee: Vec<String>,
    pub ranged: Vec<String>,
    pub support: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemyTierPoolsConfigFile {
    pub tiers: Vec<EnemyTierPoolTierConfig>,
}

impl EnemyTierPoolsConfigFile {
    fn tier_entry(&self, tier: u8) -> Option<&EnemyTierPoolTierConfig> {
        self.tiers.iter().find(|entry| entry.tier == tier)
    }

    pub fn unit_ids_for_tier_and_archetype(
        &self,
        tier: u8,
        archetype: RecruitArchetype,
    ) -> Option<&[String]> {
        let entry = self.tier_entry(tier)?;
        match archetype {
            RecruitArchetype::Infantry => Some(entry.melee.as_slice()),
            RecruitArchetype::Archer => Some(entry.ranged.as_slice()),
            RecruitArchetype::Priest => Some(entry.support.as_slice()),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormationConfig {
    pub id: String,
    pub slot_spacing: f32,
    pub offense_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub offense_while_moving_multiplier: f32,
    pub defense_multiplier: f32,
    pub anti_cavalry_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub move_speed_multiplier: f32,
    #[serde(default)]
    pub anti_entry: bool,
    #[serde(default)]
    pub allow_unlimited_enemy_inside: bool,
    #[serde(default)]
    pub shielded_block_bonus: f32,
    #[serde(default)]
    pub melee_reflect_ratio: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormationsConfigFile {
    pub square: FormationConfig,
    pub circle: FormationConfig,
    pub skean: FormationConfig,
    pub diamond: FormationConfig,
    pub shield_wall: FormationConfig,
    pub loose: FormationConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WaveConfig {
    pub time_secs: f32,
    pub count: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WavesConfigFile {
    pub waves: Vec<WaveConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpgradeConfig {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub value: f32,
    #[serde(default)]
    pub reward_lane: Option<String>,
    #[serde(default)]
    pub doctrine_tags: Vec<String>,
    #[serde(default)]
    pub stack_cap: Option<u32>,
    #[serde(default)]
    pub diminishing_factor: Option<f32>,
    #[serde(default)]
    pub downside: Option<String>,
    #[serde(default)]
    pub major_unlock_hint: Option<String>,
    // Legacy roll fields are parsed only so validation can fail with an actionable error.
    #[serde(default)]
    pub min_value: Option<f32>,
    #[serde(default)]
    pub max_value: Option<f32>,
    #[serde(default)]
    pub value_step: Option<f32>,
    #[serde(default)]
    pub weight_exponent: Option<f32>,
    #[serde(default)]
    pub one_time: bool,
    #[serde(default)]
    pub adds_to_skillbar: bool,
    #[serde(default)]
    pub formation_id: Option<String>,
    #[serde(default)]
    pub requirement_type: Option<String>,
    #[serde(default)]
    pub requirement_min_tier0_share: Option<f32>,
    #[serde(default)]
    pub requirement_active_formation: Option<String>,
    #[serde(default)]
    pub requirement_map_tag: Option<String>,
    #[serde(default)]
    pub requirement_trait: Option<String>,
    #[serde(default)]
    pub requirement_band_stat: Option<String>,
    #[serde(default)]
    pub requirement_band_at_least: Option<String>,
    #[serde(default)]
    pub requirement_band_at_most: Option<String>,
    #[serde(default)]
    pub effect_band_shift_stat: Option<String>,
    #[serde(default)]
    pub effect_band_shift_steps: Option<i32>,
    #[serde(default)]
    pub effect_band_floor_stat: Option<String>,
    #[serde(default)]
    pub effect_band_floor_min: Option<String>,
    #[serde(default)]
    pub effect_trait_hook: Option<String>,
    #[serde(default)]
    pub effect_trait_modifier_kind: Option<String>,
    #[serde(default)]
    pub effect_trait_modifier_value: Option<f32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpgradesConfigFile {
    pub upgrades: Vec<UpgradeConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MapDefinitionConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub width: f32,
    pub height: f32,
    pub allowed_factions: Vec<String>,
    #[serde(default)]
    pub spawn_profile_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MapConfig {
    pub maps: Vec<MapDefinitionConfig>,
}

impl MapConfig {
    pub fn first_map(&self) -> Option<&MapDefinitionConfig> {
        self.maps.first()
    }

    pub fn find_map(&self, map_id: &str) -> Option<&MapDefinitionConfig> {
        self.maps.iter().find(|map| map.id == map_id)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RescueConfig {
    pub spawn_count: u32,
    pub rescue_radius: f32,
    pub rescue_duration_secs: f32,
    #[serde(default = "default_rescue_recruit_pool")]
    pub recruit_pool: Vec<RescueRecruitKindConfig>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RescueRecruitKindConfig {
    PeasantInfantry,
    PeasantArcher,
    PeasantPriest,
}

impl RescueRecruitKindConfig {
    pub const fn from_recruit_archetype(archetype: RecruitArchetype) -> Self {
        match archetype {
            RecruitArchetype::Infantry => Self::PeasantInfantry,
            RecruitArchetype::Archer => Self::PeasantArcher,
            RecruitArchetype::Priest => Self::PeasantPriest,
        }
    }

    pub const fn archetype(self) -> RecruitArchetype {
        match self {
            Self::PeasantInfantry => RecruitArchetype::Infantry,
            Self::PeasantArcher => RecruitArchetype::Archer,
            Self::PeasantPriest => RecruitArchetype::Priest,
        }
    }

    pub const fn tier(self) -> u8 {
        match self {
            Self::PeasantInfantry => 0,
            Self::PeasantArcher => 0,
            Self::PeasantPriest => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HeroSubtypeConfig {
    pub id: String,
    pub unit_id: String,
    pub description: String,
    pub entries_by_faction: HashMap<PlayerFaction, Vec<HeroEntryConfig>>,
}

#[derive(Clone, Debug)]
pub struct HeroEntryConfig {
    pub hero_id: String,
    pub name: String,
    pub description: String,
    pub unit_id: String,
    pub abilities: Vec<String>,
    pub stat_notes: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct HeroesConfigFile {
    pub subtypes: HashMap<String, HeroSubtypeConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroesConfigFileRaw {
    base: HeroesBaseCatalogRaw,
    #[serde(default)]
    overrides: HashMap<String, HeroesFactionOverrideRaw>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroesBaseCatalogRaw {
    subtypes: HashMap<String, HeroSubtypeBaseRaw>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroSubtypeBaseRaw {
    unit_id: String,
    description: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroesFactionOverrideRaw {
    #[serde(default)]
    subtypes: HashMap<String, HeroSubtypeEntriesRaw>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroSubtypeEntriesRaw {
    #[serde(default)]
    entries: Vec<HeroEntryRaw>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HeroEntryRaw {
    hero_id: String,
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    unit_id: Option<String>,
    #[serde(default)]
    abilities: Vec<String>,
    #[serde(default)]
    stat_notes: Vec<String>,
}

impl<'de> Deserialize<'de> for HeroesConfigFile {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = HeroesConfigFileRaw::deserialize(deserializer)?;
        if raw.base.subtypes.is_empty() {
            return Err(serde::de::Error::custom(
                "heroes.base.subtypes must contain at least one subtype",
            ));
        }

        let mut subtypes = HashMap::new();
        for (subtype_id, base_subtype) in raw.base.subtypes {
            subtypes.insert(
                subtype_id.clone(),
                HeroSubtypeConfig {
                    id: subtype_id,
                    unit_id: base_subtype.unit_id,
                    description: base_subtype.description,
                    entries_by_faction: HashMap::new(),
                },
            );
        }

        for (faction_key, faction_override) in raw.overrides {
            let faction = parse_faction_override_key(&faction_key, "heroes.overrides")?;
            for (subtype_id, subtype_entries) in faction_override.subtypes {
                let Some(base_subtype) = subtypes.get_mut(&subtype_id) else {
                    return Err(serde::de::Error::custom(format!(
                        "heroes.overrides.{faction_key}.subtypes has unknown subtype '{subtype_id}'"
                    )));
                };
                let resolved_entries: Vec<HeroEntryConfig> = subtype_entries
                    .entries
                    .into_iter()
                    .map(|entry| HeroEntryConfig {
                        hero_id: entry.hero_id,
                        name: entry.name,
                        description: entry
                            .description
                            .unwrap_or_else(|| base_subtype.description.clone()),
                        unit_id: entry
                            .unit_id
                            .unwrap_or_else(|| base_subtype.unit_id.clone()),
                        abilities: entry.abilities,
                        stat_notes: entry.stat_notes,
                    })
                    .collect();
                base_subtype
                    .entries_by_faction
                    .insert(faction, resolved_entries);
            }
        }

        Ok(HeroesConfigFile { subtypes })
    }
}

impl HeroesConfigFile {
    pub fn subtype_for_id(&self, subtype_id: &str) -> Option<&HeroSubtypeConfig> {
        self.subtypes.get(subtype_id)
    }

    pub fn unit_id_for_subtype(&self, subtype_id: &str) -> Option<&str> {
        self.subtype_for_id(subtype_id)
            .map(|subtype| subtype.unit_id.as_str())
    }

    pub fn entries_for_faction_and_subtype(
        &self,
        faction: PlayerFaction,
        subtype_id: &str,
    ) -> Option<&[HeroEntryConfig]> {
        self.subtype_for_id(subtype_id)
            .and_then(|subtype| subtype.entries_by_faction.get(&faction))
            .map(Vec::as_slice)
    }

    pub fn roll_entry_for_faction_and_subtype(
        &self,
        faction: PlayerFaction,
        subtype_id: &str,
        seed: u32,
    ) -> Option<&HeroEntryConfig> {
        let entries = self.entries_for_faction_and_subtype(faction, subtype_id)?;
        if entries.is_empty() {
            return None;
        }
        let index = (seed as usize) % entries.len();
        entries.get(index)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DropsConfig {
    pub initial_spawn_count: u32,
    pub spawn_interval_secs: f32,
    pub pickup_radius: f32,
    #[serde(alias = "xp_per_pack")]
    pub gold_per_pack: f32,
    pub max_active_packs: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FactionGameplayConfig {
    #[serde(default = "default_multiplier")]
    pub friendly_health_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub friendly_damage_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub friendly_attack_speed_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub friendly_move_speed_multiplier: f32,
    #[serde(default)]
    pub friendly_armor_bonus: f32,
    #[serde(default = "default_multiplier")]
    pub friendly_morale_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub friendly_morale_gain_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub friendly_morale_loss_multiplier: f32,
    #[serde(default)]
    pub commander_aura_radius_bonus: f32,
    #[serde(default = "default_multiplier", alias = "xp_gain_multiplier")]
    pub gold_gain_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub rescue_time_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub authority_enemy_morale_drain_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_health_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_damage_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_attack_speed_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_move_speed_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_morale_multiplier: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FactionsConfigFile {
    pub christian: FactionGameplayConfig,
    pub muslim: FactionGameplayConfig,
}

impl FactionsConfigFile {
    pub fn for_faction(&self, faction: PlayerFaction) -> &FactionGameplayConfig {
        match faction {
            PlayerFaction::Christian => &self.christian,
            PlayerFaction::Muslim => &self.muslim,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DifficultyGameplayConfig {
    #[serde(default = "default_multiplier")]
    pub enemy_health_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_damage_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_attack_speed_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_move_speed_multiplier: f32,
    #[serde(default = "default_multiplier")]
    pub enemy_morale_multiplier: f32,
    #[serde(default, alias = "enemy_dodge_enabled")]
    pub enemy_ranged_dodge_enabled: bool,
    #[serde(default)]
    pub enemy_block_enabled: bool,
    #[serde(default)]
    pub ranged_support_avoid_melee: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DifficultiesConfigFile {
    pub recruit: DifficultyGameplayConfig,
    pub experienced: DifficultyGameplayConfig,
    pub alone_against_the_infidels: DifficultyGameplayConfig,
}

impl DifficultiesConfigFile {
    pub fn for_difficulty(&self, difficulty: GameDifficulty) -> &DifficultyGameplayConfig {
        match difficulty {
            GameDifficulty::Recruit => &self.recruit,
            GameDifficulty::Experienced => &self.experienced,
            GameDifficulty::AloneAgainstTheInfidels => &self.alone_against_the_infidels,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RosterBehaviorConfig {
    pub tracker_hound_active_secs: f32,
    pub tracker_hound_cooldown_secs: f32,
    pub tracker_hound_strike_interval_secs: f32,
    pub tracker_hound_damage_multiplier: f32,
    pub scout_raid_active_secs: f32,
    pub scout_raid_cooldown_secs: f32,
    pub scout_raid_speed_multiplier: f32,
    pub fanatic_life_leech_ratio: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RosterTuningConfigFile {
    pub tier2_units: HashMap<String, UnitStatsConfig>,
    pub behavior: RosterBehaviorConfig,
}

impl RosterTuningConfigFile {
    pub fn tier2_stats_for_kind(&self, kind: UnitKind) -> Option<&UnitStatsConfig> {
        let key = tier2_config_key_for_kind(kind)?;
        self.tier2_units.get(key.as_str())
    }
}

#[derive(Resource, Clone, Debug)]
pub struct GameData {
    pub units: UnitsConfigFile,
    pub enemies: EnemiesConfigFile,
    pub heroes: HeroesConfigFile,
    pub enemy_tier_pools: EnemyTierPoolsConfigFile,
    pub formations: FormationsConfigFile,
    pub waves: WavesConfigFile,
    pub upgrades: UpgradesConfigFile,
    pub map: MapConfig,
    pub rescue: RescueConfig,
    pub drops: DropsConfig,
    pub factions: FactionsConfigFile,
    pub difficulties: DifficultiesConfigFile,
    pub roster_tuning: RosterTuningConfigFile,
}

impl GameData {
    pub fn load_from_dir(base_dir: &Path) -> Result<Self> {
        let units: UnitsConfigFile = read_json(base_dir.join("units.json"))?;
        let enemies: EnemiesConfigFile = read_json(base_dir.join("enemies.json"))?;
        let heroes: HeroesConfigFile = read_json(base_dir.join("heroes.json"))?;
        let enemy_tier_pools: EnemyTierPoolsConfigFile =
            read_json(base_dir.join("enemy_tier_pools.json"))?;
        let formations: FormationsConfigFile = read_json(base_dir.join("formations.json"))?;
        let waves: WavesConfigFile = read_json(base_dir.join("waves.json"))?;
        let upgrades: UpgradesConfigFile = read_json(base_dir.join("upgrades.json"))?;
        let map: MapConfig = read_json(base_dir.join("map.json"))?;
        let rescue: RescueConfig = read_json(base_dir.join("rescue.json"))?;
        let drops: DropsConfig = read_json(base_dir.join("drops.json"))?;
        let factions: FactionsConfigFile = read_json(base_dir.join("factions.json"))?;
        let difficulties: DifficultiesConfigFile = read_json(base_dir.join("difficulties.json"))?;
        let roster_tuning: RosterTuningConfigFile = read_json(base_dir.join("roster_tuning.json"))?;

        validate_units(&units)?;
        validate_enemies(&enemies)?;
        validate_heroes(&heroes)?;
        validate_enemy_tier_pools(&enemy_tier_pools)?;
        validate_formations(&formations)?;
        validate_waves(&waves)?;
        validate_upgrades(&upgrades)?;
        validate_map(&map)?;
        validate_rescue(&rescue)?;
        validate_drops(&drops)?;
        validate_factions(&factions)?;
        validate_difficulties(&difficulties)?;
        validate_roster_tuning(&roster_tuning)?;

        Ok(Self {
            units,
            enemies,
            heroes,
            enemy_tier_pools,
            formations,
            waves,
            upgrades,
            map,
            rescue,
            drops,
            factions,
            difficulties,
            roster_tuning,
        })
    }
}

fn tier2_config_key_for_kind(kind: UnitKind) -> Option<String> {
    let faction = kind.faction()?;
    let unit_id = kind.unit_id();
    if !REQUIRED_TIER2_UNIT_IDS.contains(&unit_id) {
        return None;
    }
    Some(format!("{}_{}", faction.config_key(), unit_id))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|err| anyhow!("failed to parse config file {}: {err}", path.display()))
}

fn default_multiplier() -> f32 {
    1.0
}

fn default_rescue_recruit_pool() -> Vec<RescueRecruitKindConfig> {
    vec![
        RescueRecruitKindConfig::from_recruit_archetype(RecruitArchetype::Infantry),
        RescueRecruitKindConfig::from_recruit_archetype(RecruitArchetype::Archer),
        RescueRecruitKindConfig::from_recruit_archetype(RecruitArchetype::Priest),
    ]
}

fn default_enemy_collision_radius() -> f32 {
    15.0
}

fn parse_faction_override_key<E>(
    value: &str,
    context: &str,
) -> std::result::Result<PlayerFaction, E>
where
    E: serde::de::Error,
{
    let normalized = value.trim().to_ascii_lowercase();
    for faction in PlayerFaction::all() {
        if normalized == faction.config_key() {
            return Ok(faction);
        }
    }
    Err(E::custom(format!(
        "{context} has unknown faction key '{normalized}'"
    )))
}

fn is_supported_faction_key(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    PlayerFaction::all()
        .into_iter()
        .any(|faction| normalized == faction.config_key())
}

fn default_start_faction_key() -> &'static str {
    PlayerFaction::default().config_key()
}

fn has_required_map_start_faction(map: &MapDefinitionConfig, required_faction_key: &str) -> bool {
    map.allowed_factions.iter().any(|faction_key| {
        faction_key
            .trim()
            .eq_ignore_ascii_case(required_faction_key)
    })
}

fn validate_map_allowed_factions(map: &MapDefinitionConfig, index: usize) -> Result<()> {
    for faction in &map.allowed_factions {
        if !is_supported_faction_key(faction) {
            bail!("map[{index}] has unknown faction '{faction}'");
        }
    }
    Ok(())
}

fn parse_recruit_archetype_unit_id<E>(
    unit_id: &str,
    context: &str,
) -> std::result::Result<RecruitArchetype, E>
where
    E: serde::de::Error,
{
    match unit_id {
        "peasant_infantry" => Ok(RecruitArchetype::Infantry),
        "peasant_archer" => Ok(RecruitArchetype::Archer),
        "peasant_priest" => Ok(RecruitArchetype::Priest),
        _ => Err(E::custom(format!(
            "{context} has unknown unit_id '{unit_id}' (expected peasant_infantry|peasant_archer|peasant_priest)"
        ))),
    }
}

fn validate_unit_stats(unit: &UnitStatsConfig, label: &str, allow_zero_damage: bool) -> Result<()> {
    if unit.max_hp <= 0.0 {
        bail!("{label} max_hp must be > 0");
    }
    if (allow_zero_damage && unit.damage < 0.0) || (!allow_zero_damage && unit.damage <= 0.0) {
        if allow_zero_damage {
            bail!("{label} damage must be >= 0");
        }
        bail!("{label} damage must be > 0");
    }
    if unit.attack_cooldown_secs <= 0.0 {
        bail!("{label} attack_cooldown_secs must be > 0");
    }
    if unit.attack_range <= 0.0 {
        bail!("{label} attack_range must be > 0");
    }
    if unit.move_speed <= 0.0 {
        bail!("{label} move_speed must be > 0");
    }
    if unit.morale <= 0.0 {
        bail!("{label} morale must be > 0");
    }
    let ranged_fields = [
        unit.ranged_attack_damage,
        unit.ranged_attack_cooldown_secs,
        unit.ranged_attack_range,
        unit.ranged_projectile_speed,
        unit.ranged_projectile_max_distance,
    ];
    let has_any_ranged = ranged_fields.iter().any(|value| *value > 0.0);
    if has_any_ranged && ranged_fields.iter().any(|value| *value <= 0.0) {
        bail!("{label} ranged fields must all be > 0 when any ranged field is set");
    }
    Ok(())
}

fn validate_units(config: &UnitsConfigFile) -> Result<()> {
    validate_unit_stats(&config.commander_christian, "commander_christian", false)?;
    validate_unit_stats(&config.commander_muslim, "commander_muslim", false)?;
    validate_unit_stats(
        &config.recruit_christian_peasant_infantry,
        "recruit_christian_peasant_infantry",
        false,
    )?;
    validate_unit_stats(
        &config.recruit_christian_peasant_archer,
        "recruit_christian_peasant_archer",
        false,
    )?;
    validate_unit_stats(
        &config.recruit_christian_peasant_priest,
        "recruit_christian_peasant_priest",
        true,
    )?;
    validate_unit_stats(
        &config.recruit_muslim_peasant_infantry,
        "recruit_muslim_peasant_infantry",
        false,
    )?;
    validate_unit_stats(
        &config.recruit_muslim_peasant_archer,
        "recruit_muslim_peasant_archer",
        false,
    )?;
    validate_unit_stats(
        &config.recruit_muslim_peasant_priest,
        "recruit_muslim_peasant_priest",
        true,
    )
}

fn validate_enemies(config: &EnemiesConfigFile) -> Result<()> {
    validate_enemy_stats(
        &config.enemy_christian_peasant_infantry,
        "enemy_christian_peasant_infantry",
    )?;
    validate_enemy_stats(
        &config.enemy_christian_peasant_archer,
        "enemy_christian_peasant_archer",
    )?;
    validate_enemy_stats(
        &config.enemy_christian_peasant_priest,
        "enemy_christian_peasant_priest",
    )?;
    validate_enemy_stats(
        &config.enemy_muslim_peasant_infantry,
        "enemy_muslim_peasant_infantry",
    )?;
    validate_enemy_stats(
        &config.enemy_muslim_peasant_archer,
        "enemy_muslim_peasant_archer",
    )?;
    validate_enemy_stats(
        &config.enemy_muslim_peasant_priest,
        "enemy_muslim_peasant_priest",
    )?;
    Ok(())
}

fn validate_heroes(config: &HeroesConfigFile) -> Result<()> {
    if config.subtypes.is_empty() {
        bail!("heroes.subtypes must contain at least one subtype");
    }

    for required_subtype in REQUIRED_HERO_SUBTYPE_IDS {
        if !config.subtypes.contains_key(required_subtype) {
            bail!("heroes.subtypes is missing required subtype '{required_subtype}'");
        }
    }

    for (subtype_id, subtype) in &config.subtypes {
        if subtype_id.trim().is_empty() {
            bail!("heroes.subtypes has empty subtype id key");
        }
        if subtype.id.trim().is_empty() {
            bail!("heroes.subtypes.{subtype_id}.id must be non-empty");
        }
        if subtype.id != *subtype_id {
            bail!(
                "heroes.subtypes.{subtype_id}.id ('{}') must match map key",
                subtype.id
            );
        }
        if subtype.description.trim().is_empty() {
            bail!("heroes.subtypes.{subtype_id}.description must be non-empty");
        }
        if subtype.unit_id.trim().is_empty() {
            bail!("heroes.subtypes.{subtype_id}.unit_id must be non-empty");
        }

        for faction in PlayerFaction::all() {
            let Some(base_kind) =
                UnitKind::from_faction_and_unit_id(faction, &subtype.unit_id, false)
            else {
                bail!(
                    "heroes.subtypes.{subtype_id}.unit_id '{}' does not resolve for faction {:?}",
                    subtype.unit_id,
                    faction
                );
            };
            if base_kind.tier_hint().unwrap_or(0) < 5 {
                bail!(
                    "heroes.subtypes.{subtype_id}.unit_id '{}' must resolve to tier-5+ unit for faction {:?}",
                    subtype.unit_id,
                    faction
                );
            }

            let Some(entries) = subtype.entries_by_faction.get(&faction) else {
                bail!(
                    "heroes.subtypes.{subtype_id} is missing entries for faction {:?}",
                    faction
                );
            };
            if entries.is_empty() {
                bail!(
                    "heroes.subtypes.{subtype_id} entries for faction {:?} cannot be empty",
                    faction
                );
            }

            let mut seen_ids: HashSet<&str> = HashSet::new();
            for (index, entry) in entries.iter().enumerate() {
                if entry.hero_id.trim().is_empty() {
                    bail!(
                        "heroes.subtypes.{subtype_id}.entries[{index}] hero_id must be non-empty"
                    );
                }
                if !seen_ids.insert(entry.hero_id.as_str()) {
                    bail!(
                        "heroes.subtypes.{subtype_id} has duplicate hero_id '{}' for faction {:?}",
                        entry.hero_id,
                        faction
                    );
                }
                if entry.name.trim().is_empty() {
                    bail!("heroes.subtypes.{subtype_id}.entries[{index}] name must be non-empty");
                }
                if entry.description.trim().is_empty() {
                    bail!(
                        "heroes.subtypes.{subtype_id}.entries[{index}] description must be non-empty"
                    );
                }
                if entry.unit_id.trim().is_empty() {
                    bail!(
                        "heroes.subtypes.{subtype_id}.entries[{index}] unit_id must be non-empty"
                    );
                }
                let Some(kind) = UnitKind::from_faction_and_unit_id(faction, &entry.unit_id, false)
                else {
                    bail!(
                        "heroes.subtypes.{subtype_id}.entries[{index}] unit_id '{}' does not resolve for faction {:?}",
                        entry.unit_id,
                        faction
                    );
                };
                if kind.tier_hint().unwrap_or(0) < 5 {
                    bail!(
                        "heroes.subtypes.{subtype_id}.entries[{index}] unit_id '{}' must resolve to tier-5+ unit for faction {:?}",
                        entry.unit_id,
                        faction
                    );
                }
            }
            if entries.len() != HERO_ENTRIES_PER_SUBTYPE_PER_FACTION {
                bail!(
                    "heroes.subtypes.{subtype_id} entries for faction {:?} must contain exactly {} heroes (found {})",
                    faction,
                    HERO_ENTRIES_PER_SUBTYPE_PER_FACTION,
                    entries.len()
                );
            }
        }
    }

    Ok(())
}

fn validate_enemy_stats(enemy: &EnemyStatsConfig, label: &str) -> Result<()> {
    if enemy.max_hp <= 0.0 {
        bail!("{label} max_hp must be > 0");
    }
    if enemy.attack_cooldown_secs <= 0.0 {
        bail!("{label} attack_cooldown_secs must be > 0");
    }
    if enemy.damage < 0.0 {
        bail!("{label} damage must be >= 0");
    }
    if enemy.attack_range <= 0.0 {
        bail!("{label} attack_range must be > 0");
    }
    if enemy.move_speed <= 0.0 {
        bail!("{label} move_speed must be > 0");
    }
    if enemy.morale <= 0.0 {
        bail!("{label} morale must be > 0");
    }

    if enemy.collision_radius <= 0.0 {
        bail!("{label} collision_radius must be > 0");
    }
    let ranged_fields = [
        enemy.ranged_attack_damage,
        enemy.ranged_attack_cooldown_secs,
        enemy.ranged_attack_range,
        enemy.ranged_projectile_speed,
        enemy.ranged_projectile_max_distance,
    ];
    let has_any_ranged = ranged_fields.iter().any(|value| *value > 0.0);
    if has_any_ranged && ranged_fields.iter().any(|value| *value <= 0.0) {
        bail!("{label} ranged fields must all be > 0 when any ranged field is set");
    }
    Ok(())
}

fn validate_enemy_tier_pools(config: &EnemyTierPoolsConfigFile) -> Result<()> {
    if config.tiers.is_empty() {
        bail!("enemy_tier_pools.tiers must contain at least one entry");
    }
    let mut seen_tiers = HashSet::new();
    for entry in &config.tiers {
        if entry.tier > 5 {
            bail!(
                "enemy_tier_pools tier {} is out of range (expected 0..=5)",
                entry.tier
            );
        }
        if !seen_tiers.insert(entry.tier) {
            bail!("enemy_tier_pools has duplicate tier entry {}", entry.tier);
        }
        validate_enemy_tier_pool_ids(entry.tier, RecruitArchetype::Infantry, &entry.melee)?;
        validate_enemy_tier_pool_ids(entry.tier, RecruitArchetype::Archer, &entry.ranged)?;
        validate_enemy_tier_pool_ids(entry.tier, RecruitArchetype::Priest, &entry.support)?;
    }
    for required in 0..=5 {
        if !seen_tiers.contains(&required) {
            bail!("enemy_tier_pools is missing required tier {}", required);
        }
    }
    Ok(())
}

fn validate_enemy_tier_pool_ids(
    tier: u8,
    archetype: RecruitArchetype,
    ids: &[String],
) -> Result<()> {
    if ids.is_empty() {
        bail!(
            "enemy_tier_pools tier {} {:?} list cannot be empty",
            tier,
            archetype
        );
    }
    for unit_id in ids {
        for faction in PlayerFaction::all() {
            let Some(kind) = UnitKind::from_faction_and_unit_id(faction, unit_id, false) else {
                bail!(
                    "enemy_tier_pools tier {} references unknown unit_id '{}' for faction {:?}",
                    tier,
                    unit_id,
                    faction
                );
            };
            if kind.tier_hint() != Some(tier) {
                bail!(
                    "enemy_tier_pools unit_id '{}' for faction {:?} resolves to tier {:?}, expected {}",
                    unit_id,
                    faction,
                    kind.tier_hint(),
                    tier
                );
            }
            let valid_archetype = match archetype {
                RecruitArchetype::Infantry => kind.is_block_infantry_line(),
                RecruitArchetype::Archer => kind.is_archer_line(),
                RecruitArchetype::Priest => kind.is_priest_family_line(),
            };
            if !valid_archetype {
                bail!(
                    "enemy_tier_pools unit_id '{}' for faction {:?} is not valid for {:?} tier bucket",
                    unit_id,
                    faction,
                    archetype
                );
            }
        }
    }
    Ok(())
}

fn validate_formations(config: &FormationsConfigFile) -> Result<()> {
    validate_formation("square", &config.square)?;
    validate_formation("circle", &config.circle)?;
    validate_formation("skean", &config.skean)?;
    validate_formation("diamond", &config.diamond)?;
    validate_formation("shield_wall", &config.shield_wall)?;
    validate_formation("loose", &config.loose)?;
    Ok(())
}

fn validate_formation(label: &str, formation: &FormationConfig) -> Result<()> {
    if formation.slot_spacing <= 0.0 {
        bail!("{label} slot_spacing must be > 0");
    }
    if formation.offense_multiplier <= 0.0 || formation.defense_multiplier <= 0.0 {
        bail!("{label} offense/defense multipliers must be > 0");
    }
    if formation.offense_while_moving_multiplier <= 0.0 {
        bail!("{label} offense_while_moving_multiplier must be > 0");
    }
    if formation.move_speed_multiplier <= 0.0 {
        bail!("{label} move_speed_multiplier must be > 0");
    }
    if formation.anti_cavalry_multiplier <= 0.0 {
        bail!("{label} anti_cavalry_multiplier must be > 0");
    }
    if !(0.0..=0.95).contains(&formation.shielded_block_bonus) {
        bail!("{label} shielded_block_bonus must be in [0, 0.95]");
    }
    if !(0.0..=1.0).contains(&formation.melee_reflect_ratio) {
        bail!("{label} melee_reflect_ratio must be in [0, 1]");
    }
    if formation.anti_entry && formation.allow_unlimited_enemy_inside {
        bail!("{label} cannot enable both anti_entry and allow_unlimited_enemy_inside");
    }
    Ok(())
}

fn validate_waves(config: &WavesConfigFile) -> Result<()> {
    if config.waves.is_empty() {
        bail!("waves list cannot be empty");
    }
    let mut previous_time = -1.0;
    for (idx, wave) in config.waves.iter().enumerate() {
        if wave.time_secs < 0.0 {
            bail!("wave[{idx}] time_secs cannot be negative");
        }
        if wave.count == 0 {
            bail!("wave[{idx}] count must be > 0");
        }
        if wave.time_secs <= previous_time {
            bail!("waves must be strictly increasing by time_secs");
        }
        previous_time = wave.time_secs;
    }
    Ok(())
}

fn validate_upgrades(config: &UpgradesConfigFile) -> Result<()> {
    if config.upgrades.is_empty() {
        bail!("upgrades list cannot be empty");
    }
    let mut ids = HashSet::new();
    for (idx, upgrade) in config.upgrades.iter().enumerate() {
        let upgrade_id = upgrade.id.trim();
        if upgrade_id.is_empty() || upgrade.kind.trim().is_empty() {
            bail!("upgrade[{idx}] id and kind must be non-empty");
        }
        if let Some(replacement) = deprecated_upgrade_replacement(upgrade_id) {
            bail!("upgrade[{idx}] id '{upgrade_id}' is deprecated; use '{replacement}' instead");
        }
        if !ids.insert(upgrade_id.to_string()) {
            bail!("upgrade[{idx}] duplicate id '{upgrade_id}' is not allowed");
        }
        if !crate::upgrades::is_supported_upgrade_kind(upgrade.kind.as_str()) {
            bail!(
                "upgrade[{idx}] kind '{}' is not wired in runtime systems",
                upgrade.kind
            );
        }
        let Some(reward_lane) = upgrade.reward_lane.as_deref().map(str::trim) else {
            bail!("upgrade[{idx}] reward_lane is required and must be 'minor' or 'major'");
        };
        if !is_supported_upgrade_reward_lane(reward_lane) {
            bail!(
                "upgrade[{idx}] unknown reward_lane '{reward_lane}', expected one of: minor, major"
            );
        }
        let is_formation_unlock = upgrade.kind == "unlock_formation";
        if is_formation_unlock {
            let Some(formation_id) = upgrade.formation_id.as_deref() else {
                bail!("upgrade[{idx}] unlock_formation requires formation_id");
            };
            if formation_id.trim().is_empty() {
                bail!("upgrade[{idx}] formation_id must be non-empty");
            }
            if !is_supported_formation_id(formation_id) {
                bail!(
                    "upgrade[{idx}] unknown formation_id '{formation_id}', expected one of: square, circle, skean, diamond, shield_wall, loose"
                );
            }
            if !upgrade.adds_to_skillbar {
                bail!("upgrade[{idx}] unlock_formation must set adds_to_skillbar=true");
            }
        }
        if upgrade.value <= 0.0 {
            bail!("upgrade[{idx}] value must be > 0");
        }
        if upgrade.min_value.is_some()
            || upgrade.max_value.is_some()
            || upgrade.value_step.is_some()
            || upgrade.weight_exponent.is_some()
        {
            bail!(
                "upgrade[{idx}] legacy roll fields are no longer supported: min_value/max_value/value_step/weight_exponent"
            );
        }
        if let Some(stack_cap) = upgrade.stack_cap
            && stack_cap == 0
        {
            bail!("upgrade[{idx}] stack_cap must be > 0 when provided");
        }
        if let Some(diminishing_factor) = upgrade.diminishing_factor
            && !(0.0..1.0).contains(&diminishing_factor)
        {
            bail!("upgrade[{idx}] diminishing_factor must be in the range (0, 1)");
        }
        if reward_lane == "major"
            && upgrade
                .downside
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            bail!("upgrade[{idx}] major upgrades must provide a non-empty downside");
        }
        validate_upgrade_requirement(idx, upgrade)?;
        validate_upgrade_semantics(idx, upgrade)?;
    }
    Ok(())
}

fn validate_upgrade_requirement(idx: usize, upgrade: &UpgradeConfig) -> Result<()> {
    let Some(requirement_kind) = upgrade
        .requirement_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    match requirement_kind {
        "tier0_share" => {
            let Some(share) = upgrade.requirement_min_tier0_share else {
                bail!(
                    "upgrade[{idx}] tier0_share requirement requires requirement_min_tier0_share"
                );
            };
            if !(0.0..=1.0).contains(&share) {
                bail!("upgrade[{idx}] requirement_min_tier0_share must be in [0,1]");
            }
        }
        "formation_active" => {
            let Some(formation_id) = upgrade.requirement_active_formation.as_deref() else {
                bail!(
                    "upgrade[{idx}] formation_active requirement requires requirement_active_formation"
                );
            };
            if !is_supported_formation_id(formation_id) {
                bail!(
                    "upgrade[{idx}] unknown requirement_active_formation '{formation_id}', expected one of: square, circle, skean, diamond, shield_wall, loose"
                );
            }
        }
        "map_tag" => {
            let Some(tag) = upgrade.requirement_map_tag.as_deref() else {
                bail!("upgrade[{idx}] map_tag requirement requires requirement_map_tag");
            };
            if tag.trim().is_empty() {
                bail!("upgrade[{idx}] requirement_map_tag must be non-empty");
            }
        }
        "has_trait" => {
            let Some(trait_id) = upgrade.requirement_trait.as_deref().map(str::trim) else {
                bail!("upgrade[{idx}] has_trait requirement requires requirement_trait");
            };
            if !is_supported_upgrade_requirement_trait(trait_id) {
                bail!(
                    "upgrade[{idx}] unknown requirement_trait '{trait_id}', expected one of: shielded, frontline, anti_cavalry, cavalry, anti_armor, skirmisher, support"
                );
            }
        }
        "band_at_least" | "band_at_most" => {
            let Some(stat_id) = upgrade.requirement_band_stat.as_deref().map(str::trim) else {
                bail!(
                    "upgrade[{idx}] {requirement_kind} requirement requires requirement_band_stat"
                );
            };
            if !is_supported_upgrade_requirement_band_stat(stat_id) {
                bail!(
                    "upgrade[{idx}] unknown requirement_band_stat '{stat_id}', expected one of: tier0_share, shielded_share, frontline_share, anti_cavalry_share, support_share, cavalry_share, archer_share, anti_armor_share"
                );
            }
            let bound = if requirement_kind == "band_at_least" {
                upgrade
                    .requirement_band_at_least
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            } else {
                upgrade
                    .requirement_band_at_most
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            };
            let Some(band_id) = bound else {
                bail!(
                    "upgrade[{idx}] {requirement_kind} requirement requires {}",
                    if requirement_kind == "band_at_least" {
                        "requirement_band_at_least"
                    } else {
                        "requirement_band_at_most"
                    }
                );
            };
            if !is_supported_upgrade_requirement_band_id(band_id) {
                bail!(
                    "upgrade[{idx}] unknown requirement band '{band_id}', expected one of: very_low, low, moderate, high, very_high"
                );
            }
        }
        other => bail!(
            "upgrade[{idx}] unknown requirement_type={other}; supported: tier0_share, formation_active, map_tag, has_trait, band_at_least, band_at_most"
        ),
    }
    Ok(())
}

fn validate_upgrade_semantics(idx: usize, upgrade: &UpgradeConfig) -> Result<()> {
    let shift_stat = upgrade
        .effect_band_shift_stat
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let shift_steps = upgrade.effect_band_shift_steps;
    if shift_stat.is_some() || shift_steps.is_some() {
        let Some(stat_id) = shift_stat else {
            bail!("upgrade[{idx}] effect_band_shift_steps requires effect_band_shift_stat");
        };
        if !is_supported_upgrade_effect_band_stat(stat_id) {
            bail!(
                "upgrade[{idx}] unknown effect_band_shift_stat '{stat_id}', expected one of: damage, armor, move_speed, luck"
            );
        }
        let Some(steps) = shift_steps else {
            bail!("upgrade[{idx}] effect_band_shift_stat requires effect_band_shift_steps");
        };
        if steps == 0 || !(-4..=4).contains(&steps) {
            bail!("upgrade[{idx}] effect_band_shift_steps must be in [-4, -1] or [1, 4]");
        }
    }

    let floor_stat = upgrade
        .effect_band_floor_stat
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let floor_min = upgrade
        .effect_band_floor_min
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if floor_stat.is_some() || floor_min.is_some() {
        let Some(stat_id) = floor_stat else {
            bail!("upgrade[{idx}] effect_band_floor_min requires effect_band_floor_stat");
        };
        if !is_supported_upgrade_effect_band_stat(stat_id) {
            bail!(
                "upgrade[{idx}] unknown effect_band_floor_stat '{stat_id}', expected one of: damage, armor, move_speed, luck"
            );
        }
        let Some(min_band) = floor_min else {
            bail!("upgrade[{idx}] effect_band_floor_stat requires effect_band_floor_min");
        };
        if !is_supported_upgrade_requirement_band_id(min_band) {
            bail!(
                "upgrade[{idx}] unknown effect_band_floor_min '{min_band}', expected one of: very_low, low, moderate, high, very_high"
            );
        }
    }

    if let (Some(shift_stat), Some(floor_stat)) = (shift_stat, floor_stat)
        && shift_stat != floor_stat
    {
        bail!(
            "upgrade[{idx}] effect_band_shift_stat and effect_band_floor_stat must match when both are provided"
        );
    }

    let trait_hook = upgrade
        .effect_trait_hook
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let trait_modifier_kind = upgrade
        .effect_trait_modifier_kind
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let trait_modifier_value = upgrade.effect_trait_modifier_value;
    if trait_hook.is_some() || trait_modifier_kind.is_some() || trait_modifier_value.is_some() {
        let Some(trait_id) = trait_hook else {
            bail!("upgrade[{idx}] effect_trait_modifier_* fields require effect_trait_hook");
        };
        if !is_supported_upgrade_requirement_trait(trait_id) {
            bail!(
                "upgrade[{idx}] unknown effect_trait_hook '{trait_id}', expected one of: shielded, frontline, anti_cavalry, cavalry, anti_armor, skirmisher, support"
            );
        }
        let Some(modifier_kind) = trait_modifier_kind else {
            bail!("upgrade[{idx}] effect_trait_hook requires effect_trait_modifier_kind");
        };
        if !is_supported_upgrade_effect_trait_modifier_kind(modifier_kind) {
            bail!(
                "upgrade[{idx}] unknown effect_trait_modifier_kind '{modifier_kind}', expected one of: damage_multiplier, attack_speed_multiplier, move_speed_bonus, armor_bonus, morale_loss_multiplier, execute_threshold, rescue_speed_multiplier"
            );
        }
        if is_multiplicative_trait_modifier_kind(modifier_kind)
            && !is_whitelisted_multiplicative_upgrade_semantic_kind(upgrade.kind.as_str())
        {
            bail!(
                "upgrade[{idx}] effect_trait_modifier_kind '{modifier_kind}' is restricted to whitelisted doctrine kinds; use additive semantic kinds for non-whitelisted upgrades"
            );
        }
        let Some(value) = trait_modifier_value else {
            bail!("upgrade[{idx}] effect_trait_hook requires effect_trait_modifier_value");
        };
        if !value.is_finite() || value.abs() <= f32::EPSILON {
            bail!("upgrade[{idx}] effect_trait_modifier_value must be finite and non-zero");
        }
        if upgrade.requirement_type.as_deref().map(str::trim) == Some("has_trait")
            && let Some(requirement_trait) = upgrade
                .requirement_trait
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            && requirement_trait != trait_id
        {
            bail!(
                "upgrade[{idx}] effect_trait_hook must match requirement_trait when requirement_type=has_trait"
            );
        }
    }

    Ok(())
}

fn is_supported_upgrade_requirement_trait(trait_id: &str) -> bool {
    matches!(
        trait_id,
        "shielded"
            | "frontline"
            | "anti_cavalry"
            | "cavalry"
            | "anti_armor"
            | "skirmisher"
            | "support"
    )
}

fn is_supported_upgrade_effect_band_stat(stat_id: &str) -> bool {
    matches!(stat_id, "damage" | "armor" | "move_speed" | "luck")
}

fn is_supported_upgrade_effect_trait_modifier_kind(kind: &str) -> bool {
    matches!(
        kind,
        "damage_multiplier"
            | "attack_speed_multiplier"
            | "move_speed_bonus"
            | "armor_bonus"
            | "morale_loss_multiplier"
            | "execute_threshold"
            | "rescue_speed_multiplier"
    )
}

fn is_multiplicative_trait_modifier_kind(kind: &str) -> bool {
    matches!(
        kind,
        "damage_multiplier"
            | "attack_speed_multiplier"
            | "morale_loss_multiplier"
            | "rescue_speed_multiplier"
    )
}

fn is_whitelisted_multiplicative_upgrade_semantic_kind(kind: &str) -> bool {
    matches!(
        kind,
        "mob_fury"
            | "mob_mercy"
            | "doctrine_execution_rites"
            | "doctrine_countervolley"
            | "doctrine_pike_hedgehog"
    )
}

fn is_supported_upgrade_reward_lane(lane: &str) -> bool {
    matches!(lane, "minor" | "major")
}

fn deprecated_upgrade_replacement(id: &str) -> Option<&'static str> {
    DEPRECATED_UPGRADE_ID_REPLACEMENTS
        .iter()
        .find_map(|(deprecated, replacement)| (*deprecated == id).then_some(*replacement))
}

fn is_supported_formation_id(formation_id: &str) -> bool {
    matches!(
        formation_id,
        "square" | "circle" | "skean" | "diamond" | "shield_wall" | "loose"
    )
}

fn is_supported_upgrade_requirement_band_stat(stat_id: &str) -> bool {
    matches!(
        stat_id,
        "tier0_share"
            | "shielded_share"
            | "frontline_share"
            | "anti_cavalry_share"
            | "support_share"
            | "cavalry_share"
            | "archer_share"
            | "anti_armor_share"
    )
}

fn is_supported_upgrade_requirement_band_id(band_id: &str) -> bool {
    matches!(
        band_id,
        "very_low" | "low" | "moderate" | "high" | "very_high"
    )
}

fn validate_map(config: &MapConfig) -> Result<()> {
    if config.maps.is_empty() {
        bail!("map list cannot be empty");
    }
    let required_start_faction_key = default_start_faction_key();
    let mut has_required_start_map = false;
    let mut seen_ids = std::collections::HashSet::new();
    for (index, map) in config.maps.iter().enumerate() {
        if map.id.trim().is_empty() {
            bail!("map[{index}] id must be non-empty");
        }
        if !seen_ids.insert(map.id.clone()) {
            bail!("map[{index}] id '{}' is duplicated", map.id);
        }
        if map.name.trim().is_empty() {
            bail!("map[{index}] name must be non-empty");
        }
        if map.description.trim().is_empty() {
            bail!("map[{index}] description must be non-empty");
        }
        if map.width <= 0.0 || map.height <= 0.0 {
            bail!("map[{index}] width and height must be > 0");
        }
        if map.allowed_factions.is_empty() {
            bail!("map[{index}] allowed_factions cannot be empty");
        }
        validate_map_allowed_factions(map, index)?;
        if has_required_map_start_faction(map, required_start_faction_key) {
            has_required_start_map = true;
        }
    }
    if !has_required_start_map {
        bail!("at least one map must allow '{required_start_faction_key}' faction");
    }
    Ok(())
}

fn validate_rescue(config: &RescueConfig) -> Result<()> {
    if config.spawn_count == 0 {
        bail!("rescue spawn_count must be > 0");
    }
    if config.rescue_radius <= 0.0 || config.rescue_duration_secs <= 0.0 {
        bail!("rescue radius/duration must be > 0");
    }
    if config.recruit_pool.is_empty() {
        bail!("rescue recruit_pool must contain at least one entry");
    }
    for (index, recruit_kind) in config.recruit_pool.iter().enumerate() {
        if recruit_kind.tier() != 0 {
            bail!("rescue recruit_pool[{index}] must be a tier-0 recruit");
        }
    }
    Ok(())
}

fn validate_drops(config: &DropsConfig) -> Result<()> {
    if config.initial_spawn_count == 0 {
        bail!("drops initial_spawn_count must be > 0");
    }
    if config.spawn_interval_secs <= 0.0 {
        bail!("drops spawn_interval_secs must be > 0");
    }
    if config.pickup_radius <= 0.0 {
        bail!("drops pickup_radius must be > 0");
    }
    if config.gold_per_pack <= 0.0 {
        bail!("drops gold_per_pack must be > 0");
    }
    if config.max_active_packs == 0 {
        bail!("drops max_active_packs must be > 0");
    }
    Ok(())
}

fn validate_factions(config: &FactionsConfigFile) -> Result<()> {
    validate_faction_profile("factions.christian", &config.christian)?;
    validate_faction_profile("factions.muslim", &config.muslim)?;
    Ok(())
}

fn validate_difficulties(config: &DifficultiesConfigFile) -> Result<()> {
    validate_difficulty_profile("difficulties.recruit", &config.recruit)?;
    validate_difficulty_profile("difficulties.experienced", &config.experienced)?;
    validate_difficulty_profile(
        "difficulties.alone_against_the_infidels",
        &config.alone_against_the_infidels,
    )?;
    Ok(())
}

fn validate_multiplier_field(label: &str, value: f32) -> Result<()> {
    if value <= 0.0 {
        bail!("{label} must be > 0");
    }
    Ok(())
}

fn validate_faction_profile(label: &str, profile: &FactionGameplayConfig) -> Result<()> {
    validate_multiplier_field(
        &format!("{label}.friendly_health_multiplier"),
        profile.friendly_health_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.friendly_damage_multiplier"),
        profile.friendly_damage_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.friendly_attack_speed_multiplier"),
        profile.friendly_attack_speed_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.friendly_move_speed_multiplier"),
        profile.friendly_move_speed_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.friendly_morale_multiplier"),
        profile.friendly_morale_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.friendly_morale_gain_multiplier"),
        profile.friendly_morale_gain_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.friendly_morale_loss_multiplier"),
        profile.friendly_morale_loss_multiplier,
    )?;

    validate_multiplier_field(
        &format!("{label}.gold_gain_multiplier"),
        profile.gold_gain_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.rescue_time_multiplier"),
        profile.rescue_time_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.authority_enemy_morale_drain_multiplier"),
        profile.authority_enemy_morale_drain_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_health_multiplier"),
        profile.enemy_health_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_damage_multiplier"),
        profile.enemy_damage_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_attack_speed_multiplier"),
        profile.enemy_attack_speed_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_move_speed_multiplier"),
        profile.enemy_move_speed_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_morale_multiplier"),
        profile.enemy_morale_multiplier,
    )?;

    Ok(())
}

fn validate_difficulty_profile(label: &str, profile: &DifficultyGameplayConfig) -> Result<()> {
    validate_multiplier_field(
        &format!("{label}.enemy_health_multiplier"),
        profile.enemy_health_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_damage_multiplier"),
        profile.enemy_damage_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_attack_speed_multiplier"),
        profile.enemy_attack_speed_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_move_speed_multiplier"),
        profile.enemy_move_speed_multiplier,
    )?;
    validate_multiplier_field(
        &format!("{label}.enemy_morale_multiplier"),
        profile.enemy_morale_multiplier,
    )?;
    Ok(())
}

fn validate_roster_tuning(config: &RosterTuningConfigFile) -> Result<()> {
    for faction in PlayerFaction::all() {
        for unit_id in REQUIRED_TIER2_UNIT_IDS {
            let Some(kind) = UnitKind::from_faction_and_unit_id(faction, unit_id, false) else {
                bail!(
                    "failed to resolve tier-2 unit '{unit_id}' for faction '{}'",
                    faction.config_key()
                );
            };
            let key = tier2_config_key_for_kind(kind).expect("tier2 key should exist");
            let Some(stats) = config.tier2_units.get(key.as_str()) else {
                bail!("roster_tuning.tier2_units is missing required entry '{key}'");
            };
            let allow_zero_damage = matches!(unit_id, "squire" | "devoted_one");
            validate_unit_stats(
                stats,
                &format!("roster_tuning.tier2_units.{key}"),
                allow_zero_damage,
            )?;
        }
    }

    let behavior = &config.behavior;
    if behavior.tracker_hound_active_secs <= 0.0 {
        bail!("roster_tuning.behavior.tracker_hound_active_secs must be > 0");
    }
    if behavior.tracker_hound_cooldown_secs <= 0.0 {
        bail!("roster_tuning.behavior.tracker_hound_cooldown_secs must be > 0");
    }
    if behavior.tracker_hound_strike_interval_secs <= 0.0 {
        bail!("roster_tuning.behavior.tracker_hound_strike_interval_secs must be > 0");
    }
    if behavior.tracker_hound_damage_multiplier <= 0.0 {
        bail!("roster_tuning.behavior.tracker_hound_damage_multiplier must be > 0");
    }
    if behavior.scout_raid_active_secs <= 0.0 {
        bail!("roster_tuning.behavior.scout_raid_active_secs must be > 0");
    }
    if behavior.scout_raid_cooldown_secs <= 0.0 {
        bail!("roster_tuning.behavior.scout_raid_cooldown_secs must be > 0");
    }
    if behavior.scout_raid_speed_multiplier <= 0.0 {
        bail!("roster_tuning.behavior.scout_raid_speed_multiplier must be > 0");
    }
    if !(0.0..=1.0).contains(&behavior.fanatic_life_leech_ratio) {
        bail!("roster_tuning.behavior.fanatic_life_leech_ratio must be between 0 and 1");
    }
    Ok(())
}

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Boot), load_data_on_boot);
    }
}

fn load_data_on_boot(mut commands: Commands) {
    let game_data = GameData::load_from_dir(Path::new("assets/data"))
        .expect("failed to load game data from assets/data");
    commands.insert_resource(game_data);
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::Value;
    use tempfile::TempDir;

    use crate::model::{PlayerFaction, RecruitArchetype, UnitKind};

    use super::GameData;

    fn write_config(dir: &Path, file: &str, content: &str) {
        fs::write(dir.join(file), content).expect("write config");
    }

    fn write_valid_set(dir: &Path) {
        write_config(
            dir,
            "units.json",
            r#"{
              "base":{
                "commander":{"id":"baldiun","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
                "recruits":{
                  "peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
                  "peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
                  "peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
                }
              },
              "overrides":{
                "muslim":{
                  "commander":{"id":"saladin","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
                  "recruits":{
                    "peasant_infantry":{"id":"m1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
                    "peasant_archer":{"id":"m2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
                    "peasant_priest":{"id":"m3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
                  }
                }
              }
            }"#,
        );
        write_config(
            dir,
            "enemies.json",
            r#"{
              "base":{
                "profiles":{
                  "peasant_infantry":{"id":"ec_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                  "peasant_archer":{"id":"ec_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                  "peasant_priest":{"id":"ec_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
                }
              },
              "overrides":{
                "muslim":{
                  "profiles":{
                    "peasant_infantry":{"id":"em_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                    "peasant_archer":{"id":"em_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                    "peasant_priest":{"id":"em_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
                  }
                }
              }
            }"#,
        );
        write_config(
            dir,
            "heroes.json",
            r#"{
              "base":{
                "subtypes":{
                  "sword_shield":{"unit_id":"citadel_guard","description":"Shield-led frontline hero."},
                  "spear":{"unit_id":"armored_halberdier","description":"Anti-cavalry spear hero."},
                  "two_handed_sword":{"unit_id":"elite_heavy_knight","description":"Heavy striker hero."},
                  "bow":{"unit_id":"elite_longbowman","description":"Long-range precision hero."},
                  "javelin":{"unit_id":"siege_crossbowman","description":"Armor-piercing ranged hero."},
                  "beast_master":{"unit_id":"elite_houndmaster","description":"Hound-command ranged hero."},
                  "super_priest":{"unit_id":"divine_speaker","description":"Elite support priest hero."},
                  "super_fanatic":{"unit_id":"divine_judge","description":"Zealot melee support hero."},
                  "super_knight":{"unit_id":"elite_shock_cavalry","description":"Charge cavalry hero."}
                }
              },
              "overrides":{
                "christian":{
                  "subtypes":{
                    "sword_shield":{"entries":[{"hero_id":"c_sword_01","name":"Aldric","abilities":["Shield Rally"],"stat_notes":["High armor"]}]},
                    "spear":{"entries":[{"hero_id":"c_spear_01","name":"Berengar","abilities":["Brace"],"stat_notes":["Anti-cavalry"]}]},
                    "two_handed_sword":{"entries":[{"hero_id":"c_2h_01","name":"Godfrey","abilities":["Overhead Cleave"],"stat_notes":["High melee damage"]}]},
                    "bow":{"entries":[{"hero_id":"c_bow_01","name":"Odo","abilities":["Longshot Volley"],"stat_notes":["High range"]}]},
                    "javelin":{"entries":[{"hero_id":"c_javelin_01","name":"Renaud","abilities":["Armor Splitter"],"stat_notes":["Anti-armor"]}]},
                    "beast_master":{"entries":[{"hero_id":"c_beast_01","name":"Hugh","abilities":["Hound Pack"],"stat_notes":["Summons hounds"]}]},
                    "super_priest":{"entries":[{"hero_id":"c_priest_01","name":"Anselm","abilities":["Sanctified Ward"],"stat_notes":["Support aura"]}]},
                    "super_fanatic":{"entries":[{"hero_id":"c_fanatic_01","name":"Etienne","abilities":["Judgement"],"stat_notes":["Life leech"]}]},
                    "super_knight":{"entries":[{"hero_id":"c_knight_01","name":"Raynald","abilities":["Shock Charge"],"stat_notes":["Charge burst"]}]}
                  }
                },
                "muslim":{
                  "subtypes":{
                    "sword_shield":{"entries":[{"hero_id":"m_sword_01","name":"Husam","abilities":["Shield Rally"],"stat_notes":["High armor"]}]},
                    "spear":{"entries":[{"hero_id":"m_spear_01","name":"Qutayba","abilities":["Brace"],"stat_notes":["Anti-cavalry"]}]},
                    "two_handed_sword":{"entries":[{"hero_id":"m_2h_01","name":"Nadir","abilities":["Overhead Cleave"],"stat_notes":["High melee damage"]}]},
                    "bow":{"entries":[{"hero_id":"m_bow_01","name":"Samir","abilities":["Longshot Volley"],"stat_notes":["High range"]}]},
                    "javelin":{"entries":[{"hero_id":"m_javelin_01","name":"Faris","abilities":["Armor Splitter"],"stat_notes":["Anti-armor"]}]},
                    "beast_master":{"entries":[{"hero_id":"m_beast_01","name":"Zayd","abilities":["Hound Pack"],"stat_notes":["Summons hounds"]}]},
                    "super_priest":{"entries":[{"hero_id":"m_priest_01","name":"Yunus","abilities":["Sanctified Ward"],"stat_notes":["Support aura"]}]},
                    "super_fanatic":{"entries":[{"hero_id":"m_fanatic_01","name":"Jalal","abilities":["Judgement"],"stat_notes":["Life leech"]}]},
                    "super_knight":{"entries":[{"hero_id":"m_knight_01","name":"Amir","abilities":["Shock Charge"],"stat_notes":["Charge burst"]}]}
                  }
                }
              }
            }"#,
        );
        write_config(
            dir,
            "enemy_tier_pools.json",
            r#"{
              "tiers":[
                {"tier":0,"melee":["peasant_infantry"],"ranged":["peasant_archer"],"support":["peasant_priest"]},
                {"tier":1,"melee":["men_at_arms"],"ranged":["bowman"],"support":["devoted"]},
                {"tier":2,"melee":["shield_infantry"],"ranged":["experienced_bowman"],"support":["devoted_one"]},
                {"tier":3,"melee":["experienced_shield_infantry"],"ranged":["elite_bowman"],"support":["cardinal"]},
                {"tier":4,"melee":["elite_shield_infantry"],"ranged":["longbowman"],"support":["elite_cardinal"]},
                {"tier":5,"melee":["citadel_guard"],"ranged":["elite_longbowman"],"support":["divine_speaker"]}
              ]
            }"#,
        );
        write_config(
            dir,
            "formations.json",
            r#"{
              "square":{"id":"square","slot_spacing":20.0,"offense_multiplier":1.0,"offense_while_moving_multiplier":1.0,"defense_multiplier":1.0,"anti_cavalry_multiplier":1.0,"move_speed_multiplier":1.0},
              "circle":{"id":"circle","slot_spacing":20.0,"offense_multiplier":0.96,"offense_while_moving_multiplier":0.96,"defense_multiplier":1.10,"anti_cavalry_multiplier":1.0,"move_speed_multiplier":0.96},
              "skean":{"id":"skean","slot_spacing":20.0,"offense_multiplier":1.0,"offense_while_moving_multiplier":1.25,"defense_multiplier":0.82,"anti_cavalry_multiplier":0.92,"move_speed_multiplier":1.15},
              "diamond":{"id":"diamond","slot_spacing":20.0,"offense_multiplier":1.0,"offense_while_moving_multiplier":1.2,"defense_multiplier":0.95,"anti_cavalry_multiplier":0.95,"move_speed_multiplier":1.12,"anti_entry":true},
              "shield_wall":{"id":"shield_wall","slot_spacing":20.0,"offense_multiplier":0.92,"offense_while_moving_multiplier":0.85,"defense_multiplier":1.2,"anti_cavalry_multiplier":1.1,"move_speed_multiplier":0.72,"anti_entry":true,"shielded_block_bonus":0.1,"melee_reflect_ratio":0.3},
              "loose":{"id":"loose","slot_spacing":28.0,"offense_multiplier":1.04,"offense_while_moving_multiplier":1.04,"defense_multiplier":0.94,"anti_cavalry_multiplier":0.94,"move_speed_multiplier":1.08,"allow_unlimited_enemy_inside":true}
            }"#,
        );
        write_config(
            dir,
            "waves.json",
            r#"{"waves":[{"time_secs":0.0,"count":1},{"time_secs":1.0,"count":1}]}"#,
        );
        write_config(
            dir,
            "upgrades.json",
            r#"{"upgrades":[{"id":"u","kind":"damage","value":1.0,"reward_lane":"minor"}]}"#,
        );
        write_config(
            dir,
            "map.json",
            r#"{
              "maps":[
                {
                  "id":"desert_battlefield",
                  "name":"Desert Battlefield",
                  "description":"Open sand arena for early runs.",
                  "width":1000.0,
                  "height":1000.0,
                  "allowed_factions":["christian"],
                  "spawn_profile_id":"default"
                }
              ]
            }"#,
        );
        write_config(
            dir,
            "rescue.json",
            r#"{
              "spawn_count":1,
              "rescue_radius":10.0,
              "rescue_duration_secs":1.0,
              "recruit_pool":["peasant_infantry"]
            }"#,
        );
        write_config(
            dir,
            "drops.json",
            r#"{"initial_spawn_count":3,"spawn_interval_secs":1.5,"pickup_radius":15.0,"gold_per_pack":5.0,"max_active_packs":30}"#,
        );
        write_config(
            dir,
            "factions.json",
            r#"{
              "christian":{
                "friendly_health_multiplier":1.0,
                "friendly_damage_multiplier":1.0,
                "friendly_attack_speed_multiplier":1.0,
                "friendly_move_speed_multiplier":1.0,
                "friendly_armor_bonus":0.0,
                "friendly_morale_multiplier":1.0,
                "friendly_morale_gain_multiplier":1.0,
                "friendly_morale_loss_multiplier":1.0,
                "commander_aura_radius_bonus":0.0,
                "gold_gain_multiplier":1.0,
                "rescue_time_multiplier":1.0,
                "authority_enemy_morale_drain_multiplier":1.0,
                "enemy_health_multiplier":1.0,
                "enemy_damage_multiplier":1.0,
                "enemy_attack_speed_multiplier":1.0,
                "enemy_move_speed_multiplier":1.0,
                "enemy_morale_multiplier":1.0
              },
              "muslim":{
                "friendly_health_multiplier":1.0,
                "friendly_damage_multiplier":1.0,
                "friendly_attack_speed_multiplier":1.0,
                "friendly_move_speed_multiplier":1.0,
                "friendly_armor_bonus":0.0,
                "friendly_morale_multiplier":1.0,
                "friendly_morale_gain_multiplier":1.0,
                "friendly_morale_loss_multiplier":1.0,
                "commander_aura_radius_bonus":0.0,
                "gold_gain_multiplier":1.0,
                "rescue_time_multiplier":1.0,
                "authority_enemy_morale_drain_multiplier":1.0,
                "enemy_health_multiplier":1.0,
                "enemy_damage_multiplier":1.0,
                "enemy_attack_speed_multiplier":1.0,
                "enemy_move_speed_multiplier":1.0,
                "enemy_morale_multiplier":1.0
              }
            }"#,
        );
        write_config(
            dir,
            "difficulties.json",
            r#"{
              "recruit":{
                "enemy_health_multiplier":1.0,
                "enemy_damage_multiplier":1.0,
                "enemy_attack_speed_multiplier":1.0,
                "enemy_move_speed_multiplier":1.0,
                "enemy_morale_multiplier":1.0,
                "enemy_ranged_dodge_enabled":false,
                "enemy_block_enabled":false,
                "ranged_support_avoid_melee":false
              },
              "experienced":{
                "enemy_health_multiplier":1.2,
                "enemy_damage_multiplier":1.15,
                "enemy_attack_speed_multiplier":1.1,
                "enemy_move_speed_multiplier":1.05,
                "enemy_morale_multiplier":1.1,
                "enemy_ranged_dodge_enabled":true,
                "enemy_block_enabled":true,
                "ranged_support_avoid_melee":false
              },
              "alone_against_the_infidels":{
                "enemy_health_multiplier":1.35,
                "enemy_damage_multiplier":1.3,
                "enemy_attack_speed_multiplier":1.2,
                "enemy_move_speed_multiplier":1.1,
                "enemy_morale_multiplier":1.2,
                "enemy_ranged_dodge_enabled":true,
                "enemy_block_enabled":true,
                "ranged_support_avoid_melee":true
              }
            }"#,
        );
        write_config(
            dir,
            "roster_tuning.json",
            r#"{
              "tier2_units":{
                "christian_shield_infantry":{"id":"christian_shield_infantry","max_hp":184.464,"armor":10.0,"damage":6.336,"attack_cooldown_secs":1.23552,"attack_range":36.0,"move_speed":135.375,"morale":123.2},
                "christian_spearman":{"id":"christian_spearman","max_hp":143.64,"armor":8.6,"damage":7.8848,"attack_cooldown_secs":1.16424,"attack_range":48.0,"move_speed":141.075,"morale":118.72},
                "christian_unmounted_knight":{"id":"christian_unmounted_knight","max_hp":136.08,"armor":10.3,"damage":11.04,"attack_cooldown_secs":1.0692,"attack_range":36.0,"move_speed":152.475,"morale":120.96},
                "christian_squire":{"id":"christian_squire","max_hp":146.8392,"armor":3.0,"damage":0.0,"attack_cooldown_secs":1.428,"attack_range":20.0,"move_speed":153.9792,"morale":158.112},
                "christian_experienced_bowman":{"id":"christian_experienced_bowman","max_hp":81.7048,"armor":2.525,"damage":2.0664,"attack_cooldown_secs":1.336175,"attack_range":26.0,"move_speed":168.1372,"morale":107.3088,"ranged_attack_damage":16.128,"ranged_attack_cooldown_secs":0.78936,"ranged_attack_range":630.0,"ranged_projectile_speed":338.0,"ranged_projectile_max_distance":750.0},
                "christian_crossbowman":{"id":"christian_crossbowman","max_hp":78.6216,"armor":2.925,"damage":1.4616,"attack_cooldown_secs":1.446375,"attack_range":26.0,"move_speed":158.3428,"morale":104.328,"ranged_attack_damage":20.88,"ranged_attack_cooldown_secs":1.13022,"ranged_attack_range":525.0,"ranged_projectile_speed":362.0,"ranged_projectile_max_distance":670.0},
                "christian_tracker":{"id":"christian_tracker","max_hp":83.2464,"armor":2.525,"damage":1.89,"attack_cooldown_secs":1.308625,"attack_range":26.0,"move_speed":173.0344,"morale":109.296,"ranged_attack_damage":10.368,"ranged_attack_cooldown_secs":0.8073,"ranged_attack_range":505.0,"ranged_projectile_speed":330.0,"ranged_projectile_max_distance":640.0},
                "christian_scout":{"id":"christian_scout","max_hp":84.788,"armor":2.01875,"damage":3.15,"attack_cooldown_secs":1.18465,"attack_range":34.0,"move_speed":192.6232,"morale":107.3088},
                "christian_devoted_one":{"id":"christian_devoted_one","max_hp":146.8392,"armor":3.8,"damage":0.0,"attack_cooldown_secs":1.4,"attack_range":20.0,"move_speed":155.4888,"morale":152.928},
                "christian_fanatic":{"id":"christian_fanatic","max_hp":111.996,"armor":0.0,"damage":14.0,"attack_cooldown_secs":1.148,"attack_range":23.0,"move_speed":169.0752,"morale":139.968},
                "muslim_shield_infantry":{"id":"muslim_shield_infantry","max_hp":184.464,"armor":10.0,"damage":6.336,"attack_cooldown_secs":1.23552,"attack_range":36.0,"move_speed":135.375,"morale":123.2},
                "muslim_spearman":{"id":"muslim_spearman","max_hp":143.64,"armor":8.6,"damage":7.8848,"attack_cooldown_secs":1.16424,"attack_range":48.0,"move_speed":141.075,"morale":118.72},
                "muslim_unmounted_knight":{"id":"muslim_unmounted_knight","max_hp":136.08,"armor":10.3,"damage":11.04,"attack_cooldown_secs":1.0692,"attack_range":36.0,"move_speed":152.475,"morale":120.96},
                "muslim_squire":{"id":"muslim_squire","max_hp":146.8392,"armor":3.0,"damage":0.0,"attack_cooldown_secs":1.428,"attack_range":20.0,"move_speed":153.9792,"morale":158.112},
                "muslim_experienced_bowman":{"id":"muslim_experienced_bowman","max_hp":81.7048,"armor":2.525,"damage":2.0664,"attack_cooldown_secs":1.336175,"attack_range":26.0,"move_speed":168.1372,"morale":107.3088,"ranged_attack_damage":16.128,"ranged_attack_cooldown_secs":0.78936,"ranged_attack_range":630.0,"ranged_projectile_speed":338.0,"ranged_projectile_max_distance":750.0},
                "muslim_crossbowman":{"id":"muslim_crossbowman","max_hp":78.6216,"armor":2.925,"damage":1.4616,"attack_cooldown_secs":1.446375,"attack_range":26.0,"move_speed":158.3428,"morale":104.328,"ranged_attack_damage":20.88,"ranged_attack_cooldown_secs":1.13022,"ranged_attack_range":525.0,"ranged_projectile_speed":362.0,"ranged_projectile_max_distance":670.0},
                "muslim_tracker":{"id":"muslim_tracker","max_hp":83.2464,"armor":2.525,"damage":1.89,"attack_cooldown_secs":1.308625,"attack_range":26.0,"move_speed":173.0344,"morale":109.296,"ranged_attack_damage":10.368,"ranged_attack_cooldown_secs":0.8073,"ranged_attack_range":505.0,"ranged_projectile_speed":330.0,"ranged_projectile_max_distance":640.0},
                "muslim_scout":{"id":"muslim_scout","max_hp":84.788,"armor":2.01875,"damage":3.15,"attack_cooldown_secs":1.18465,"attack_range":34.0,"move_speed":192.6232,"morale":107.3088},
                "muslim_devoted_one":{"id":"muslim_devoted_one","max_hp":146.8392,"armor":3.8,"damage":0.0,"attack_cooldown_secs":1.4,"attack_range":20.0,"move_speed":155.4888,"morale":152.928},
                "muslim_fanatic":{"id":"muslim_fanatic","max_hp":111.996,"armor":0.0,"damage":14.0,"attack_cooldown_secs":1.148,"attack_range":23.0,"move_speed":169.0752,"morale":139.968}
              },
              "behavior":{
                "tracker_hound_active_secs":10.0,
                "tracker_hound_cooldown_secs":20.0,
                "tracker_hound_strike_interval_secs":0.45,
                "tracker_hound_damage_multiplier":0.55,
                "scout_raid_active_secs":10.0,
                "scout_raid_cooldown_secs":20.0,
                "scout_raid_speed_multiplier":1.28,
                "fanatic_life_leech_ratio":0.08
              }
            }"#,
        );
        fs::copy("assets/data/heroes.json", dir.join("heroes.json")).expect("copy heroes fixture");
    }

    #[test]
    fn loads_valid_config_set() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        let data = GameData::load_from_dir(tmp.path()).expect("load");
        assert_eq!(data.waves.waves.len(), 2);
        assert_eq!(data.upgrades.upgrades.len(), 1);
    }

    #[test]
    fn opposing_enemy_pool_swaps_by_player_faction() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        let data = GameData::load_from_dir(tmp.path()).expect("load");

        let christian_player_pool = data.enemies.opposing_enemy_pool(PlayerFaction::Christian);
        assert_eq!(
            christian_player_pool,
            [
                UnitKind::MuslimPeasantInfantry,
                UnitKind::MuslimPeasantArcher,
                UnitKind::MuslimPeasantPriest,
            ]
        );

        let muslim_player_pool = data.enemies.opposing_enemy_pool(PlayerFaction::Muslim);
        assert_eq!(
            muslim_player_pool,
            [
                UnitKind::ChristianPeasantInfantry,
                UnitKind::ChristianPeasantArcher,
                UnitKind::ChristianPeasantPriest,
            ]
        );
    }

    #[test]
    fn unit_and_enemy_resolvers_support_faction_plus_archetype_queries() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        let data = GameData::load_from_dir(tmp.path()).expect("load");

        let muslim_archer = data
            .units
            .recruit_for_faction_and_archetype(PlayerFaction::Muslim, RecruitArchetype::Archer);
        assert_eq!(muslim_archer.id, "m2");

        let christian_priest_enemy = data
            .enemies
            .enemy_profile_for_faction_and_archetype(
                PlayerFaction::Christian,
                RecruitArchetype::Priest,
            )
            .expect("christian priest enemy profile");
        assert_eq!(christian_priest_enemy.id, "ec_p");

        let christian_tier3_archer = data
            .enemies
            .enemy_profile_for_kind(UnitKind::ChristianEliteBowman)
            .expect("tier-3 archer line should resolve enemy archer profile");
        assert_eq!(christian_tier3_archer.id, "ec_a");

        let muslim_tier5_support = data
            .enemies
            .enemy_profile_for_kind(UnitKind::MuslimDivineSpeaker)
            .expect("tier-5 support line should resolve enemy priest profile");
        assert_eq!(muslim_tier5_support.id, "em_p");
    }

    #[test]
    fn tier2_tuning_key_uses_faction_prefix_plus_generic_unit_id() {
        let christian =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "shield_infantry", false)
                .expect("christian tier2 kind");
        let muslim =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Muslim, "shield_infantry", false)
                .expect("muslim tier2 kind");
        let tier1 =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "men_at_arms", false)
                .expect("tier1 kind");

        assert_eq!(
            super::tier2_config_key_for_kind(christian).as_deref(),
            Some("christian_shield_infantry")
        );
        assert_eq!(
            super::tier2_config_key_for_kind(muslim).as_deref(),
            Some("muslim_shield_infantry")
        );
        assert!(super::tier2_config_key_for_kind(tier1).is_none());
    }

    #[test]
    fn enemy_profile_for_kind_rejects_rescuable_variants() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        let data = GameData::load_from_dir(tmp.path()).expect("load");
        assert!(
            data.enemies
                .enemy_profile_for_kind(UnitKind::RescuableChristianPeasantInfantry)
                .is_none()
        );
    }

    #[test]
    fn rejects_invalid_unit_cooldown() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "units.json",
            r#"{
              "base":{
                "commander":{"id":"baldiun","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":-1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
                "recruits":{
                  "peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
                  "peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
                  "peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
                }
              },
              "overrides":{
                "muslim":{
                  "commander":{"id":"saladin","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
                  "recruits":{
                    "peasant_infantry":{"id":"m1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
                    "peasant_archer":{"id":"m2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
                    "peasant_priest":{"id":"m3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
                  }
                }
              }
            }"#,
        );

        let err = GameData::load_from_dir(tmp.path()).expect_err("expected invalid config");
        assert!(err.to_string().contains("attack_cooldown_secs"));
    }

    #[test]
    fn rejects_enemy_with_partial_ranged_field_set() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "enemies.json",
            r#"{
              "base":{
                "profiles":{
                  "peasant_infantry":{"id":"ec_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                  "peasant_archer":{"id":"ec_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"ranged_attack_damage":2.0,"move_speed":80.0,"morale":85.0},
                  "peasant_priest":{"id":"ec_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
                }
              },
              "overrides":{
                "muslim":{
                  "profiles":{
                    "peasant_infantry":{"id":"em_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                    "peasant_archer":{"id":"em_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                    "peasant_priest":{"id":"em_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
                  }
                }
              }
            }"#,
        );

        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected invalid enemy ranged config");
        assert!(err.to_string().contains("ranged fields"));
    }

    #[test]
    fn rejects_units_override_with_unknown_recruit_id() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "units.json",
            r#"{
              "base":{
                "commander":{"id":"baldiun","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
                "recruits":{
                  "peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
                  "peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
                  "peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
                }
              },
              "overrides":{
                "muslim":{
                  "recruits":{
                    "invalid_recruit":{"id":"bad","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0}
                  }
                }
              }
            }"#,
        );
        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected invalid recruit override id");
        let err_text = err.to_string();
        assert!(
            err_text.contains("unknown unit_id"),
            "unexpected error: {err_text}"
        );
    }

    #[test]
    fn rejects_enemy_override_with_unknown_faction_key() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "enemies.json",
            r#"{
              "base":{
                "profiles":{
                  "peasant_infantry":{"id":"ec_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                  "peasant_archer":{"id":"ec_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
                  "peasant_priest":{"id":"ec_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
                }
              },
              "overrides":{
                "unknown_faction":{
                  "profiles":{
                    "peasant_infantry":{"id":"bad","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
                  }
                }
              }
            }"#,
        );
        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected invalid faction override");
        let err_text = err.to_string();
        assert!(
            err_text.contains("unknown faction key"),
            "unexpected error: {err_text}"
        );
    }

    #[test]
    fn rejects_heroes_missing_required_subtype() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "heroes.json",
            r#"{
              "base":{
                "subtypes":{
                  "sword_shield":{"unit_id":"citadel_guard","description":"Shield hero."}
                }
              },
              "overrides":{
                "christian":{
                  "subtypes":{
                    "sword_shield":{"entries":[{"hero_id":"c01","name":"Aldric"}]}
                  }
                },
                "muslim":{
                  "subtypes":{
                    "sword_shield":{"entries":[{"hero_id":"m01","name":"Husam"}]}
                  }
                }
              }
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected missing hero subtype");
        assert!(err.to_string().contains("missing required subtype"));
    }

    #[test]
    fn rejects_heroes_entry_with_unknown_unit_id() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());

        let heroes_path = tmp.path().join("heroes.json");
        let raw = fs::read_to_string(&heroes_path).expect("read heroes");
        let mut parsed: Value = serde_json::from_str(&raw).expect("parse heroes json");
        parsed["overrides"]["christian"]["subtypes"]["sword_shield"]["entries"][0]["unit_id"] =
            Value::String("unknown_unit".to_string());
        fs::write(
            &heroes_path,
            serde_json::to_string_pretty(&parsed).expect("serialize heroes json"),
        )
        .expect("write heroes");

        let err = GameData::load_from_dir(tmp.path()).expect_err("expected bad hero unit id");
        assert!(err.to_string().contains("does not resolve for faction"));
    }

    #[test]
    fn rejects_heroes_subtype_with_non_ten_entries_per_faction() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());

        let heroes_path = tmp.path().join("heroes.json");
        let raw = fs::read_to_string(&heroes_path).expect("read heroes");
        let mut parsed: Value = serde_json::from_str(&raw).expect("parse heroes json");
        let entries = parsed["overrides"]["christian"]["subtypes"]["sword_shield"]["entries"]
            .as_array_mut()
            .expect("sword_shield entries should be array");
        entries.pop();
        fs::write(
            &heroes_path,
            serde_json::to_string_pretty(&parsed).expect("serialize heroes json"),
        )
        .expect("write heroes");

        let err = GameData::load_from_dir(tmp.path()).expect_err("expected hero count validation");
        let err_text = err.to_string();
        assert!(
            err_text.contains("must contain exactly 10 heroes"),
            "unexpected error: {err_text}"
        );
    }

    #[test]
    fn asset_heroes_have_exactly_ten_entries_per_subtype_per_faction() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load asset data");
        for (subtype_id, subtype) in &data.heroes.subtypes {
            for faction in PlayerFaction::all() {
                let entries = subtype
                    .entries_by_faction
                    .get(&faction)
                    .expect("entries should exist for faction");
                assert_eq!(
                    entries.len(),
                    super::HERO_ENTRIES_PER_SUBTYPE_PER_FACTION,
                    "subtype={subtype_id}, faction={:?}",
                    faction
                );
            }
        }
    }

    #[test]
    fn rejects_missing_file() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        fs::remove_file(tmp.path().join("waves.json")).expect("remove");
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected missing file");
        assert!(err.to_string().contains("waves.json"));
    }

    #[test]
    fn rejects_enemy_tier_pools_without_all_required_tiers() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "enemy_tier_pools.json",
            r#"{
              "tiers":[
                {"tier":0,"melee":["peasant_infantry"],"ranged":["peasant_archer"],"support":["peasant_priest"]}
              ]
            }"#,
        );
        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected invalid enemy tier pools");
        assert!(err.to_string().contains("missing required tier"));
    }

    #[test]
    fn rejects_unsorted_wave_times() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "waves.json",
            r#"{"waves":[{"time_secs":5.0,"count":1},{"time_secs":2.0,"count":2}]}"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected invalid wave order");
        assert!(err.to_string().contains("strictly increasing"));
    }

    #[test]
    fn rejects_map_config_without_entries() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(tmp.path(), "map.json", r#"{"maps":[]}"#);
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected invalid map list");
        assert!(err.to_string().contains("map list cannot be empty"));
    }

    #[test]
    fn rejects_map_with_unknown_allowed_faction() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "map.json",
            r#"{
              "maps":[
                {
                  "id":"bad_map",
                  "name":"Bad Map",
                  "description":"Invalid faction tag.",
                  "width":1000.0,
                  "height":1000.0,
                  "allowed_factions":["pirates"]
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected invalid faction tag");
        assert!(err.to_string().contains("unknown faction"));
    }

    #[test]
    fn faction_key_helpers_follow_player_faction_registry() {
        for faction in PlayerFaction::all() {
            assert!(super::is_supported_faction_key(faction.config_key()));
            assert!(super::is_supported_faction_key(
                &faction.config_key().to_ascii_uppercase()
            ));
        }
        assert!(!super::is_supported_faction_key("unknown_faction"));
        assert_eq!(
            super::default_start_faction_key(),
            PlayerFaction::default().config_key()
        );
    }

    #[test]
    fn map_validation_accepts_case_insensitive_supported_faction_keys() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "map.json",
            r#"{
              "maps":[
                {
                  "id":"case_map",
                  "name":"Case Map",
                  "description":"Uppercase faction key should resolve.",
                  "width":1000.0,
                  "height":1000.0,
                  "allowed_factions":["CHRISTIAN"]
                }
              ]
            }"#,
        );
        let loaded =
            GameData::load_from_dir(tmp.path()).expect("expected case-insensitive map load");
        assert_eq!(loaded.map.maps.len(), 1);
    }

    #[test]
    fn accepts_rescue_pool_with_all_tier0_entries() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "rescue.json",
            r#"{
              "spawn_count":2,
              "rescue_radius":30.0,
              "rescue_duration_secs":1.5,
              "recruit_pool":[
                "peasant_infantry",
                "peasant_archer",
                "peasant_priest"
              ]
            }"#,
        );
        let loaded = GameData::load_from_dir(tmp.path()).expect("expected valid rescue pool");
        assert_eq!(loaded.rescue.recruit_pool.len(), 3);
    }

    #[test]
    fn rejects_upgrade_with_unknown_requirement_type() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"mob_custom",
                  "kind":"mob_fury",
                  "value":1.0,
                  "reward_lane":"minor",
                  "requirement_type":"unknown_gate"
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected bad requirement type");
        assert!(
            err.to_string()
                .contains("supported: tier0_share, formation_active, map_tag, has_trait, band_at_least, band_at_most")
        );
    }

    #[test]
    fn accepts_upgrade_trait_and_band_requirements() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"trait_gate",
                  "kind":"mob_fury",
                  "value":1.0,
                  "reward_lane":"minor",
                  "requirement_type":"has_trait",
                  "requirement_trait":"shielded"
                },
                {
                  "id":"band_gate",
                  "kind":"mob_justice",
                  "value":1.0,
                  "reward_lane":"minor",
                  "requirement_type":"band_at_least",
                  "requirement_band_stat":"frontline_share",
                  "requirement_band_at_least":"moderate"
                }
              ]
            }"#,
        );
        let loaded = GameData::load_from_dir(tmp.path()).expect("expected valid requirement types");
        assert_eq!(loaded.upgrades.upgrades.len(), 2);
    }

    #[test]
    fn rejects_upgrade_band_requirement_with_unknown_band() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"bad_band",
                  "kind":"mob_fury",
                  "value":1.0,
                  "reward_lane":"minor",
                  "requirement_type":"band_at_most",
                  "requirement_band_stat":"support_share",
                  "requirement_band_at_most":"legendary"
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected bad band id");
        assert!(err.to_string().contains("unknown requirement band"));
    }

    #[test]
    fn accepts_upgrade_semantic_effect_primitives() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"band_shift_ok",
                  "kind":"damage",
                  "value":7.0,
                  "reward_lane":"minor",
                  "effect_band_shift_stat":"damage",
                  "effect_band_shift_steps":1
                },
                {
                  "id":"trait_hook_ok",
                  "kind":"mob_fury",
                  "value":1.0,
                  "reward_lane":"major",
                  "downside":"Locks doctrine around frontline-heavy compositions.",
                  "effect_trait_hook":"frontline",
                  "effect_trait_modifier_kind":"damage_multiplier",
                  "effect_trait_modifier_value":0.15
                }
              ]
            }"#,
        );
        let loaded =
            GameData::load_from_dir(tmp.path()).expect("expected valid semantic effect fields");
        assert_eq!(loaded.upgrades.upgrades.len(), 2);
    }

    #[test]
    fn rejects_upgrade_semantic_effect_with_invalid_band_shift() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"band_shift_bad",
                  "kind":"damage",
                  "value":7.0,
                  "reward_lane":"minor",
                  "effect_band_shift_stat":"damage",
                  "effect_band_shift_steps":0
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path())
            .expect_err("expected semantic validation failure for shift steps");
        assert!(
            err.to_string()
                .contains("effect_band_shift_steps must be in")
        );
    }

    #[test]
    fn rejects_upgrade_semantic_effect_with_unknown_trait_modifier_kind() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"trait_modifier_bad",
                  "kind":"mob_fury",
                  "value":1.0,
                  "reward_lane":"major",
                  "downside":"Locks doctrine around frontline-heavy compositions.",
                  "effect_trait_hook":"frontline",
                  "effect_trait_modifier_kind":"mystery_kind",
                  "effect_trait_modifier_value":0.15
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path())
            .expect_err("expected semantic validation failure for trait modifier kind");
        assert!(
            err.to_string()
                .contains("unknown effect_trait_modifier_kind")
        );
    }

    #[test]
    fn rejects_non_whitelisted_multiplicative_trait_modifier_kind() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"bad_mul_semantic",
                  "kind":"damage",
                  "value":7.0,
                  "reward_lane":"minor",
                  "effect_trait_hook":"frontline",
                  "effect_trait_modifier_kind":"damage_multiplier",
                  "effect_trait_modifier_value":0.12
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path())
            .expect_err("expected multiplicative semantic whitelist failure");
        assert!(
            err.to_string()
                .contains("effect_trait_modifier_kind 'damage_multiplier' is restricted")
        );
    }

    #[test]
    fn rejects_upgrade_with_unknown_formation_requirement() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"formation_gate",
                  "kind":"mob_fury",
                  "value":1.0,
                  "reward_lane":"minor",
                  "requirement_type":"formation_active",
                  "requirement_active_formation":"wedge"
                }
              ]
            }"#,
        );
        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected bad formation requirement");
        assert!(
            err.to_string()
                .contains("unknown requirement_active_formation")
        );
    }

    #[test]
    fn rejects_upgrade_with_unwired_kind() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"mystery_up",
                  "kind":"totally_new_kind",
                  "value":1.0,
                  "reward_lane":"minor"
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected unknown kind failure");
        assert!(err.to_string().contains("is not wired in runtime systems"));
    }

    #[test]
    fn rejects_deprecated_upgrade_ids_with_replacement_hint() {
        let legacy_ids = [
            ("fast_learner_up", "quartermaster_up"),
            ("fast_learner_up_10", "quartermaster_up"),
            ("fast_learner_up_15", "quartermaster_up"),
            ("mob_fury_shielded_host", "mob_fury"),
            ("mob_justice_frontline_bias", "mob_justice"),
            ("mob_mercy_support_ceiling", "mob_mercy"),
        ];

        for (legacy_id, replacement_id) in legacy_ids {
            let tmp = TempDir::new().expect("tmp");
            write_valid_set(tmp.path());
            write_config(
                tmp.path(),
                "upgrades.json",
                &format!(
                    r#"{{
                      "upgrades":[
                        {{
                          "id":"{legacy_id}",
                          "kind":"mob_fury",
                          "value":1.0,
                          "reward_lane":"minor"
                        }}
                      ]
                    }}"#
                ),
            );
            let err = GameData::load_from_dir(tmp.path())
                .expect_err("expected deprecated upgrade id failure");
            let message = err.to_string();
            assert!(message.contains("is deprecated"), "message: {message}");
            assert!(
                message.contains(replacement_id),
                "expected replacement {replacement_id} in message: {message}"
            );
        }
    }

    #[test]
    fn rejects_duplicate_upgrade_ids() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {"id":"dup","kind":"damage","value":1.0,"reward_lane":"minor"},
                {"id":"dup","kind":"armor","value":1.0,"reward_lane":"minor"}
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected duplicate id failure");
        assert!(err.to_string().contains("duplicate id"));
    }

    #[test]
    fn rejects_legacy_upgrade_roll_fields() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"legacy_roll",
                  "kind":"damage",
                  "value":1.0,
                  "reward_lane":"minor",
                  "min_value":1.0
                }
              ]
            }"#,
        );
        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected legacy roll field failure");
        assert!(
            err.to_string()
                .contains("legacy roll fields are no longer supported")
        );
    }

    #[test]
    fn rejects_upgrade_with_invalid_diminishing_factor() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"bad_diminishing",
                  "kind":"damage",
                  "value":1.0,
                  "reward_lane":"minor",
                  "diminishing_factor":1.2
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path())
            .expect_err("expected diminishing factor validation failure");
        assert!(
            err.to_string()
                .contains("diminishing_factor must be in the range (0, 1)")
        );
    }

    #[test]
    fn rejects_major_upgrade_without_downside() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "upgrades.json",
            r#"{
              "upgrades":[
                {
                  "id":"major_missing_downside",
                  "kind":"doctrine_command_net",
                  "value":20.0,
                  "reward_lane":"major",
                  "one_time":true
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path())
            .expect_err("expected major downside validation failure");
        assert!(
            err.to_string()
                .contains("major upgrades must provide a non-empty downside")
        );
    }

    #[test]
    fn rejects_non_positive_faction_multiplier() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "factions.json",
            r#"{
              "christian":{"friendly_health_multiplier":0.0},
              "muslim":{"friendly_health_multiplier":1.0}
            }"#,
        );

        let err = GameData::load_from_dir(tmp.path()).expect_err("expected bad faction config");
        assert!(err.to_string().contains("friendly_health_multiplier"));
    }

    #[test]
    fn rejects_non_positive_difficulty_multiplier() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "difficulties.json",
            r#"{
              "recruit":{"enemy_health_multiplier":0.0},
              "experienced":{"enemy_health_multiplier":1.0},
              "alone_against_the_infidels":{"enemy_health_multiplier":1.0}
            }"#,
        );

        let err = GameData::load_from_dir(tmp.path()).expect_err("expected bad difficulty config");
        assert!(
            err.to_string()
                .contains("difficulties.recruit.enemy_health_multiplier")
        );
    }
}
