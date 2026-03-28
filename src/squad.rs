use bevy::prelude::*;
use bevy::sprite::{ColorMaterial, MaterialMesh2dBundle, Mesh2dHandle};
use bevy::{math::primitives::Circle, render::mesh::Mesh};
use std::collections::HashMap;

use crate::banner::{BannerMovementPenalty, BannerState};
use crate::combat::{RangedAttackCooldown, RangedAttackProfile};
use crate::data::{FactionGameplayConfig, GameData, RosterBehaviorConfig, UnitStatsConfig};
use crate::enemies::{WaveCompletedEvent, is_major_army_wave};
use crate::formation::{
    ActiveFormation, FormationModifiers, active_formation_config, formation_contains_position,
};
use crate::inventory::{
    InventoryState, gear_bonuses_for_unit_with_banner_state, scaled_skill_cooldown,
    scaled_skill_duration,
};
use crate::map::{MapBounds, playable_bounds};
use crate::model::{
    Armor, AttackCooldown, AttackProfile, BaseMaxHealth, ColliderRadius, CommanderUnit,
    DamageEvent, EnemyUnit, FriendlyUnit, GameState, GlobalBuffs, Health, MatchSetupSelection,
    Morale, MoveSpeed, PlayerControlled, PlayerFaction, RecruitEvent, RecruitUnitKind,
    RescuableUnit, StartRunEvent, Team, Unit, UnitDiedEvent, UnitKind, UnitTier,
    level_cap_from_locked_budget,
};
use crate::morale::morale_movement_multiplier;
use crate::upgrades::{ConditionalUpgradeEffects, Progression, SkillTimingBuffs};
use crate::visuals::ArtAssets;

const ENEMY_INSIDE_FORMATION_SLOWDOWN_PER_UNIT: f32 = 0.04;
const ENEMY_INSIDE_FORMATION_MIN_SPEED_MULTIPLIER: f32 = 0.5;
const ENEMY_INSIDE_FORMATION_PADDING_SLOTS: f32 = 0.35;
const PRIEST_BLESSING_RANGE: f32 = 190.0;
const PRIEST_BLESSING_DURATION_SECS: f32 = 10.0;
const PRIEST_BLESSING_COOLDOWN_SECS: f32 = 20.0;
const PRIEST_BLESSING_ATTACK_SPEED_MULTIPLIER: f32 = 1.24;
const PRIEST_BLESSING_VFX_Y_OFFSET: f32 = -8.0;
const PRIEST_BLESSING_VFX_Z_OFFSET: f32 = -0.25;
const PRIEST_BLESSING_VFX_ALPHA: f32 = 0.28;
const PRIEST_BLESSING_VFX_SCALE_X: f32 = 1.75;
const PRIEST_BLESSING_VFX_SCALE_Y: f32 = 0.92;
const PRIEST_BLESSING_VFX_MIN_RADIUS: f32 = 8.0;
const TIER0_CONVERSION_GOLD_COST_PER_UNIT: f32 = 18.0;
const PROMOTION_GOLD_COST_PER_STEP: f32 = 65.0;
const PROMOTION_GOLD_COST_PER_TIER: f32 = 24.0;
pub const HERO_TIER_UNLOCK_MAJOR_WAVE: u32 = 60;

#[derive(Resource, Clone, Debug, Default)]
pub struct SquadRoster {
    pub commander: Option<Entity>,
    pub friendly_count: usize,
    pub casualties: u32,
}

#[derive(Resource, Clone, Debug, Eq, PartialEq)]
pub struct RosterEconomy {
    pub locked_levels: u32,
    pub allowed_max_level: u32,
    pub tier0_retinue_count: u32,
    pub total_retinue_count: u32,
    pub infantry_count: u32,
    pub archer_count: u32,
    pub priest_count: u32,
    pub kind_counts: HashMap<UnitKind, u32>,
}

impl Default for RosterEconomy {
    fn default() -> Self {
        Self {
            locked_levels: 0,
            allowed_max_level: level_cap_from_locked_budget(0),
            tier0_retinue_count: 0,
            total_retinue_count: 0,
            infantry_count: 0,
            archer_count: 0,
            priest_count: 0,
            kind_counts: HashMap::new(),
        }
    }
}

#[derive(Resource, Clone, Debug, Default, Eq, PartialEq)]
pub struct RosterEconomyFeedback {
    pub blocked_upgrade_reason: Option<String>,
}

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UpgradeTierUnlockState {
    pub unlocked_tier: u8,
    pub highest_major_wave_defeated: u32,
    pub hero_tier_unlocked: bool,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CommanderMotionState {
    pub is_moving: bool,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct UnitLevelCost(pub u32);

#[derive(Component, Clone, Copy, Debug)]
pub struct PriestSupportCaster {
    pub cooldown: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct PriestAttackSpeedBlessing {
    pub remaining_secs: f32,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct TrackerHoundSummoner {
    pub cooldown_secs: f32,
    pub active_secs: f32,
    pub strike_cooldown_secs: f32,
}

impl Default for TrackerHoundSummoner {
    fn default() -> Self {
        Self {
            cooldown_secs: 0.0,
            active_secs: 0.0,
            strike_cooldown_secs: 0.0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ScoutRaidBehavior {
    pub cooldown_secs: f32,
    pub active_secs: f32,
}

impl Default for ScoutRaidBehavior {
    fn default() -> Self {
        Self {
            cooldown_secs: 0.0,
            active_secs: 0.0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct OutOfFormation;

#[derive(Component, Clone, Copy, Debug)]
pub struct ArmorLockedZero;

#[derive(Component)]
struct PriestBlessingVfx;

#[derive(Resource, Clone)]
struct PriestBlessingVfxAssets {
    mesh: Handle<Mesh>,
    material: Handle<ColorMaterial>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PriestBlessingVfxAction {
    Spawn,
    Despawn,
    Keep,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct PromoteUnitsEvent {
    pub from_kind: UnitKind,
    pub to_kind: UnitKind,
    pub count: u32,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct ConvertTierZeroUnitsEvent {
    pub from_kind: UnitKind,
    pub to_kind: UnitKind,
    pub count: u32,
}

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadRoster>()
            .init_resource::<RosterEconomy>()
            .init_resource::<RosterEconomyFeedback>()
            .init_resource::<UpgradeTierUnlockState>()
            .init_resource::<CommanderMotionState>()
            .add_systems(Startup, setup_priest_blessing_vfx_assets)
            .add_event::<RecruitEvent>()
            .add_event::<PromoteUnitsEvent>()
            .add_event::<ConvertTierZeroUnitsEvent>()
            .add_event::<UnitDiedEvent>()
            .add_event::<WaveCompletedEvent>()
            .add_systems(Update, (handle_start_run, update_upgrade_tier_unlock_state))
            .add_systems(
                Update,
                commander_movement.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (
                    apply_recruit_events,
                    apply_promotion_events,
                    apply_tier0_conversion_events,
                    run_tracker_hound_logic,
                    run_scout_raid_behavior,
                    run_priest_support_logic,
                    sync_priest_blessing_vfx,
                    sync_roster,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
        app.add_systems(Update, on_unit_died);
    }
}

fn setup_priest_blessing_vfx_assets(
    mut commands: Commands,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<ColorMaterial>>>,
) {
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let mesh = meshes.add(Mesh::from(Circle::new(1.0)));
    let material = materials.add(ColorMaterial::from(Color::srgba(
        1.0,
        0.83,
        0.29,
        PRIEST_BLESSING_VFX_ALPHA,
    )));
    commands.insert_resource(PriestBlessingVfxAssets { mesh, material });
}

#[allow(clippy::too_many_arguments)]
fn handle_start_run(
    mut commands: Commands,
    mut roster: ResMut<SquadRoster>,
    mut economy: ResMut<RosterEconomy>,
    mut economy_feedback: ResMut<RosterEconomyFeedback>,
    mut unlock_state: ResMut<UpgradeTierUnlockState>,
    mut motion: ResMut<CommanderMotionState>,
    mut start_events: EventReader<StartRunEvent>,
    existing_units: Query<Entity, With<Unit>>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    setup_selection: Option<Res<MatchSetupSelection>>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in existing_units.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let faction = setup_selection
        .as_ref()
        .map(|selection| selection.faction)
        .unwrap_or(crate::model::PlayerFaction::Christian);
    let commander_id = setup_selection
        .as_ref()
        .map(|selection| selection.commander_id.as_str())
        .unwrap_or_else(|| crate::data::UnitsConfigFile::default_commander_id_for_faction(faction));
    let commander = spawn_commander(&mut commands, &data, &art, faction, commander_id);
    roster.commander = Some(commander);
    roster.friendly_count = 1;
    roster.casualties = 0;
    motion.is_moving = false;
    *economy = RosterEconomy::default();
    *economy_feedback = RosterEconomyFeedback::default();
    *unlock_state = UpgradeTierUnlockState::default();
}

fn spawn_commander(
    commands: &mut Commands,
    data: &GameData,
    art: &ArtAssets,
    faction: crate::model::PlayerFaction,
    commander_id: &str,
) -> Entity {
    let selected_commander = data
        .units
        .commander_option_for_faction_and_id(faction, commander_id)
        .or_else(|| {
            data.units.commander_option_for_faction_and_id(
                faction,
                crate::data::UnitsConfigFile::default_commander_id_for_faction(faction),
            )
        })
        .expect("commander option should exist for faction");
    let cfg = faction_adjusted_friendly_stats(
        &selected_commander.stats,
        data.factions.for_faction(faction),
    );
    let texture = match faction {
        crate::model::PlayerFaction::Christian => art.commander_idle.clone(),
        crate::model::PlayerFaction::Muslim => art.commander_saladin_idle.clone(),
    };
    let tint = match faction {
        crate::model::PlayerFaction::Christian => Color::srgb(1.0, 0.88, 0.88),
        crate::model::PlayerFaction::Muslim => Color::srgb(0.88, 0.96, 1.0),
    };
    let mut entity = commands.spawn((
        Unit {
            team: Team::Friendly,
            kind: UnitKind::Commander,
            level: 1,
        },
        UnitTier(0),
        UnitLevelCost(0),
        CommanderUnit,
        FriendlyUnit,
        PlayerControlled,
        Health::new(cfg.max_hp),
        BaseMaxHealth(cfg.max_hp),
        Morale::new(cfg.morale),
        Armor(cfg.armor),
        ColliderRadius(14.0),
        AttackProfile {
            damage: cfg.damage,
            range: cfg.attack_range,
            cooldown_secs: cfg.attack_cooldown_secs,
        },
        AttackCooldown(Timer::from_seconds(
            cfg.attack_cooldown_secs,
            TimerMode::Repeating,
        )),
        MoveSpeed(cfg.move_speed),
        SpriteBundle {
            texture,
            sprite: Sprite {
                color: tint,
                custom_size: Some(Vec2::splat(36.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 10.0),
            ..default()
        },
    ));

    if cfg.ranged_attack_damage > 0.0 {
        entity.insert((
            RangedAttackProfile {
                damage: cfg.ranged_attack_damage,
                range: cfg.ranged_attack_range,
                projectile_speed: cfg.ranged_projectile_speed,
                projectile_max_distance: cfg.ranged_projectile_max_distance,
            },
            RangedAttackCooldown(Timer::from_seconds(
                cfg.ranged_attack_cooldown_secs,
                TimerMode::Repeating,
            )),
        ));
    }

    entity.id()
}

fn update_upgrade_tier_unlock_state(
    mut unlock_state: ResMut<UpgradeTierUnlockState>,
    mut wave_completed_events: EventReader<WaveCompletedEvent>,
) {
    for event in wave_completed_events.read() {
        if !is_major_army_wave(event.wave_number) {
            continue;
        }
        unlock_state.highest_major_wave_defeated = unlock_state
            .highest_major_wave_defeated
            .max(event.wave_number);
        unlock_state.unlocked_tier = unlock_state
            .unlocked_tier
            .max(unlocked_upgrade_tier_for_major_wave(event.wave_number));
        unlock_state.hero_tier_unlocked =
            is_hero_tier_unlocked(unlock_state.highest_major_wave_defeated);
    }
}

fn spawn_recruit(
    commands: &mut Commands,
    data: &GameData,
    art: &ArtAssets,
    recruit_kind: RecruitUnitKind,
    position: Vec2,
) -> Entity {
    let cfg = faction_adjusted_friendly_stats(
        data.units.recruit_for_kind(recruit_kind),
        data.factions.for_faction(recruit_kind.faction()),
    );
    let unit_kind = recruit_kind.as_unit_kind();
    let (texture, collider_radius, sprite_tint) = match recruit_kind {
        RecruitUnitKind::ChristianPeasantInfantry => (
            art.friendly_peasant_infantry_idle.clone(),
            12.0,
            Color::WHITE,
        ),
        RecruitUnitKind::ChristianPeasantArcher => (
            art.friendly_peasant_archer_idle.clone(),
            11.0,
            Color::srgb(0.94, 1.0, 0.94),
        ),
        RecruitUnitKind::ChristianPeasantPriest => (
            art.friendly_peasant_priest_idle.clone(),
            11.0,
            Color::srgb(0.96, 0.93, 1.0),
        ),
        RecruitUnitKind::MuslimPeasantInfantry => (
            art.muslim_peasant_infantry_idle.clone(),
            12.0,
            Color::srgb(0.94, 0.96, 1.0),
        ),
        RecruitUnitKind::MuslimPeasantArcher => (
            art.muslim_peasant_archer_idle.clone(),
            11.0,
            Color::srgb(0.88, 0.96, 0.88),
        ),
        RecruitUnitKind::MuslimPeasantPriest => (
            art.muslim_peasant_priest_idle.clone(),
            11.0,
            Color::srgb(0.94, 0.9, 1.0),
        ),
    };
    let tier = 0u8;
    let level_cost = 0u32;
    let is_priest = unit_kind.is_priest();

    let mut entity = commands.spawn((
        Unit {
            team: Team::Friendly,
            kind: unit_kind,
            level: 1,
        },
        UnitTier(tier),
        UnitLevelCost(level_cost),
        FriendlyUnit,
        Health::new(cfg.max_hp),
        BaseMaxHealth(cfg.max_hp),
        Morale::new(cfg.morale),
        Armor(cfg.armor),
        ColliderRadius(collider_radius),
        AttackProfile {
            damage: cfg.damage,
            range: cfg.attack_range,
            cooldown_secs: cfg.attack_cooldown_secs,
        },
        AttackCooldown(Timer::from_seconds(
            cfg.attack_cooldown_secs,
            TimerMode::Repeating,
        )),
        MoveSpeed(cfg.move_speed),
        SpriteBundle {
            texture,
            sprite: Sprite {
                color: sprite_tint,
                custom_size: Some(Vec2::splat(32.0)),
                ..default()
            },
            transform: Transform::from_xyz(position.x, position.y, 10.0),
            ..default()
        },
    ));

    if is_priest {
        entity
            .insert(PriestSupportCaster {
                cooldown: PRIEST_BLESSING_COOLDOWN_SECS,
            })
            .remove::<(
                AttackProfile,
                AttackCooldown,
                RangedAttackProfile,
                RangedAttackCooldown,
                PriestAttackSpeedBlessing,
            )>();
    } else if cfg.ranged_attack_damage > 0.0 {
        entity.insert((
            RangedAttackProfile {
                damage: cfg.ranged_attack_damage,
                range: cfg.ranged_attack_range,
                projectile_speed: cfg.ranged_projectile_speed,
                projectile_max_distance: cfg.ranged_projectile_max_distance,
            },
            RangedAttackCooldown(Timer::from_seconds(
                cfg.ranged_attack_cooldown_secs,
                TimerMode::Repeating,
            )),
        ));
    }

    entity.id()
}

fn faction_adjusted_friendly_stats(
    base: &UnitStatsConfig,
    faction: &FactionGameplayConfig,
) -> UnitStatsConfig {
    let mut adjusted = base.clone();
    adjusted.max_hp = (adjusted.max_hp * faction.friendly_health_multiplier).max(1.0);
    adjusted.armor = (adjusted.armor + faction.friendly_armor_bonus).max(0.0);
    adjusted.damage = (adjusted.damage * faction.friendly_damage_multiplier).max(0.0);
    adjusted.attack_cooldown_secs = scale_cooldown_for_attack_speed(
        adjusted.attack_cooldown_secs,
        faction.friendly_attack_speed_multiplier,
        0.05,
    );
    adjusted.move_speed = (adjusted.move_speed * faction.friendly_move_speed_multiplier).max(1.0);
    adjusted.morale = (adjusted.morale * faction.friendly_morale_multiplier).max(1.0);

    if adjusted.ranged_attack_damage > 0.0 {
        adjusted.ranged_attack_damage =
            (adjusted.ranged_attack_damage * faction.friendly_damage_multiplier).max(0.0);
        adjusted.ranged_attack_cooldown_secs = scale_cooldown_for_attack_speed(
            adjusted.ranged_attack_cooldown_secs,
            faction.friendly_attack_speed_multiplier,
            0.05,
        );
    }
    adjusted
}

fn scale_cooldown_for_attack_speed(base_cooldown: f32, speed_multiplier: f32, min: f32) -> f32 {
    let safe_speed = speed_multiplier.max(0.01);
    (base_cooldown / safe_speed).max(min)
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn commander_movement(
    time: Res<Time>,
    data: Res<GameData>,
    inventory: Option<Res<InventoryState>>,
    active_formation: Res<ActiveFormation>,
    formation_mods: Res<FormationModifiers>,
    buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    banner_state: Option<Res<BannerState>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut commander_motion: ResMut<CommanderMotionState>,
    bounds: Option<Res<MapBounds>>,
    banner_penalty: Option<Res<BannerMovementPenalty>>,
    friendlies: Query<Entity, (With<FriendlyUnit>, Without<CommanderUnit>)>,
    enemies: Query<&Transform, (With<EnemyUnit>, Without<CommanderUnit>)>,
    mut commanders: Query<
        (
            &Unit,
            Option<&UnitTier>,
            &MoveSpeed,
            &Morale,
            &mut Transform,
        ),
        (With<PlayerControlled>, With<CommanderUnit>),
    >,
) {
    let Some(keys) = keyboard else {
        commander_motion.is_moving = false;
        return;
    };

    let mut axis = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        axis.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        axis.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        axis.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        axis.x += 1.0;
    }

    commander_motion.is_moving = axis.length_squared() > 0.0;
    if axis.length_squared() == 0.0 {
        return;
    }

    let direction = axis.normalize();
    let speed_multiplier = banner_penalty
        .as_ref()
        .map(|penalty| penalty.friendly_speed_multiplier)
        .unwrap_or(1.0);
    let recruit_count = friendlies.iter().count();
    let formation_cfg = active_formation_config(&data, *active_formation);
    let slot_spacing = formation_cfg.slot_spacing;
    let movement_bonus = buffs
        .as_ref()
        .map(|value| value.move_speed_bonus)
        .unwrap_or(0.0)
        + conditional_effects
            .as_deref()
            .map(|value| value.friendly_move_speed_bonus)
            .unwrap_or(0.0);
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);

    for (unit, tier, move_speed, morale, mut transform) in &mut commanders {
        let commander_position = transform.translation.truncate();
        let inside_enemy_count = enemies
            .iter()
            .filter(|enemy_transform| {
                enemy_inside_active_formation(
                    commander_position,
                    enemy_transform.translation.truncate(),
                    recruit_count,
                    slot_spacing,
                    *active_formation,
                )
            })
            .count();
        let formation_slowdown =
            movement_multiplier_from_inside_enemy_count(inside_enemy_count as u32);
        let gear = inventory
            .as_deref()
            .map(|inv| {
                gear_bonuses_for_unit_with_banner_state(
                    inv,
                    unit.kind,
                    tier.map(|value| value.0),
                    banner_item_active,
                )
            })
            .unwrap_or_default();
        let effective_speed = (move_speed.0 + movement_bonus + gear.move_speed_bonus).max(1.0)
            * formation_mods.move_speed_multiplier
            * morale_movement_multiplier(morale.ratio());
        let delta = direction * effective_speed * speed_multiplier * time.delta_seconds();
        transform.translation.x += delta.x * formation_slowdown;
        transform.translation.y += delta.y * formation_slowdown;
        if let Some(map_bounds) = &bounds {
            let playable = playable_bounds(**map_bounds);
            transform.translation.x = transform
                .translation
                .x
                .clamp(-playable.half_width, playable.half_width);
            transform.translation.y = transform
                .translation
                .y
                .clamp(-playable.half_height, playable.half_height);
        }
    }
}

pub fn enemy_inside_active_formation(
    commander_position: Vec2,
    enemy_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
    formation: ActiveFormation,
) -> bool {
    formation_contains_position(
        formation,
        commander_position,
        enemy_position,
        recruit_count,
        slot_spacing,
        ENEMY_INSIDE_FORMATION_PADDING_SLOTS,
    )
}

pub fn movement_multiplier_from_inside_enemy_count(inside_enemy_count: u32) -> f32 {
    (1.0 - inside_enemy_count as f32 * ENEMY_INSIDE_FORMATION_SLOWDOWN_PER_UNIT)
        .clamp(ENEMY_INSIDE_FORMATION_MIN_SPEED_MULTIPLIER, 1.0)
}

fn apply_recruit_events(
    mut commands: Commands,
    mut recruit_events: EventReader<RecruitEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
) {
    for event in recruit_events.read() {
        spawn_recruit(
            &mut commands,
            &data,
            &art,
            event.recruit_kind,
            event.world_position,
        );
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn apply_promotion_events(
    mut commands: Commands,
    mut promote_events: EventReader<PromoteUnitsEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    progression: Option<ResMut<Progression>>,
    unlock_state: Res<UpgradeTierUnlockState>,
    economy: Res<RosterEconomy>,
    mut feedback: ResMut<RosterEconomyFeedback>,
    friendlies: Query<
        (Entity, &Unit, &UnitTier, Option<&UnitLevelCost>),
        (With<FriendlyUnit>, Without<CommanderUnit>),
    >,
) {
    if promote_events.is_empty() {
        return;
    }
    let Some(mut progression) = progression else {
        return;
    };
    let current_level = progression.level;
    feedback.blocked_upgrade_reason = None;

    for event in promote_events.read() {
        if event.count == 0 || event.from_kind == event.to_kind {
            feedback.blocked_upgrade_reason = Some(
                "Promotion ignored: invalid count or identical source/target unit type."
                    .to_string(),
            );
            continue;
        }
        let Some(target_tier) = friendly_tier_for_kind(event.to_kind) else {
            feedback.blocked_upgrade_reason = Some(format!(
                "Promotion blocked: target '{}' is not a valid friendly promotion tier.",
                unit_kind_label(event.to_kind)
            ));
            continue;
        };
        if !is_upgrade_tier_unlocked(target_tier, unlock_state.unlocked_tier) {
            let unlock_wave = unlock_boss_wave_for_tier(target_tier).unwrap_or(10);
            feedback.blocked_upgrade_reason = Some(format!(
                "Promotion blocked: tier {target_tier} upgrades unlock after defeating the major army on wave {unlock_wave} (highest defeated major wave {}).",
                unlock_state.highest_major_wave_defeated
            ));
            continue;
        }
        let Some(min_step_cost) = promotion_step_cost(event.from_kind, event.to_kind) else {
            feedback.blocked_upgrade_reason = Some(format!(
                "Promotion blocked: '{}' cannot be promoted into '{}'.",
                unit_kind_label(event.from_kind),
                unit_kind_label(event.to_kind)
            ));
            continue;
        };

        let mut candidates: Vec<(Entity, Unit, UnitTier, UnitLevelCost)> = friendlies
            .iter()
            .filter_map(|(entity, unit, tier, level_cost)| {
                (unit.kind == event.from_kind).then_some((
                    entity,
                    *unit,
                    *tier,
                    level_cost.copied().unwrap_or(UnitLevelCost(0)),
                ))
            })
            .collect();
        candidates.sort_by_key(|(entity, _, _, _)| entity.index());

        let mut promoted = 0u32;
        let mut predicted_locked = economy.locked_levels;
        for (entity, unit, tier, level_cost) in candidates {
            if promoted >= event.count {
                break;
            }
            let step_cost = target_tier.saturating_sub(tier.0).max(min_step_cost as u8) as u32;
            if step_cost == 0 {
                continue;
            }
            let gold_cost = promotion_gold_cost(step_cost, target_tier);
            if level_cap_from_locked_budget(predicted_locked.saturating_add(step_cost))
                < current_level
            {
                feedback.blocked_upgrade_reason = Some(format!(
                    "Promotion blocked: '{}' -> '{}' would exceed level budget at commander level {}.",
                    unit_kind_label(event.from_kind),
                    unit_kind_label(event.to_kind),
                    current_level
                ));
                break;
            }
            if progression.gold + 0.001 < gold_cost {
                feedback.blocked_upgrade_reason = Some(format!(
                    "Promotion blocked: requires {:.1} gold for '{}' -> '{}', current treasury is {:.1}.",
                    gold_cost,
                    unit_kind_label(event.from_kind),
                    unit_kind_label(event.to_kind),
                    progression.gold
                ));
                break;
            }

            let Some(target_profile) = friendly_profile_for_kind(&data, &art, event.to_kind) else {
                continue;
            };
            let friendly_faction = event.to_kind.faction().unwrap_or(PlayerFaction::Christian);
            let cfg = faction_adjusted_friendly_stats(
                &target_profile.stats,
                data.factions.for_faction(friendly_faction),
            );

            let mut updated_unit = unit;
            updated_unit.kind = event.to_kind;
            let upgraded_level_cost = level_cost.0.saturating_add(step_cost);
            apply_friendly_kind_loadout(
                &mut commands,
                entity,
                updated_unit,
                target_tier,
                upgraded_level_cost,
                &cfg,
                target_profile.texture.clone(),
                target_profile.collider_radius,
                target_profile.has_melee,
                target_profile.has_ranged,
                target_profile.priest_kind,
                target_profile.tracker_kind,
                target_profile.scout_kind,
                target_profile.armor_locked_zero,
                &data.roster_tuning.behavior,
            );

            predicted_locked = predicted_locked.saturating_add(step_cost);
            promoted = promoted.saturating_add(1);
            progression.gold = (progression.gold - gold_cost).max(0.0);
        }
        if promoted == 0 && feedback.blocked_upgrade_reason.is_none() {
            feedback.blocked_upgrade_reason = Some(format!(
                "Promotion blocked: no eligible '{}' units available.",
                unit_kind_label(event.from_kind)
            ));
        }
        if promoted > 0 {
            feedback.blocked_upgrade_reason = None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn apply_tier0_conversion_events(
    mut commands: Commands,
    mut convert_events: EventReader<ConvertTierZeroUnitsEvent>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    progression: Option<ResMut<Progression>>,
    mut feedback: ResMut<RosterEconomyFeedback>,
    friendlies: Query<
        (Entity, &Unit, &UnitTier, Option<&UnitLevelCost>),
        (With<FriendlyUnit>, Without<CommanderUnit>),
    >,
) {
    if convert_events.is_empty() {
        return;
    }
    let Some(mut progression) = progression else {
        return;
    };
    feedback.blocked_upgrade_reason = None;

    for event in convert_events.read() {
        if event.count == 0 || event.from_kind == event.to_kind {
            feedback.blocked_upgrade_reason = Some(
                "Tier-0 conversion ignored: invalid count or identical source/target unit type."
                    .to_string(),
            );
            continue;
        }
        let Some(from_tier) = friendly_tier_for_kind(event.from_kind) else {
            feedback.blocked_upgrade_reason =
                Some("Tier-0 conversion blocked: invalid source unit type.".to_string());
            continue;
        };
        let Some(to_tier) = friendly_tier_for_kind(event.to_kind) else {
            feedback.blocked_upgrade_reason =
                Some("Tier-0 conversion blocked: invalid target unit type.".to_string());
            continue;
        };
        if from_tier != 0 || to_tier != 0 {
            feedback.blocked_upgrade_reason = Some(
                "Tier-0 conversion blocked: both source and target must be tier-0 units."
                    .to_string(),
            );
            continue;
        }
        let from_faction = event.from_kind.faction();
        let to_faction = event.to_kind.faction();
        if from_faction.is_none() || from_faction != to_faction {
            feedback.blocked_upgrade_reason = Some(
                "Tier-0 conversion blocked: source and target must be from the same faction."
                    .to_string(),
            );
            continue;
        }

        let mut candidates: Vec<(Entity, Unit, UnitTier, UnitLevelCost)> = friendlies
            .iter()
            .filter_map(|(entity, unit, tier, level_cost)| {
                (unit.kind == event.from_kind).then_some((
                    entity,
                    *unit,
                    *tier,
                    level_cost.copied().unwrap_or(UnitLevelCost(0)),
                ))
            })
            .collect();
        candidates.sort_by_key(|(entity, _, _, _)| entity.index());
        let convertible = event.count.min(candidates.len() as u32);
        if convertible == 0 {
            feedback.blocked_upgrade_reason = Some(format!(
                "Tier-0 conversion blocked: no eligible '{}' units available.",
                unit_kind_label(event.from_kind)
            ));
            continue;
        }

        let gold_cost_per_unit = tier0_conversion_gold_cost();
        let total_gold_cost = gold_cost_per_unit * convertible as f32;
        if progression.gold + 0.001 < total_gold_cost {
            feedback.blocked_upgrade_reason = Some(format!(
                "Tier-0 conversion blocked: requires {:.1} gold for {} swap(s), current treasury is {:.1}.",
                total_gold_cost, convertible, progression.gold
            ));
            continue;
        }

        let Some(target_profile) = friendly_profile_for_kind(&data, &art, event.to_kind) else {
            feedback.blocked_upgrade_reason =
                Some("Tier-0 conversion blocked: missing target unit profile data.".to_string());
            continue;
        };
        let friendly_faction = event.to_kind.faction().unwrap_or(PlayerFaction::Christian);
        let adjusted_cfg = faction_adjusted_friendly_stats(
            &target_profile.stats,
            data.factions.for_faction(friendly_faction),
        );

        for (entity, unit, tier, level_cost) in candidates.into_iter().take(convertible as usize) {
            let mut updated_unit = unit;
            updated_unit.kind = event.to_kind;
            apply_friendly_kind_loadout(
                &mut commands,
                entity,
                updated_unit,
                tier.0,
                level_cost.0,
                &adjusted_cfg,
                target_profile.texture.clone(),
                target_profile.collider_radius,
                target_profile.has_melee,
                target_profile.has_ranged,
                target_profile.priest_kind,
                target_profile.tracker_kind,
                target_profile.scout_kind,
                target_profile.armor_locked_zero,
                &data.roster_tuning.behavior,
            );
        }

        progression.gold = (progression.gold - total_gold_cost).max(0.0);
        feedback.blocked_upgrade_reason = None;
    }
}

#[allow(clippy::type_complexity)]
fn run_tracker_hound_logic(
    time: Res<Time>,
    data: Res<GameData>,
    damage_events: Option<ResMut<Events<DamageEvent>>>,
    mut trackers: Query<(&Unit, &Transform, &AttackProfile, &mut TrackerHoundSummoner)>,
    targets: Query<(Entity, &Unit, &Transform, &Health), Without<RescuableUnit>>,
) {
    let Some(mut damage_events) = damage_events else {
        return;
    };
    let ability = &data.roster_tuning.behavior;
    let dt = time.delta_seconds();
    let target_snapshot: Vec<(Entity, Team, Vec2)> = targets
        .iter()
        .filter_map(|(entity, unit, transform, health)| {
            if unit.team == Team::Neutral || health.current <= 0.0 {
                None
            } else {
                Some((entity, unit.team, transform.translation.truncate()))
            }
        })
        .collect();
    if target_snapshot.is_empty() {
        return;
    }

    for (unit, tracker_transform, attack_profile, mut summoner) in &mut trackers {
        if !matches!(
            unit.kind,
            UnitKind::ChristianTracker
                | UnitKind::ChristianPathfinder
                | UnitKind::ChristianHoundmaster
                | UnitKind::ChristianEliteHoundmaster
                | UnitKind::MuslimTracker
                | UnitKind::MuslimPathfinder
                | UnitKind::MuslimHoundmaster
                | UnitKind::MuslimEliteHoundmaster
        ) {
            continue;
        }
        if unit.team == Team::Neutral {
            continue;
        }

        if summoner.active_secs > 0.0 {
            summoner.active_secs = (summoner.active_secs - dt).max(0.0);
            summoner.strike_cooldown_secs -= dt;
            while summoner.active_secs > 0.0 && summoner.strike_cooldown_secs <= 0.0 {
                let tracker_position = tracker_transform.translation.truncate();
                let nearest_target = target_snapshot
                    .iter()
                    .filter(|(_, team, _)| *team != unit.team)
                    .min_by(|(_, _, a), (_, _, b)| {
                        tracker_position
                            .distance_squared(*a)
                            .partial_cmp(&tracker_position.distance_squared(*b))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                if let Some((target_entity, _, _)) = nearest_target {
                    damage_events.send(DamageEvent {
                        target: *target_entity,
                        source_team: unit.team,
                        amount: (attack_profile.damage * ability.tracker_hound_damage_multiplier)
                            .max(1.0),
                        execute: false,
                        critical: false,
                        source_entity: None,
                    });
                }
                summoner.strike_cooldown_secs += ability.tracker_hound_strike_interval_secs;
            }
            if summoner.active_secs <= 0.0 {
                summoner.cooldown_secs = ability.tracker_hound_cooldown_secs;
                summoner.strike_cooldown_secs = 0.0;
            }
        } else {
            summoner.cooldown_secs = (summoner.cooldown_secs - dt).max(0.0);
            if summoner.cooldown_secs <= 0.0 {
                summoner.active_secs = ability.tracker_hound_active_secs;
                summoner.strike_cooldown_secs = 0.0;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn run_scout_raid_behavior(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    mut scouts: Query<(
        Entity,
        &Unit,
        &MoveSpeed,
        &mut Transform,
        &mut ScoutRaidBehavior,
    )>,
    targets: Query<
        (&Unit, &Transform, &Health),
        (Without<RescuableUnit>, Without<ScoutRaidBehavior>),
    >,
) {
    let ability = &data.roster_tuning.behavior;
    let dt = time.delta_seconds();
    let target_snapshot: Vec<(Team, Vec2)> = targets
        .iter()
        .filter_map(|(unit, transform, health)| {
            if unit.team == Team::Neutral || health.current <= 0.0 {
                None
            } else {
                Some((unit.team, transform.translation.truncate()))
            }
        })
        .collect();

    for (entity, unit, speed, mut transform, mut scout) in &mut scouts {
        if !matches!(
            unit.kind,
            UnitKind::ChristianScout
                | UnitKind::ChristianMountedScout
                | UnitKind::ChristianShockCavalry
                | UnitKind::ChristianEliteShockCavalry
                | UnitKind::MuslimScout
                | UnitKind::MuslimMountedScout
                | UnitKind::MuslimShockCavalry
                | UnitKind::MuslimEliteShockCavalry
        ) {
            continue;
        }
        if unit.team == Team::Neutral {
            continue;
        }

        if scout.active_secs > 0.0 {
            scout.active_secs = (scout.active_secs - dt).max(0.0);
            commands.entity(entity).insert(OutOfFormation);
            let current_position = transform.translation.truncate();
            if let Some((_, target_position)) = target_snapshot
                .iter()
                .filter(|(team, _)| *team != unit.team)
                .min_by(|(_, a), (_, b)| {
                    current_position
                        .distance_squared(*a)
                        .partial_cmp(&current_position.distance_squared(*b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            {
                let direction = (*target_position - current_position)
                    .try_normalize()
                    .unwrap_or(Vec2::ZERO);
                let step = direction * speed.0 * ability.scout_raid_speed_multiplier * dt;
                transform.translation.x += step.x;
                transform.translation.y += step.y;
            }
            if scout.active_secs <= 0.0 {
                scout.cooldown_secs = ability.scout_raid_cooldown_secs;
                commands.entity(entity).remove::<OutOfFormation>();
            }
        } else {
            commands.entity(entity).remove::<OutOfFormation>();
            scout.cooldown_secs = (scout.cooldown_secs - dt).max(0.0);
            if scout.cooldown_secs <= 0.0 {
                scout.active_secs = ability.scout_raid_active_secs;
                commands.entity(entity).insert(OutOfFormation);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn run_priest_support_logic(
    mut commands: Commands,
    time: Res<Time>,
    banner_state: Option<Res<BannerState>>,
    inventory: Option<Res<InventoryState>>,
    skill_timing: Option<Res<SkillTimingBuffs>>,
    mut priests: Query<(
        &Unit,
        Option<&UnitTier>,
        &Transform,
        &mut PriestSupportCaster,
    )>,
    units: Query<(Entity, &Transform, &Unit), Without<RescuableUnit>>,
    mut blessings: Query<(Entity, &Unit, &mut PriestAttackSpeedBlessing), Without<RescuableUnit>>,
) {
    let dt = time.delta_seconds();
    let banner_item_active = !banner_state
        .as_deref()
        .map(|state| state.is_dropped)
        .unwrap_or(false);
    let skill_duration_multiplier = skill_timing
        .as_deref()
        .map(|value| value.duration_multiplier.max(1.0))
        .unwrap_or(1.0);
    let cooldown_reduction_percent = skill_timing
        .as_deref()
        .map(|value| value.cooldown_reduction)
        .unwrap_or(0.0);
    for (entity, unit, mut blessing) in &mut blessings {
        if unit.team == Team::Neutral {
            commands
                .entity(entity)
                .remove::<PriestAttackSpeedBlessing>();
            continue;
        }
        blessing.remaining_secs = (blessing.remaining_secs - dt).max(0.0);
        if blessing.remaining_secs <= 0.0 {
            commands
                .entity(entity)
                .remove::<PriestAttackSpeedBlessing>();
        }
    }

    let unit_positions: Vec<(Entity, Vec2, Team)> = units
        .iter()
        .filter_map(|(entity, transform, unit)| {
            if unit.team == Team::Neutral {
                None
            } else {
                Some((entity, transform.translation.truncate(), unit.team))
            }
        })
        .collect();
    if unit_positions.is_empty() {
        return;
    }

    for (priest_unit, priest_tier, priest_transform, mut caster) in &mut priests {
        if !priest_unit.kind.is_priest() || priest_unit.team == Team::Neutral {
            continue;
        }
        caster.cooldown = tick_priest_cooldown(caster.cooldown, dt);
        if !priest_should_cast(caster.cooldown) {
            continue;
        }
        let priest_position = priest_transform.translation.truncate();
        let range_sq = PRIEST_BLESSING_RANGE * PRIEST_BLESSING_RANGE;
        for (entity, position, team) in &unit_positions {
            if *team != priest_unit.team {
                continue;
            }
            if position.distance_squared(priest_position) <= range_sq {
                commands.entity(*entity).insert(PriestAttackSpeedBlessing {
                    remaining_secs: refresh_priest_blessing_remaining(
                        0.0,
                        skill_duration_multiplier,
                    ),
                });
            }
        }
        let cooldown_reduction = if priest_unit.team == Team::Friendly {
            inventory
                .as_deref()
                .map(|inv| {
                    gear_bonuses_for_unit_with_banner_state(
                        inv,
                        priest_unit.kind,
                        priest_tier.map(|value| value.0),
                        banner_item_active,
                    )
                })
                .unwrap_or_default()
                .cooldown_reduction_secs
                .max(0.0)
        } else {
            0.0
        };
        caster.cooldown = scaled_skill_cooldown(
            PRIEST_BLESSING_COOLDOWN_SECS,
            cooldown_reduction,
            cooldown_reduction_percent,
            4.0,
        );
    }
}

#[allow(clippy::type_complexity)]
fn sync_priest_blessing_vfx(
    mut commands: Commands,
    assets: Option<Res<PriestBlessingVfxAssets>>,
    units: Query<(
        Entity,
        &Unit,
        Option<&PriestAttackSpeedBlessing>,
        Option<&Children>,
        Option<&ColliderRadius>,
    )>,
    vfx_nodes: Query<Entity, With<PriestBlessingVfx>>,
) {
    let Some(assets) = assets else {
        return;
    };

    for (entity, unit, blessing, children, collider_radius) in &units {
        let mut vfx_children: Vec<Entity> = Vec::new();
        if let Some(children) = children {
            for child in children.iter() {
                if vfx_nodes.get(*child).is_ok() {
                    vfx_children.push(*child);
                }
            }
        }

        let has_blessing = unit.team != Team::Neutral && blessing.is_some();
        match priest_blessing_vfx_action(has_blessing, !vfx_children.is_empty()) {
            PriestBlessingVfxAction::Spawn => {
                let base_radius = collider_radius
                    .map(|radius| radius.0)
                    .unwrap_or(PRIEST_BLESSING_VFX_MIN_RADIUS)
                    .max(PRIEST_BLESSING_VFX_MIN_RADIUS);
                let shadow_scale = priest_blessing_shadow_scale(base_radius);
                let vfx = commands
                    .spawn((
                        Name::new("PriestBlessingVfx"),
                        PriestBlessingVfx,
                        MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(assets.mesh.clone()),
                            material: assets.material.clone(),
                            transform: Transform::from_xyz(
                                0.0,
                                PRIEST_BLESSING_VFX_Y_OFFSET,
                                PRIEST_BLESSING_VFX_Z_OFFSET,
                            )
                            .with_scale(Vec3::new(
                                shadow_scale.x,
                                shadow_scale.y,
                                1.0,
                            )),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(entity).add_child(vfx);
            }
            PriestBlessingVfxAction::Despawn => {
                for child in vfx_children {
                    commands.entity(child).despawn_recursive();
                }
            }
            PriestBlessingVfxAction::Keep => {
                // Clean up accidental duplicates while keeping one marker active.
                for duplicate in vfx_children.into_iter().skip(1) {
                    commands.entity(duplicate).despawn_recursive();
                }
            }
        }
    }
}

fn priest_blessing_vfx_action(has_blessing: bool, has_vfx: bool) -> PriestBlessingVfxAction {
    match (has_blessing, has_vfx) {
        (true, false) => PriestBlessingVfxAction::Spawn,
        (false, true) => PriestBlessingVfxAction::Despawn,
        _ => PriestBlessingVfxAction::Keep,
    }
}

fn priest_blessing_shadow_scale(collider_radius: f32) -> Vec2 {
    let radius = collider_radius.max(PRIEST_BLESSING_VFX_MIN_RADIUS);
    Vec2::new(
        radius * PRIEST_BLESSING_VFX_SCALE_X,
        radius * PRIEST_BLESSING_VFX_SCALE_Y,
    )
}

pub fn tick_priest_cooldown(current_cooldown: f32, delta_seconds: f32) -> f32 {
    (current_cooldown - delta_seconds.max(0.0)).max(0.0)
}

pub fn priest_should_cast(cooldown_after_tick: f32) -> bool {
    cooldown_after_tick <= 0.0
}

pub fn refresh_priest_blessing_remaining(_current_remaining: f32, duration_multiplier: f32) -> f32 {
    scaled_skill_duration(PRIEST_BLESSING_DURATION_SECS, duration_multiplier)
}

#[allow(clippy::type_complexity)]
fn sync_roster(
    mut roster: ResMut<SquadRoster>,
    mut economy: ResMut<RosterEconomy>,
    friendlies: Query<
        (
            Entity,
            Option<&CommanderUnit>,
            &Unit,
            Option<&UnitTier>,
            Option<&UnitLevelCost>,
        ),
        With<FriendlyUnit>,
    >,
) {
    let next_friendly_count = friendlies.iter().count();
    let next_commander = friendlies
        .iter()
        .find_map(|(entity, commander, _, _, _)| commander.map(|_| entity));
    if roster.friendly_count != next_friendly_count {
        roster.friendly_count = next_friendly_count;
    }
    if roster.commander != next_commander {
        roster.commander = next_commander;
    }

    let mut locked = 0u32;
    let mut tier0 = 0u32;
    let mut total_retinue = 0u32;
    let mut infantry = 0u32;
    let mut archer = 0u32;
    let mut priest = 0u32;
    let mut kind_counts: HashMap<UnitKind, u32> = HashMap::new();

    for (_, commander, unit, tier, level_cost) in &friendlies {
        if commander.is_some() {
            continue;
        }
        total_retinue = total_retinue.saturating_add(1);
        kind_counts
            .entry(unit.kind)
            .and_modify(|count| *count = count.saturating_add(1))
            .or_insert(1);
        let unit_tier = tier.copied().unwrap_or(UnitTier(0)).0;
        let unit_cost = level_cost
            .copied()
            .unwrap_or(UnitLevelCost(unit_tier as u32))
            .0;
        locked = locked.saturating_add(unit_cost);
        if unit_tier == 0 {
            tier0 = tier0.saturating_add(1);
        }
        match unit.kind {
            UnitKind::ChristianPeasantInfantry | UnitKind::MuslimPeasantInfantry => {
                infantry = infantry.saturating_add(1)
            }
            UnitKind::ChristianPeasantArcher | UnitKind::MuslimPeasantArcher => {
                archer = archer.saturating_add(1)
            }
            UnitKind::ChristianPeasantPriest | UnitKind::MuslimPeasantPriest => {
                priest = priest.saturating_add(1)
            }
            _ => {}
        }
    }

    let next_economy = RosterEconomy {
        locked_levels: locked,
        allowed_max_level: level_cap_from_locked_budget(locked),
        tier0_retinue_count: tier0,
        total_retinue_count: total_retinue,
        infantry_count: infantry,
        archer_count: archer,
        priest_count: priest,
        kind_counts,
    };
    if *economy != next_economy {
        *economy = next_economy;
    }
}

fn on_unit_died(mut roster: ResMut<SquadRoster>, mut death_events: EventReader<UnitDiedEvent>) {
    for event in death_events.read() {
        if event.team == Team::Friendly {
            roster.casualties = roster.casualties.saturating_add(1);
        }
    }
}

pub fn friendly_tier_for_kind(kind: UnitKind) -> Option<u8> {
    match kind {
        UnitKind::ChristianPeasantInfantry
        | UnitKind::ChristianPeasantArcher
        | UnitKind::ChristianPeasantPriest
        | UnitKind::MuslimPeasantInfantry
        | UnitKind::MuslimPeasantArcher
        | UnitKind::MuslimPeasantPriest => Some(0),
        UnitKind::ChristianMenAtArms
        | UnitKind::ChristianBowman
        | UnitKind::ChristianDevoted
        | UnitKind::MuslimMenAtArms
        | UnitKind::MuslimBowman
        | UnitKind::MuslimDevoted => Some(1),
        UnitKind::ChristianShieldInfantry
        | UnitKind::ChristianSpearman
        | UnitKind::ChristianUnmountedKnight
        | UnitKind::ChristianSquire
        | UnitKind::ChristianExperiencedBowman
        | UnitKind::ChristianCrossbowman
        | UnitKind::ChristianTracker
        | UnitKind::ChristianScout
        | UnitKind::ChristianDevotedOne
        | UnitKind::ChristianFanatic
        | UnitKind::MuslimShieldInfantry
        | UnitKind::MuslimSpearman
        | UnitKind::MuslimUnmountedKnight
        | UnitKind::MuslimSquire
        | UnitKind::MuslimExperiencedBowman
        | UnitKind::MuslimCrossbowman
        | UnitKind::MuslimTracker
        | UnitKind::MuslimScout
        | UnitKind::MuslimDevotedOne
        | UnitKind::MuslimFanatic => Some(2),
        UnitKind::ChristianExperiencedShieldInfantry
        | UnitKind::ChristianShieldedSpearman
        | UnitKind::ChristianKnight
        | UnitKind::ChristianBannerman
        | UnitKind::ChristianEliteBowman
        | UnitKind::ChristianArmoredCrossbowman
        | UnitKind::ChristianPathfinder
        | UnitKind::ChristianMountedScout
        | UnitKind::ChristianCardinal
        | UnitKind::ChristianFlagellant
        | UnitKind::MuslimExperiencedShieldInfantry
        | UnitKind::MuslimShieldedSpearman
        | UnitKind::MuslimKnight
        | UnitKind::MuslimBannerman
        | UnitKind::MuslimEliteBowman
        | UnitKind::MuslimArmoredCrossbowman
        | UnitKind::MuslimPathfinder
        | UnitKind::MuslimMountedScout
        | UnitKind::MuslimCardinal
        | UnitKind::MuslimFlagellant => Some(3),
        UnitKind::ChristianEliteShieldInfantry
        | UnitKind::ChristianHalberdier
        | UnitKind::ChristianHeavyKnight
        | UnitKind::ChristianEliteBannerman
        | UnitKind::ChristianLongbowman
        | UnitKind::ChristianEliteCrossbowman
        | UnitKind::ChristianHoundmaster
        | UnitKind::ChristianShockCavalry
        | UnitKind::ChristianEliteCardinal
        | UnitKind::ChristianEliteFlagellant
        | UnitKind::MuslimEliteShieldInfantry
        | UnitKind::MuslimHalberdier
        | UnitKind::MuslimHeavyKnight
        | UnitKind::MuslimEliteBannerman
        | UnitKind::MuslimLongbowman
        | UnitKind::MuslimEliteCrossbowman
        | UnitKind::MuslimHoundmaster
        | UnitKind::MuslimShockCavalry
        | UnitKind::MuslimEliteCardinal
        | UnitKind::MuslimEliteFlagellant => Some(4),
        UnitKind::ChristianCitadelGuard
        | UnitKind::ChristianArmoredHalberdier
        | UnitKind::ChristianEliteHeavyKnight
        | UnitKind::ChristianGodsChosen
        | UnitKind::ChristianEliteLongbowman
        | UnitKind::ChristianSiegeCrossbowman
        | UnitKind::ChristianEliteHoundmaster
        | UnitKind::ChristianEliteShockCavalry
        | UnitKind::ChristianDivineSpeaker
        | UnitKind::ChristianDivineJudge
        | UnitKind::MuslimCitadelGuard
        | UnitKind::MuslimArmoredHalberdier
        | UnitKind::MuslimEliteHeavyKnight
        | UnitKind::MuslimGodsChosen
        | UnitKind::MuslimEliteLongbowman
        | UnitKind::MuslimSiegeCrossbowman
        | UnitKind::MuslimEliteHoundmaster
        | UnitKind::MuslimEliteShockCavalry
        | UnitKind::MuslimDivineSpeaker
        | UnitKind::MuslimDivineJudge => Some(5),
        _ => None,
    }
}

pub fn tier1_promotion_target_for_kind(from_kind: UnitKind) -> Option<UnitKind> {
    promotion_targets_for_kind(from_kind).into_iter().next()
}

pub fn promotion_targets_for_kind(from_kind: UnitKind) -> Vec<UnitKind> {
    match from_kind {
        UnitKind::ChristianPeasantInfantry => vec![UnitKind::ChristianMenAtArms],
        UnitKind::ChristianPeasantArcher => vec![UnitKind::ChristianBowman],
        UnitKind::ChristianPeasantPriest => vec![UnitKind::ChristianDevoted],
        UnitKind::MuslimPeasantInfantry => vec![UnitKind::MuslimMenAtArms],
        UnitKind::MuslimPeasantArcher => vec![UnitKind::MuslimBowman],
        UnitKind::MuslimPeasantPriest => vec![UnitKind::MuslimDevoted],
        UnitKind::ChristianMenAtArms => vec![
            UnitKind::ChristianShieldInfantry,
            UnitKind::ChristianSpearman,
            UnitKind::ChristianUnmountedKnight,
            UnitKind::ChristianSquire,
        ],
        UnitKind::ChristianBowman => vec![
            UnitKind::ChristianExperiencedBowman,
            UnitKind::ChristianCrossbowman,
            UnitKind::ChristianTracker,
            UnitKind::ChristianScout,
        ],
        UnitKind::ChristianDevoted => {
            vec![UnitKind::ChristianDevotedOne, UnitKind::ChristianFanatic]
        }
        UnitKind::ChristianShieldInfantry => vec![UnitKind::ChristianExperiencedShieldInfantry],
        UnitKind::ChristianSpearman => vec![UnitKind::ChristianShieldedSpearman],
        UnitKind::ChristianUnmountedKnight => vec![UnitKind::ChristianKnight],
        UnitKind::ChristianSquire => vec![UnitKind::ChristianBannerman],
        UnitKind::ChristianExperiencedBowman => vec![UnitKind::ChristianEliteBowman],
        UnitKind::ChristianCrossbowman => vec![UnitKind::ChristianArmoredCrossbowman],
        UnitKind::ChristianTracker => vec![UnitKind::ChristianPathfinder],
        UnitKind::ChristianScout => vec![UnitKind::ChristianMountedScout],
        UnitKind::ChristianDevotedOne => vec![UnitKind::ChristianCardinal],
        UnitKind::ChristianFanatic => vec![UnitKind::ChristianFlagellant],
        UnitKind::ChristianExperiencedShieldInfantry => {
            vec![UnitKind::ChristianEliteShieldInfantry]
        }
        UnitKind::ChristianShieldedSpearman => vec![UnitKind::ChristianHalberdier],
        UnitKind::ChristianKnight => vec![UnitKind::ChristianHeavyKnight],
        UnitKind::ChristianBannerman => vec![UnitKind::ChristianEliteBannerman],
        UnitKind::ChristianEliteBowman => vec![UnitKind::ChristianLongbowman],
        UnitKind::ChristianArmoredCrossbowman => vec![UnitKind::ChristianEliteCrossbowman],
        UnitKind::ChristianPathfinder => vec![UnitKind::ChristianHoundmaster],
        UnitKind::ChristianMountedScout => vec![UnitKind::ChristianShockCavalry],
        UnitKind::ChristianCardinal => vec![UnitKind::ChristianEliteCardinal],
        UnitKind::ChristianFlagellant => vec![UnitKind::ChristianEliteFlagellant],
        UnitKind::ChristianEliteShieldInfantry => vec![UnitKind::ChristianCitadelGuard],
        UnitKind::ChristianHalberdier => vec![UnitKind::ChristianArmoredHalberdier],
        UnitKind::ChristianHeavyKnight => vec![UnitKind::ChristianEliteHeavyKnight],
        UnitKind::ChristianEliteBannerman => vec![UnitKind::ChristianGodsChosen],
        UnitKind::ChristianLongbowman => vec![UnitKind::ChristianEliteLongbowman],
        UnitKind::ChristianEliteCrossbowman => vec![UnitKind::ChristianSiegeCrossbowman],
        UnitKind::ChristianHoundmaster => vec![UnitKind::ChristianEliteHoundmaster],
        UnitKind::ChristianShockCavalry => vec![UnitKind::ChristianEliteShockCavalry],
        UnitKind::ChristianEliteCardinal => vec![UnitKind::ChristianDivineSpeaker],
        UnitKind::ChristianEliteFlagellant => vec![UnitKind::ChristianDivineJudge],
        UnitKind::MuslimMenAtArms => vec![
            UnitKind::MuslimShieldInfantry,
            UnitKind::MuslimSpearman,
            UnitKind::MuslimUnmountedKnight,
            UnitKind::MuslimSquire,
        ],
        UnitKind::MuslimBowman => vec![
            UnitKind::MuslimExperiencedBowman,
            UnitKind::MuslimCrossbowman,
            UnitKind::MuslimTracker,
            UnitKind::MuslimScout,
        ],
        UnitKind::MuslimDevoted => vec![UnitKind::MuslimDevotedOne, UnitKind::MuslimFanatic],
        UnitKind::MuslimShieldInfantry => vec![UnitKind::MuslimExperiencedShieldInfantry],
        UnitKind::MuslimSpearman => vec![UnitKind::MuslimShieldedSpearman],
        UnitKind::MuslimUnmountedKnight => vec![UnitKind::MuslimKnight],
        UnitKind::MuslimSquire => vec![UnitKind::MuslimBannerman],
        UnitKind::MuslimExperiencedBowman => vec![UnitKind::MuslimEliteBowman],
        UnitKind::MuslimCrossbowman => vec![UnitKind::MuslimArmoredCrossbowman],
        UnitKind::MuslimTracker => vec![UnitKind::MuslimPathfinder],
        UnitKind::MuslimScout => vec![UnitKind::MuslimMountedScout],
        UnitKind::MuslimDevotedOne => vec![UnitKind::MuslimCardinal],
        UnitKind::MuslimFanatic => vec![UnitKind::MuslimFlagellant],
        UnitKind::MuslimExperiencedShieldInfantry => vec![UnitKind::MuslimEliteShieldInfantry],
        UnitKind::MuslimShieldedSpearman => vec![UnitKind::MuslimHalberdier],
        UnitKind::MuslimKnight => vec![UnitKind::MuslimHeavyKnight],
        UnitKind::MuslimBannerman => vec![UnitKind::MuslimEliteBannerman],
        UnitKind::MuslimEliteBowman => vec![UnitKind::MuslimLongbowman],
        UnitKind::MuslimArmoredCrossbowman => vec![UnitKind::MuslimEliteCrossbowman],
        UnitKind::MuslimPathfinder => vec![UnitKind::MuslimHoundmaster],
        UnitKind::MuslimMountedScout => vec![UnitKind::MuslimShockCavalry],
        UnitKind::MuslimCardinal => vec![UnitKind::MuslimEliteCardinal],
        UnitKind::MuslimFlagellant => vec![UnitKind::MuslimEliteFlagellant],
        UnitKind::MuslimEliteShieldInfantry => vec![UnitKind::MuslimCitadelGuard],
        UnitKind::MuslimHalberdier => vec![UnitKind::MuslimArmoredHalberdier],
        UnitKind::MuslimHeavyKnight => vec![UnitKind::MuslimEliteHeavyKnight],
        UnitKind::MuslimEliteBannerman => vec![UnitKind::MuslimGodsChosen],
        UnitKind::MuslimLongbowman => vec![UnitKind::MuslimEliteLongbowman],
        UnitKind::MuslimEliteCrossbowman => vec![UnitKind::MuslimSiegeCrossbowman],
        UnitKind::MuslimHoundmaster => vec![UnitKind::MuslimEliteHoundmaster],
        UnitKind::MuslimShockCavalry => vec![UnitKind::MuslimEliteShockCavalry],
        UnitKind::MuslimEliteCardinal => vec![UnitKind::MuslimDivineSpeaker],
        UnitKind::MuslimEliteFlagellant => vec![UnitKind::MuslimDivineJudge],
        _ => Vec::new(),
    }
}

fn is_valid_promotion_path(from_kind: UnitKind, to_kind: UnitKind) -> bool {
    promotion_targets_for_kind(from_kind)
        .into_iter()
        .any(|target| target == to_kind)
}

pub fn promotion_step_cost(from_kind: UnitKind, to_kind: UnitKind) -> Option<u32> {
    let from_tier = friendly_tier_for_kind(from_kind)?;
    let to_tier = friendly_tier_for_kind(to_kind)?;
    if to_tier <= from_tier || !is_valid_promotion_path(from_kind, to_kind) {
        return None;
    }
    Some((to_tier - from_tier) as u32)
}

pub const fn tier0_conversion_gold_cost() -> f32 {
    TIER0_CONVERSION_GOLD_COST_PER_UNIT
}

pub fn promotion_gold_cost(step_cost: u32, target_tier: u8) -> f32 {
    if step_cost == 0 {
        return 0.0;
    }
    let per_step =
        PROMOTION_GOLD_COST_PER_STEP + target_tier.max(1) as f32 * PROMOTION_GOLD_COST_PER_TIER;
    step_cost as f32 * per_step
}

pub fn unlocked_upgrade_tier_for_major_wave(wave_number: u32) -> u8 {
    (wave_number / 10).min(5) as u8
}

pub const fn is_hero_tier_unlocked(highest_major_wave_defeated: u32) -> bool {
    highest_major_wave_defeated >= HERO_TIER_UNLOCK_MAJOR_WAVE
}

pub fn unlock_boss_wave_for_tier(tier: u8) -> Option<u32> {
    match tier {
        0 => Some(0),
        1..=5 => Some(tier as u32 * 10),
        _ => None,
    }
}

pub fn is_upgrade_tier_unlocked(tier: u8, unlocked_tier: u8) -> bool {
    tier <= unlocked_tier
}

pub fn unit_kind_label(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Commander => "Commander",
        UnitKind::ChristianPeasantInfantry => "Christian Peasant Infantry",
        UnitKind::ChristianPeasantArcher => "Christian Peasant Archer",
        UnitKind::ChristianPeasantPriest => "Christian Peasant Priest",
        UnitKind::ChristianMenAtArms => "Christian Men-at-Arms",
        UnitKind::ChristianBowman => "Christian Bowman",
        UnitKind::ChristianDevoted => "Christian Devoted",
        UnitKind::ChristianShieldInfantry => "Christian Shield Infantry",
        UnitKind::ChristianSpearman => "Christian Spearman",
        UnitKind::ChristianUnmountedKnight => "Christian Unmounted Knight",
        UnitKind::ChristianSquire => "Christian Squire",
        UnitKind::ChristianExperiencedBowman => "Christian Experienced Bowman",
        UnitKind::ChristianCrossbowman => "Christian Crossbowman",
        UnitKind::ChristianTracker => "Christian Tracker",
        UnitKind::ChristianScout => "Christian Scout",
        UnitKind::ChristianDevotedOne => "Christian Devoted One",
        UnitKind::ChristianFanatic => "Christian Fanatic",
        UnitKind::ChristianExperiencedShieldInfantry => "Christian Experienced Shield Infantry",
        UnitKind::ChristianShieldedSpearman => "Christian Shielded Spearman",
        UnitKind::ChristianKnight => "Christian Knight",
        UnitKind::ChristianBannerman => "Christian Bannerman",
        UnitKind::ChristianEliteBowman => "Christian Elite Bowman",
        UnitKind::ChristianArmoredCrossbowman => "Christian Armored Crossbowman",
        UnitKind::ChristianPathfinder => "Christian Pathfinder",
        UnitKind::ChristianMountedScout => "Christian Mounted Scout",
        UnitKind::ChristianCardinal => "Christian Cardinal",
        UnitKind::ChristianFlagellant => "Christian Flagellant",
        UnitKind::ChristianEliteShieldInfantry => "Christian Elite Shield Infantry",
        UnitKind::ChristianHalberdier => "Christian Halberdier",
        UnitKind::ChristianHeavyKnight => "Christian Heavy Knight",
        UnitKind::ChristianEliteBannerman => "Christian Elite Bannerman",
        UnitKind::ChristianLongbowman => "Christian Longbowman",
        UnitKind::ChristianEliteCrossbowman => "Christian Elite Crossbowman",
        UnitKind::ChristianHoundmaster => "Christian Houndmaster",
        UnitKind::ChristianShockCavalry => "Christian Shock Cavalry",
        UnitKind::ChristianEliteCardinal => "Christian Elite Cardinal",
        UnitKind::ChristianEliteFlagellant => "Christian Elite Flagellant",
        UnitKind::ChristianCitadelGuard => "Christian Citadel Guard",
        UnitKind::ChristianArmoredHalberdier => "Christian Armored Halberdier",
        UnitKind::ChristianEliteHeavyKnight => "Christian Elite Heavy Knight",
        UnitKind::ChristianGodsChosen => "Christian God's Chosen",
        UnitKind::ChristianEliteLongbowman => "Christian Elite Longbowman",
        UnitKind::ChristianSiegeCrossbowman => "Christian Siege Crossbowman",
        UnitKind::ChristianEliteHoundmaster => "Christian Elite Houndmaster",
        UnitKind::ChristianEliteShockCavalry => "Christian Elite Shock Cavalry",
        UnitKind::ChristianDivineSpeaker => "Christian Divine Speaker",
        UnitKind::ChristianDivineJudge => "Christian Divine Judge",
        UnitKind::MuslimPeasantInfantry => "Muslim Peasant Infantry",
        UnitKind::MuslimPeasantArcher => "Muslim Peasant Archer",
        UnitKind::MuslimPeasantPriest => "Muslim Peasant Priest",
        UnitKind::MuslimMenAtArms => "Muslim Men-at-Arms",
        UnitKind::MuslimBowman => "Muslim Bowman",
        UnitKind::MuslimDevoted => "Muslim Devoted",
        UnitKind::MuslimShieldInfantry => "Muslim Shield Infantry",
        UnitKind::MuslimSpearman => "Muslim Spearman",
        UnitKind::MuslimUnmountedKnight => "Muslim Unmounted Knight",
        UnitKind::MuslimSquire => "Muslim Squire",
        UnitKind::MuslimExperiencedBowman => "Muslim Experienced Bowman",
        UnitKind::MuslimCrossbowman => "Muslim Crossbowman",
        UnitKind::MuslimTracker => "Muslim Tracker",
        UnitKind::MuslimScout => "Muslim Scout",
        UnitKind::MuslimDevotedOne => "Muslim Devoted One",
        UnitKind::MuslimFanatic => "Muslim Fanatic",
        UnitKind::MuslimExperiencedShieldInfantry => "Muslim Experienced Shield Infantry",
        UnitKind::MuslimShieldedSpearman => "Muslim Shielded Spearman",
        UnitKind::MuslimKnight => "Muslim Knight",
        UnitKind::MuslimBannerman => "Muslim Bannerman",
        UnitKind::MuslimEliteBowman => "Muslim Elite Bowman",
        UnitKind::MuslimArmoredCrossbowman => "Muslim Armored Crossbowman",
        UnitKind::MuslimPathfinder => "Muslim Pathfinder",
        UnitKind::MuslimMountedScout => "Muslim Mounted Scout",
        UnitKind::MuslimCardinal => "Muslim Cardinal",
        UnitKind::MuslimFlagellant => "Muslim Flagellant",
        UnitKind::MuslimEliteShieldInfantry => "Muslim Elite Shield Infantry",
        UnitKind::MuslimHalberdier => "Muslim Halberdier",
        UnitKind::MuslimHeavyKnight => "Muslim Heavy Knight",
        UnitKind::MuslimEliteBannerman => "Muslim Elite Bannerman",
        UnitKind::MuslimLongbowman => "Muslim Longbowman",
        UnitKind::MuslimEliteCrossbowman => "Muslim Elite Crossbowman",
        UnitKind::MuslimHoundmaster => "Muslim Houndmaster",
        UnitKind::MuslimShockCavalry => "Muslim Shock Cavalry",
        UnitKind::MuslimEliteCardinal => "Muslim Elite Cardinal",
        UnitKind::MuslimEliteFlagellant => "Muslim Elite Flagellant",
        UnitKind::MuslimCitadelGuard => "Muslim Citadel Guard",
        UnitKind::MuslimArmoredHalberdier => "Muslim Armored Halberdier",
        UnitKind::MuslimEliteHeavyKnight => "Muslim Elite Heavy Knight",
        UnitKind::MuslimGodsChosen => "Muslim God's Chosen",
        UnitKind::MuslimEliteLongbowman => "Muslim Elite Longbowman",
        UnitKind::MuslimSiegeCrossbowman => "Muslim Siege Crossbowman",
        UnitKind::MuslimEliteHoundmaster => "Muslim Elite Houndmaster",
        UnitKind::MuslimEliteShockCavalry => "Muslim Elite Shock Cavalry",
        UnitKind::MuslimDivineSpeaker => "Muslim Divine Speaker",
        UnitKind::MuslimDivineJudge => "Muslim Divine Judge",
        UnitKind::RescuableChristianPeasantInfantry => "Rescuable Christian Peasant Infantry",
        UnitKind::RescuableChristianPeasantArcher => "Rescuable Christian Peasant Archer",
        UnitKind::RescuableChristianPeasantPriest => "Rescuable Christian Peasant Priest",
        UnitKind::RescuableMuslimPeasantInfantry => "Rescuable Muslim Peasant Infantry",
        UnitKind::RescuableMuslimPeasantArcher => "Rescuable Muslim Peasant Archer",
        UnitKind::RescuableMuslimPeasantPriest => "Rescuable Muslim Peasant Priest",
    }
}

#[derive(Clone, Debug)]
struct FriendlyKindProfile {
    stats: UnitStatsConfig,
    texture: Handle<Image>,
    collider_radius: f32,
    has_melee: bool,
    has_ranged: bool,
    priest_kind: bool,
    tracker_kind: bool,
    scout_kind: bool,
    armor_locked_zero: bool,
}

pub fn friendly_stats_for_kind(data: &GameData, kind: UnitKind) -> Option<UnitStatsConfig> {
    match kind {
        UnitKind::ChristianPeasantInfantry => {
            Some(data.units.recruit_christian_peasant_infantry.clone())
        }
        UnitKind::ChristianPeasantArcher => {
            Some(data.units.recruit_christian_peasant_archer.clone())
        }
        UnitKind::ChristianPeasantPriest => {
            Some(data.units.recruit_christian_peasant_priest.clone())
        }
        UnitKind::ChristianMenAtArms => Some(tier1_men_at_arms_stats(
            &data.units.recruit_christian_peasant_infantry,
            "christian_men_at_arms",
        )),
        UnitKind::ChristianBowman => Some(tier1_bowman_stats(
            &data.units.recruit_christian_peasant_archer,
            "christian_bowman",
        )),
        UnitKind::ChristianDevoted => Some(tier1_devoted_stats(
            &data.units.recruit_christian_peasant_priest,
            "christian_devoted",
        )),
        UnitKind::ChristianShieldInfantry
        | UnitKind::ChristianSpearman
        | UnitKind::ChristianUnmountedKnight
        | UnitKind::ChristianSquire
        | UnitKind::ChristianExperiencedBowman
        | UnitKind::ChristianCrossbowman
        | UnitKind::ChristianTracker
        | UnitKind::ChristianScout
        | UnitKind::ChristianDevotedOne
        | UnitKind::ChristianFanatic => data.roster_tuning.tier2_stats_for_kind(kind).cloned(),
        UnitKind::ChristianExperiencedShieldInfantry
        | UnitKind::ChristianShieldedSpearman
        | UnitKind::ChristianKnight
        | UnitKind::ChristianBannerman
        | UnitKind::ChristianEliteBowman
        | UnitKind::ChristianArmoredCrossbowman
        | UnitKind::ChristianPathfinder
        | UnitKind::ChristianMountedScout
        | UnitKind::ChristianCardinal
        | UnitKind::ChristianFlagellant => tier3_stats_for_kind(data, kind),
        UnitKind::ChristianEliteShieldInfantry
        | UnitKind::ChristianHalberdier
        | UnitKind::ChristianHeavyKnight
        | UnitKind::ChristianEliteBannerman
        | UnitKind::ChristianLongbowman
        | UnitKind::ChristianEliteCrossbowman
        | UnitKind::ChristianHoundmaster
        | UnitKind::ChristianShockCavalry
        | UnitKind::ChristianEliteCardinal
        | UnitKind::ChristianEliteFlagellant => tier4_stats_for_kind(data, kind),
        UnitKind::ChristianCitadelGuard
        | UnitKind::ChristianArmoredHalberdier
        | UnitKind::ChristianEliteHeavyKnight
        | UnitKind::ChristianGodsChosen
        | UnitKind::ChristianEliteLongbowman
        | UnitKind::ChristianSiegeCrossbowman
        | UnitKind::ChristianEliteHoundmaster
        | UnitKind::ChristianEliteShockCavalry
        | UnitKind::ChristianDivineSpeaker
        | UnitKind::ChristianDivineJudge => tier5_stats_for_kind(data, kind),
        UnitKind::MuslimPeasantInfantry => Some(data.units.recruit_muslim_peasant_infantry.clone()),
        UnitKind::MuslimPeasantArcher => Some(data.units.recruit_muslim_peasant_archer.clone()),
        UnitKind::MuslimPeasantPriest => Some(data.units.recruit_muslim_peasant_priest.clone()),
        UnitKind::MuslimMenAtArms => Some(tier1_men_at_arms_stats(
            &data.units.recruit_muslim_peasant_infantry,
            "muslim_men_at_arms",
        )),
        UnitKind::MuslimBowman => Some(tier1_bowman_stats(
            &data.units.recruit_muslim_peasant_archer,
            "muslim_bowman",
        )),
        UnitKind::MuslimDevoted => Some(tier1_devoted_stats(
            &data.units.recruit_muslim_peasant_priest,
            "muslim_devoted",
        )),
        UnitKind::MuslimShieldInfantry
        | UnitKind::MuslimSpearman
        | UnitKind::MuslimUnmountedKnight
        | UnitKind::MuslimSquire
        | UnitKind::MuslimExperiencedBowman
        | UnitKind::MuslimCrossbowman
        | UnitKind::MuslimTracker
        | UnitKind::MuslimScout
        | UnitKind::MuslimDevotedOne
        | UnitKind::MuslimFanatic => data.roster_tuning.tier2_stats_for_kind(kind).cloned(),
        UnitKind::MuslimExperiencedShieldInfantry
        | UnitKind::MuslimShieldedSpearman
        | UnitKind::MuslimKnight
        | UnitKind::MuslimBannerman
        | UnitKind::MuslimEliteBowman
        | UnitKind::MuslimArmoredCrossbowman
        | UnitKind::MuslimPathfinder
        | UnitKind::MuslimMountedScout
        | UnitKind::MuslimCardinal
        | UnitKind::MuslimFlagellant => tier3_stats_for_kind(data, kind),
        UnitKind::MuslimEliteShieldInfantry
        | UnitKind::MuslimHalberdier
        | UnitKind::MuslimHeavyKnight
        | UnitKind::MuslimEliteBannerman
        | UnitKind::MuslimLongbowman
        | UnitKind::MuslimEliteCrossbowman
        | UnitKind::MuslimHoundmaster
        | UnitKind::MuslimShockCavalry
        | UnitKind::MuslimEliteCardinal
        | UnitKind::MuslimEliteFlagellant => tier4_stats_for_kind(data, kind),
        UnitKind::MuslimCitadelGuard
        | UnitKind::MuslimArmoredHalberdier
        | UnitKind::MuslimEliteHeavyKnight
        | UnitKind::MuslimGodsChosen
        | UnitKind::MuslimEliteLongbowman
        | UnitKind::MuslimSiegeCrossbowman
        | UnitKind::MuslimEliteHoundmaster
        | UnitKind::MuslimEliteShockCavalry
        | UnitKind::MuslimDivineSpeaker
        | UnitKind::MuslimDivineJudge => tier5_stats_for_kind(data, kind),
        _ => None,
    }
}

fn tier1_men_at_arms_stats(base: &UnitStatsConfig, id: &str) -> UnitStatsConfig {
    let mut stats = base.clone();
    stats.id = id.to_string();
    stats.max_hp = (base.max_hp * 1.35).max(base.max_hp + 12.0);
    stats.armor = base.armor + 2.2;
    stats.damage = (base.damage * 0.88).max(1.0);
    stats.attack_cooldown_secs = (base.attack_cooldown_secs * 1.08).max(0.08);
    stats.move_speed = (base.move_speed * 0.95).max(1.0);
    stats.morale = (base.morale * 1.12).max(1.0);
    stats
}

fn tier1_bowman_stats(base: &UnitStatsConfig, id: &str) -> UnitStatsConfig {
    let mut stats = base.clone();
    stats.id = id.to_string();
    stats.max_hp = (base.max_hp * 0.94).max(1.0);
    stats.armor = (base.armor * 0.85).max(0.0);
    stats.damage = (base.damage * 0.72).max(0.0);
    stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.95).max(0.08);
    stats.move_speed = (base.move_speed * 1.06).max(1.0);
    stats.morale = (base.morale * 1.08).max(1.0);
    stats.ranged_attack_damage = (base.ranged_attack_damage * 0.9).max(0.0);
    stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.78).max(0.08);
    stats.ranged_attack_range = base.ranged_attack_range + 90.0;
    stats.ranged_projectile_speed = base.ranged_projectile_speed + 20.0;
    stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 110.0;
    stats
}

fn tier1_devoted_stats(base: &UnitStatsConfig, id: &str) -> UnitStatsConfig {
    let mut stats = base.clone();
    stats.id = id.to_string();
    stats.max_hp = (base.max_hp * 1.22).max(base.max_hp + 8.0);
    stats.armor = base.armor + 1.0;
    stats.damage = 0.0;
    stats.move_speed = (base.move_speed * 1.02).max(1.0);
    stats.morale = (base.morale * 1.2).max(1.0);
    stats.ranged_attack_damage = 0.0;
    stats.ranged_attack_cooldown_secs = 0.0;
    stats.ranged_attack_range = 0.0;
    stats.ranged_projectile_speed = 0.0;
    stats.ranged_projectile_max_distance = 0.0;
    stats
}

fn tier2_source_kind_for_tier3(kind: UnitKind) -> Option<UnitKind> {
    match kind {
        UnitKind::ChristianExperiencedShieldInfantry => Some(UnitKind::ChristianShieldInfantry),
        UnitKind::ChristianShieldedSpearman => Some(UnitKind::ChristianSpearman),
        UnitKind::ChristianKnight => Some(UnitKind::ChristianUnmountedKnight),
        UnitKind::ChristianBannerman => Some(UnitKind::ChristianSquire),
        UnitKind::ChristianEliteBowman => Some(UnitKind::ChristianExperiencedBowman),
        UnitKind::ChristianArmoredCrossbowman => Some(UnitKind::ChristianCrossbowman),
        UnitKind::ChristianPathfinder => Some(UnitKind::ChristianTracker),
        UnitKind::ChristianMountedScout => Some(UnitKind::ChristianScout),
        UnitKind::ChristianCardinal => Some(UnitKind::ChristianDevotedOne),
        UnitKind::ChristianFlagellant => Some(UnitKind::ChristianFanatic),
        UnitKind::MuslimExperiencedShieldInfantry => Some(UnitKind::MuslimShieldInfantry),
        UnitKind::MuslimShieldedSpearman => Some(UnitKind::MuslimSpearman),
        UnitKind::MuslimKnight => Some(UnitKind::MuslimUnmountedKnight),
        UnitKind::MuslimBannerman => Some(UnitKind::MuslimSquire),
        UnitKind::MuslimEliteBowman => Some(UnitKind::MuslimExperiencedBowman),
        UnitKind::MuslimArmoredCrossbowman => Some(UnitKind::MuslimCrossbowman),
        UnitKind::MuslimPathfinder => Some(UnitKind::MuslimTracker),
        UnitKind::MuslimMountedScout => Some(UnitKind::MuslimScout),
        UnitKind::MuslimCardinal => Some(UnitKind::MuslimDevotedOne),
        UnitKind::MuslimFlagellant => Some(UnitKind::MuslimFanatic),
        _ => None,
    }
}

fn tier3_stats_for_kind(data: &GameData, kind: UnitKind) -> Option<UnitStatsConfig> {
    let tier2_kind = tier2_source_kind_for_tier3(kind)?;
    let base = data.roster_tuning.tier2_stats_for_kind(tier2_kind)?;
    let mut stats = base.clone();
    stats.id = match kind {
        UnitKind::ChristianExperiencedShieldInfantry => "christian_experienced_shield_infantry",
        UnitKind::ChristianShieldedSpearman => "christian_shielded_spearman",
        UnitKind::ChristianKnight => "christian_knight",
        UnitKind::ChristianBannerman => "christian_bannerman",
        UnitKind::ChristianEliteBowman => "christian_elite_bowman",
        UnitKind::ChristianArmoredCrossbowman => "christian_armored_crossbowman",
        UnitKind::ChristianPathfinder => "christian_pathfinder",
        UnitKind::ChristianMountedScout => "christian_mounted_scout",
        UnitKind::ChristianCardinal => "christian_cardinal",
        UnitKind::ChristianFlagellant => "christian_flagellant",
        UnitKind::MuslimExperiencedShieldInfantry => "muslim_experienced_shield_infantry",
        UnitKind::MuslimShieldedSpearman => "muslim_shielded_spearman",
        UnitKind::MuslimKnight => "muslim_knight",
        UnitKind::MuslimBannerman => "muslim_bannerman",
        UnitKind::MuslimEliteBowman => "muslim_elite_bowman",
        UnitKind::MuslimArmoredCrossbowman => "muslim_armored_crossbowman",
        UnitKind::MuslimPathfinder => "muslim_pathfinder",
        UnitKind::MuslimMountedScout => "muslim_mounted_scout",
        UnitKind::MuslimCardinal => "muslim_cardinal",
        UnitKind::MuslimFlagellant => "muslim_flagellant",
        _ => base.id.as_str(),
    }
    .to_string();

    match kind {
        UnitKind::ChristianExperiencedShieldInfantry
        | UnitKind::MuslimExperiencedShieldInfantry => {
            stats.max_hp = (base.max_hp * 1.22).max(base.max_hp + 14.0);
            stats.armor = base.armor + 1.8;
            stats.damage = (base.damage * 1.14).max(base.damage + 0.8);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.93).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 0.97).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
        }
        UnitKind::ChristianShieldedSpearman | UnitKind::MuslimShieldedSpearman => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.6;
            stats.damage = (base.damage * 1.16).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.92).max(0.08);
            stats.attack_range = base.attack_range + 6.0;
            stats.move_speed = (base.move_speed * 1.01).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
        }
        UnitKind::ChristianKnight | UnitKind::MuslimKnight => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 10.0);
            stats.armor = base.armor + 2.0;
            stats.damage = (base.damage * 1.22).max(base.damage + 1.2);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.90).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
        }
        UnitKind::ChristianBannerman | UnitKind::MuslimBannerman => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 10.0);
            stats.armor = base.armor + 1.2;
            stats.damage = 0.0;
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.95).max(0.08);
            stats.attack_range = 20.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.14).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianEliteBowman | UnitKind::MuslimEliteBowman => {
            stats.max_hp = (base.max_hp * 1.12).max(base.max_hp + 8.0);
            stats.armor = base.armor + 0.8;
            stats.damage = (base.damage * 1.08).max(base.damage + 0.5);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.93).max(0.08);
            stats.move_speed = (base.move_speed * 1.03).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.20).max(base.ranged_attack_damage + 2.0);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.90).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 40.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 18.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 50.0;
        }
        UnitKind::ChristianArmoredCrossbowman | UnitKind::MuslimArmoredCrossbowman => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 9.0);
            stats.armor = base.armor + 1.4;
            stats.damage = (base.damage * 1.10).max(base.damage + 0.5);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.95).max(0.08);
            stats.move_speed = (base.move_speed * 0.98).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.24).max(base.ranged_attack_damage + 2.4);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.92).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 24.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 22.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 40.0;
        }
        UnitKind::ChristianPathfinder | UnitKind::MuslimPathfinder => {
            stats.max_hp = (base.max_hp * 1.12).max(base.max_hp + 8.0);
            stats.armor = base.armor + 0.8;
            stats.damage = (base.damage * 1.10).max(base.damage + 0.4);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.92).max(0.08);
            stats.move_speed = (base.move_speed * 1.06).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.16).max(base.ranged_attack_damage + 1.6);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.90).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 24.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 26.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 38.0;
        }
        UnitKind::ChristianMountedScout | UnitKind::MuslimMountedScout => {
            stats.max_hp = (base.max_hp * 1.14).max(base.max_hp + 8.0);
            stats.armor = base.armor + 1.0;
            stats.damage = (base.damage * 1.22).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.90).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.18).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianCardinal | UnitKind::MuslimCardinal => {
            stats.max_hp = (base.max_hp * 1.20).max(base.max_hp + 10.0);
            stats.armor = base.armor + 1.5;
            stats.damage = 0.0;
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.95).max(0.08);
            stats.attack_range = 20.0;
            stats.move_speed = (base.move_speed * 1.01).max(1.0);
            stats.morale = (base.morale * 1.14).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianFlagellant | UnitKind::MuslimFlagellant => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 12.0);
            stats.armor = 0.0;
            stats.damage = (base.damage * 1.26).max(base.damage + 1.5);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.88).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.04).max(1.0);
            stats.morale = (base.morale * 1.14).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        _ => {}
    }

    Some(stats)
}

fn tier3_source_kind_for_tier4(kind: UnitKind) -> Option<UnitKind> {
    match kind {
        UnitKind::ChristianEliteShieldInfantry => {
            Some(UnitKind::ChristianExperiencedShieldInfantry)
        }
        UnitKind::ChristianHalberdier => Some(UnitKind::ChristianShieldedSpearman),
        UnitKind::ChristianHeavyKnight => Some(UnitKind::ChristianKnight),
        UnitKind::ChristianEliteBannerman => Some(UnitKind::ChristianBannerman),
        UnitKind::ChristianLongbowman => Some(UnitKind::ChristianEliteBowman),
        UnitKind::ChristianEliteCrossbowman => Some(UnitKind::ChristianArmoredCrossbowman),
        UnitKind::ChristianHoundmaster => Some(UnitKind::ChristianPathfinder),
        UnitKind::ChristianShockCavalry => Some(UnitKind::ChristianMountedScout),
        UnitKind::ChristianEliteCardinal => Some(UnitKind::ChristianCardinal),
        UnitKind::ChristianEliteFlagellant => Some(UnitKind::ChristianFlagellant),
        UnitKind::MuslimEliteShieldInfantry => Some(UnitKind::MuslimExperiencedShieldInfantry),
        UnitKind::MuslimHalberdier => Some(UnitKind::MuslimShieldedSpearman),
        UnitKind::MuslimHeavyKnight => Some(UnitKind::MuslimKnight),
        UnitKind::MuslimEliteBannerman => Some(UnitKind::MuslimBannerman),
        UnitKind::MuslimLongbowman => Some(UnitKind::MuslimEliteBowman),
        UnitKind::MuslimEliteCrossbowman => Some(UnitKind::MuslimArmoredCrossbowman),
        UnitKind::MuslimHoundmaster => Some(UnitKind::MuslimPathfinder),
        UnitKind::MuslimShockCavalry => Some(UnitKind::MuslimMountedScout),
        UnitKind::MuslimEliteCardinal => Some(UnitKind::MuslimCardinal),
        UnitKind::MuslimEliteFlagellant => Some(UnitKind::MuslimFlagellant),
        _ => None,
    }
}

fn tier4_stats_for_kind(data: &GameData, kind: UnitKind) -> Option<UnitStatsConfig> {
    let tier3_kind = tier3_source_kind_for_tier4(kind)?;
    let base = tier3_stats_for_kind(data, tier3_kind)?;
    let mut stats = base.clone();
    stats.id = match kind {
        UnitKind::ChristianEliteShieldInfantry => "christian_elite_shield_infantry",
        UnitKind::ChristianHalberdier => "christian_halberdier",
        UnitKind::ChristianHeavyKnight => "christian_heavy_knight",
        UnitKind::ChristianEliteBannerman => "christian_elite_bannerman",
        UnitKind::ChristianLongbowman => "christian_longbowman",
        UnitKind::ChristianEliteCrossbowman => "christian_elite_crossbowman",
        UnitKind::ChristianHoundmaster => "christian_houndmaster",
        UnitKind::ChristianShockCavalry => "christian_shock_cavalry",
        UnitKind::ChristianEliteCardinal => "christian_elite_cardinal",
        UnitKind::ChristianEliteFlagellant => "christian_elite_flagellant",
        UnitKind::MuslimEliteShieldInfantry => "muslim_elite_shield_infantry",
        UnitKind::MuslimHalberdier => "muslim_halberdier",
        UnitKind::MuslimHeavyKnight => "muslim_heavy_knight",
        UnitKind::MuslimEliteBannerman => "muslim_elite_bannerman",
        UnitKind::MuslimLongbowman => "muslim_longbowman",
        UnitKind::MuslimEliteCrossbowman => "muslim_elite_crossbowman",
        UnitKind::MuslimHoundmaster => "muslim_houndmaster",
        UnitKind::MuslimShockCavalry => "muslim_shock_cavalry",
        UnitKind::MuslimEliteCardinal => "muslim_elite_cardinal",
        UnitKind::MuslimEliteFlagellant => "muslim_elite_flagellant",
        _ => base.id.as_str(),
    }
    .to_string();

    match kind {
        UnitKind::ChristianEliteShieldInfantry | UnitKind::MuslimEliteShieldInfantry => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 14.0);
            stats.armor = base.armor + 2.0;
            stats.damage = (base.damage * 1.12).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.93).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 0.98).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
        }
        UnitKind::ChristianHalberdier | UnitKind::MuslimHalberdier => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.8;
            stats.damage = (base.damage * 1.16).max(base.damage + 1.2);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.91).max(0.08);
            stats.attack_range = base.attack_range + 6.0;
            stats.move_speed = (base.move_speed * 1.01).max(1.0);
            stats.morale = (base.morale * 1.09).max(1.0);
        }
        UnitKind::ChristianHeavyKnight | UnitKind::MuslimHeavyKnight => {
            stats.max_hp = (base.max_hp * 1.15).max(base.max_hp + 12.0);
            stats.armor = base.armor + 2.2;
            stats.damage = (base.damage * 1.20).max(base.damage + 1.4);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.90).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
        }
        UnitKind::ChristianEliteBannerman | UnitKind::MuslimEliteBannerman => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.6;
            stats.damage = 0.0;
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.95).max(0.08);
            stats.attack_range = 20.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianLongbowman | UnitKind::MuslimLongbowman => {
            stats.max_hp = (base.max_hp * 1.14).max(base.max_hp + 10.0);
            stats.armor = base.armor + 1.0;
            stats.damage = (base.damage * 1.10).max(base.damage + 0.8);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.92).max(0.08);
            stats.move_speed = (base.move_speed * 1.03).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.20).max(base.ranged_attack_damage + 2.4);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.90).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 34.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 22.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 44.0;
        }
        UnitKind::ChristianEliteCrossbowman | UnitKind::MuslimEliteCrossbowman => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 10.0);
            stats.armor = base.armor + 1.6;
            stats.damage = (base.damage * 1.12).max(base.damage + 0.8);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.94).max(0.08);
            stats.move_speed = (base.move_speed * 0.99).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.22).max(base.ranged_attack_damage + 2.8);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.90).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 26.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 24.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 40.0;
        }
        UnitKind::ChristianHoundmaster | UnitKind::MuslimHoundmaster => {
            stats.max_hp = (base.max_hp * 1.14).max(base.max_hp + 10.0);
            stats.armor = base.armor + 1.0;
            stats.damage = (base.damage * 1.12).max(base.damage + 0.8);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.92).max(0.08);
            stats.move_speed = (base.move_speed * 1.06).max(1.0);
            stats.morale = (base.morale * 1.08).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.18).max(base.ranged_attack_damage + 2.0);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.88).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 20.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 28.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 34.0;
        }
        UnitKind::ChristianShockCavalry | UnitKind::MuslimShockCavalry => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.8;
            stats.damage = (base.damage * 1.22).max(base.damage + 1.6);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.89).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.14).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianEliteCardinal | UnitKind::MuslimEliteCardinal => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.8;
            stats.damage = 0.0;
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.94).max(0.08);
            stats.attack_range = 20.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianEliteFlagellant | UnitKind::MuslimEliteFlagellant => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 14.0);
            stats.armor = 0.0;
            stats.damage = (base.damage * 1.24).max(base.damage + 1.8);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.86).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.05).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        _ => {}
    }

    Some(stats)
}

fn tier4_source_kind_for_tier5(kind: UnitKind) -> Option<UnitKind> {
    match kind {
        UnitKind::ChristianCitadelGuard => Some(UnitKind::ChristianEliteShieldInfantry),
        UnitKind::ChristianArmoredHalberdier => Some(UnitKind::ChristianHalberdier),
        UnitKind::ChristianEliteHeavyKnight => Some(UnitKind::ChristianHeavyKnight),
        UnitKind::ChristianGodsChosen => Some(UnitKind::ChristianEliteBannerman),
        UnitKind::ChristianEliteLongbowman => Some(UnitKind::ChristianLongbowman),
        UnitKind::ChristianSiegeCrossbowman => Some(UnitKind::ChristianEliteCrossbowman),
        UnitKind::ChristianEliteHoundmaster => Some(UnitKind::ChristianHoundmaster),
        UnitKind::ChristianEliteShockCavalry => Some(UnitKind::ChristianShockCavalry),
        UnitKind::ChristianDivineSpeaker => Some(UnitKind::ChristianEliteCardinal),
        UnitKind::ChristianDivineJudge => Some(UnitKind::ChristianEliteFlagellant),
        UnitKind::MuslimCitadelGuard => Some(UnitKind::MuslimEliteShieldInfantry),
        UnitKind::MuslimArmoredHalberdier => Some(UnitKind::MuslimHalberdier),
        UnitKind::MuslimEliteHeavyKnight => Some(UnitKind::MuslimHeavyKnight),
        UnitKind::MuslimGodsChosen => Some(UnitKind::MuslimEliteBannerman),
        UnitKind::MuslimEliteLongbowman => Some(UnitKind::MuslimLongbowman),
        UnitKind::MuslimSiegeCrossbowman => Some(UnitKind::MuslimEliteCrossbowman),
        UnitKind::MuslimEliteHoundmaster => Some(UnitKind::MuslimHoundmaster),
        UnitKind::MuslimEliteShockCavalry => Some(UnitKind::MuslimShockCavalry),
        UnitKind::MuslimDivineSpeaker => Some(UnitKind::MuslimEliteCardinal),
        UnitKind::MuslimDivineJudge => Some(UnitKind::MuslimEliteFlagellant),
        _ => None,
    }
}

fn tier5_stats_for_kind(data: &GameData, kind: UnitKind) -> Option<UnitStatsConfig> {
    let tier4_kind = tier4_source_kind_for_tier5(kind)?;
    let base = tier4_stats_for_kind(data, tier4_kind)?;
    let mut stats = base.clone();
    stats.id = match kind {
        UnitKind::ChristianCitadelGuard => "christian_citadel_guard",
        UnitKind::ChristianArmoredHalberdier => "christian_armored_halberdier",
        UnitKind::ChristianEliteHeavyKnight => "christian_elite_heavy_knight",
        UnitKind::ChristianGodsChosen => "christian_gods_chosen",
        UnitKind::ChristianEliteLongbowman => "christian_elite_longbowman",
        UnitKind::ChristianSiegeCrossbowman => "christian_siege_crossbowman",
        UnitKind::ChristianEliteHoundmaster => "christian_elite_houndmaster",
        UnitKind::ChristianEliteShockCavalry => "christian_elite_shock_cavalry",
        UnitKind::ChristianDivineSpeaker => "christian_divine_speaker",
        UnitKind::ChristianDivineJudge => "christian_divine_judge",
        UnitKind::MuslimCitadelGuard => "muslim_citadel_guard",
        UnitKind::MuslimArmoredHalberdier => "muslim_armored_halberdier",
        UnitKind::MuslimEliteHeavyKnight => "muslim_elite_heavy_knight",
        UnitKind::MuslimGodsChosen => "muslim_gods_chosen",
        UnitKind::MuslimEliteLongbowman => "muslim_elite_longbowman",
        UnitKind::MuslimSiegeCrossbowman => "muslim_siege_crossbowman",
        UnitKind::MuslimEliteHoundmaster => "muslim_elite_houndmaster",
        UnitKind::MuslimEliteShockCavalry => "muslim_elite_shock_cavalry",
        UnitKind::MuslimDivineSpeaker => "muslim_divine_speaker",
        UnitKind::MuslimDivineJudge => "muslim_divine_judge",
        _ => base.id.as_str(),
    }
    .to_string();

    match kind {
        UnitKind::ChristianCitadelGuard | UnitKind::MuslimCitadelGuard => {
            stats.max_hp = (base.max_hp * 1.20).max(base.max_hp + 16.0);
            stats.armor = base.armor + 2.4;
            stats.damage = (base.damage * 1.10).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.94).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 0.97).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
        }
        UnitKind::ChristianArmoredHalberdier | UnitKind::MuslimArmoredHalberdier => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 14.0);
            stats.armor = base.armor + 2.1;
            stats.damage = (base.damage * 1.16).max(base.damage + 1.4);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.90).max(0.08);
            stats.attack_range = base.attack_range + 6.0;
            stats.move_speed = (base.move_speed * 1.01).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
        }
        UnitKind::ChristianEliteHeavyKnight | UnitKind::MuslimEliteHeavyKnight => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 14.0);
            stats.armor = base.armor + 2.5;
            stats.damage = (base.damage * 1.22).max(base.damage + 1.8);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.88).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
        }
        UnitKind::ChristianGodsChosen | UnitKind::MuslimGodsChosen => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 14.0);
            stats.armor = base.armor + 2.0;
            stats.damage = 0.0;
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.95).max(0.08);
            stats.attack_range = 20.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.14).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianEliteLongbowman | UnitKind::MuslimEliteLongbowman => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.2;
            stats.damage = (base.damage * 1.12).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.91).max(0.08);
            stats.move_speed = (base.move_speed * 1.03).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.24).max(base.ranged_attack_damage + 3.0);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.89).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 38.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 22.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 48.0;
        }
        UnitKind::ChristianSiegeCrossbowman | UnitKind::MuslimSiegeCrossbowman => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.9;
            stats.damage = (base.damage * 1.14).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.93).max(0.08);
            stats.move_speed = (base.move_speed * 0.99).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.28).max(base.ranged_attack_damage + 3.2);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.90).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 24.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 28.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 42.0;
        }
        UnitKind::ChristianEliteHoundmaster | UnitKind::MuslimEliteHoundmaster => {
            stats.max_hp = (base.max_hp * 1.16).max(base.max_hp + 12.0);
            stats.armor = base.armor + 1.2;
            stats.damage = (base.damage * 1.14).max(base.damage + 1.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.90).max(0.08);
            stats.move_speed = (base.move_speed * 1.07).max(1.0);
            stats.morale = (base.morale * 1.10).max(1.0);
            stats.ranged_attack_damage =
                (base.ranged_attack_damage * 1.22).max(base.ranged_attack_damage + 2.4);
            stats.ranged_attack_cooldown_secs = (base.ranged_attack_cooldown_secs * 0.87).max(0.08);
            stats.ranged_attack_range = base.ranged_attack_range + 24.0;
            stats.ranged_projectile_speed = base.ranged_projectile_speed + 32.0;
            stats.ranged_projectile_max_distance = base.ranged_projectile_max_distance + 38.0;
        }
        UnitKind::ChristianEliteShockCavalry | UnitKind::MuslimEliteShockCavalry => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 14.0);
            stats.armor = base.armor + 2.0;
            stats.damage = (base.damage * 1.24).max(base.damage + 2.0);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.87).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.16).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianDivineSpeaker | UnitKind::MuslimDivineSpeaker => {
            stats.max_hp = (base.max_hp * 1.20).max(base.max_hp + 14.0);
            stats.armor = base.armor + 2.0;
            stats.damage = 0.0;
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.93).max(0.08);
            stats.attack_range = 20.0;
            stats.move_speed = (base.move_speed * 1.02).max(1.0);
            stats.morale = (base.morale * 1.14).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        UnitKind::ChristianDivineJudge | UnitKind::MuslimDivineJudge => {
            stats.max_hp = (base.max_hp * 1.18).max(base.max_hp + 16.0);
            stats.armor = 0.0;
            stats.damage = (base.damage * 1.28).max(base.damage + 2.2);
            stats.attack_cooldown_secs = (base.attack_cooldown_secs * 0.85).max(0.08);
            stats.attack_range = base.attack_range + 2.0;
            stats.move_speed = (base.move_speed * 1.06).max(1.0);
            stats.morale = (base.morale * 1.12).max(1.0);
            stats.ranged_attack_damage = 0.0;
            stats.ranged_attack_cooldown_secs = 0.0;
            stats.ranged_attack_range = 0.0;
            stats.ranged_projectile_speed = 0.0;
            stats.ranged_projectile_max_distance = 0.0;
        }
        _ => {}
    }

    Some(stats)
}

fn friendly_profile_for_kind(
    data: &GameData,
    art: &ArtAssets,
    kind: UnitKind,
) -> Option<FriendlyKindProfile> {
    let stats = friendly_stats_for_kind(data, kind)?;
    let (
        texture,
        collider_radius,
        has_melee,
        has_ranged,
        priest_kind,
        tracker_kind,
        scout_kind,
        armor_locked_zero,
    ) = match kind {
        UnitKind::ChristianPeasantInfantry
        | UnitKind::ChristianMenAtArms
        | UnitKind::ChristianShieldInfantry
        | UnitKind::ChristianExperiencedShieldInfantry
        | UnitKind::ChristianEliteShieldInfantry
        | UnitKind::ChristianSpearman
        | UnitKind::ChristianShieldedSpearman
        | UnitKind::ChristianHalberdier
        | UnitKind::ChristianUnmountedKnight
        | UnitKind::ChristianKnight
        | UnitKind::ChristianHeavyKnight
        | UnitKind::ChristianCitadelGuard
        | UnitKind::ChristianArmoredHalberdier
        | UnitKind::ChristianEliteHeavyKnight => (
            art.friendly_peasant_infantry_idle.clone(),
            12.0,
            true,
            false,
            false,
            false,
            false,
            false,
        ),
        UnitKind::ChristianPeasantArcher
        | UnitKind::ChristianBowman
        | UnitKind::ChristianExperiencedBowman
        | UnitKind::ChristianEliteBowman
        | UnitKind::ChristianLongbowman
        | UnitKind::ChristianCrossbowman
        | UnitKind::ChristianArmoredCrossbowman
        | UnitKind::ChristianEliteCrossbowman
        | UnitKind::ChristianSiegeCrossbowman
        | UnitKind::ChristianTracker
        | UnitKind::ChristianPathfinder
        | UnitKind::ChristianHoundmaster
        | UnitKind::ChristianEliteHoundmaster
        | UnitKind::ChristianScout
        | UnitKind::ChristianMountedScout
        | UnitKind::ChristianShockCavalry
        | UnitKind::ChristianEliteLongbowman
        | UnitKind::ChristianEliteShockCavalry => (
            art.friendly_peasant_archer_idle.clone(),
            11.0,
            true,
            !matches!(
                kind,
                UnitKind::ChristianScout
                    | UnitKind::ChristianMountedScout
                    | UnitKind::ChristianShockCavalry
                    | UnitKind::ChristianEliteShockCavalry
            ),
            false,
            matches!(
                kind,
                UnitKind::ChristianTracker
                    | UnitKind::ChristianPathfinder
                    | UnitKind::ChristianHoundmaster
                    | UnitKind::ChristianEliteHoundmaster
            ),
            matches!(
                kind,
                UnitKind::ChristianScout
                    | UnitKind::ChristianMountedScout
                    | UnitKind::ChristianShockCavalry
                    | UnitKind::ChristianEliteShockCavalry
            ),
            false,
        ),
        UnitKind::ChristianPeasantPriest
        | UnitKind::ChristianDevoted
        | UnitKind::ChristianSquire
        | UnitKind::ChristianBannerman
        | UnitKind::ChristianEliteBannerman
        | UnitKind::ChristianGodsChosen
        | UnitKind::ChristianDevotedOne
        | UnitKind::ChristianCardinal
        | UnitKind::ChristianEliteCardinal
        | UnitKind::ChristianDivineSpeaker
        | UnitKind::ChristianFanatic
        | UnitKind::ChristianFlagellant
        | UnitKind::ChristianEliteFlagellant
        | UnitKind::ChristianDivineJudge => (
            art.friendly_peasant_priest_idle.clone(),
            11.0,
            matches!(
                kind,
                UnitKind::ChristianFanatic
                    | UnitKind::ChristianFlagellant
                    | UnitKind::ChristianEliteFlagellant
                    | UnitKind::ChristianDivineJudge
            ),
            false,
            matches!(
                kind,
                UnitKind::ChristianPeasantPriest
                    | UnitKind::ChristianDevoted
                    | UnitKind::ChristianSquire
                    | UnitKind::ChristianBannerman
                    | UnitKind::ChristianEliteBannerman
                    | UnitKind::ChristianGodsChosen
                    | UnitKind::ChristianDevotedOne
                    | UnitKind::ChristianCardinal
                    | UnitKind::ChristianEliteCardinal
                    | UnitKind::ChristianDivineSpeaker
            ),
            false,
            false,
            matches!(
                kind,
                UnitKind::ChristianFanatic
                    | UnitKind::ChristianFlagellant
                    | UnitKind::ChristianEliteFlagellant
                    | UnitKind::ChristianDivineJudge
            ),
        ),
        UnitKind::MuslimPeasantInfantry
        | UnitKind::MuslimMenAtArms
        | UnitKind::MuslimShieldInfantry
        | UnitKind::MuslimExperiencedShieldInfantry
        | UnitKind::MuslimEliteShieldInfantry
        | UnitKind::MuslimSpearman
        | UnitKind::MuslimShieldedSpearman
        | UnitKind::MuslimHalberdier
        | UnitKind::MuslimUnmountedKnight
        | UnitKind::MuslimKnight
        | UnitKind::MuslimHeavyKnight
        | UnitKind::MuslimCitadelGuard
        | UnitKind::MuslimArmoredHalberdier
        | UnitKind::MuslimEliteHeavyKnight => (
            art.muslim_peasant_infantry_idle.clone(),
            12.0,
            true,
            false,
            false,
            false,
            false,
            false,
        ),
        UnitKind::MuslimPeasantArcher
        | UnitKind::MuslimBowman
        | UnitKind::MuslimExperiencedBowman
        | UnitKind::MuslimEliteBowman
        | UnitKind::MuslimLongbowman
        | UnitKind::MuslimCrossbowman
        | UnitKind::MuslimArmoredCrossbowman
        | UnitKind::MuslimEliteCrossbowman
        | UnitKind::MuslimSiegeCrossbowman
        | UnitKind::MuslimTracker
        | UnitKind::MuslimPathfinder
        | UnitKind::MuslimHoundmaster
        | UnitKind::MuslimEliteHoundmaster
        | UnitKind::MuslimScout
        | UnitKind::MuslimMountedScout
        | UnitKind::MuslimShockCavalry
        | UnitKind::MuslimEliteLongbowman
        | UnitKind::MuslimEliteShockCavalry => (
            art.muslim_peasant_archer_idle.clone(),
            11.0,
            true,
            !matches!(
                kind,
                UnitKind::MuslimScout
                    | UnitKind::MuslimMountedScout
                    | UnitKind::MuslimShockCavalry
                    | UnitKind::MuslimEliteShockCavalry
            ),
            false,
            matches!(
                kind,
                UnitKind::MuslimTracker
                    | UnitKind::MuslimPathfinder
                    | UnitKind::MuslimHoundmaster
                    | UnitKind::MuslimEliteHoundmaster
            ),
            matches!(
                kind,
                UnitKind::MuslimScout
                    | UnitKind::MuslimMountedScout
                    | UnitKind::MuslimShockCavalry
                    | UnitKind::MuslimEliteShockCavalry
            ),
            false,
        ),
        UnitKind::MuslimPeasantPriest
        | UnitKind::MuslimDevoted
        | UnitKind::MuslimSquire
        | UnitKind::MuslimBannerman
        | UnitKind::MuslimEliteBannerman
        | UnitKind::MuslimGodsChosen
        | UnitKind::MuslimDevotedOne
        | UnitKind::MuslimCardinal
        | UnitKind::MuslimEliteCardinal
        | UnitKind::MuslimDivineSpeaker
        | UnitKind::MuslimFanatic
        | UnitKind::MuslimFlagellant
        | UnitKind::MuslimEliteFlagellant
        | UnitKind::MuslimDivineJudge => (
            art.muslim_peasant_priest_idle.clone(),
            11.0,
            matches!(
                kind,
                UnitKind::MuslimFanatic
                    | UnitKind::MuslimFlagellant
                    | UnitKind::MuslimEliteFlagellant
                    | UnitKind::MuslimDivineJudge
            ),
            false,
            matches!(
                kind,
                UnitKind::MuslimPeasantPriest
                    | UnitKind::MuslimDevoted
                    | UnitKind::MuslimSquire
                    | UnitKind::MuslimBannerman
                    | UnitKind::MuslimEliteBannerman
                    | UnitKind::MuslimGodsChosen
                    | UnitKind::MuslimDevotedOne
                    | UnitKind::MuslimCardinal
                    | UnitKind::MuslimEliteCardinal
                    | UnitKind::MuslimDivineSpeaker
            ),
            false,
            false,
            matches!(
                kind,
                UnitKind::MuslimFanatic
                    | UnitKind::MuslimFlagellant
                    | UnitKind::MuslimEliteFlagellant
                    | UnitKind::MuslimDivineJudge
            ),
        ),
        _ => return None,
    };

    Some(FriendlyKindProfile {
        stats,
        texture,
        collider_radius,
        has_melee,
        has_ranged,
        priest_kind,
        tracker_kind,
        scout_kind,
        armor_locked_zero,
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_friendly_kind_loadout(
    commands: &mut Commands,
    entity: Entity,
    updated_unit: Unit,
    target_tier: u8,
    level_cost: u32,
    cfg: &UnitStatsConfig,
    texture: Handle<Image>,
    collider_radius: f32,
    has_melee: bool,
    has_ranged: bool,
    priest_kind: bool,
    tracker_kind: bool,
    scout_kind: bool,
    armor_locked_zero: bool,
    ability_tuning: &RosterBehaviorConfig,
) {
    commands.entity(entity).insert((
        updated_unit,
        UnitTier(target_tier),
        UnitLevelCost(level_cost),
        Health::new(cfg.max_hp),
        BaseMaxHealth(cfg.max_hp),
        Morale::new(cfg.morale),
        Armor(if armor_locked_zero { 0.0 } else { cfg.armor }),
        MoveSpeed(cfg.move_speed),
        ColliderRadius(collider_radius),
        texture,
    ));

    if has_melee {
        commands.entity(entity).insert((
            AttackProfile {
                damage: cfg.damage,
                range: cfg.attack_range,
                cooldown_secs: cfg.attack_cooldown_secs,
            },
            AttackCooldown(Timer::from_seconds(
                cfg.attack_cooldown_secs,
                TimerMode::Repeating,
            )),
        ));
    } else {
        commands
            .entity(entity)
            .remove::<(AttackProfile, AttackCooldown)>();
    }

    if has_ranged {
        commands.entity(entity).insert((
            RangedAttackProfile {
                damage: cfg.ranged_attack_damage,
                range: cfg.ranged_attack_range,
                projectile_speed: cfg.ranged_projectile_speed,
                projectile_max_distance: cfg.ranged_projectile_max_distance,
            },
            RangedAttackCooldown(Timer::from_seconds(
                cfg.ranged_attack_cooldown_secs,
                TimerMode::Repeating,
            )),
        ));
    } else {
        commands
            .entity(entity)
            .remove::<(RangedAttackProfile, RangedAttackCooldown)>();
    }

    if priest_kind {
        commands.entity(entity).insert(PriestSupportCaster {
            cooldown: PRIEST_BLESSING_COOLDOWN_SECS,
        });
    } else {
        commands
            .entity(entity)
            .remove::<(PriestSupportCaster, PriestAttackSpeedBlessing)>();
    }

    if tracker_kind {
        commands.entity(entity).insert(TrackerHoundSummoner {
            cooldown_secs: ability_tuning.tracker_hound_cooldown_secs,
            active_secs: 0.0,
            strike_cooldown_secs: 0.0,
        });
    } else {
        commands.entity(entity).remove::<TrackerHoundSummoner>();
    }

    if scout_kind {
        commands.entity(entity).insert(ScoutRaidBehavior {
            cooldown_secs: ability_tuning.scout_raid_cooldown_secs,
            active_secs: 0.0,
        });
    } else {
        commands
            .entity(entity)
            .remove::<(ScoutRaidBehavior, OutOfFormation)>();
    }

    if armor_locked_zero {
        commands
            .entity(entity)
            .insert((ArmorLockedZero, Armor(0.0)));
    } else {
        commands.entity(entity).remove::<ArmorLockedZero>();
    }
}

pub fn priest_attack_speed_multiplier(active_blessing: Option<&PriestAttackSpeedBlessing>) -> f32 {
    if active_blessing
        .map(|blessing| blessing.remaining_secs > 0.0)
        .unwrap_or(false)
    {
        PRIEST_BLESSING_ATTACK_SPEED_MULTIPLIER
    } else {
        1.0
    }
}

#[allow(dead_code)]
fn _satisfy_query_markers(_enemy: Option<EnemyUnit>, _rescue: Option<RescuableUnit>) {}

#[cfg(test)]
mod tests {
    use bevy::ecs::event::ManualEventReader;
    use bevy::prelude::*;

    use crate::combat::RangedAttackProfile;
    use crate::data::GameData;
    use crate::formation::{ActiveFormation, FormationModifiers};
    use crate::model::{
        Armor, AttackProfile, CommanderUnit, DamageEvent, EnemyUnit, FriendlyUnit, GameState,
        Health, MoveSpeed, RecruitEvent, RecruitUnitKind, StartRunEvent, Team, Unit, UnitKind,
        UnitTier,
    };
    use crate::squad::{
        ArmorLockedZero, ConvertTierZeroUnitsEvent, OutOfFormation, PriestSupportCaster,
        PromoteUnitsEvent, RosterEconomy, RosterEconomyFeedback, ScoutRaidBehavior, SquadPlugin,
        TrackerHoundSummoner, enemy_inside_active_formation, is_upgrade_tier_unlocked,
        movement_multiplier_from_inside_enemy_count, priest_should_cast,
        refresh_priest_blessing_remaining, tick_priest_cooldown, tier0_conversion_gold_cost,
        unlock_boss_wave_for_tier, unlocked_upgrade_tier_for_major_wave,
    };
    use crate::upgrades::Progression;
    use crate::visuals::ArtAssets;

    #[test]
    fn starts_with_only_commander_on_run_start() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();

        let count = {
            let world = app.world_mut();
            let mut query = world.query_filtered::<Entity, With<CommanderUnit>>();
            query.iter(world).count()
        };
        assert_eq!(count, 1);
    }

    #[test]
    fn formation_enemy_slowdown_caps_at_minimum_multiplier() {
        assert!((movement_multiplier_from_inside_enemy_count(0) - 1.0).abs() < 0.001);
        assert!(movement_multiplier_from_inside_enemy_count(4) < 1.0);
        assert!((movement_multiplier_from_inside_enemy_count(40) - 0.5).abs() < 0.001);
    }

    #[test]
    fn enemy_inside_formation_check_requires_recruits() {
        assert!(!enemy_inside_active_formation(
            Vec2::ZERO,
            Vec2::new(10.0, 5.0),
            0,
            30.0,
            ActiveFormation::Square,
        ));
        assert!(enemy_inside_active_formation(
            Vec2::ZERO,
            Vec2::new(8.0, 6.0),
            9,
            30.0,
            ActiveFormation::Square,
        ));
    }

    #[test]
    fn archer_recruit_gets_ranged_attack_profile() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(24.0, 8.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantArcher,
        });
        app.update();

        let found_archer_with_ranged = {
            let world = app.world_mut();
            let mut query = world.query::<(&crate::model::Unit, Option<&RangedAttackProfile>)>();
            query.iter(world).any(|(unit, ranged_profile)| {
                unit.kind == crate::model::UnitKind::ChristianPeasantArcher
                    && ranged_profile.is_some()
            })
        };
        assert!(found_archer_with_ranged);
    }

    #[test]
    fn priest_recruit_starts_as_support_without_direct_attack_profiles() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(24.0, 8.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantPriest,
        });
        app.update();

        let mut found = false;
        let mut has_support = false;
        let mut has_direct_attack = false;
        {
            let world = app.world_mut();
            let mut query = world.query::<(
                &crate::model::Unit,
                Option<&AttackProfile>,
                Option<&RangedAttackProfile>,
                Option<&PriestSupportCaster>,
            )>();
            for (unit, melee, ranged, support) in query.iter(world) {
                if unit.kind != UnitKind::ChristianPeasantPriest {
                    continue;
                }
                found = true;
                has_support |= support.is_some();
                has_direct_attack |= melee.is_some() || ranged.is_some();
            }
        }

        assert!(found);
        assert!(has_support);
        assert!(!has_direct_attack);
    }

    #[test]
    fn priest_support_logic_applies_blessing_to_nearby_friendlies() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, super::run_priest_support_logic);

        let _priest = app.world_mut().spawn((
            Unit {
                team: Team::Friendly,
                kind: UnitKind::ChristianPeasantPriest,
                level: 1,
            },
            FriendlyUnit,
            Transform::from_xyz(0.0, 0.0, 0.0),
            PriestSupportCaster { cooldown: 0.0 },
        ));
        let ally = app
            .world_mut()
            .spawn((
                Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianPeasantInfantry,
                    level: 1,
                },
                FriendlyUnit,
                Transform::from_xyz(30.0, 0.0, 0.0),
            ))
            .id();

        app.update();

        let blessing = app.world().get::<super::PriestAttackSpeedBlessing>(ally);
        assert!(blessing.is_some());
    }

    #[test]
    fn enemy_priest_support_logic_applies_blessing_to_nearby_enemies_only() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, super::run_priest_support_logic);

        let _enemy_priest = app.world_mut().spawn((
            Unit {
                team: Team::Enemy,
                kind: UnitKind::MuslimPeasantPriest,
                level: 1,
            },
            EnemyUnit,
            Transform::from_xyz(0.0, 0.0, 0.0),
            PriestSupportCaster { cooldown: 0.0 },
        ));
        let enemy_ally = app
            .world_mut()
            .spawn((
                Unit {
                    team: Team::Enemy,
                    kind: UnitKind::MuslimPeasantInfantry,
                    level: 1,
                },
                EnemyUnit,
                Transform::from_xyz(30.0, 0.0, 0.0),
            ))
            .id();
        let friendly_unit = app
            .world_mut()
            .spawn((
                Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianPeasantInfantry,
                    level: 1,
                },
                FriendlyUnit,
                Transform::from_xyz(30.0, 0.0, 0.0),
            ))
            .id();

        app.update();

        let enemy_blessing = app
            .world()
            .get::<super::PriestAttackSpeedBlessing>(enemy_ally);
        let friendly_blessing = app
            .world()
            .get::<super::PriestAttackSpeedBlessing>(friendly_unit);
        assert!(enemy_blessing.is_some());
        assert!(friendly_blessing.is_none());
    }

    #[test]
    fn locked_level_budget_updates_on_recruit_and_death() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(16.0, 0.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantArcher,
        });
        app.update();

        let economy_after_recruit = app.world().resource::<RosterEconomy>();
        assert_eq!(economy_after_recruit.locked_levels, 0);
        assert_eq!(economy_after_recruit.allowed_max_level, 100);

        let archer_entity = {
            let world = app.world_mut();
            let mut query = world.query::<(Entity, &Unit)>();
            query
                .iter(world)
                .find_map(|(entity, unit)| {
                    (unit.kind == UnitKind::ChristianPeasantArcher).then_some(entity)
                })
                .expect("expected recruited archer")
        };

        app.world_mut()
            .entity_mut(archer_entity)
            .despawn_recursive();
        app.update();

        let economy_after_death = app.world().resource::<RosterEconomy>();
        assert_eq!(economy_after_death.locked_levels, 0);
        assert_eq!(economy_after_death.allowed_max_level, 100);
    }

    #[test]
    fn promotion_is_blocked_when_level_budget_would_be_exceeded() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 0.0,
            level: 100,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantInfantry,
        });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantInfantry,
            to_kind: UnitKind::ChristianPeasantArcher,
            count: 1,
        });
        app.update();

        let mut archer_count = 0usize;
        let mut infantry_count = 0usize;
        {
            let world = app.world_mut();
            let mut query = world.query::<(&Unit, Option<&FriendlyUnit>)>();
            for (unit, friendly) in query.iter(world) {
                if friendly.is_none() {
                    continue;
                }
                match unit.kind {
                    UnitKind::ChristianPeasantArcher => archer_count += 1,
                    UnitKind::ChristianPeasantInfantry => infantry_count += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(archer_count, 0);
        assert!(infantry_count >= 1);

        let feedback = app.world().resource::<RosterEconomyFeedback>();
        assert!(feedback.blocked_upgrade_reason.is_some());
    }

    #[test]
    fn promotion_step_cost_rejects_non_upgrade_paths() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantInfantry,
                UnitKind::ChristianPeasantArcher
            ),
            None
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantArcher,
                UnitKind::ChristianPeasantPriest
            ),
            None
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantInfantry,
                UnitKind::MuslimPeasantArcher
            ),
            None
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantInfantry,
                UnitKind::ChristianBowman
            ),
            None
        );
    }

    #[test]
    fn promotion_step_cost_accepts_tier1_branch_paths() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantInfantry,
                UnitKind::ChristianMenAtArms
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantArcher,
                UnitKind::ChristianBowman
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPeasantPriest,
                UnitKind::ChristianDevoted
            ),
            Some(1)
        );
    }

    #[test]
    fn promotion_step_cost_accepts_tier2_branch_paths() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianMenAtArms,
                UnitKind::ChristianShieldInfantry
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianBowman,
                UnitKind::ChristianTracker
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianDevoted,
                UnitKind::ChristianFanatic
            ),
            Some(1)
        );
    }

    #[test]
    fn promotion_step_cost_accepts_tier3_branch_paths() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianShieldInfantry,
                UnitKind::ChristianExperiencedShieldInfantry
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianTracker,
                UnitKind::ChristianPathfinder
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianFanatic,
                UnitKind::ChristianFlagellant
            ),
            Some(1)
        );
    }

    #[test]
    fn promotion_step_cost_accepts_tier4_branch_paths() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianExperiencedShieldInfantry,
                UnitKind::ChristianEliteShieldInfantry
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPathfinder,
                UnitKind::ChristianHoundmaster
            ),
            Some(1)
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianFlagellant,
                UnitKind::ChristianEliteFlagellant
            ),
            Some(1)
        );
    }

    #[test]
    fn promotion_step_cost_rejects_illegal_tier2_cross_links() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianBowman,
                UnitKind::ChristianSpearman
            ),
            None
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianDevoted,
                UnitKind::ChristianTracker
            ),
            None
        );
    }

    #[test]
    fn promotion_step_cost_rejects_illegal_tier3_cross_links() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianShieldInfantry,
                UnitKind::ChristianPathfinder
            ),
            None
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianTracker,
                UnitKind::ChristianCardinal
            ),
            None
        );
    }

    #[test]
    fn promotion_step_cost_rejects_illegal_tier4_cross_links() {
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianPathfinder,
                UnitKind::ChristianEliteCardinal
            ),
            None
        );
        assert_eq!(
            crate::squad::promotion_step_cost(
                UnitKind::ChristianCardinal,
                UnitKind::ChristianHeavyKnight
            ),
            None
        );
    }

    #[test]
    fn tracker_hound_logic_emits_damage_when_active() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.add_event::<DamageEvent>();
        app.add_systems(Update, super::run_tracker_hound_logic);

        let enemy = app
            .world_mut()
            .spawn((
                Unit {
                    team: Team::Enemy,
                    kind: UnitKind::MuslimPeasantInfantry,
                    level: 1,
                },
                Health::new(120.0),
                Transform::from_xyz(20.0, 0.0, 0.0),
            ))
            .id();
        app.world_mut().spawn((
            Unit {
                team: Team::Friendly,
                kind: UnitKind::ChristianTracker,
                level: 1,
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
            AttackProfile {
                damage: 20.0,
                range: 30.0,
                cooldown_secs: 0.5,
            },
            TrackerHoundSummoner {
                cooldown_secs: 0.0,
                active_secs: 1.0,
                strike_cooldown_secs: 0.0,
            },
        ));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.1));

        app.update();

        let events = app.world().resource::<Events<DamageEvent>>();
        let mut reader = ManualEventReader::<DamageEvent>::default();
        let emitted = reader.read(events).copied().collect::<Vec<_>>();
        assert!(!emitted.is_empty());
        assert!(emitted.iter().any(|event| {
            event.target == enemy
                && event.source_team == Team::Friendly
                && event.amount >= 1.0
                && !event.critical
        }));
    }

    #[test]
    fn scout_raid_behavior_toggles_out_of_formation_and_returns() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.add_systems(Update, super::run_scout_raid_behavior);

        let scout = app
            .world_mut()
            .spawn((
                Unit {
                    team: Team::Friendly,
                    kind: UnitKind::ChristianScout,
                    level: 1,
                },
                MoveSpeed(120.0),
                Transform::from_xyz(0.0, 0.0, 0.0),
                ScoutRaidBehavior {
                    cooldown_secs: 0.0,
                    active_secs: 0.0,
                },
            ))
            .id();
        app.world_mut().spawn((
            Unit {
                team: Team::Enemy,
                kind: UnitKind::MuslimPeasantInfantry,
                level: 1,
            },
            Health::new(120.0),
            Transform::from_xyz(150.0, 0.0, 0.0),
        ));

        app.update();
        assert!(app.world().get::<OutOfFormation>(scout).is_some());

        if let Some(mut behavior) = app.world_mut().get_mut::<ScoutRaidBehavior>(scout) {
            behavior.active_secs = 0.0;
            behavior.cooldown_secs = 20.0;
        }
        app.update();
        assert!(app.world().get::<OutOfFormation>(scout).is_none());
    }

    #[test]
    fn tier3_branch_promotions_preserve_tracker_and_scout_behaviors() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 1_200.0,
            level: 40,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantArcher,
        });
        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(16.0, 8.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantArcher,
        });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 10 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 20 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 30 });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantArcher,
            to_kind: UnitKind::ChristianBowman,
            count: 2,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianBowman,
            to_kind: UnitKind::ChristianTracker,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianBowman,
            to_kind: UnitKind::ChristianScout,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianTracker,
            to_kind: UnitKind::ChristianPathfinder,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianScout,
            to_kind: UnitKind::ChristianMountedScout,
            count: 1,
        });
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(
            &Unit,
            &UnitTier,
            Option<&TrackerHoundSummoner>,
            Option<&ScoutRaidBehavior>,
        )>();
        let pathfinder = query
            .iter(world)
            .find(|(unit, _, _, _)| unit.kind == UnitKind::ChristianPathfinder)
            .expect("pathfinder should exist");
        assert_eq!(pathfinder.1.0, 3);
        assert!(pathfinder.2.is_some());
        let mounted_scout = query
            .iter(world)
            .find(|(unit, _, _, _)| unit.kind == UnitKind::ChristianMountedScout)
            .expect("mounted scout should exist");
        assert_eq!(mounted_scout.1.0, 3);
        assert!(mounted_scout.3.is_some());
    }

    #[test]
    fn tier4_branch_promotions_preserve_tracker_and_scout_behaviors() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 2_400.0,
            level: 60,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantArcher,
        });
        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(16.0, 8.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantArcher,
        });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 10 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 20 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 30 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 40 });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantArcher,
            to_kind: UnitKind::ChristianBowman,
            count: 2,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianBowman,
            to_kind: UnitKind::ChristianTracker,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianBowman,
            to_kind: UnitKind::ChristianScout,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianTracker,
            to_kind: UnitKind::ChristianPathfinder,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianScout,
            to_kind: UnitKind::ChristianMountedScout,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPathfinder,
            to_kind: UnitKind::ChristianHoundmaster,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianMountedScout,
            to_kind: UnitKind::ChristianShockCavalry,
            count: 1,
        });
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(
            &Unit,
            &UnitTier,
            Option<&TrackerHoundSummoner>,
            Option<&ScoutRaidBehavior>,
        )>();
        let houndmaster = query
            .iter(world)
            .find(|(unit, _, _, _)| unit.kind == UnitKind::ChristianHoundmaster)
            .expect("houndmaster should exist");
        assert_eq!(houndmaster.1.0, 4);
        assert!(houndmaster.2.is_some());
        let shock_cavalry = query
            .iter(world)
            .find(|(unit, _, _, _)| unit.kind == UnitKind::ChristianShockCavalry)
            .expect("shock cavalry should exist");
        assert_eq!(shock_cavalry.1.0, 4);
        assert!(shock_cavalry.3.is_some());
    }

    #[test]
    fn tier1_promotion_requires_unlock_then_succeeds() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 150.0,
            level: 10,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantInfantry,
        });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantInfantry,
            to_kind: UnitKind::ChristianMenAtArms,
            count: 1,
        });
        app.update();

        let blocked_feedback = app
            .world()
            .resource::<RosterEconomyFeedback>()
            .blocked_upgrade_reason
            .clone()
            .unwrap_or_default();
        assert!(blocked_feedback.contains("unlock"));

        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 10 });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantInfantry,
            to_kind: UnitKind::ChristianMenAtArms,
            count: 1,
        });
        app.update();

        let mut tier1_count = 0usize;
        let mut tier0_count = 0usize;
        {
            let world = app.world_mut();
            let mut query = world.query::<(&Unit, &UnitTier, Option<&FriendlyUnit>)>();
            for (unit, tier, friendly) in query.iter(world) {
                if friendly.is_none() {
                    continue;
                }
                match unit.kind {
                    UnitKind::ChristianMenAtArms => {
                        tier1_count += 1;
                        assert_eq!(tier.0, 1);
                    }
                    UnitKind::ChristianPeasantInfantry => {
                        tier0_count += 1;
                        assert_eq!(tier.0, 0);
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(tier1_count, 1);
        assert_eq!(tier0_count, 0);
    }

    #[test]
    fn fanatic_promotion_applies_armor_lock() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 450.0,
            level: 30,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantPriest,
        });
        app.update();

        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 10 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 20 });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantPriest,
            to_kind: UnitKind::ChristianDevoted,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianDevoted,
            to_kind: UnitKind::ChristianFanatic,
            count: 1,
        });
        app.update();

        let world = app.world_mut();
        let mut query = world.query::<(&Unit, Option<&ArmorLockedZero>, Option<&Armor>)>();
        let fanatic = query
            .iter(world)
            .find(|(unit, _, _)| unit.kind == UnitKind::ChristianFanatic)
            .expect("fanatic should exist after promotion");
        assert!(fanatic.1.is_some());
        assert_eq!(fanatic.2.map(|armor| armor.0), Some(0.0));
    }

    #[test]
    fn elite_flagellant_promotion_keeps_armor_lock() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 950.0,
            level: 50,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantPriest,
        });
        app.update();

        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 10 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 20 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 30 });
        app.world_mut()
            .send_event(crate::enemies::WaveCompletedEvent { wave_number: 40 });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantPriest,
            to_kind: UnitKind::ChristianDevoted,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianDevoted,
            to_kind: UnitKind::ChristianFanatic,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianFanatic,
            to_kind: UnitKind::ChristianFlagellant,
            count: 1,
        });
        app.update();
        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianFlagellant,
            to_kind: UnitKind::ChristianEliteFlagellant,
            count: 1,
        });
        app.update();

        let world = app.world_mut();
        let mut query =
            world.query::<(&Unit, &UnitTier, Option<&ArmorLockedZero>, Option<&Armor>)>();
        let elite_flagellant = query
            .iter(world)
            .find(|(unit, _, _, _)| unit.kind == UnitKind::ChristianEliteFlagellant)
            .expect("elite flagellant should exist after promotion");
        assert_eq!(elite_flagellant.1.0, 4);
        assert!(elite_flagellant.2.is_some());
        assert_eq!(elite_flagellant.3.map(|armor| armor.0), Some(0.0));
    }

    #[test]
    fn tier0_conversion_cost_has_fixed_gold_price() {
        assert!((tier0_conversion_gold_cost() - 18.0).abs() < 0.001);
    }

    #[test]
    fn tier0_conversion_swaps_unit_and_consumes_gold() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 30.0,
            level: 8,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantInfantry,
        });
        app.update();

        app.world_mut().send_event(ConvertTierZeroUnitsEvent {
            from_kind: UnitKind::ChristianPeasantInfantry,
            to_kind: UnitKind::ChristianPeasantArcher,
            count: 1,
        });
        app.update();

        let mut infantry_count = 0usize;
        let mut archer_count = 0usize;
        {
            let world = app.world_mut();
            let mut query = world.query::<(&Unit, Option<&FriendlyUnit>)>();
            for (unit, friendly) in query.iter(world) {
                if friendly.is_none() {
                    continue;
                }
                match unit.kind {
                    UnitKind::ChristianPeasantInfantry => infantry_count += 1,
                    UnitKind::ChristianPeasantArcher => archer_count += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(archer_count, 1);
        assert_eq!(infantry_count, 0);

        let progression = app.world().resource::<Progression>();
        assert!((progression.gold - 12.0).abs() < 0.001);
    }

    #[test]
    fn tier0_conversion_is_blocked_when_gold_is_insufficient() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 4.0,
            level: 8,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantInfantry,
        });
        app.update();

        app.world_mut().send_event(ConvertTierZeroUnitsEvent {
            from_kind: UnitKind::ChristianPeasantInfantry,
            to_kind: UnitKind::ChristianPeasantArcher,
            count: 1,
        });
        app.update();

        let mut infantry_count = 0usize;
        let mut archer_count = 0usize;
        {
            let world = app.world_mut();
            let mut query = world.query::<(&Unit, Option<&FriendlyUnit>)>();
            for (unit, friendly) in query.iter(world) {
                if friendly.is_none() {
                    continue;
                }
                match unit.kind {
                    UnitKind::ChristianPeasantInfantry => infantry_count += 1,
                    UnitKind::ChristianPeasantArcher => archer_count += 1,
                    _ => {}
                }
            }
        }
        assert_eq!(archer_count, 0);
        assert_eq!(infantry_count, 1);
        let feedback = app.world().resource::<RosterEconomyFeedback>();
        assert!(
            feedback
                .blocked_upgrade_reason
                .as_deref()
                .unwrap_or_default()
                .contains("requires")
        );
    }

    #[test]
    fn priest_cast_timer_triggers_on_twenty_second_cadence() {
        let mut cooldown = 20.0;
        let mut elapsed = 0u32;
        let mut casts = Vec::new();
        while elapsed < 60 {
            elapsed += 1;
            cooldown = tick_priest_cooldown(cooldown, 1.0);
            if priest_should_cast(cooldown) {
                casts.push(elapsed);
                cooldown = 20.0;
            }
        }
        assert_eq!(casts, vec![20, 40, 60]);
    }

    #[test]
    fn priest_refresh_resets_blessing_duration_instead_of_stacking() {
        let first_apply = refresh_priest_blessing_remaining(0.0, 1.0);
        let overlap_refresh = refresh_priest_blessing_remaining(5.0, 1.0);
        assert!((first_apply - 10.0).abs() < 0.001);
        assert!((overlap_refresh - 10.0).abs() < 0.001);
    }

    #[test]
    fn priest_blessing_vfx_action_matches_blessing_state() {
        assert_eq!(
            super::priest_blessing_vfx_action(true, false),
            super::PriestBlessingVfxAction::Spawn
        );
        assert_eq!(
            super::priest_blessing_vfx_action(false, true),
            super::PriestBlessingVfxAction::Despawn
        );
        assert_eq!(
            super::priest_blessing_vfx_action(true, true),
            super::PriestBlessingVfxAction::Keep
        );
        assert_eq!(
            super::priest_blessing_vfx_action(false, false),
            super::PriestBlessingVfxAction::Keep
        );
    }

    #[test]
    fn priest_blessing_shadow_scale_respects_minimum_radius() {
        let below_min = super::priest_blessing_shadow_scale(2.0);
        let at_min = super::priest_blessing_shadow_scale(8.0);
        let larger = super::priest_blessing_shadow_scale(12.0);
        assert_eq!(below_min, at_min);
        assert!(larger.x > at_min.x);
        assert!(larger.y > at_min.y);
    }

    #[test]
    fn priest_promotion_event_is_blocked_without_paths() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.add_event::<StartRunEvent>();
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("data"),
        );
        app.insert_resource(ArtAssets::default());
        app.insert_resource(ActiveFormation::Square);
        app.insert_resource(FormationModifiers::default());
        app.insert_resource(Progression {
            gold: 0.0,
            level: 5,
            pending_level_ups: 0,
        });
        app.add_plugins(SquadPlugin);

        app.world_mut().send_event(StartRunEvent);
        app.update();
        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::InRun);
        app.update();

        app.world_mut().send_event(RecruitEvent {
            world_position: Vec2::new(10.0, 5.0),
            recruit_kind: RecruitUnitKind::ChristianPeasantInfantry,
        });
        app.update();

        app.world_mut().send_event(PromoteUnitsEvent {
            from_kind: UnitKind::ChristianPeasantInfantry,
            to_kind: UnitKind::ChristianPeasantPriest,
            count: 1,
        });
        app.update();

        let mut priest_count = 0usize;
        let mut infantry_count = 0usize;
        {
            let world = app.world_mut();
            let mut query = world.query::<(
                &Unit,
                Option<&AttackProfile>,
                Option<&RangedAttackProfile>,
                Option<&PriestSupportCaster>,
            )>();
            for (unit, melee, ranged, support) in query.iter(world) {
                match unit.kind {
                    UnitKind::ChristianPeasantPriest => {
                        priest_count += 1;
                        assert!(support.is_some());
                        assert!(melee.is_none());
                        assert!(ranged.is_none());
                    }
                    UnitKind::ChristianPeasantInfantry => {
                        infantry_count += 1;
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(priest_count, 0);
        assert!(infantry_count >= 1);
        let feedback = app.world().resource::<RosterEconomyFeedback>();
        assert!(feedback.blocked_upgrade_reason.is_some());
    }

    #[test]
    fn upgrade_tier_unlocks_follow_major_boss_defeats() {
        assert_eq!(unlocked_upgrade_tier_for_major_wave(1), 0);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(9), 0);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(10), 1);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(20), 2);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(30), 3);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(40), 4);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(50), 5);
        assert_eq!(unlocked_upgrade_tier_for_major_wave(100), 5);

        assert_eq!(unlock_boss_wave_for_tier(0), Some(0));
        assert_eq!(unlock_boss_wave_for_tier(1), Some(10));
        assert_eq!(unlock_boss_wave_for_tier(2), Some(20));
        assert_eq!(unlock_boss_wave_for_tier(5), Some(50));
        assert_eq!(unlock_boss_wave_for_tier(6), None);

        assert!(is_upgrade_tier_unlocked(0, 0));
        assert!(!is_upgrade_tier_unlocked(1, 0));
        assert!(is_upgrade_tier_unlocked(1, 1));
        assert!(!is_upgrade_tier_unlocked(3, 2));
        assert!(is_upgrade_tier_unlocked(3, 3));
    }
}
