use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EquipmentUnitType {
    Commander,
    Tier0,
    Tier1,
    Tier2,
    Tier3,
    Tier4,
    Tier5,
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
        }
    }

    pub const fn all() -> [Self; 7] {
        [
            Self::Commander,
            Self::Tier0,
            Self::Tier1,
            Self::Tier2,
            Self::Tier3,
            Self::Tier4,
            Self::Tier5,
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GearItemEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EquippedSlot {
    pub slot_id: String,
    pub display_name: String,
    pub item_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnitEquipmentSetup {
    pub unit_type: EquipmentUnitType,
    pub slots: Vec<EquippedSlot>,
}

#[derive(Resource, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InventoryState {
    pub bag: Vec<GearItemEntry>,
    pub setups: Vec<UnitEquipmentSetup>,
}

impl InventoryState {
    pub fn setup_for(&self, unit_type: EquipmentUnitType) -> Option<&UnitEquipmentSetup> {
        self.setups
            .iter()
            .find(|setup| setup.unit_type == unit_type)
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
            bag: Vec::new(),
            setups,
        }
    }
}

fn default_equipment_slots(unit_type: EquipmentUnitType) -> Vec<EquippedSlot> {
    let (first, second, third) = match unit_type {
        EquipmentUnitType::Commander => ("Banner", "Instrument", "Chant"),
        _ => ("Melee Weapon", "Ranged Weapon", "Armor"),
    };
    vec![
        EquippedSlot {
            slot_id: slot_id_from_label(first),
            display_name: first.to_string(),
            item_id: None,
        },
        EquippedSlot {
            slot_id: slot_id_from_label(second),
            display_name: second.to_string(),
            item_id: None,
        },
        EquippedSlot {
            slot_id: slot_id_from_label(third),
            display_name: third.to_string(),
            item_id: None,
        },
    ]
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
    use super::{EquipmentUnitType, InventoryState};

    #[test]
    fn default_inventory_contains_setups_for_all_unit_types() {
        let inventory = InventoryState::default();
        for unit_type in EquipmentUnitType::all() {
            let setup = inventory
                .setup_for(unit_type)
                .expect("setup should exist for each unit type");
            assert_eq!(setup.unit_type, unit_type);
            assert_eq!(setup.slots.len(), 3);
            if unit_type == EquipmentUnitType::Commander {
                assert_eq!(setup.slots[0].display_name, "Banner");
            } else {
                assert_eq!(setup.slots[0].display_name, "Melee Weapon");
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
}
