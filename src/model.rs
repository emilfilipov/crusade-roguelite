use bevy::prelude::*;

#[derive(States, Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Boot,
    MainMenu,
    InRun,
    Paused,
    GameOver,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct RunSession {
    pub survived_seconds: f32,
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
    pub morale_weight: f32,
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
    pub cohesion_bonus: f32,
    pub commander_aura_bonus: f32,
}

impl Default for GlobalBuffs {
    fn default() -> Self {
        Self {
            damage_multiplier: 1.0,
            armor_bonus: 0.0,
            attack_speed_multiplier: 1.0,
            cohesion_bonus: 0.0,
            commander_aura_bonus: 0.0,
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
pub struct UnitDiedEvent {
    pub team: Team,
    pub kind: UnitKind,
    pub world_position: Vec2,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct GainXpEvent(pub f32);
