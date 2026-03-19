use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(States, Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Boot,
    MainMenu,
    Settings,
    InRun,
    LevelUp,
    Paused,
    GameOver,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct RunSession {
    pub survived_seconds: f32,
}

#[derive(Resource, Clone, Copy, Debug, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
pub enum FrameRateCap {
    #[default]
    #[serde(rename = "60")]
    Fps60,
    #[serde(rename = "90")]
    Fps90,
    #[serde(rename = "120")]
    Fps120,
}

impl FrameRateCap {
    pub const fn as_u32(self) -> u32 {
        match self {
            Self::Fps60 => 60,
            Self::Fps90 => 90,
            Self::Fps120 => 120,
        }
    }

    pub const fn all() -> [FrameRateCap; 3] {
        [
            FrameRateCap::Fps60,
            FrameRateCap::Fps90,
            FrameRateCap::Fps120,
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Team {
    Friendly,
    Enemy,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnitKind {
    Commander,
    InfantryKnight,
    EnemyBanditRaider,
    RescuableInfantry,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Unit {
    pub team: Team,
    pub kind: UnitKind,
    pub level: u32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BaseMaxHealth(pub f32);

#[derive(Component, Clone, Copy, Debug)]
pub struct Morale {
    pub current: f32,
    pub max: f32,
}

impl Morale {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn ratio(self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Armor(pub f32);

#[derive(Component, Clone, Copy, Debug)]
pub struct AttackProfile {
    pub damage: f32,
    pub range: f32,
    pub cooldown_secs: f32,
}

#[derive(Component, Clone, Debug)]
pub struct AttackCooldown(pub Timer);

#[derive(Component, Clone, Copy, Debug)]
pub struct MoveSpeed(pub f32);

#[derive(Component, Clone, Copy, Debug)]
pub struct ColliderRadius(pub f32);

#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerControlled;

#[derive(Component, Clone, Copy, Debug)]
pub struct FriendlyUnit;

#[derive(Component, Clone, Copy, Debug)]
pub struct EnemyUnit;

#[derive(Component, Clone, Copy, Debug)]
pub struct RescuableUnit;

#[derive(Component, Clone, Copy, Debug)]
pub struct CommanderUnit;

#[derive(Resource, Clone, Debug)]
pub struct GlobalBuffs {
    pub damage_multiplier: f32,
    pub armor_bonus: f32,
    pub attack_speed_multiplier: f32,
    pub pickup_radius_bonus: f32,
    pub move_speed_bonus: f32,
    pub commander_aura_radius_bonus: f32,
    pub authority_friendly_loss_resistance: f32,
    pub authority_enemy_morale_drain_per_sec: f32,
    pub hospitalier_hp_regen_per_sec: f32,
    pub hospitalier_cohesion_regen_per_sec: f32,
    pub hospitalier_morale_regen_per_sec: f32,
}

impl Default for GlobalBuffs {
    fn default() -> Self {
        Self {
            damage_multiplier: 1.0,
            armor_bonus: 0.0,
            attack_speed_multiplier: 1.0,
            pickup_radius_bonus: 0.0,
            move_speed_bonus: 0.0,
            commander_aura_radius_bonus: 0.0,
            authority_friendly_loss_resistance: 0.0,
            authority_enemy_morale_drain_per_sec: 0.0,
            hospitalier_hp_regen_per_sec: 0.0,
            hospitalier_cohesion_regen_per_sec: 0.0,
            hospitalier_morale_regen_per_sec: 0.0,
        }
    }
}

#[derive(Event, Clone, Copy, Debug, Default)]
pub struct StartRunEvent;

#[derive(Event, Clone, Copy, Debug)]
pub struct RecruitEvent {
    pub world_position: Vec2,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct DamageEvent {
    pub target: Entity,
    pub source_team: Team,
    pub amount: f32,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct UnitDamagedEvent {
    pub target: Entity,
    pub team: Team,
    pub amount: f32,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct UnitDiedEvent {
    pub team: Team,
    pub kind: UnitKind,
    pub max_health: f32,
    pub world_position: Vec2,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct GainXpEvent(pub f32);

#[derive(Event, Clone, Copy, Debug)]
pub struct SpawnExpPackEvent {
    pub world_position: Vec2,
    pub xp_value_override: Option<f32>,
    pub pickup_delay_secs: Option<f32>,
}
