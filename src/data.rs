use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use bevy::prelude::*;
use serde::Deserialize;

use crate::model::GameState;

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
}

#[derive(Clone, Debug, Deserialize)]
pub struct UnitsConfigFile {
    pub commander: UnitStatsConfig,
    pub recruit_infantry_knight: UnitStatsConfig,
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
    pub defense_multiplier: f32,
    pub anti_cavalry_multiplier: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FormationsConfigFile {
    pub square: FormationConfig,
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
    pub value: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpgradesConfigFile {
    pub upgrades: Vec<UpgradeConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MapConfig {
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RescueConfig {
    pub spawn_count: u32,
    pub rescue_radius: f32,
    pub rescue_duration_secs: f32,
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
    Ok(())
}

fn validate_units(config: &UnitsConfigFile) -> Result<()> {
    validate_unit_stats(&config.commander, "commander")?;
    validate_unit_stats(&config.recruit_infantry_knight, "recruit_infantry_knight")
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
    Ok(())
}

fn validate_formations(config: &FormationsConfigFile) -> Result<()> {
    if config.square.slot_spacing <= 0.0 {
        bail!("square slot_spacing must be > 0");
    }
    if config.square.offense_multiplier <= 0.0 || config.square.defense_multiplier <= 0.0 {
        bail!("square multipliers must be > 0");
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
    }
    Ok(())
}

fn validate_map(config: &MapConfig) -> Result<()> {
    if config.width <= 0.0 || config.height <= 0.0 {
        bail!("map width and height must be > 0");
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
              "recruit_infantry_knight":{"id":"r","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0}
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
            r#"{"square":{"id":"square","slot_spacing":20.0,"offense_multiplier":1.0,"defense_multiplier":1.0,"anti_cavalry_multiplier":1.0}}"#,
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
        write_config(dir, "map.json", r#"{"width":1000.0,"height":1000.0}"#);
        write_config(
            dir,
            "rescue.json",
            r#"{"spawn_count":1,"rescue_radius":10.0,"rescue_duration_secs":1.0}"#,
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
              "recruit_infantry_knight":{"id":"r","max_hp":9.0,"armor":1.0,"damage":2.0,"attack_cooldown_secs":1.0,"attack_range":20.0,"move_speed":90.0,"morale":90.0}
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
}
