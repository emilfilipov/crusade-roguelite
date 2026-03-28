use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use bevy::prelude::*;
use serde::Deserialize;

use crate::model::{GameDifficulty, GameState, PlayerFaction, RecruitUnitKind, UnitKind};

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

#[derive(Clone, Debug, Deserialize)]
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

impl UnitsConfigFile {
    pub fn commander_for_faction(&self, faction: PlayerFaction) -> &UnitStatsConfig {
        match faction {
            PlayerFaction::Christian => &self.commander_christian,
            PlayerFaction::Muslim => &self.commander_muslim,
        }
    }

    pub fn recruit_for_kind(&self, kind: RecruitUnitKind) -> &UnitStatsConfig {
        match kind {
            RecruitUnitKind::ChristianPeasantInfantry => &self.recruit_christian_peasant_infantry,
            RecruitUnitKind::ChristianPeasantArcher => &self.recruit_christian_peasant_archer,
            RecruitUnitKind::ChristianPeasantPriest => &self.recruit_christian_peasant_priest,
            RecruitUnitKind::MuslimPeasantInfantry => &self.recruit_muslim_peasant_infantry,
            RecruitUnitKind::MuslimPeasantArcher => &self.recruit_muslim_peasant_archer,
            RecruitUnitKind::MuslimPeasantPriest => &self.recruit_muslim_peasant_priest,
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

#[derive(Clone, Debug, Deserialize)]
pub struct EnemiesConfigFile {
    pub enemy_christian_peasant_infantry: EnemyStatsConfig,
    pub enemy_christian_peasant_archer: EnemyStatsConfig,
    pub enemy_christian_peasant_priest: EnemyStatsConfig,
    pub enemy_muslim_peasant_infantry: EnemyStatsConfig,
    pub enemy_muslim_peasant_archer: EnemyStatsConfig,
    pub enemy_muslim_peasant_priest: EnemyStatsConfig,
}

impl EnemiesConfigFile {
    pub fn enemy_profile_for_kind(&self, kind: UnitKind) -> Option<&EnemyStatsConfig> {
        match kind {
            UnitKind::ChristianPeasantInfantry => Some(&self.enemy_christian_peasant_infantry),
            UnitKind::ChristianPeasantArcher => Some(&self.enemy_christian_peasant_archer),
            UnitKind::ChristianPeasantPriest => Some(&self.enemy_christian_peasant_priest),
            UnitKind::MuslimPeasantInfantry => Some(&self.enemy_muslim_peasant_infantry),
            UnitKind::MuslimPeasantArcher => Some(&self.enemy_muslim_peasant_archer),
            UnitKind::MuslimPeasantPriest => Some(&self.enemy_muslim_peasant_priest),
            _ => None,
        }
    }

    pub fn opposing_enemy_pool(&self, player_faction: PlayerFaction) -> [UnitKind; 3] {
        match player_faction.opposing() {
            PlayerFaction::Christian => [
                UnitKind::ChristianPeasantInfantry,
                UnitKind::ChristianPeasantArcher,
                UnitKind::ChristianPeasantPriest,
            ],
            PlayerFaction::Muslim => [
                UnitKind::MuslimPeasantInfantry,
                UnitKind::MuslimPeasantArcher,
                UnitKind::MuslimPeasantPriest,
            ],
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
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormationsConfigFile {
    pub square: FormationConfig,
    pub diamond: FormationConfig,
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
pub struct UpgradeConfig {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub value: f32,
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
    ChristianPeasantInfantry,
    ChristianPeasantArcher,
    ChristianPeasantPriest,
    MuslimPeasantInfantry,
    MuslimPeasantArcher,
    MuslimPeasantPriest,
}

impl RescueRecruitKindConfig {
    pub const fn as_recruit_unit_kind(self) -> RecruitUnitKind {
        match self {
            Self::ChristianPeasantInfantry => RecruitUnitKind::ChristianPeasantInfantry,
            Self::ChristianPeasantArcher => RecruitUnitKind::ChristianPeasantArcher,
            Self::ChristianPeasantPriest => RecruitUnitKind::ChristianPeasantPriest,
            Self::MuslimPeasantInfantry => RecruitUnitKind::MuslimPeasantInfantry,
            Self::MuslimPeasantArcher => RecruitUnitKind::MuslimPeasantArcher,
            Self::MuslimPeasantPriest => RecruitUnitKind::MuslimPeasantPriest,
        }
    }

    pub const fn tier(self) -> u8 {
        match self {
            Self::ChristianPeasantInfantry => 0,
            Self::ChristianPeasantArcher => 0,
            Self::ChristianPeasantPriest => 0,
            Self::MuslimPeasantInfantry => 0,
            Self::MuslimPeasantArcher => 0,
            Self::MuslimPeasantPriest => 0,
        }
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
        self.tier2_units.get(key)
    }
}

#[derive(Resource, Clone, Debug)]
pub struct GameData {
    pub units: UnitsConfigFile,
    pub enemies: EnemiesConfigFile,
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

fn tier2_config_key_for_kind(kind: UnitKind) -> Option<&'static str> {
    match kind {
        UnitKind::ChristianShieldInfantry => Some("christian_shield_infantry"),
        UnitKind::ChristianSpearman => Some("christian_spearman"),
        UnitKind::ChristianUnmountedKnight => Some("christian_unmounted_knight"),
        UnitKind::ChristianSquire => Some("christian_squire"),
        UnitKind::ChristianExperiencedBowman => Some("christian_experienced_bowman"),
        UnitKind::ChristianCrossbowman => Some("christian_crossbowman"),
        UnitKind::ChristianTracker => Some("christian_tracker"),
        UnitKind::ChristianScout => Some("christian_scout"),
        UnitKind::ChristianDevotedOne => Some("christian_devoted_one"),
        UnitKind::ChristianFanatic => Some("christian_fanatic"),
        UnitKind::MuslimShieldInfantry => Some("muslim_shield_infantry"),
        UnitKind::MuslimSpearman => Some("muslim_spearman"),
        UnitKind::MuslimUnmountedKnight => Some("muslim_unmounted_knight"),
        UnitKind::MuslimSquire => Some("muslim_squire"),
        UnitKind::MuslimExperiencedBowman => Some("muslim_experienced_bowman"),
        UnitKind::MuslimCrossbowman => Some("muslim_crossbowman"),
        UnitKind::MuslimTracker => Some("muslim_tracker"),
        UnitKind::MuslimScout => Some("muslim_scout"),
        UnitKind::MuslimDevotedOne => Some("muslim_devoted_one"),
        UnitKind::MuslimFanatic => Some("muslim_fanatic"),
        _ => None,
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<T> {
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse config file {}", path.display()))
}

fn default_multiplier() -> f32 {
    1.0
}

fn default_rescue_recruit_pool() -> Vec<RescueRecruitKindConfig> {
    vec![
        RescueRecruitKindConfig::ChristianPeasantInfantry,
        RescueRecruitKindConfig::ChristianPeasantArcher,
        RescueRecruitKindConfig::ChristianPeasantPriest,
        RescueRecruitKindConfig::MuslimPeasantInfantry,
        RescueRecruitKindConfig::MuslimPeasantArcher,
        RescueRecruitKindConfig::MuslimPeasantPriest,
    ]
}

fn default_enemy_collision_radius() -> f32 {
    15.0
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

fn validate_formations(config: &FormationsConfigFile) -> Result<()> {
    validate_formation("square", &config.square)?;
    validate_formation("diamond", &config.diamond)?;
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
    for (idx, upgrade) in config.upgrades.iter().enumerate() {
        if upgrade.id.trim().is_empty() || upgrade.kind.trim().is_empty() {
            bail!("upgrade[{idx}] id and kind must be non-empty");
        }
        if !crate::upgrades::is_supported_upgrade_kind(upgrade.kind.as_str()) {
            bail!(
                "upgrade[{idx}] kind '{}' is not wired in runtime systems",
                upgrade.kind
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
            if !upgrade.adds_to_skillbar {
                bail!("upgrade[{idx}] unlock_formation must set adds_to_skillbar=true");
            }
        }
        if let (Some(min_value), Some(max_value)) = (upgrade.min_value, upgrade.max_value) {
            if min_value <= 0.0 || max_value <= 0.0 {
                bail!("upgrade[{idx}] min_value and max_value must be > 0");
            }
            if max_value < min_value {
                bail!("upgrade[{idx}] max_value must be >= min_value");
            }
        } else if upgrade.value <= 0.0 {
            bail!("upgrade[{idx}] value must be > 0 when min/max are omitted");
        }
        if let Some(step) = upgrade.value_step
            && step <= 0.0
        {
            bail!("upgrade[{idx}] value_step must be > 0");
        }
        if let Some(exponent) = upgrade.weight_exponent
            && exponent <= 0.0
        {
            bail!("upgrade[{idx}] weight_exponent must be > 0");
        }
        validate_upgrade_requirement(idx, upgrade)?;
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
            if !matches!(formation_id, "square" | "diamond") {
                bail!(
                    "upgrade[{idx}] unknown requirement_active_formation '{formation_id}', expected 'square' or 'diamond'"
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
        other => bail!(
            "upgrade[{idx}] unknown requirement_type={other}; supported: tier0_share, formation_active, map_tag"
        ),
    }
    Ok(())
}

fn validate_map(config: &MapConfig) -> Result<()> {
    if config.maps.is_empty() {
        bail!("map list cannot be empty");
    }
    let mut has_christian_map = false;
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
        for faction in &map.allowed_factions {
            if faction != "christian" && faction != "muslim" {
                bail!("map[{index}] has unknown faction '{faction}'");
            }
            if faction == "christian" {
                has_christian_map = true;
            }
        }
    }
    if !has_christian_map {
        bail!("at least one map must allow christian faction");
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
    let required_tier2_kinds = [
        UnitKind::ChristianShieldInfantry,
        UnitKind::ChristianSpearman,
        UnitKind::ChristianUnmountedKnight,
        UnitKind::ChristianSquire,
        UnitKind::ChristianExperiencedBowman,
        UnitKind::ChristianCrossbowman,
        UnitKind::ChristianTracker,
        UnitKind::ChristianScout,
        UnitKind::ChristianDevotedOne,
        UnitKind::ChristianFanatic,
        UnitKind::MuslimShieldInfantry,
        UnitKind::MuslimSpearman,
        UnitKind::MuslimUnmountedKnight,
        UnitKind::MuslimSquire,
        UnitKind::MuslimExperiencedBowman,
        UnitKind::MuslimCrossbowman,
        UnitKind::MuslimTracker,
        UnitKind::MuslimScout,
        UnitKind::MuslimDevotedOne,
        UnitKind::MuslimFanatic,
    ];
    for kind in required_tier2_kinds {
        let key = tier2_config_key_for_kind(kind).expect("tier2 key should exist");
        let Some(stats) = config.tier2_units.get(key) else {
            bail!("roster_tuning.tier2_units is missing required entry '{key}'");
        };
        let allow_zero_damage = matches!(
            kind,
            UnitKind::ChristianSquire
                | UnitKind::ChristianDevotedOne
                | UnitKind::MuslimSquire
                | UnitKind::MuslimDevotedOne
        );
        validate_unit_stats(
            stats,
            &format!("roster_tuning.tier2_units.{key}"),
            allow_zero_damage,
        )?;
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

    use tempfile::TempDir;

    use crate::model::{PlayerFaction, UnitKind};

    use super::GameData;

    fn write_config(dir: &Path, file: &str, content: &str) {
        fs::write(dir.join(file), content).expect("write config");
    }

    fn write_valid_set(dir: &Path) {
        write_config(
            dir,
            "units.json",
            r#"{
              "commander_christian":{"id":"baldiun","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
              "commander_muslim":{"id":"saladin","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
              "recruit_christian_peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
              "recruit_christian_peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
              "recruit_christian_peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0},
              "recruit_muslim_peasant_infantry":{"id":"m1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
              "recruit_muslim_peasant_archer":{"id":"m2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
              "recruit_muslim_peasant_priest":{"id":"m3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
            }"#,
        );
        write_config(
            dir,
            "enemies.json",
            r#"{
              "enemy_christian_peasant_infantry":{"id":"ec_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_christian_peasant_archer":{"id":"ec_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_christian_peasant_priest":{"id":"ec_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_muslim_peasant_infantry":{"id":"em_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_muslim_peasant_archer":{"id":"em_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_muslim_peasant_priest":{"id":"em_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
            }"#,
        );
        write_config(
            dir,
            "formations.json",
            r#"{"square":{"id":"square","slot_spacing":20.0,"offense_multiplier":1.0,"offense_while_moving_multiplier":1.0,"defense_multiplier":1.0,"anti_cavalry_multiplier":1.0,"move_speed_multiplier":1.0},"diamond":{"id":"diamond","slot_spacing":20.0,"offense_multiplier":1.0,"offense_while_moving_multiplier":1.1,"defense_multiplier":0.9,"anti_cavalry_multiplier":1.0,"move_speed_multiplier":1.05}}"#,
        );
        write_config(
            dir,
            "waves.json",
            r#"{"waves":[{"time_secs":0.0,"count":1},{"time_secs":1.0,"count":1}]}"#,
        );
        write_config(
            dir,
            "upgrades.json",
            r#"{"upgrades":[{"id":"u","kind":"damage","value":1.0}]}"#,
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
              "recruit_pool":["christian_peasant_infantry"]
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
    fn rejects_invalid_unit_cooldown() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "units.json",
            r#"{
              "commander_christian":{"id":"baldiun","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":-1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
              "commander_muslim":{"id":"saladin","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
              "recruit_christian_peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
              "recruit_christian_peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
              "recruit_christian_peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0},
              "recruit_muslim_peasant_infantry":{"id":"m1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
              "recruit_muslim_peasant_archer":{"id":"m2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
              "recruit_muslim_peasant_priest":{"id":"m3","max_hp":8.0,"armor":0.5,"damage":0.0,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
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
              "enemy_christian_peasant_infantry":{"id":"ec_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_christian_peasant_archer":{"id":"ec_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"ranged_attack_damage":2.0,"move_speed":80.0,"morale":85.0},
              "enemy_christian_peasant_priest":{"id":"ec_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_muslim_peasant_infantry":{"id":"em_i","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_muslim_peasant_archer":{"id":"em_a","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0},
              "enemy_muslim_peasant_priest":{"id":"em_p","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}
            }"#,
        );

        let err =
            GameData::load_from_dir(tmp.path()).expect_err("expected invalid enemy ranged config");
        assert!(err.to_string().contains("ranged fields"));
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
                "christian_peasant_infantry",
                "christian_peasant_archer",
                "christian_peasant_priest"
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
                  "requirement_type":"unknown_gate"
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected bad requirement type");
        assert!(
            err.to_string()
                .contains("supported: tier0_share, formation_active, map_tag")
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
                  "value":1.0
                }
              ]
            }"#,
        );
        let err = GameData::load_from_dir(tmp.path()).expect_err("expected unknown kind failure");
        assert!(err.to_string().contains("is not wired in runtime systems"));
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
