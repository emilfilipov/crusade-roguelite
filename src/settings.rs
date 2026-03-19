use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::model::FrameRateCap;

const SETTINGS_DIR_NAME: &str = "CrusadeRoguelite";
const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Resource, Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub frame_rate_cap: FrameRateCap,
}

#[derive(Resource, Clone, Debug)]
struct SettingsFilePath(PathBuf);

impl Default for SettingsFilePath {
    fn default() -> Self {
        Self(default_settings_file_path())
    }
}

pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameRateCap>()
            .init_resource::<AppSettings>()
            .init_resource::<SettingsFilePath>()
            .add_systems(Startup, load_settings_on_startup)
            .add_systems(Update, persist_settings_when_changed);
    }
}

fn load_settings_on_startup(
    mut settings: ResMut<AppSettings>,
    mut frame_cap: ResMut<FrameRateCap>,
    mut settings_path: ResMut<SettingsFilePath>,
) {
    if cfg!(test) {
        *frame_cap = settings.frame_rate_cap;
        return;
    }

    settings_path.0 = default_settings_file_path();

    if settings_path.0.is_file() {
        match fs::read_to_string(&settings_path.0) {
            Ok(raw) => match deserialize_settings_json(&raw) {
                Ok(loaded) => {
                    *settings = loaded;
                }
                Err(err) => {
                    warn!(
                        "Failed to parse settings file {} ({err}); using defaults.",
                        settings_path.0.display()
                    );
                }
            },
            Err(err) => {
                warn!(
                    "Failed to read settings file {} ({err}); using defaults.",
                    settings_path.0.display()
                );
            }
        }
    }

    *frame_cap = settings.frame_rate_cap;

    if !settings_path.0.is_file()
        && let Err(err) = write_settings_file(&settings_path.0, &settings)
    {
        warn!(
            "Failed to write initial settings file {} ({err}).",
            settings_path.0.display()
        );
    }
}

fn persist_settings_when_changed(settings: Res<AppSettings>, settings_path: Res<SettingsFilePath>) {
    if cfg!(test) {
        return;
    }
    if !settings.is_changed() {
        return;
    }
    if let Err(err) = write_settings_file(&settings_path.0, &settings) {
        warn!(
            "Failed to persist settings to {} ({err}).",
            settings_path.0.display()
        );
    }
}

fn default_settings_file_path() -> PathBuf {
    let fallback_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    settings_file_path_for(local_app_data.as_deref(), &fallback_dir)
}

fn settings_file_path_for(local_app_data: Option<&Path>, fallback_dir: &Path) -> PathBuf {
    if let Some(local_app_data) = local_app_data {
        local_app_data
            .join(SETTINGS_DIR_NAME)
            .join(SETTINGS_FILE_NAME)
    } else {
        fallback_dir.join("settings").join(SETTINGS_FILE_NAME)
    }
}

fn deserialize_settings_json(raw: &str) -> Result<AppSettings, serde_json::Error> {
    serde_json::from_str(raw)
}

fn serialize_settings_json(settings: &AppSettings) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(settings)
}

fn write_settings_file(path: &Path, settings: &AppSettings) -> std::io::Result<()> {
    let encoded = serialize_settings_json(settings)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, encoded)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use tempfile::TempDir;

    use crate::model::FrameRateCap;
    use crate::settings::{
        AppSettings, deserialize_settings_json, settings_file_path_for, write_settings_file,
    };

    #[test]
    fn settings_deserialize_defaults_when_fields_missing() {
        let parsed = deserialize_settings_json("{}").expect("deserialize");
        assert_eq!(parsed.frame_rate_cap, FrameRateCap::Fps60);
    }

    #[test]
    fn settings_deserialize_ignores_unknown_fields() {
        let raw = r#"{"frame_rate_cap":"120","legacy_setting":"deprecated"}"#;
        let parsed = deserialize_settings_json(raw).expect("deserialize");
        assert_eq!(parsed.frame_rate_cap, FrameRateCap::Fps120);
    }

    #[test]
    fn settings_path_prefers_local_app_data() {
        let local = Path::new("C:/LocalAppData");
        let fallback = Path::new("C:/Project");
        let path = settings_file_path_for(Some(local), fallback);
        assert_eq!(
            path,
            Path::new("C:/LocalAppData/CrusadeRoguelite/settings.json")
        );
    }

    #[test]
    fn settings_path_falls_back_to_project_directory() {
        let fallback = Path::new("C:/Project");
        let path = settings_file_path_for(None, fallback);
        assert_eq!(path, Path::new("C:/Project/settings/settings.json"));
    }

    #[test]
    fn settings_write_persists_json_to_disk() {
        let temp = TempDir::new().expect("temp dir");
        let path = temp.path().join("settings").join("settings.json");
        let settings = AppSettings {
            frame_rate_cap: FrameRateCap::Fps90,
        };
        write_settings_file(&path, &settings).expect("write settings");
        let raw = std::fs::read_to_string(path).expect("read settings");
        assert!(raw.contains(r#""frame_rate_cap": "90""#));
    }
}
