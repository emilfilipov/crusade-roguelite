use bevy::prelude::*;

use crate::banner::BannerMovementPenalty;
use crate::combat::{RangedAttackCooldown, RangedAttackProfile};
use crate::data::GameData;
use crate::formation::{
    ActiveFormation, FormationModifiers, active_formation_config, formation_contains_position,
};
use crate::map::{MapBounds, playable_bounds};
use crate::model::{
    Armor, AttackCooldown, AttackProfile, BaseMaxHealth, ColliderRadius, CommanderUnit, EnemyUnit,
    FriendlyUnit, GameState, GlobalBuffs, Health, Morale, MoveSpeed, PlayerControlled,
    RecruitEvent, RecruitUnitKind, RescuableUnit, StartRunEvent, Team, Unit, UnitDiedEvent,
    UnitKind,
};
use crate::visuals::ArtAssets;

const ENEMY_INSIDE_FORMATION_SLOWDOWN_PER_UNIT: f32 = 0.04;
const ENEMY_INSIDE_FORMATION_MIN_SPEED_MULTIPLIER: f32 = 0.5;
const ENEMY_INSIDE_FORMATION_PADDING_SLOTS: f32 = 0.35;

#[derive(Resource, Clone, Debug, Default)]
pub struct SquadRoster {
    pub commander: Option<Entity>,
    pub friendly_count: usize,
    pub casualties: u32,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct CommanderMotionState {
    pub is_moving: bool,
}

pub struct SquadPlugin;

impl Plugin for SquadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SquadRoster>()
            .init_resource::<CommanderMotionState>()
            .add_event::<RecruitEvent>()
            .add_event::<UnitDiedEvent>()
            .add_systems(Update, handle_start_run)
            .add_systems(
                Update,
                (
                    commander_movement.run_if(in_state(GameState::InRun)),
                    apply_recruit_events.run_if(in_state(GameState::InRun)),
                    sync_roster.run_if(in_state(GameState::InRun)),
                    on_unit_died,
                ),
            );
    }
}

fn handle_start_run(
    mut commands: Commands,
    mut roster: ResMut<SquadRoster>,
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
}

fn spawn_commander(commands: &mut Commands, data: &GameData, art: &ArtAssets) -> Entity {
    let cfg = &data.units.commander;
    let mut entity = commands.spawn((
        Unit {
            team: Team::Friendly,
            kind: UnitKind::Commander,
            level: 1,
        },
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
    let (cfg, unit_kind, texture, collider_radius, sprite_tint) = match recruit_kind {
        RecruitUnitKind::ChristianPeasantInfantry => (
            &data.units.recruit_christian_peasant_infantry,
            UnitKind::ChristianPeasantInfantry,
            art.friendly_peasant_infantry_idle.clone(),
            12.0,
            Color::WHITE,
        ),
        RecruitUnitKind::ChristianPeasantArcher => (
            &data.units.recruit_christian_peasant_archer,
            UnitKind::ChristianPeasantArcher,
            art.friendly_peasant_archer_idle.clone(),
            11.0,
            Color::srgb(0.94, 1.0, 0.94),
        ),
    };

    let mut entity = commands.spawn((
        Unit {
            team: Team::Friendly,
            kind: unit_kind,
            level: 1,
        },
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

fn sync_roster(
    mut roster: ResMut<SquadRoster>,
    friendlies: Query<(Entity, Option<&CommanderUnit>), With<FriendlyUnit>>,
) {
    roster.friendly_count = friendlies.iter().count();
    roster.commander = friendlies
        .iter()
        .find_map(|(entity, commander)| commander.map(|_| entity));
}

fn on_unit_died(mut roster: ResMut<SquadRoster>, mut death_events: EventReader<UnitDiedEvent>) {
    for event in death_events.read() {
        if event.team == Team::Friendly {
            roster.casualties = roster.casualties.saturating_add(1);
        }
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
    use crate::model::{CommanderUnit, GameState, RecruitEvent, RecruitUnitKind, StartRunEvent};
    use crate::squad::{
        SquadPlugin, enemy_inside_active_formation, movement_multiplier_from_inside_enemy_count,
    };
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
}
