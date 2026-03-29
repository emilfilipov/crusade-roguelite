use bevy::prelude::*;

use crate::data::GameData;
use crate::model::{PlayerFaction, RecruitArchetype};
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
    let commander_christian = data.units.commander_for_faction(PlayerFaction::Christian);
    let commander_muslim = data.units.commander_for_faction(PlayerFaction::Muslim);
    let infantry_christian = data
        .units
        .recruit_for_faction_and_archetype(PlayerFaction::Christian, RecruitArchetype::Infantry);
    let infantry_muslim = data
        .units
        .recruit_for_faction_and_archetype(PlayerFaction::Muslim, RecruitArchetype::Infantry);
    let enemy_infantry_christian = data
        .enemies
        .enemy_profile_for_faction_and_archetype(
            PlayerFaction::Christian,
            RecruitArchetype::Infantry,
        )
        .expect("enemy infantry profile should exist");
    let enemy_infantry_muslim = data
        .enemies
        .enemy_profile_for_faction_and_archetype(PlayerFaction::Muslim, RecruitArchetype::Infantry)
        .expect("enemy infantry profile should exist");

    let mut entries = vec![
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Baldiun (Commander)".to_string(),
            description: format!(
                "Support commander. HP {}, Armor {}, Damage {}, Aura {}.",
                commander_christian.max_hp,
                commander_christian.armor,
                commander_christian.damage,
                commander_christian.aura_radius,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Saladin (Commander)".to_string(),
            description: format!(
                "Support commander. HP {}, Armor {}, Damage {}, Aura {}.",
                commander_muslim.max_hp,
                commander_muslim.armor,
                commander_muslim.damage,
                commander_muslim.aura_radius,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Peasant Infantry".to_string(),
            description: format!(
                "Melee retinue. Christian profile: HP {}, Armor {}, Damage {}.",
                infantry_christian.max_hp,
                infantry_christian.armor,
                infantry_christian.damage,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Peasant Archer".to_string(),
            description: "Hybrid unit with weak melee and stronger ranged arrows.".to_string(),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Peasant Priest".to_string(),
            description: "Support caster with periodic attack-speed blessing.".to_string(),
            icon: Some(UpgradeCardIcon::HospitalierAura),
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Peasant Infantry (Alt Profile)".to_string(),
            description: format!(
                "Melee retinue. Muslim profile: HP {}, Armor {}, Damage {}.",
                infantry_muslim.max_hp,
                infantry_muslim.armor,
                infantry_muslim.damage,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Peasant Archer (Alt Profile)".to_string(),
            description: "Hybrid unit with weak melee and stronger ranged arrows (Muslim profile)."
                .to_string(),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Units,
            title: "Peasant Priest (Alt Profile)".to_string(),
            description: "Support caster with periodic attack-speed blessing (Muslim profile)."
                .to_string(),
            icon: Some(UpgradeCardIcon::HospitalierAura),
        },
        ArchiveEntry {
            category: ArchiveCategory::Enemies,
            title: "Enemy Infantry".to_string(),
            description: format!(
                "Melee enemy profile A. HP {}, Damage {}, Move {}.",
                enemy_infantry_christian.max_hp,
                enemy_infantry_christian.damage,
                enemy_infantry_christian.move_speed,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Enemies,
            title: "Enemy Infantry (Alt Profile)".to_string(),
            description: format!(
                "Melee enemy profile B. HP {}, Damage {}, Move {}.",
                enemy_infantry_muslim.max_hp,
                enemy_infantry_muslim.damage,
                enemy_infantry_muslim.move_speed,
            ),
            icon: None,
        },
        ArchiveEntry {
            category: ArchiveCategory::Stats,
            title: "Morale".to_string(),
            description: "Morale is the army’s single discipline stat. From 51-100 it grants gradually increasing damage, armor, and tiny HP regen; below 50 it shifts into penalties (armor down, escape speed up). Prolonged encirclement drains morale after a short delay, and at 0 morale 10% of the retinue is dropped as rescuable units before morale resets after a brief delay."
                .to_string(),
            icon: Some(UpgradeCardIcon::AuthorityAura),
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
            title: "Gold Pack".to_string(),
            description: format!(
                "Pickup grants gold. Base pack value {} and scales by wave/level.",
                data.drops.gold_per_pack,
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
