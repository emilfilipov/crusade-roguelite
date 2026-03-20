use bevy::prelude::*;

use crate::banner::BannerMovementPenalty;
use crate::combat::{RangedAttackCooldown, RangedAttackProfile};
use crate::data::{GameData, UnitStatsConfig};
use crate::formation::{
    ActiveFormation, FormationModifiers, active_formation_config, formation_contains_position,
};
use crate::map::{MapBounds, playable_bounds};
use crate::model::{
    Armor, AttackCooldown, AttackProfile, BaseMaxHealth, ColliderRadius, CommanderUnit, EnemyUnit,
    FriendlyUnit, GameState, GlobalBuffs, Health, Morale, MoveSpeed, PlayerControlled,
    RecruitEvent, RecruitUnitKind, RescuableUnit, StartRunEvent, Team, Unit, UnitDiedEvent,
    UnitKind, UnitTier, level_cap_from_locked_budget,
};
use crate::upgrades::{ConditionalUpgradeEffects, Progression};
use crate::visuals::ArtAssets;

const ENEMY_INSIDE_FORMATION_SLOWDOWN_PER_UNIT: f32 = 0.04;
const ENEMY_INSIDE_FORMATION_MIN_SPEED_MULTIPLIER: f32 = 0.5;
const ENEMY_INSIDE_FORMATION_PADDING_SLOTS: f32 = 0.35;
const PRIEST_BLESSING_RANGE: f32 = 190.0;
const PRIEST_BLESSING_DURATION_SECS: f32 = 10.0;
const PRIEST_BLESSING_COOLDOWN_SECS: f32 = 20.0;
const PRIEST_BLESSING_ATTACK_SPEED_MULTIPLIER: f32 = 1.24;

#[derive(Resource, Clone, Debug, Default)]
pub struct SquadRoster {
    pub commander: Option<Entity>,
    pub friendly_count: usize,
    pub casualties: u32,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct RosterEconomy {
    pub locked_levels: u32,
    pub allowed_max_level: u32,
    pub tier0_retinue_count: u32,
    pub total_retinue_count: u32,
    pub infantry_count: u32,
    pub archer_count: u32,
    pub priest_count: u32,
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
        }
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct RosterEconomyFeedback {
    pub blocked_upgrade_reason: Option<String>,
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

#[derive(Event, Clone, Copy, Debug)]
pub struct PromoteUnitsEvent {
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
            .init_resource::<CommanderMotionState>()
            .add_event::<RecruitEvent>()
            .add_event::<PromoteUnitsEvent>()
            .add_event::<UnitDiedEvent>()
            .add_systems(Update, handle_start_run)
            .add_systems(
                Update,
                commander_movement.run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Update,
                (
                    apply_recruit_events,
                    apply_promotion_events,
                    run_priest_support_logic,
                    sync_roster,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
        app.add_systems(Update, on_unit_died);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_start_run(
    mut commands: Commands,
    mut roster: ResMut<SquadRoster>,
    mut economy: ResMut<RosterEconomy>,
    mut economy_feedback: ResMut<RosterEconomyFeedback>,
    mut motion: ResMut<CommanderMotionState>,
    mut start_events: EventReader<StartRunEvent>,
    existing_units: Query<Entity, With<Unit>>,
    data: Res<GameData>,
    art: Res<ArtAssets>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    for entity in existing_units.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let commander = spawn_commander(&mut commands, &data, &art);
    roster.commander = Some(commander);
    roster.friendly_count = 1;
    roster.casualties = 0;
    motion.is_moving = false;
    *economy = RosterEconomy::default();
    *economy_feedback = RosterEconomyFeedback::default();
}

fn spawn_commander(commands: &mut Commands, data: &GameData, art: &ArtAssets) -> Entity {
    let cfg = &data.units.commander;
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
            texture: art.commander_idle.clone(),
            sprite: Sprite {
                color: Color::srgb(1.0, 0.88, 0.88),
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

fn spawn_recruit(
    commands: &mut Commands,
    data: &GameData,
    art: &ArtAssets,
    recruit_kind: RecruitUnitKind,
    position: Vec2,
) -> Entity {
    let (cfg, unit_kind, texture, collider_radius, sprite_tint, tier, level_cost) =
        match recruit_kind {
            RecruitUnitKind::ChristianPeasantInfantry => (
                &data.units.recruit_christian_peasant_infantry,
                UnitKind::ChristianPeasantInfantry,
                art.friendly_peasant_infantry_idle.clone(),
                12.0,
                Color::WHITE,
                0u8,
                0u32,
            ),
            RecruitUnitKind::ChristianPeasantArcher => (
                &data.units.recruit_christian_peasant_archer,
                UnitKind::ChristianPeasantArcher,
                art.friendly_peasant_archer_idle.clone(),
                11.0,
                Color::srgb(0.94, 1.0, 0.94),
                1u8,
                1u32,
            ),
        };

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

#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn commander_movement(
    time: Res<Time>,
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    formation_mods: Res<FormationModifiers>,
    buffs: Option<Res<GlobalBuffs>>,
    conditional_effects: Option<Res<ConditionalUpgradeEffects>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut commander_motion: ResMut<CommanderMotionState>,
    bounds: Option<Res<MapBounds>>,
    banner_penalty: Option<Res<BannerMovementPenalty>>,
    friendlies: Query<Entity, (With<FriendlyUnit>, Without<CommanderUnit>)>,
    enemies: Query<&Transform, (With<EnemyUnit>, Without<CommanderUnit>)>,
    mut commanders: Query<
        (&MoveSpeed, &mut Transform),
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

    for (move_speed, mut transform) in &mut commanders {
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
        let effective_speed =
            (move_speed.0 + movement_bonus).max(1.0) * formation_mods.move_speed_multiplier;
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
    progression: Option<Res<Progression>>,
    economy: Res<RosterEconomy>,
    mut feedback: ResMut<RosterEconomyFeedback>,
    friendlies: Query<(Entity, &Unit, &UnitTier), (With<FriendlyUnit>, Without<CommanderUnit>)>,
) {
    let Some(current_level) = progression.as_ref().map(|value| value.level) else {
        return;
    };
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

        let mut candidates: Vec<(Entity, Unit, UnitTier)> = friendlies
            .iter()
            .filter_map(|(entity, unit, tier)| {
                (unit.kind == event.from_kind).then_some((entity, *unit, *tier))
            })
            .collect();
        candidates.sort_by_key(|(entity, _, _)| entity.index());

        let mut promoted = 0u32;
        let mut predicted_locked = economy.locked_levels;
        for (entity, unit, tier) in candidates {
            if promoted >= event.count {
                break;
            }
            let step_cost = target_tier.saturating_sub(tier.0) as u32;
            if step_cost == 0 {
                continue;
            }
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

            let Some((cfg, new_texture, new_collider_radius, has_melee, has_ranged, priest_kind)) =
                friendly_profile_for_kind(&data, &art, event.to_kind)
            else {
                continue;
            };

            let mut updated_unit = unit;
            updated_unit.kind = event.to_kind;
            commands.entity(entity).insert((
                updated_unit,
                UnitTier(target_tier),
                UnitLevelCost(target_tier as u32),
                Health::new(cfg.max_hp),
                BaseMaxHealth(cfg.max_hp),
                Morale::new(cfg.morale),
                Armor(cfg.armor),
                MoveSpeed(cfg.move_speed),
                ColliderRadius(new_collider_radius),
                new_texture,
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
                    cooldown: PRIEST_BLESSING_COOLDOWN_SECS * 0.2,
                });
            } else {
                commands
                    .entity(entity)
                    .remove::<(PriestSupportCaster, PriestAttackSpeedBlessing)>();
            }

            predicted_locked = predicted_locked.saturating_add(step_cost);
            promoted = promoted.saturating_add(1);
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

#[allow(clippy::type_complexity)]
fn run_priest_support_logic(
    mut commands: Commands,
    time: Res<Time>,
    mut priests: Query<(&Transform, &mut PriestSupportCaster), With<FriendlyUnit>>,
    friendlies: Query<(Entity, &Transform), With<FriendlyUnit>>,
    mut blessings: Query<(Entity, &mut PriestAttackSpeedBlessing), With<FriendlyUnit>>,
) {
    let dt = time.delta_seconds();
    for (entity, mut blessing) in &mut blessings {
        blessing.remaining_secs = (blessing.remaining_secs - dt).max(0.0);
        if blessing.remaining_secs <= 0.0 {
            commands
                .entity(entity)
                .remove::<PriestAttackSpeedBlessing>();
        }
    }

    let friendly_positions: Vec<(Entity, Vec2)> = friendlies
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect();
    if friendly_positions.is_empty() {
        return;
    }

    for (priest_transform, mut caster) in &mut priests {
        caster.cooldown -= dt;
        if caster.cooldown > 0.0 {
            continue;
        }
        let priest_position = priest_transform.translation.truncate();
        let range_sq = PRIEST_BLESSING_RANGE * PRIEST_BLESSING_RANGE;
        for (entity, position) in &friendly_positions {
            if position.distance_squared(priest_position) <= range_sq {
                commands.entity(*entity).insert(PriestAttackSpeedBlessing {
                    remaining_secs: PRIEST_BLESSING_DURATION_SECS,
                });
            }
        }
        caster.cooldown = PRIEST_BLESSING_COOLDOWN_SECS;
    }
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
    roster.friendly_count = friendlies.iter().count();
    roster.commander = friendlies
        .iter()
        .find_map(|(entity, commander, _, _, _)| commander.map(|_| entity));

    let mut locked = 0u32;
    let mut tier0 = 0u32;
    let mut total_retinue = 0u32;
    let mut infantry = 0u32;
    let mut archer = 0u32;
    let mut priest = 0u32;

    for (_, commander, unit, tier, level_cost) in &friendlies {
        if commander.is_some() {
            continue;
        }
        total_retinue = total_retinue.saturating_add(1);
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
            UnitKind::ChristianPeasantInfantry => infantry = infantry.saturating_add(1),
            UnitKind::ChristianPeasantArcher => archer = archer.saturating_add(1),
            UnitKind::ChristianPeasantPriest => priest = priest.saturating_add(1),
            _ => {}
        }
    }

    economy.locked_levels = locked;
    economy.allowed_max_level = level_cap_from_locked_budget(locked);
    economy.tier0_retinue_count = tier0;
    economy.total_retinue_count = total_retinue;
    economy.infantry_count = infantry;
    economy.archer_count = archer;
    economy.priest_count = priest;
}

fn on_unit_died(mut roster: ResMut<SquadRoster>, mut death_events: EventReader<UnitDiedEvent>) {
    for event in death_events.read() {
        if event.team == Team::Friendly {
            roster.casualties = roster.casualties.saturating_add(1);
        }
    }
}

fn friendly_tier_for_kind(kind: UnitKind) -> Option<u8> {
    match kind {
        UnitKind::ChristianPeasantInfantry => Some(0),
        UnitKind::ChristianPeasantArcher | UnitKind::ChristianPeasantPriest => Some(1),
        _ => None,
    }
}

fn unit_kind_label(kind: UnitKind) -> &'static str {
    match kind {
        UnitKind::Commander => "Commander",
        UnitKind::ChristianPeasantInfantry => "Christian Peasant Infantry",
        UnitKind::ChristianPeasantArcher => "Christian Peasant Archer",
        UnitKind::ChristianPeasantPriest => "Christian Peasant Priest",
        UnitKind::EnemyBanditRaider => "Bandit Raider",
        UnitKind::RescuableChristianPeasantInfantry => "Rescuable Christian Peasant Infantry",
        UnitKind::RescuableChristianPeasantArcher => "Rescuable Christian Peasant Archer",
    }
}

#[allow(clippy::type_complexity)]
fn friendly_profile_for_kind<'a>(
    data: &'a GameData,
    art: &ArtAssets,
    kind: UnitKind,
) -> Option<(&'a UnitStatsConfig, Handle<Image>, f32, bool, bool, bool)> {
    match kind {
        UnitKind::ChristianPeasantInfantry => Some((
            &data.units.recruit_christian_peasant_infantry,
            art.friendly_peasant_infantry_idle.clone(),
            12.0,
            true,
            false,
            false,
        )),
        UnitKind::ChristianPeasantArcher => Some((
            &data.units.recruit_christian_peasant_archer,
            art.friendly_peasant_archer_idle.clone(),
            11.0,
            true,
            true,
            false,
        )),
        UnitKind::ChristianPeasantPriest => Some((
            &data.units.recruit_christian_peasant_priest,
            art.friendly_peasant_priest_idle.clone(),
            11.0,
            false,
            false,
            true,
        )),
        _ => None,
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
    use bevy::prelude::*;

    use crate::combat::RangedAttackProfile;
    use crate::data::GameData;
    use crate::formation::{ActiveFormation, FormationModifiers};
    use crate::model::{
        CommanderUnit, FriendlyUnit, GameState, RecruitEvent, RecruitUnitKind, StartRunEvent, Unit,
        UnitKind,
    };
    use crate::squad::{
        PromoteUnitsEvent, RosterEconomy, RosterEconomyFeedback, SquadPlugin,
        enemy_inside_active_formation, movement_multiplier_from_inside_enemy_count,
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
        assert_eq!(economy_after_recruit.locked_levels, 1);
        assert_eq!(economy_after_recruit.allowed_max_level, 199);

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
        assert_eq!(economy_after_death.allowed_max_level, 200);
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
            xp: 0.0,
            level: 200,
            next_level_xp: f32::INFINITY,
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
}
