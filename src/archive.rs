use bevy::prelude::*;

use crate::data::GameData;
use crate::upgrades::{
    UpgradeCardIcon, upgrade_card_icon, upgrade_display_description, upgrade_display_title,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ArchiveCategory {
    Units,
    Enemies,
    Skills,
    Stats,
    Bonuses,
    Drops,
}

impl ArchiveCategory {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Units => "Units",
            Self::Enemies => "Enemies",
            Self::Skills => "Skills",
            Self::Stats => "Stats",
            Self::Bonuses => "Bonuses",
            Self::Drops => "Drops",
        }
    }

    pub const fn all() -> [Self; 6] {
        [
            Self::Units,
            Self::Enemies,
            Self::Skills,
            Self::Stats,
            Self::Bonuses,
            Self::Drops,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    pub category: ArchiveCategory,
    pub title: String,
    pub description: String,
    pub icon: Option<UpgradeCardIcon>,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ArchiveDataset {
    pub entries: Vec<ArchiveEntry>,
}

pub struct ArchivePlugin;

impl Plugin for ArchivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArchiveDataset>()
            .add_systems(Update, sync_archive_dataset);
    }
}

fn sync_archive_dataset(data: Option<Res<GameData>>, mut archive: ResMut<ArchiveDataset>) {
    let Some(data) = data else {
        return;
    };
    if archive.entries.is_empty() || data.is_changed() {
        archive.entries = build_archive_entries(&data);
    }
}

pub fn build_archive_entries(data: &GameData) -> Vec<ArchiveEntry> {
    let mut entries = vec![
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Baldiun (Commander)".to_string(),
            description: format!(
                "Support commander. HP {}, Armor {}, Damage {}, Aura {}.",
                data.units.commander_christian.max_hp,
                data.units.commander_christian.armor,
                data.units.commander_christian.damage,
                data.units.commander_christian.aura_radius,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Saladin (Commander)".to_string(),
            description: format!(
                "Support commander. HP {}, Armor {}, Damage {}, Aura {}.",
                data.units.commander_muslim.max_hp,
                data.units.commander_muslim.armor,
                data.units.commander_muslim.damage,
                data.units.commander_muslim.aura_radius,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Christian Peasant Infantry".to_string(),
            description: format!(
                "Melee retinue. HP {}, Armor {}, Damage {}.",
                data.units.recruit_christian_peasant_infantry.max_hp,
                data.units.recruit_christian_peasant_infantry.armor,
                data.units.recruit_christian_peasant_infantry.damage,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Christian Peasant Archer".to_string(),
            description: "Hybrid unit with weak melee and stronger ranged arrows.".to_string(),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Christian Peasant Priest".to_string(),
            description: "Support caster with periodic attack-speed blessing.".to_string(),
            icon: Some(UpgradeCardIcon::HospitalierAura),
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Muslim Peasant Infantry".to_string(),
            description: format!(
                "Melee retinue. HP {}, Armor {}, Damage {}.",
                data.units.recruit_muslim_peasant_infantry.max_hp,
                data.units.recruit_muslim_peasant_infantry.armor,
                data.units.recruit_muslim_peasant_infantry.damage,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Muslim Peasant Archer".to_string(),
            description: "Hybrid unit with weak melee and stronger ranged arrows.".to_string(),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Muslim Peasant Priest".to_string(),
            description: "Support caster with periodic attack-speed blessing.".to_string(),
            icon: Some(UpgradeCardIcon::HospitalierAura),
        },
        ArchiveEntry {
            category: ArchiveCategory::Enemies,
            title: "Christian Enemy Infantry".to_string(),
            description: format!(
                "Melee enemy. HP {}, Damage {}, Move {}.",
                data.enemies.enemy_christian_peasant_infantry.max_hp,
                data.enemies.enemy_christian_peasant_infantry.damage,
                data.enemies.enemy_christian_peasant_infantry.move_speed,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Enemies,
            title: "Muslim Enemy Infantry".to_string(),
            description: format!(
                "Melee enemy. HP {}, Damage {}, Move {}.",
                data.enemies.enemy_muslim_peasant_infantry.max_hp,
                data.enemies.enemy_muslim_peasant_infantry.damage,
                data.enemies.enemy_muslim_peasant_infantry.move_speed,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Stats,
            title: "Morale".to_string(),
            description: "Low morale reduces movement speed (max 25% slow). At 0 average morale, the banner drops and banner-item bonuses are disabled until recovered."
                .to_string(),
            icon: Some(UpgradeCardIcon::AuthorityAura),
        },
        ArchiveEntry {
            category: ArchiveCategory::Stats,
            title: "Cohesion".to_string(),
            description: "Cohesion affects damage output only when below 50%. If cohesion reaches 0, 10% of the retinue is lost, cohesion resets, and collapse enters a short grace period."
                .to_string(),
            icon: Some(UpgradeCardIcon::FormationSquare),
        },
        ArchiveEntry {
            category: ArchiveCategory::Bonuses,
            title: "Square Formation".to_string(),
            description: "Neutral baseline formation for defensive control.".to_string(),
            icon: Some(UpgradeCardIcon::FormationSquare),
        },
        ArchiveEntry {
            category: ArchiveCategory::Bonuses,
            title: "Diamond Formation".to_string(),
            description: "Higher moving offense and speed, lower defense.".to_string(),
            icon: Some(UpgradeCardIcon::FormationDiamond),
        },
        ArchiveEntry {
            category: ArchiveCategory::Drops,
            title: "XP Pack".to_string(),
            description: format!(
                "Pickup grants XP. Base pack value {} and scales by wave/level.",
                data.drops.xp_per_pack,
            ),
            icon: Some(UpgradeCardIcon::PickupRadius),
        },
        ArchiveEntry {
            category: ArchiveCategory::Drops,
            title: "Rescue Recruit".to_string(),
            description: format!(
                "Channel near neutral unit for {:.1}s to recruit.",
                data.rescue.rescue_duration_secs,
            ),
            icon: Some(UpgradeCardIcon::MoveSpeed),
        },
    ];

    for upgrade in &data.upgrades.upgrades {
        entries.push(ArchiveEntry {
            category: ArchiveCategory::Skills,
            title: upgrade_display_title(upgrade).to_string(),
            description: upgrade_display_description(upgrade),
            icon: Some(upgrade_card_icon(upgrade)),
        });
    }

    entries
}

pub fn validate_archive_entries(entries: &[ArchiveEntry]) -> Result<(), String> {
    if entries.is_empty() {
        return Err("archive entries cannot be empty".to_string());
    }
    for (index, entry) in entries.iter().enumerate() {
        if entry.title.trim().is_empty() {
            return Err(format!("archive entry #{index} has empty title"));
        }
        if entry.description.trim().is_empty() {
            return Err(format!("archive entry #{index} has empty description"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::archive::{
        ArchiveCategory, ArchiveEntry, build_archive_entries, validate_archive_entries,
    };
    use crate::data::GameData;

    #[test]
    fn generated_archive_entries_validate_and_cover_core_categories() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load game data");
        let entries = build_archive_entries(&data);
        validate_archive_entries(&entries).expect("archive entries should validate");

        for category in [
            ArchiveCategory::Units,
            ArchiveCategory::Enemies,
            ArchiveCategory::Skills,
            ArchiveCategory::Drops,
        ] {
            assert!(
                entries.iter().any(|entry| entry.category == category),
                "missing category: {:?}",
                category
            );
        }
    }

    #[test]
    fn validator_rejects_empty_titles_or_descriptions() {
        let invalid = vec![ArchiveEntry {
            category: ArchiveCategory::Units,
            title: " ".to_string(),
            description: "x".to_string(),
            icon: None,
        }];
        assert!(validate_archive_entries(&invalid).is_err());
    }
}
