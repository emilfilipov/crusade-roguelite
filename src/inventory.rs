use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::model::UnitKind;

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
    pub const fn label(self) -> &'static str {
        match self {
            Self::Commander => "Commander",
            Self::Tier0 => "Tier 0 Units",
            Self::Tier1 => "Tier 1 Units",
            Self::Tier2 => "Tier 2 Units",
            Self::Tier3 => "Tier 3 Units",
            Self::Tier4 => "Tier 4 Units",
            Self::Tier5 => "Tier 5 Units",
            Self::Hero => "Hero Units",
        }
    }

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GearItemEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_key: String,
    #[serde(default)]
    pub base_bonus: f32,
    #[serde(default)]
    pub melee_damage_bonus: f32,
    #[serde(default)]
    pub ranged_damage_bonus: f32,
    #[serde(default)]
    pub armor_bonus: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EquippedSlot {
    pub slot_id: String,
    pub display_name: String,
    pub item_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnitEquipmentSetup {
    pub unit_type: EquipmentUnitType,
    pub slots: Vec<EquippedSlot>,
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    pub bag: Vec<GearItemEntry>,
    pub setups: Vec<UnitEquipmentSetup>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct UnitEquipmentBonuses {
    pub melee_damage_bonus: f32,
    pub ranged_damage_bonus: f32,
    pub armor_bonus: f32,
}

impl UnitEquipmentBonuses {
    pub const fn zero() -> Self {
        Self {
            melee_damage_bonus: 0.0,
            ranged_damage_bonus: 0.0,
            armor_bonus: 0.0,
        }
    }
}

impl InventoryState {
    pub fn setup_for(&self, unit_type: EquipmentUnitType) -> Option<&UnitEquipmentSetup> {
        self.setups
            .iter()
            .find(|setup| setup.unit_type == unit_type)
    }
}

pub fn equipment_unit_type_for_unit(kind: UnitKind, tier: Option<u8>) -> Option<EquipmentUnitType> {
    match kind {
        UnitKind::Commander => Some(EquipmentUnitType::Commander),
        UnitKind::ChristianPeasantInfantry
        | UnitKind::ChristianPeasantArcher
        | UnitKind::ChristianPeasantPriest => EquipmentUnitType::from_tier(tier.unwrap_or(0)),
        _ => None,
    }
}

pub fn gear_bonuses_for_unit(
    inventory: &InventoryState,
    kind: UnitKind,
    tier: Option<u8>,
) -> UnitEquipmentBonuses {
    let Some(unit_type) = equipment_unit_type_for_unit(kind, tier) else {
        return UnitEquipmentBonuses::zero();
    };
    let Some(setup) = inventory.setup_for(unit_type) else {
        return UnitEquipmentBonuses::zero();
    };

    let mut bonuses = UnitEquipmentBonuses::zero();
    for slot in &setup.slots {
        let Some(item_id) = slot.item_id.as_deref() else {
            continue;
        };
        let Some(item) = inventory.bag.iter().find(|entry| entry.id == item_id) else {
            continue;
        };

        // Default slot behavior:
        // - melee_weapon => base bonus affects melee only
        // - ranged_weapon => base bonus affects ranged only
        // - armor => base bonus affects armor only
        match slot.slot_id.as_str() {
            "melee_weapon" => {
                bonuses.melee_damage_bonus += item.base_bonus;
            }
            "ranged_weapon" => {
                bonuses.ranged_damage_bonus += item.base_bonus;
            }
            "armor" => {
                bonuses.armor_bonus += item.base_bonus;
            }
            _ => {}
        }

        // Explicit item bonuses can cross over regardless of slot.
        bonuses.melee_damage_bonus += item.melee_damage_bonus;
        bonuses.ranged_damage_bonus += item.ranged_damage_bonus;
        bonuses.armor_bonus += item.armor_bonus;
    }
    bonuses
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
            bag: Vec::new(),
            setups,
        }
    }
}

fn default_equipment_slots(unit_type: EquipmentUnitType) -> Vec<EquippedSlot> {
    match unit_type {
        EquipmentUnitType::Commander => vec![
            EquippedSlot {
                slot_id: slot_id_from_label("Banner"),
                display_name: "Banner".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: slot_id_from_label("Instrument"),
                display_name: "Instrument".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: slot_id_from_label("Chant"),
                display_name: "Chant".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: slot_id_from_label("Squire"),
                display_name: "Squire".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: slot_id_from_label("Symbol"),
                display_name: "Symbol".to_string(),
                item_id: None,
            },
        ],
        _ => vec![
            EquippedSlot {
                slot_id: "melee_weapon".to_string(),
                display_name: "Melee".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: "ranged_weapon".to_string(),
                display_name: "Ranged".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: "armor".to_string(),
                display_name: "Armor".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: "banner".to_string(),
                display_name: "Banner".to_string(),
                item_id: None,
            },
            EquippedSlot {
                slot_id: "squire".to_string(),
                display_name: "Squire".to_string(),
                item_id: None,
            },
        ],
    }
}

fn slot_id_from_label(label: &str) -> String {
    label.to_ascii_lowercase().replace([' ', '-'], "_")
}

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryState>();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EquipmentUnitType, GearItemEntry, InventoryState, UnitEquipmentBonuses,
        gear_bonuses_for_unit,
    };
    use crate::model::UnitKind;

    #[test]
    fn default_inventory_contains_setups_for_all_unit_types() {
        let inventory = InventoryState::default();
        for unit_type in EquipmentUnitType::all() {
            let setup = inventory
                .setup_for(unit_type)
                .expect("setup should exist for each unit type");
            assert_eq!(setup.unit_type, unit_type);
            assert_eq!(setup.slots.len(), 5);
            if unit_type == EquipmentUnitType::Commander {
                assert_eq!(setup.slots[0].display_name, "Banner");
                assert_eq!(setup.slots[3].display_name, "Squire");
                assert_eq!(setup.slots[4].display_name, "Symbol");
            } else {
                assert_eq!(setup.slots[0].display_name, "Melee");
                assert_eq!(setup.slots[3].display_name, "Banner");
                assert_eq!(setup.slots[4].display_name, "Squire");
            }
        }
    }

    #[test]
    fn inventory_state_round_trip_json_is_lossless() {
        let inventory = InventoryState::default();
        let json = serde_json::to_string(&inventory).expect("serialize inventory");
        let decoded: InventoryState = serde_json::from_str(&json).expect("deserialize inventory");
        assert_eq!(decoded, inventory);
    }

    #[test]
    fn slot_defaults_apply_bonus_only_to_matching_stat() {
        let mut inventory = InventoryState::default();
        inventory.bag.push(GearItemEntry {
            id: "melee_sword".to_string(),
            name: "Sword".to_string(),
            description: "Melee weapon.".to_string(),
            icon_key: "sword".to_string(),
            base_bonus: 3.0,
            melee_damage_bonus: 0.0,
            ranged_damage_bonus: 0.0,
            armor_bonus: 0.0,
        });
        inventory.bag.push(GearItemEntry {
            id: "ranged_bow".to_string(),
            name: "Bow".to_string(),
            description: "Ranged weapon.".to_string(),
            icon_key: "bow".to_string(),
            base_bonus: 5.0,
            melee_damage_bonus: 0.0,
            ranged_damage_bonus: 0.0,
            armor_bonus: 0.0,
        });
        inventory.bag.push(GearItemEntry {
            id: "armor_vest".to_string(),
            name: "Vest".to_string(),
            description: "Armor.".to_string(),
            icon_key: "vest".to_string(),
            base_bonus: 2.0,
            melee_damage_bonus: 0.0,
            ranged_damage_bonus: 0.0,
            armor_bonus: 0.0,
        });

        let setup = inventory
            .setups
            .iter_mut()
            .find(|entry| entry.unit_type == EquipmentUnitType::Tier0)
            .expect("tier0 setup");
        setup.slots[0].item_id = Some("melee_sword".to_string());
        setup.slots[1].item_id = Some("ranged_bow".to_string());
        setup.slots[2].item_id = Some("armor_vest".to_string());

        let bonuses =
            gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        assert_eq!(
            bonuses,
            UnitEquipmentBonuses {
                melee_damage_bonus: 3.0,
                ranged_damage_bonus: 5.0,
                armor_bonus: 2.0,
            }
        );
    }

    #[test]
    fn explicit_item_cross_bonus_is_applied() {
        let mut inventory = InventoryState::default();
        inventory.bag.push(GearItemEntry {
            id: "melee_special".to_string(),
            name: "Special".to_string(),
            description: "Cross bonus.".to_string(),
            icon_key: "special".to_string(),
            base_bonus: 2.0,
            melee_damage_bonus: 0.0,
            ranged_damage_bonus: 1.5,
            armor_bonus: 0.0,
        });
        let setup = inventory
            .setups
            .iter_mut()
            .find(|entry| entry.unit_type == EquipmentUnitType::Tier0)
            .expect("tier0 setup");
        setup.slots[0].item_id = Some("melee_special".to_string());

        let bonuses =
            gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        assert_eq!(
            bonuses,
            UnitEquipmentBonuses {
                melee_damage_bonus: 2.0,
                ranged_damage_bonus: 1.5,
                armor_bonus: 0.0,
            }
        );
    }

    #[test]
    fn gear_applies_only_to_equipped_tier() {
        let mut inventory = InventoryState::default();
        inventory.bag.push(GearItemEntry {
            id: "tier0_sword".to_string(),
            name: "Tier0 Sword".to_string(),
            description: "Tier0 only.".to_string(),
            icon_key: "sword".to_string(),
            base_bonus: 4.0,
            melee_damage_bonus: 0.0,
            ranged_damage_bonus: 0.0,
            armor_bonus: 0.0,
        });
        let setup = inventory
            .setups
            .iter_mut()
            .find(|entry| entry.unit_type == EquipmentUnitType::Tier0)
            .expect("tier0 setup");
        setup.slots[0].item_id = Some("tier0_sword".to_string());

        let tier0_bonuses =
            gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(0));
        let tier1_bonuses =
            gear_bonuses_for_unit(&inventory, UnitKind::ChristianPeasantInfantry, Some(1));
        assert_eq!(tier0_bonuses.melee_damage_bonus, 4.0);
        assert_eq!(tier1_bonuses, UnitEquipmentBonuses::zero());
    }
}
