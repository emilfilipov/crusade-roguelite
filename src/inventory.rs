use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EquipmentUnitType {
    Commander,
    ChristianPeasantInfantry,
    ChristianPeasantArcher,
    ChristianPeasantPriest,
}

impl EquipmentUnitType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Commander => "Commander",
            Self::ChristianPeasantInfantry => "Christian Peasant Infantry",
            Self::ChristianPeasantArcher => "Christian Peasant Archer",
            Self::ChristianPeasantPriest => "Christian Peasant Priest",
        }
    }

    pub const fn all() -> [Self; 4] {
        [
            Self::Commander,
            Self::ChristianPeasantInfantry,
            Self::ChristianPeasantArcher,
            Self::ChristianPeasantPriest,
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
                slots: default_equipment_slots(),
            })
            .collect();
        Self {
            bag: Vec::new(),
            setups,
        }
    }
}

fn default_equipment_slots() -> Vec<EquippedSlot> {
    vec![
        EquippedSlot {
            slot_id: "weapon".to_string(),
            display_name: "Weapon".to_string(),
            item_id: None,
        },
        EquippedSlot {
            slot_id: "armor".to_string(),
            display_name: "Armor".to_string(),
            item_id: None,
        },
        EquippedSlot {
            slot_id: "trinket".to_string(),
            display_name: "Trinket".to_string(),
            item_id: None,
        },
    ]
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
