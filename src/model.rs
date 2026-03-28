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

    pub const fn all() -> [Self; 2] {
        [Self::Christian, Self::Muslim]
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
pub enum EnemySpawnLane {
    Small,
    Minor,
    Major,
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnemySpawnSource {
    pub lane: EnemySpawnLane,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct UnitRef {
    pub faction: Option<PlayerFaction>,
    pub unit_id: &'static str,
    pub rescuable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct HeroRef {
    pub faction: PlayerFaction,
    pub hero_id: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ItemRef {
    pub faction: Option<PlayerFaction>,
    pub item_id: &'static str,
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

    pub const fn unit_id(self) -> &'static str {
        self.as_unit_kind().unit_id()
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
    pub const fn unit_id(self) -> &'static str {
        match self {
            Self::Commander => "commander",
            Self::ChristianPeasantInfantry
            | Self::MuslimPeasantInfantry
            | Self::RescuableChristianPeasantInfantry
            | Self::RescuableMuslimPeasantInfantry => "peasant_infantry",
            Self::ChristianPeasantArcher
            | Self::MuslimPeasantArcher
            | Self::RescuableChristianPeasantArcher
            | Self::RescuableMuslimPeasantArcher => "peasant_archer",
            Self::ChristianPeasantPriest
            | Self::MuslimPeasantPriest
            | Self::RescuableChristianPeasantPriest
            | Self::RescuableMuslimPeasantPriest => "peasant_priest",
            Self::ChristianMenAtArms | Self::MuslimMenAtArms => "men_at_arms",
            Self::ChristianBowman | Self::MuslimBowman => "bowman",
            Self::ChristianDevoted | Self::MuslimDevoted => "devoted",
            Self::ChristianShieldInfantry | Self::MuslimShieldInfantry => "shield_infantry",
            Self::ChristianSpearman | Self::MuslimSpearman => "spearman",
            Self::ChristianUnmountedKnight | Self::MuslimUnmountedKnight => "unmounted_knight",
            Self::ChristianSquire | Self::MuslimSquire => "squire",
            Self::ChristianExperiencedBowman | Self::MuslimExperiencedBowman => {
                "experienced_bowman"
            }
            Self::ChristianCrossbowman | Self::MuslimCrossbowman => "crossbowman",
            Self::ChristianTracker | Self::MuslimTracker => "tracker",
            Self::ChristianScout | Self::MuslimScout => "scout",
            Self::ChristianDevotedOne | Self::MuslimDevotedOne => "devoted_one",
            Self::ChristianFanatic | Self::MuslimFanatic => "fanatic",
            Self::ChristianExperiencedShieldInfantry | Self::MuslimExperiencedShieldInfantry => {
                "experienced_shield_infantry"
            }
            Self::ChristianShieldedSpearman | Self::MuslimShieldedSpearman => "shielded_spearman",
            Self::ChristianKnight | Self::MuslimKnight => "knight",
            Self::ChristianBannerman | Self::MuslimBannerman => "bannerman",
            Self::ChristianEliteBowman | Self::MuslimEliteBowman => "elite_bowman",
            Self::ChristianArmoredCrossbowman | Self::MuslimArmoredCrossbowman => {
                "armored_crossbowman"
            }
            Self::ChristianPathfinder | Self::MuslimPathfinder => "pathfinder",
            Self::ChristianMountedScout | Self::MuslimMountedScout => "mounted_scout",
            Self::ChristianCardinal | Self::MuslimCardinal => "cardinal",
            Self::ChristianFlagellant | Self::MuslimFlagellant => "flagellant",
            Self::ChristianEliteShieldInfantry | Self::MuslimEliteShieldInfantry => {
                "elite_shield_infantry"
            }
            Self::ChristianHalberdier | Self::MuslimHalberdier => "halberdier",
            Self::ChristianHeavyKnight | Self::MuslimHeavyKnight => "heavy_knight",
            Self::ChristianEliteBannerman | Self::MuslimEliteBannerman => "elite_bannerman",
            Self::ChristianLongbowman | Self::MuslimLongbowman => "longbowman",
            Self::ChristianEliteCrossbowman | Self::MuslimEliteCrossbowman => "elite_crossbowman",
            Self::ChristianHoundmaster | Self::MuslimHoundmaster => "houndmaster",
            Self::ChristianShockCavalry | Self::MuslimShockCavalry => "shock_cavalry",
            Self::ChristianEliteCardinal | Self::MuslimEliteCardinal => "elite_cardinal",
            Self::ChristianEliteFlagellant | Self::MuslimEliteFlagellant => "elite_flagellant",
            Self::ChristianCitadelGuard | Self::MuslimCitadelGuard => "citadel_guard",
            Self::ChristianArmoredHalberdier | Self::MuslimArmoredHalberdier => {
                "armored_halberdier"
            }
            Self::ChristianEliteHeavyKnight | Self::MuslimEliteHeavyKnight => "elite_heavy_knight",
            Self::ChristianGodsChosen | Self::MuslimGodsChosen => "gods_chosen",
            Self::ChristianEliteLongbowman | Self::MuslimEliteLongbowman => "elite_longbowman",
            Self::ChristianSiegeCrossbowman | Self::MuslimSiegeCrossbowman => "siege_crossbowman",
            Self::ChristianEliteHoundmaster | Self::MuslimEliteHoundmaster => "elite_houndmaster",
            Self::ChristianEliteShockCavalry | Self::MuslimEliteShockCavalry => {
                "elite_shock_cavalry"
            }
            Self::ChristianDivineSpeaker | Self::MuslimDivineSpeaker => "divine_speaker",
            Self::ChristianDivineJudge | Self::MuslimDivineJudge => "divine_judge",
        }
    }

    pub fn from_faction_and_unit_id(
        faction: PlayerFaction,
        unit_id: &str,
        rescuable: bool,
    ) -> Option<Self> {
        if rescuable {
            return match (faction, unit_id) {
                (PlayerFaction::Christian, "peasant_infantry") => {
                    Some(Self::RescuableChristianPeasantInfantry)
                }
                (PlayerFaction::Christian, "peasant_archer") => {
                    Some(Self::RescuableChristianPeasantArcher)
                }
                (PlayerFaction::Christian, "peasant_priest") => {
                    Some(Self::RescuableChristianPeasantPriest)
                }
                (PlayerFaction::Muslim, "peasant_infantry") => {
                    Some(Self::RescuableMuslimPeasantInfantry)
                }
                (PlayerFaction::Muslim, "peasant_archer") => {
                    Some(Self::RescuableMuslimPeasantArcher)
                }
                (PlayerFaction::Muslim, "peasant_priest") => {
                    Some(Self::RescuableMuslimPeasantPriest)
                }
                _ => None,
            };
        }

        match (faction, unit_id) {
            (PlayerFaction::Christian, "peasant_infantry") => Some(Self::ChristianPeasantInfantry),
            (PlayerFaction::Christian, "peasant_archer") => Some(Self::ChristianPeasantArcher),
            (PlayerFaction::Christian, "peasant_priest") => Some(Self::ChristianPeasantPriest),
            (PlayerFaction::Christian, "men_at_arms") => Some(Self::ChristianMenAtArms),
            (PlayerFaction::Christian, "bowman") => Some(Self::ChristianBowman),
            (PlayerFaction::Christian, "devoted") => Some(Self::ChristianDevoted),
            (PlayerFaction::Christian, "shield_infantry") => Some(Self::ChristianShieldInfantry),
            (PlayerFaction::Christian, "spearman") => Some(Self::ChristianSpearman),
            (PlayerFaction::Christian, "unmounted_knight") => Some(Self::ChristianUnmountedKnight),
            (PlayerFaction::Christian, "squire") => Some(Self::ChristianSquire),
            (PlayerFaction::Christian, "experienced_bowman") => {
                Some(Self::ChristianExperiencedBowman)
            }
            (PlayerFaction::Christian, "crossbowman") => Some(Self::ChristianCrossbowman),
            (PlayerFaction::Christian, "tracker") => Some(Self::ChristianTracker),
            (PlayerFaction::Christian, "scout") => Some(Self::ChristianScout),
            (PlayerFaction::Christian, "devoted_one") => Some(Self::ChristianDevotedOne),
            (PlayerFaction::Christian, "fanatic") => Some(Self::ChristianFanatic),
            (PlayerFaction::Christian, "experienced_shield_infantry") => {
                Some(Self::ChristianExperiencedShieldInfantry)
            }
            (PlayerFaction::Christian, "shielded_spearman") => {
                Some(Self::ChristianShieldedSpearman)
            }
            (PlayerFaction::Christian, "knight") => Some(Self::ChristianKnight),
            (PlayerFaction::Christian, "bannerman") => Some(Self::ChristianBannerman),
            (PlayerFaction::Christian, "elite_bowman") => Some(Self::ChristianEliteBowman),
            (PlayerFaction::Christian, "armored_crossbowman") => {
                Some(Self::ChristianArmoredCrossbowman)
            }
            (PlayerFaction::Christian, "pathfinder") => Some(Self::ChristianPathfinder),
            (PlayerFaction::Christian, "mounted_scout") => Some(Self::ChristianMountedScout),
            (PlayerFaction::Christian, "cardinal") => Some(Self::ChristianCardinal),
            (PlayerFaction::Christian, "flagellant") => Some(Self::ChristianFlagellant),
            (PlayerFaction::Christian, "elite_shield_infantry") => {
                Some(Self::ChristianEliteShieldInfantry)
            }
            (PlayerFaction::Christian, "halberdier") => Some(Self::ChristianHalberdier),
            (PlayerFaction::Christian, "heavy_knight") => Some(Self::ChristianHeavyKnight),
            (PlayerFaction::Christian, "elite_bannerman") => Some(Self::ChristianEliteBannerman),
            (PlayerFaction::Christian, "longbowman") => Some(Self::ChristianLongbowman),
            (PlayerFaction::Christian, "elite_crossbowman") => {
                Some(Self::ChristianEliteCrossbowman)
            }
            (PlayerFaction::Christian, "houndmaster") => Some(Self::ChristianHoundmaster),
            (PlayerFaction::Christian, "shock_cavalry") => Some(Self::ChristianShockCavalry),
            (PlayerFaction::Christian, "elite_cardinal") => Some(Self::ChristianEliteCardinal),
            (PlayerFaction::Christian, "elite_flagellant") => Some(Self::ChristianEliteFlagellant),
            (PlayerFaction::Christian, "citadel_guard") => Some(Self::ChristianCitadelGuard),
            (PlayerFaction::Christian, "armored_halberdier") => {
                Some(Self::ChristianArmoredHalberdier)
            }
            (PlayerFaction::Christian, "elite_heavy_knight") => {
                Some(Self::ChristianEliteHeavyKnight)
            }
            (PlayerFaction::Christian, "gods_chosen") => Some(Self::ChristianGodsChosen),
            (PlayerFaction::Christian, "elite_longbowman") => Some(Self::ChristianEliteLongbowman),
            (PlayerFaction::Christian, "siege_crossbowman") => {
                Some(Self::ChristianSiegeCrossbowman)
            }
            (PlayerFaction::Christian, "elite_houndmaster") => {
                Some(Self::ChristianEliteHoundmaster)
            }
            (PlayerFaction::Christian, "elite_shock_cavalry") => {
                Some(Self::ChristianEliteShockCavalry)
            }
            (PlayerFaction::Christian, "divine_speaker") => Some(Self::ChristianDivineSpeaker),
            (PlayerFaction::Christian, "divine_judge") => Some(Self::ChristianDivineJudge),
            (PlayerFaction::Muslim, "peasant_infantry") => Some(Self::MuslimPeasantInfantry),
            (PlayerFaction::Muslim, "peasant_archer") => Some(Self::MuslimPeasantArcher),
            (PlayerFaction::Muslim, "peasant_priest") => Some(Self::MuslimPeasantPriest),
            (PlayerFaction::Muslim, "men_at_arms") => Some(Self::MuslimMenAtArms),
            (PlayerFaction::Muslim, "bowman") => Some(Self::MuslimBowman),
            (PlayerFaction::Muslim, "devoted") => Some(Self::MuslimDevoted),
            (PlayerFaction::Muslim, "shield_infantry") => Some(Self::MuslimShieldInfantry),
            (PlayerFaction::Muslim, "spearman") => Some(Self::MuslimSpearman),
            (PlayerFaction::Muslim, "unmounted_knight") => Some(Self::MuslimUnmountedKnight),
            (PlayerFaction::Muslim, "squire") => Some(Self::MuslimSquire),
            (PlayerFaction::Muslim, "experienced_bowman") => Some(Self::MuslimExperiencedBowman),
            (PlayerFaction::Muslim, "crossbowman") => Some(Self::MuslimCrossbowman),
            (PlayerFaction::Muslim, "tracker") => Some(Self::MuslimTracker),
            (PlayerFaction::Muslim, "scout") => Some(Self::MuslimScout),
            (PlayerFaction::Muslim, "devoted_one") => Some(Self::MuslimDevotedOne),
            (PlayerFaction::Muslim, "fanatic") => Some(Self::MuslimFanatic),
            (PlayerFaction::Muslim, "experienced_shield_infantry") => {
                Some(Self::MuslimExperiencedShieldInfantry)
            }
            (PlayerFaction::Muslim, "shielded_spearman") => Some(Self::MuslimShieldedSpearman),
            (PlayerFaction::Muslim, "knight") => Some(Self::MuslimKnight),
            (PlayerFaction::Muslim, "bannerman") => Some(Self::MuslimBannerman),
            (PlayerFaction::Muslim, "elite_bowman") => Some(Self::MuslimEliteBowman),
            (PlayerFaction::Muslim, "armored_crossbowman") => Some(Self::MuslimArmoredCrossbowman),
            (PlayerFaction::Muslim, "pathfinder") => Some(Self::MuslimPathfinder),
            (PlayerFaction::Muslim, "mounted_scout") => Some(Self::MuslimMountedScout),
            (PlayerFaction::Muslim, "cardinal") => Some(Self::MuslimCardinal),
            (PlayerFaction::Muslim, "flagellant") => Some(Self::MuslimFlagellant),
            (PlayerFaction::Muslim, "elite_shield_infantry") => {
                Some(Self::MuslimEliteShieldInfantry)
            }
            (PlayerFaction::Muslim, "halberdier") => Some(Self::MuslimHalberdier),
            (PlayerFaction::Muslim, "heavy_knight") => Some(Self::MuslimHeavyKnight),
            (PlayerFaction::Muslim, "elite_bannerman") => Some(Self::MuslimEliteBannerman),
            (PlayerFaction::Muslim, "longbowman") => Some(Self::MuslimLongbowman),
            (PlayerFaction::Muslim, "elite_crossbowman") => Some(Self::MuslimEliteCrossbowman),
            (PlayerFaction::Muslim, "houndmaster") => Some(Self::MuslimHoundmaster),
            (PlayerFaction::Muslim, "shock_cavalry") => Some(Self::MuslimShockCavalry),
            (PlayerFaction::Muslim, "elite_cardinal") => Some(Self::MuslimEliteCardinal),
            (PlayerFaction::Muslim, "elite_flagellant") => Some(Self::MuslimEliteFlagellant),
            (PlayerFaction::Muslim, "citadel_guard") => Some(Self::MuslimCitadelGuard),
            (PlayerFaction::Muslim, "armored_halberdier") => Some(Self::MuslimArmoredHalberdier),
            (PlayerFaction::Muslim, "elite_heavy_knight") => Some(Self::MuslimEliteHeavyKnight),
            (PlayerFaction::Muslim, "gods_chosen") => Some(Self::MuslimGodsChosen),
            (PlayerFaction::Muslim, "elite_longbowman") => Some(Self::MuslimEliteLongbowman),
            (PlayerFaction::Muslim, "siege_crossbowman") => Some(Self::MuslimSiegeCrossbowman),
            (PlayerFaction::Muslim, "elite_houndmaster") => Some(Self::MuslimEliteHoundmaster),
            (PlayerFaction::Muslim, "elite_shock_cavalry") => Some(Self::MuslimEliteShockCavalry),
            (PlayerFaction::Muslim, "divine_speaker") => Some(Self::MuslimDivineSpeaker),
            (PlayerFaction::Muslim, "divine_judge") => Some(Self::MuslimDivineJudge),
            _ => None,
        }
    }

    pub const fn is_rescuable_variant(self) -> bool {
        matches!(
            self,
            Self::RescuableChristianPeasantInfantry
                | Self::RescuableChristianPeasantArcher
                | Self::RescuableChristianPeasantPriest
                | Self::RescuableMuslimPeasantInfantry
                | Self::RescuableMuslimPeasantArcher
                | Self::RescuableMuslimPeasantPriest
        )
    }

    pub const fn as_unit_ref(self) -> UnitRef {
        UnitRef {
            faction: self.faction(),
            unit_id: self.unit_id(),
            rescuable: self.is_rescuable_variant(),
        }
    }

    pub const fn recruit_archetype(self) -> Option<RecruitArchetype> {
        match self {
            Self::ChristianPeasantInfantry
            | Self::MuslimPeasantInfantry
            | Self::RescuableChristianPeasantInfantry
            | Self::RescuableMuslimPeasantInfantry => Some(RecruitArchetype::Infantry),
            Self::ChristianPeasantArcher
            | Self::MuslimPeasantArcher
            | Self::RescuableChristianPeasantArcher
            | Self::RescuableMuslimPeasantArcher => Some(RecruitArchetype::Archer),
            Self::ChristianPeasantPriest
            | Self::MuslimPeasantPriest
            | Self::RescuableChristianPeasantPriest
            | Self::RescuableMuslimPeasantPriest => Some(RecruitArchetype::Priest),
            _ => None,
        }
    }

    pub fn is_tracker_line(self) -> bool {
        matches!(
            self.unit_id(),
            "tracker" | "pathfinder" | "houndmaster" | "elite_houndmaster"
        )
    }

    pub fn is_scout_line(self) -> bool {
        matches!(
            self.unit_id(),
            "scout" | "mounted_scout" | "shock_cavalry" | "elite_shock_cavalry"
        )
    }

    pub fn is_fanatic_line(self) -> bool {
        matches!(
            self.unit_id(),
            "fanatic" | "flagellant" | "elite_flagellant" | "divine_judge"
        )
    }

    pub fn is_archer_line(self) -> bool {
        matches!(
            self.unit_id(),
            "peasant_archer"
                | "bowman"
                | "experienced_bowman"
                | "elite_bowman"
                | "longbowman"
                | "elite_longbowman"
                | "crossbowman"
                | "armored_crossbowman"
                | "elite_crossbowman"
                | "siege_crossbowman"
                | "tracker"
                | "pathfinder"
                | "houndmaster"
                | "elite_houndmaster"
                | "scout"
                | "mounted_scout"
                | "shock_cavalry"
                | "elite_shock_cavalry"
        )
    }

    pub fn is_support_priest_line(self) -> bool {
        matches!(
            self.unit_id(),
            "peasant_priest"
                | "devoted"
                | "squire"
                | "bannerman"
                | "elite_bannerman"
                | "gods_chosen"
                | "devoted_one"
                | "cardinal"
                | "elite_cardinal"
                | "divine_speaker"
        )
    }

    pub fn is_priest_family_line(self) -> bool {
        self.is_support_priest_line() || self.is_fanatic_line()
    }

    pub fn is_block_infantry_line(self) -> bool {
        matches!(
            self.unit_id(),
            "peasant_infantry"
                | "men_at_arms"
                | "shield_infantry"
                | "experienced_shield_infantry"
                | "elite_shield_infantry"
                | "spearman"
                | "shielded_spearman"
                | "halberdier"
                | "unmounted_knight"
                | "knight"
                | "heavy_knight"
                | "citadel_guard"
                | "armored_halberdier"
                | "elite_heavy_knight"
        )
    }

    pub fn tier_hint(self) -> Option<u8> {
        if !self.is_friendly_recruit() {
            return None;
        }
        match self.unit_id() {
            "peasant_infantry" | "peasant_archer" | "peasant_priest" => Some(0),
            "men_at_arms" | "bowman" | "devoted" => Some(1),
            "shield_infantry" | "spearman" | "unmounted_knight" | "squire"
            | "experienced_bowman" | "crossbowman" | "tracker" | "scout" | "devoted_one"
            | "fanatic" => Some(2),
            "experienced_shield_infantry"
            | "shielded_spearman"
            | "knight"
            | "bannerman"
            | "elite_bowman"
            | "armored_crossbowman"
            | "pathfinder"
            | "mounted_scout"
            | "cardinal"
            | "flagellant" => Some(3),
            "elite_shield_infantry"
            | "halberdier"
            | "heavy_knight"
            | "elite_bannerman"
            | "longbowman"
            | "elite_crossbowman"
            | "houndmaster"
            | "shock_cavalry"
            | "elite_cardinal"
            | "elite_flagellant" => Some(4),
            "citadel_guard"
            | "armored_halberdier"
            | "elite_heavy_knight"
            | "gods_chosen"
            | "elite_longbowman"
            | "siege_crossbowman"
            | "elite_houndmaster"
            | "elite_shock_cavalry"
            | "divine_speaker"
            | "divine_judge" => Some(5),
            _ => None,
        }
    }

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
    pub enemy_spawn_lane: Option<EnemySpawnLane>,
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
    use super::{PlayerFaction, RecruitUnitKind, UnitKind};

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

    #[test]
    fn unit_ref_contract_is_generic_and_faction_scoped() {
        let christian = UnitKind::ChristianPeasantInfantry.as_unit_ref();
        assert_eq!(christian.unit_id, "peasant_infantry");
        assert_eq!(christian.faction, Some(PlayerFaction::Christian));
        assert!(!christian.rescuable);

        let muslim = UnitKind::MuslimPeasantInfantry.as_unit_ref();
        assert_eq!(muslim.unit_id, "peasant_infantry");
        assert_eq!(muslim.faction, Some(PlayerFaction::Muslim));
        assert!(!muslim.rescuable);

        let rescuable = UnitKind::RescuableMuslimPeasantInfantry.as_unit_ref();
        assert_eq!(rescuable.unit_id, "peasant_infantry");
        assert_eq!(rescuable.faction, Some(PlayerFaction::Muslim));
        assert!(rescuable.rescuable);
    }

    #[test]
    fn unit_identity_helpers_resolve_shared_lineage_tags() {
        assert!(UnitKind::ChristianTracker.is_tracker_line());
        assert!(UnitKind::MuslimTracker.is_tracker_line());
        assert!(UnitKind::ChristianShockCavalry.is_scout_line());
        assert!(UnitKind::MuslimDivineJudge.is_fanatic_line());
        assert!(UnitKind::ChristianDevoted.is_support_priest_line());
        assert!(UnitKind::ChristianDivineJudge.is_priest_family_line());
        assert!(UnitKind::MuslimPeasantInfantry.is_block_infantry_line());
        assert!(UnitKind::ChristianEliteLongbowman.is_archer_line());
        assert_eq!(UnitKind::ChristianBowman.tier_hint(), Some(1));
        assert_eq!(UnitKind::MuslimEliteShockCavalry.tier_hint(), Some(5));
        assert_eq!(
            UnitKind::RescuableChristianPeasantInfantry.tier_hint(),
            None
        );
    }

    #[test]
    fn unit_kind_resolver_supports_generic_unit_ids_with_faction_context() {
        let christian =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "pathfinder", false);
        let muslim = UnitKind::from_faction_and_unit_id(PlayerFaction::Muslim, "pathfinder", false);
        let rescuable =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Muslim, "peasant_archer", true);

        assert_eq!(christian, Some(UnitKind::ChristianPathfinder));
        assert_eq!(muslim, Some(UnitKind::MuslimPathfinder));
        assert_eq!(rescuable, Some(UnitKind::RescuableMuslimPeasantArcher));

        assert!(
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "unknown_unit", false)
                .is_none()
        );
        assert!(
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "tracker", true).is_none()
        );
    }
}
