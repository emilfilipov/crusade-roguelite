use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(States, Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Boot,
    MainMenu,
    MatchSetup,
    Archive,
    Settings,
    InRun,
    LevelUp,
    Paused,
    GameOver,
    Victory,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct RunSession {
    pub survived_seconds: f32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PlayerFaction {
    #[default]
    Christian,
    Muslim,
}

impl PlayerFaction {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Christian => "Christian",
            Self::Muslim => "Muslim",
        }
    }

    pub const fn config_key(self) -> &'static str {
        match self {
            Self::Christian => "christian",
            Self::Muslim => "muslim",
        }
    }

    pub const fn opposing(self) -> Self {
        match self {
            Self::Christian => Self::Muslim,
            Self::Muslim => Self::Christian,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameDifficulty {
    #[default]
    Recruit,
    Experienced,
    AloneAgainstTheInfidels,
}

impl GameDifficulty {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Recruit => "Recruit",
            Self::Experienced => "Experienced",
            Self::AloneAgainstTheInfidels => "Alone Against the Infidels",
        }
    }

    pub const fn config_key(self) -> &'static str {
        match self {
            Self::Recruit => "recruit",
            Self::Experienced => "experienced",
            Self::AloneAgainstTheInfidels => "alone_against_the_infidels",
        }
    }

    pub const fn all() -> [GameDifficulty; 3] {
        [
            GameDifficulty::Recruit,
            GameDifficulty::Experienced,
            GameDifficulty::AloneAgainstTheInfidels,
        ]
    }
}

#[derive(Resource, Clone, Debug)]
pub struct MatchSetupSelection {
    pub faction: PlayerFaction,
    pub map_id: String,
    pub commander_id: String,
    pub difficulty: GameDifficulty,
}

impl Default for MatchSetupSelection {
    fn default() -> Self {
        Self {
            faction: PlayerFaction::Christian,
            map_id: String::new(),
            commander_id: "baldiun".to_string(),
            difficulty: GameDifficulty::Recruit,
        }
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Team {
    Friendly,
    Enemy,
    Neutral,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UnitKind {
    Commander,
    ChristianPeasantInfantry,
    ChristianPeasantArcher,
    ChristianPeasantPriest,
    ChristianMenAtArms,
    ChristianBowman,
    ChristianDevoted,
    ChristianShieldInfantry,
    ChristianSpearman,
    ChristianUnmountedKnight,
    ChristianSquire,
    ChristianExperiencedBowman,
    ChristianCrossbowman,
    ChristianTracker,
    ChristianScout,
    ChristianDevotedOne,
    ChristianFanatic,
    ChristianExperiencedShieldInfantry,
    ChristianShieldedSpearman,
    ChristianKnight,
    ChristianBannerman,
    ChristianEliteBowman,
    ChristianArmoredCrossbowman,
    ChristianPathfinder,
    ChristianMountedScout,
    ChristianCardinal,
    ChristianFlagellant,
    ChristianEliteShieldInfantry,
    ChristianHalberdier,
    ChristianHeavyKnight,
    ChristianEliteBannerman,
    ChristianLongbowman,
    ChristianEliteCrossbowman,
    ChristianHoundmaster,
    ChristianShockCavalry,
    ChristianEliteCardinal,
    ChristianEliteFlagellant,
    ChristianCitadelGuard,
    ChristianArmoredHalberdier,
    ChristianEliteHeavyKnight,
    ChristianGodsChosen,
    ChristianEliteLongbowman,
    ChristianSiegeCrossbowman,
    ChristianEliteHoundmaster,
    ChristianEliteShockCavalry,
    ChristianDivineSpeaker,
    ChristianDivineJudge,
    MuslimPeasantInfantry,
    MuslimPeasantArcher,
    MuslimPeasantPriest,
    MuslimMenAtArms,
    MuslimBowman,
    MuslimDevoted,
    MuslimShieldInfantry,
    MuslimSpearman,
    MuslimUnmountedKnight,
    MuslimSquire,
    MuslimExperiencedBowman,
    MuslimCrossbowman,
    MuslimTracker,
    MuslimScout,
    MuslimDevotedOne,
    MuslimFanatic,
    MuslimExperiencedShieldInfantry,
    MuslimShieldedSpearman,
    MuslimKnight,
    MuslimBannerman,
    MuslimEliteBowman,
    MuslimArmoredCrossbowman,
    MuslimPathfinder,
    MuslimMountedScout,
    MuslimCardinal,
    MuslimFlagellant,
    MuslimEliteShieldInfantry,
    MuslimHalberdier,
    MuslimHeavyKnight,
    MuslimEliteBannerman,
    MuslimLongbowman,
    MuslimEliteCrossbowman,
    MuslimHoundmaster,
    MuslimShockCavalry,
    MuslimEliteCardinal,
    MuslimEliteFlagellant,
    MuslimCitadelGuard,
    MuslimArmoredHalberdier,
    MuslimEliteHeavyKnight,
    MuslimGodsChosen,
    MuslimEliteLongbowman,
    MuslimSiegeCrossbowman,
    MuslimEliteHoundmaster,
    MuslimEliteShockCavalry,
    MuslimDivineSpeaker,
    MuslimDivineJudge,
    RescuableChristianPeasantInfantry,
    RescuableChristianPeasantArcher,
    RescuableChristianPeasantPriest,
    RescuableMuslimPeasantInfantry,
    RescuableMuslimPeasantArcher,
    RescuableMuslimPeasantPriest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecruitUnitKind {
    ChristianPeasantInfantry,
    ChristianPeasantArcher,
    ChristianPeasantPriest,
    MuslimPeasantInfantry,
    MuslimPeasantArcher,
    MuslimPeasantPriest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecruitArchetype {
    Infantry,
    Archer,
    Priest,
}

impl RecruitUnitKind {
    pub const fn all_for_faction(faction: PlayerFaction) -> [Self; 3] {
        match faction {
            PlayerFaction::Christian => [
                Self::ChristianPeasantInfantry,
                Self::ChristianPeasantArcher,
                Self::ChristianPeasantPriest,
            ],
            PlayerFaction::Muslim => [
                Self::MuslimPeasantInfantry,
                Self::MuslimPeasantArcher,
                Self::MuslimPeasantPriest,
            ],
        }
    }

    pub const fn archetype(self) -> RecruitArchetype {
        match self {
            Self::ChristianPeasantInfantry | Self::MuslimPeasantInfantry => {
                RecruitArchetype::Infantry
            }
            Self::ChristianPeasantArcher | Self::MuslimPeasantArcher => RecruitArchetype::Archer,
            Self::ChristianPeasantPriest | Self::MuslimPeasantPriest => RecruitArchetype::Priest,
        }
    }

    pub const fn faction(self) -> PlayerFaction {
        match self {
            Self::ChristianPeasantInfantry
            | Self::ChristianPeasantArcher
            | Self::ChristianPeasantPriest => PlayerFaction::Christian,
            Self::MuslimPeasantInfantry | Self::MuslimPeasantArcher | Self::MuslimPeasantPriest => {
                PlayerFaction::Muslim
            }
        }
    }

    pub const fn from_faction_and_archetype(
        faction: PlayerFaction,
        archetype: RecruitArchetype,
    ) -> Self {
        match (faction, archetype) {
            (PlayerFaction::Christian, RecruitArchetype::Infantry) => {
                Self::ChristianPeasantInfantry
            }
            (PlayerFaction::Christian, RecruitArchetype::Archer) => Self::ChristianPeasantArcher,
            (PlayerFaction::Christian, RecruitArchetype::Priest) => Self::ChristianPeasantPriest,
            (PlayerFaction::Muslim, RecruitArchetype::Infantry) => Self::MuslimPeasantInfantry,
            (PlayerFaction::Muslim, RecruitArchetype::Archer) => Self::MuslimPeasantArcher,
            (PlayerFaction::Muslim, RecruitArchetype::Priest) => Self::MuslimPeasantPriest,
        }
    }

    pub const fn as_unit_kind(self) -> UnitKind {
        match self {
            Self::ChristianPeasantInfantry => UnitKind::ChristianPeasantInfantry,
            Self::ChristianPeasantArcher => UnitKind::ChristianPeasantArcher,
            Self::ChristianPeasantPriest => UnitKind::ChristianPeasantPriest,
            Self::MuslimPeasantInfantry => UnitKind::MuslimPeasantInfantry,
            Self::MuslimPeasantArcher => UnitKind::MuslimPeasantArcher,
            Self::MuslimPeasantPriest => UnitKind::MuslimPeasantPriest,
        }
    }

    pub const fn as_rescuable_unit_kind(self) -> UnitKind {
        match self {
            Self::ChristianPeasantInfantry => UnitKind::RescuableChristianPeasantInfantry,
            Self::ChristianPeasantArcher => UnitKind::RescuableChristianPeasantArcher,
            Self::ChristianPeasantPriest => UnitKind::RescuableChristianPeasantPriest,
            Self::MuslimPeasantInfantry => UnitKind::RescuableMuslimPeasantInfantry,
            Self::MuslimPeasantArcher => UnitKind::RescuableMuslimPeasantArcher,
            Self::MuslimPeasantPriest => UnitKind::RescuableMuslimPeasantPriest,
        }
    }
}

impl UnitKind {
    pub const fn faction(self) -> Option<PlayerFaction> {
        match self {
            Self::ChristianPeasantInfantry
            | Self::ChristianPeasantArcher
            | Self::ChristianPeasantPriest
            | Self::ChristianMenAtArms
            | Self::ChristianBowman
            | Self::ChristianDevoted
            | Self::ChristianShieldInfantry
            | Self::ChristianSpearman
            | Self::ChristianUnmountedKnight
            | Self::ChristianSquire
            | Self::ChristianExperiencedBowman
            | Self::ChristianCrossbowman
            | Self::ChristianTracker
            | Self::ChristianScout
            | Self::ChristianDevotedOne
            | Self::ChristianFanatic
            | Self::ChristianExperiencedShieldInfantry
            | Self::ChristianShieldedSpearman
            | Self::ChristianKnight
            | Self::ChristianBannerman
            | Self::ChristianEliteBowman
            | Self::ChristianArmoredCrossbowman
            | Self::ChristianPathfinder
            | Self::ChristianMountedScout
            | Self::ChristianCardinal
            | Self::ChristianFlagellant
            | Self::ChristianEliteShieldInfantry
            | Self::ChristianHalberdier
            | Self::ChristianHeavyKnight
            | Self::ChristianEliteBannerman
            | Self::ChristianLongbowman
            | Self::ChristianEliteCrossbowman
            | Self::ChristianHoundmaster
            | Self::ChristianShockCavalry
            | Self::ChristianEliteCardinal
            | Self::ChristianEliteFlagellant
            | Self::ChristianCitadelGuard
            | Self::ChristianArmoredHalberdier
            | Self::ChristianEliteHeavyKnight
            | Self::ChristianGodsChosen
            | Self::ChristianEliteLongbowman
            | Self::ChristianSiegeCrossbowman
            | Self::ChristianEliteHoundmaster
            | Self::ChristianEliteShockCavalry
            | Self::ChristianDivineSpeaker
            | Self::ChristianDivineJudge
            | Self::RescuableChristianPeasantInfantry
            | Self::RescuableChristianPeasantArcher
            | Self::RescuableChristianPeasantPriest => Some(PlayerFaction::Christian),
            Self::MuslimPeasantInfantry
            | Self::MuslimPeasantArcher
            | Self::MuslimPeasantPriest
            | Self::MuslimMenAtArms
            | Self::MuslimBowman
            | Self::MuslimDevoted
            | Self::MuslimShieldInfantry
            | Self::MuslimSpearman
            | Self::MuslimUnmountedKnight
            | Self::MuslimSquire
            | Self::MuslimExperiencedBowman
            | Self::MuslimCrossbowman
            | Self::MuslimTracker
            | Self::MuslimScout
            | Self::MuslimDevotedOne
            | Self::MuslimFanatic
            | Self::MuslimExperiencedShieldInfantry
            | Self::MuslimShieldedSpearman
            | Self::MuslimKnight
            | Self::MuslimBannerman
            | Self::MuslimEliteBowman
            | Self::MuslimArmoredCrossbowman
            | Self::MuslimPathfinder
            | Self::MuslimMountedScout
            | Self::MuslimCardinal
            | Self::MuslimFlagellant
            | Self::MuslimEliteShieldInfantry
            | Self::MuslimHalberdier
            | Self::MuslimHeavyKnight
            | Self::MuslimEliteBannerman
            | Self::MuslimLongbowman
            | Self::MuslimEliteCrossbowman
            | Self::MuslimHoundmaster
            | Self::MuslimShockCavalry
            | Self::MuslimEliteCardinal
            | Self::MuslimEliteFlagellant
            | Self::MuslimCitadelGuard
            | Self::MuslimArmoredHalberdier
            | Self::MuslimEliteHeavyKnight
            | Self::MuslimGodsChosen
            | Self::MuslimEliteLongbowman
            | Self::MuslimSiegeCrossbowman
            | Self::MuslimEliteHoundmaster
            | Self::MuslimEliteShockCavalry
            | Self::MuslimDivineSpeaker
            | Self::MuslimDivineJudge
            | Self::RescuableMuslimPeasantInfantry
            | Self::RescuableMuslimPeasantArcher
            | Self::RescuableMuslimPeasantPriest => Some(PlayerFaction::Muslim),
            Self::Commander => None,
        }
    }

    pub const fn is_friendly_recruit(self) -> bool {
        matches!(
            self,
            Self::ChristianPeasantInfantry
                | Self::ChristianPeasantArcher
                | Self::ChristianPeasantPriest
                | Self::ChristianMenAtArms
                | Self::ChristianBowman
                | Self::ChristianDevoted
                | Self::ChristianShieldInfantry
                | Self::ChristianSpearman
                | Self::ChristianUnmountedKnight
                | Self::ChristianSquire
                | Self::ChristianExperiencedBowman
                | Self::ChristianCrossbowman
                | Self::ChristianTracker
                | Self::ChristianScout
                | Self::ChristianDevotedOne
                | Self::ChristianFanatic
                | Self::ChristianExperiencedShieldInfantry
                | Self::ChristianShieldedSpearman
                | Self::ChristianKnight
                | Self::ChristianBannerman
                | Self::ChristianEliteBowman
                | Self::ChristianArmoredCrossbowman
                | Self::ChristianPathfinder
                | Self::ChristianMountedScout
                | Self::ChristianCardinal
                | Self::ChristianFlagellant
                | Self::ChristianEliteShieldInfantry
                | Self::ChristianHalberdier
                | Self::ChristianHeavyKnight
                | Self::ChristianEliteBannerman
                | Self::ChristianLongbowman
                | Self::ChristianEliteCrossbowman
                | Self::ChristianHoundmaster
                | Self::ChristianShockCavalry
                | Self::ChristianEliteCardinal
                | Self::ChristianEliteFlagellant
                | Self::ChristianCitadelGuard
                | Self::ChristianArmoredHalberdier
                | Self::ChristianEliteHeavyKnight
                | Self::ChristianGodsChosen
                | Self::ChristianEliteLongbowman
                | Self::ChristianSiegeCrossbowman
                | Self::ChristianEliteHoundmaster
                | Self::ChristianEliteShockCavalry
                | Self::ChristianDivineSpeaker
                | Self::ChristianDivineJudge
                | Self::MuslimPeasantInfantry
                | Self::MuslimPeasantArcher
                | Self::MuslimPeasantPriest
                | Self::MuslimMenAtArms
                | Self::MuslimBowman
                | Self::MuslimDevoted
                | Self::MuslimShieldInfantry
                | Self::MuslimSpearman
                | Self::MuslimUnmountedKnight
                | Self::MuslimSquire
                | Self::MuslimExperiencedBowman
                | Self::MuslimCrossbowman
                | Self::MuslimTracker
                | Self::MuslimScout
                | Self::MuslimDevotedOne
                | Self::MuslimFanatic
                | Self::MuslimExperiencedShieldInfantry
                | Self::MuslimShieldedSpearman
                | Self::MuslimKnight
                | Self::MuslimBannerman
                | Self::MuslimEliteBowman
                | Self::MuslimArmoredCrossbowman
                | Self::MuslimPathfinder
                | Self::MuslimMountedScout
                | Self::MuslimCardinal
                | Self::MuslimFlagellant
                | Self::MuslimEliteShieldInfantry
                | Self::MuslimHalberdier
                | Self::MuslimHeavyKnight
                | Self::MuslimEliteBannerman
                | Self::MuslimLongbowman
                | Self::MuslimEliteCrossbowman
                | Self::MuslimHoundmaster
                | Self::MuslimShockCavalry
                | Self::MuslimEliteCardinal
                | Self::MuslimEliteFlagellant
                | Self::MuslimCitadelGuard
                | Self::MuslimArmoredHalberdier
                | Self::MuslimEliteHeavyKnight
                | Self::MuslimGodsChosen
                | Self::MuslimEliteLongbowman
                | Self::MuslimSiegeCrossbowman
                | Self::MuslimEliteHoundmaster
                | Self::MuslimEliteShockCavalry
                | Self::MuslimDivineSpeaker
                | Self::MuslimDivineJudge
        )
    }

    pub const fn is_priest(self) -> bool {
        matches!(
            self,
            Self::ChristianPeasantPriest
                | Self::MuslimPeasantPriest
                | Self::ChristianDevoted
                | Self::MuslimDevoted
                | Self::ChristianSquire
                | Self::MuslimSquire
                | Self::ChristianDevotedOne
                | Self::MuslimDevotedOne
                | Self::ChristianBannerman
                | Self::MuslimBannerman
                | Self::ChristianCardinal
                | Self::MuslimCardinal
                | Self::ChristianEliteBannerman
                | Self::MuslimEliteBannerman
                | Self::ChristianEliteCardinal
                | Self::MuslimEliteCardinal
                | Self::ChristianGodsChosen
                | Self::MuslimGodsChosen
                | Self::ChristianDivineSpeaker
                | Self::MuslimDivineSpeaker
        )
    }

    pub const fn as_recruit_unit_kind(self) -> Option<RecruitUnitKind> {
        match self {
            Self::ChristianPeasantInfantry => Some(RecruitUnitKind::ChristianPeasantInfantry),
            Self::ChristianPeasantArcher => Some(RecruitUnitKind::ChristianPeasantArcher),
            Self::ChristianPeasantPriest => Some(RecruitUnitKind::ChristianPeasantPriest),
            Self::MuslimPeasantInfantry => Some(RecruitUnitKind::MuslimPeasantInfantry),
            Self::MuslimPeasantArcher => Some(RecruitUnitKind::MuslimPeasantArcher),
            Self::MuslimPeasantPriest => Some(RecruitUnitKind::MuslimPeasantPriest),
            _ => None,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Unit {
    pub team: Team,
    pub kind: UnitKind,
    pub level: u32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct UnitTier(pub u8);

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
pub struct RescuableUnit {
    pub recruit_kind: RecruitUnitKind,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct CommanderUnit;

#[derive(Resource, Clone, Debug)]
pub struct GlobalBuffs {
    pub damage_multiplier: f32,
    pub armor_bonus: f32,
    pub attack_speed_multiplier: f32,
    pub gold_gain_multiplier: f32,
    pub crit_chance_bonus: f32,
    pub crit_damage_multiplier: f32,
    pub pickup_radius_bonus: f32,
    pub move_speed_bonus: f32,
    pub inside_formation_damage_multiplier: f32,
    pub commander_aura_radius_bonus: f32,
    pub authority_friendly_loss_resistance: f32,
    pub authority_enemy_morale_drain_per_sec: f32,
    pub hospitalier_hp_regen_per_sec: f32,
    pub hospitalier_morale_regen_per_sec: f32,
}

impl Default for GlobalBuffs {
    fn default() -> Self {
        Self {
            damage_multiplier: 1.0,
            armor_bonus: 0.0,
            attack_speed_multiplier: 1.0,
            gold_gain_multiplier: 1.0,
            crit_chance_bonus: 0.0,
            crit_damage_multiplier: 1.2,
            pickup_radius_bonus: 0.0,
            move_speed_bonus: 0.0,
            inside_formation_damage_multiplier: 1.0,
            commander_aura_radius_bonus: 0.0,
            authority_friendly_loss_resistance: 0.0,
            authority_enemy_morale_drain_per_sec: 0.0,
            hospitalier_hp_regen_per_sec: 0.0,
            hospitalier_morale_regen_per_sec: 0.0,
        }
    }
}

#[derive(Event, Clone, Copy, Debug, Default)]
pub struct StartRunEvent;

#[derive(Event, Clone, Copy, Debug)]
pub struct RecruitEvent {
    pub world_position: Vec2,
    pub recruit_kind: RecruitUnitKind,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct DamageEvent {
    pub target: Entity,
    pub source_team: Team,
    pub source_entity: Option<Entity>,
    pub amount: f32,
    pub execute: bool,
    pub critical: bool,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct UnitDamagedEvent {
    pub target: Entity,
    pub team: Team,
    pub amount: f32,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct DamageTextEvent {
    pub world_position: Vec2,
    pub target_team: Team,
    pub amount: f32,
    pub execute: bool,
    pub critical: bool,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct UnitDiedEvent {
    pub team: Team,
    pub kind: UnitKind,
    pub max_health: f32,
    pub world_position: Vec2,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct GainGoldEvent(pub f32);

#[derive(Event, Clone, Copy, Debug)]
pub struct GainHearTheCallTokenEvent(pub u32);

#[derive(Event, Clone, Copy, Debug)]
pub struct SpawnGoldPackEvent {
    pub world_position: Vec2,
    pub gold_value_override: Option<f32>,
    pub pickup_delay_secs: Option<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum RunModalScreen {
    Inventory,
    Chest,
    Stats,
    SkillBook,
    Archive,
    UnitUpgrade,
}

#[derive(Resource, Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum RunModalState {
    #[default]
    None,
    Open(RunModalScreen),
}

impl RunModalState {
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open(_))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunModalAction {
    Open(RunModalScreen),
    Toggle(RunModalScreen),
    Close,
}

#[derive(Event, Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunModalRequestEvent {
    pub action: RunModalAction,
}

pub const MAX_COMMANDER_LEVEL: u32 = 100;

pub fn level_cap_from_locked_budget(locked_levels: u32) -> u32 {
    MAX_COMMANDER_LEVEL.saturating_sub(locked_levels)
}

#[cfg(test)]
mod tests {
    use super::{RecruitUnitKind, UnitKind};

    #[test]
    fn recruit_unit_kind_maps_from_friendly_unit_kind() {
        assert_eq!(
            UnitKind::ChristianPeasantInfantry.as_recruit_unit_kind(),
            Some(RecruitUnitKind::ChristianPeasantInfantry)
        );
        assert_eq!(
            UnitKind::MuslimPeasantArcher.as_recruit_unit_kind(),
            Some(RecruitUnitKind::MuslimPeasantArcher)
        );
        assert_eq!(UnitKind::Commander.as_recruit_unit_kind(), None);
        assert_eq!(
            UnitKind::RescuableChristianPeasantPriest.as_recruit_unit_kind(),
            None
        );
    }
}
