use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use bevy::prelude::*;
use serde::Deserialize;

use crate::model::{GameState, RecruitUnitKind};

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

#[derive(Clone, Debug, Deserialize)]
pub struct UnitsConfigFile {
    pub commander: UnitStatsConfig,
    pub recruit_christian_peasant_infantry: UnitStatsConfig,
    pub recruit_christian_peasant_archer: UnitStatsConfig,
    pub recruit_christian_peasant_priest: UnitStatsConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemyStatsConfig {
    pub id: String,
    pub max_hp: f32,
    pub armor: f32,
    pub damage: f32,
    pub attack_cooldown_secs: f32,
    pub attack_range: f32,
    pub move_speed: f32,
    pub morale: f32,
    #[serde(default = "default_enemy_collision_radius")]
    pub collision_radius: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EnemiesConfigFile {
    pub bandit_raider: EnemyStatsConfig,
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
}

impl RescueRecruitKindConfig {
    pub const fn as_recruit_unit_kind(self) -> RecruitUnitKind {
        match self {
            Self::ChristianPeasantInfantry => RecruitUnitKind::ChristianPeasantInfantry,
            Self::ChristianPeasantArcher => RecruitUnitKind::ChristianPeasantArcher,
            Self::ChristianPeasantPriest => RecruitUnitKind::ChristianPeasantPriest,
        }
    }

    pub const fn tier(self) -> u8 {
        match self {
            Self::ChristianPeasantInfantry => 0,
            Self::ChristianPeasantArcher => 0,
            Self::ChristianPeasantPriest => 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DropsConfig {
    pub initial_spawn_count: u32,
    pub spawn_interval_secs: f32,
    pub pickup_radius: f32,
    pub xp_per_pack: f32,
    pub max_active_packs: u32,
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

        validate_units(&units)?;
        validate_enemies(&enemies)?;
        validate_formations(&formations)?;
        validate_waves(&waves)?;
        validate_upgrades(&upgrades)?;
        validate_map(&map)?;
        validate_rescue(&rescue)?;
        validate_drops(&drops)?;

        Ok(Self {
            units,
            enemies,
            formations,
            waves,
            upgrades,
            map,
            rescue,
            drops,
        })
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
    ]
}

fn default_enemy_collision_radius() -> f32 {
    15.0
}

fn validate_unit_stats(unit: &UnitStatsConfig, label: &str) -> Result<()> {
    if unit.max_hp <= 0.0 {
        bail!("{label} max_hp must be > 0");
    }
    if unit.damage <= 0.0 {
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
    validate_unit_stats(&config.commander, "commander")?;
    validate_unit_stats(
        &config.recruit_christian_peasant_infantry,
        "recruit_christian_peasant_infantry",
    )?;
    validate_unit_stats(
        &config.recruit_christian_peasant_archer,
        "recruit_christian_peasant_archer",
    )?;
    validate_unit_stats(
        &config.recruit_christian_peasant_priest,
        "recruit_christian_peasant_priest",
    )
}

fn validate_enemies(config: &EnemiesConfigFile) -> Result<()> {
    if config.bandit_raider.max_hp <= 0.0 {
        bail!("enemy bandit_raider max_hp must be > 0");
    }
    if config.bandit_raider.attack_cooldown_secs <= 0.0 {
        bail!("enemy bandit_raider attack_cooldown_secs must be > 0");
    }
    if config.bandit_raider.damage <= 0.0 {
        bail!("enemy bandit_raider damage must be > 0");
    }
    if config.bandit_raider.attack_range <= 0.0 {
        bail!("enemy bandit_raider attack_range must be > 0");
    }
    if config.bandit_raider.move_speed <= 0.0 {
        bail!("enemy bandit_raider move_speed must be > 0");
    }
    if config.bandit_raider.morale <= 0.0 {
        bail!("enemy bandit_raider morale must be > 0");
    }
    if config.bandit_raider.collision_radius <= 0.0 {
        bail!("enemy bandit_raider collision_radius must be > 0");
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
    if config.xp_per_pack <= 0.0 {
        bail!("drops xp_per_pack must be > 0");
    }
    if config.max_active_packs == 0 {
        bail!("drops max_active_packs must be > 0");
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

    use super::GameData;

    fn write_config(dir: &Path, file: &str, content: &str) {
        fs::write(dir.join(file), content).expect("write config");
    }

    fn write_valid_set(dir: &Path) {
        write_config(
            dir,
            "units.json",
            r#"{
              "commander":{"id":"c","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
              "recruit_christian_peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
              "recruit_christian_peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
              "recruit_christian_peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.1,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
            }"#,
        );
        write_config(
            dir,
            "enemies.json",
            r#"{"bandit_raider":{"id":"e","max_hp":6.0,"armor":0.0,"damage":1.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":80.0,"morale":85.0}}"#,
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
            r#"{"initial_spawn_count":3,"spawn_interval_secs":1.5,"pickup_radius":15.0,"xp_per_pack":5.0,"max_active_packs":30}"#,
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
    fn rejects_invalid_unit_cooldown() {
        let tmp = TempDir::new().expect("tmp");
        write_valid_set(tmp.path());
        write_config(
            tmp.path(),
            "units.json",
            r#"{
              "commander":{"id":"c","max_hp":10.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":-1.0,"attack_range":20.0,"move_speed":100.0,"morale":100.0,"aura_radius":10.0},
              "recruit_christian_peasant_infantry":{"id":"r1","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0},
              "recruit_christian_peasant_archer":{"id":"r2","max_hp":7.0,"armor":0.5,"damage":1.5,"attack_cooldown_secs":1.1,"attack_range":80.0,"move_speed":95.0,"morale":85.0},
              "recruit_christian_peasant_priest":{"id":"r3","max_hp":8.0,"armor":0.5,"damage":0.1,"attack_cooldown_secs":1.1,"attack_range":20.0,"move_speed":92.0,"morale":88.0}
            }"#,
        );

        let err = GameData::load_from_dir(tmp.path()).expect_err("expected invalid config");
        assert!(err.to_string().contains("attack_cooldown_secs"));
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
}
