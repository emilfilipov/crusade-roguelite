use bevy::prelude::*;
use std::collections::HashMap;

use crate::ai::{
    chase_step_distance, chase_target_positions, choose_nearest, choose_support_follow_target,
    should_move_towards_target,
};
use crate::combat::{RangedAttackCooldown, RangedAttackProfile};
use crate::data::{DifficultyGameplayConfig, EnemyTierPoolsConfigFile, GameData, WavesConfigFile};
use crate::formation::{
    ActiveFormation, active_formation_config, formation_allows_unlimited_enemy_inside,
    formation_anti_entry_enabled, formation_contains_position, formation_half_extent,
    formation_shape_perimeter_target,
};
use crate::inventory::{
    UnitCombatRole, UnitEquipmentBonuses, aggregate_item_bonuses_for_role,
    roll_chest_items_from_seed,
};
use crate::map::{MapBounds, playable_bounds};
use crate::model::{
    Armor, AttackCooldown, AttackProfile, ColliderRadius, CommanderUnit, EnemySpawnLane,
    EnemySpawnSource, EnemyUnit, FriendlyUnit, GameDifficulty, GameState, Health,
    MatchSetupSelection, Morale, MoveSpeed, PlayerFaction, StartRunEvent, Team, Unit, UnitKind,
};
use crate::morale::morale_movement_multiplier;
use crate::random::runtime_entropy_seed_u32;
use crate::squad::PriestSupportCaster;
use crate::squad::RosterEconomy;
use crate::upgrades::{Progression, major_minor_reward_counts_for_level};
use crate::visuals::ArtAssets;

#[derive(Resource, Clone, Debug, Default)]
pub struct WaveRuntime {
    pub elapsed: f32,
    pub current_wave: u32,
    pub wave_started: bool,
    pub finished_spawning: bool,
    pub victory_announced: bool,
    pub pending_batches: Vec<PendingEnemyBatch>,
    role_mix: HashMap<u32, EnemyRoleCounts>,
    pub spawn_sequence: u32,
    pub spawn_seed: u32,
    pub spawn_rng_state: u64,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct WaveCompletedEvent {
    pub wave_number: u32,
}

#[derive(Event, Clone, Copy, Debug)]
pub struct MajorArmyDefeatedEvent {
    pub wave_number: u32,
    pub position: Vec2,
}

#[derive(Component, Clone, Copy, Debug)]
struct MajorArmyUnit {
    wave_number: u32,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct MajorArmyTracker {
    active_wave: Option<u32>,
    last_position: Vec2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemyArmyLane {
    Small,
    Minor,
    Major,
}

#[derive(Clone, Debug)]
pub struct PendingEnemyBatch {
    pub remaining: u32,
    pub wave_number: u32,
    pub stat_scale: f32,
    pub lane: EnemyArmyLane,
    pub next_spawn_time: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaveArmyBatchPlan {
    lane: EnemyArmyLane,
    count: u32,
    stat_scale: f32,
}

#[derive(Clone, Debug, Default)]
struct EnemyArmyProgressionProfile {
    army_level: u32,
    major_upgrades: u32,
    minor_upgrades: u32,
    equipment_fill_ratio: f32,
    upgrade_pressure_multiplier: f32,
    equipment: EnemyEquipmentLoadoutProfile,
}

#[derive(Clone, Debug, Default)]
struct EnemyEquipmentLoadoutProfile {
    filled_slots: usize,
    template_ids: Vec<String>,
    melee_bonuses: UnitEquipmentBonuses,
    ranged_bonuses: UnitEquipmentBonuses,
    support_bonuses: UnitEquipmentBonuses,
    power_multiplier: f32,
}

impl EnemyEquipmentLoadoutProfile {
    fn bonuses_for_spawn_role(&self, role: EnemySpawnRole) -> UnitEquipmentBonuses {
        match role {
            EnemySpawnRole::Melee => self.melee_bonuses,
            EnemySpawnRole::Ranged => self.ranged_bonuses,
            EnemySpawnRole::Support => self.support_bonuses,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnemySpawnRole {
    Melee,
    Ranged,
    Support,
}

#[derive(Clone, Copy, Debug, Default)]
struct EnemyRoleCounts {
    total: u32,
    melee: u32,
    ranged: u32,
    support: u32,
}

impl EnemyRoleCounts {
    fn count_for(self, role: EnemySpawnRole) -> u32 {
        match role {
            EnemySpawnRole::Melee => self.melee,
            EnemySpawnRole::Ranged => self.ranged,
            EnemySpawnRole::Support => self.support,
        }
    }

    fn register(&mut self, role: EnemySpawnRole) {
        self.total = self.total.saturating_add(1);
        match role {
            EnemySpawnRole::Melee => {
                self.melee = self.melee.saturating_add(1);
            }
            EnemySpawnRole::Ranged => {
                self.ranged = self.ranged.saturating_add(1);
            }
            EnemySpawnRole::Support => {
                self.support = self.support.saturating_add(1);
            }
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub enum BanditVisualState {
    Idle,
    Move,
    Attack,
    Hit,
    Dead,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct BanditVisualRuntime {
    pub last_position: Vec2,
    pub state: BanditVisualState,
}

#[derive(Component, Clone, Copy, Debug)]
struct EnemyMovementState {
    moving: bool,
    crowd_hold_secs: f32,
    last_position: Vec2,
    stuck_secs: f32,
}

const ENEMY_BASE_SPEED_MULTIPLIER: f32 = 0.72;
const WAVE_DURATION_SECS: f32 = 30.0;
pub const MAX_WAVES: u32 = 100;
const STOP_FACTOR: f32 = 0.82;
const RESUME_FACTOR: f32 = 0.98;
const ENEMY_INSIDE_FORMATION_PADDING_SLOTS: f32 = 0.35;
const ENEMY_FORMATION_REPEL_MARGIN_SLOTS: f32 = 0.12;
const WAVE_UNITS_MULTIPLIER: f32 = 2.0;
const MAX_NON_ARMY_ENEMIES_PER_WAVE: f32 = 200.0;
const MAX_ARMY_ENEMIES_PER_WAVE: f32 = 320.0;
const POST_SCRIPTED_WAVE_COUNT_GROWTH: f32 = 1.18;
const WAVE_STAT_GROWTH_PER_WAVE: f32 = 0.102;
const WAVE_BATCH_SIZE: u32 = 7;
const WAVE_BATCH_INTERVAL_SECS: f32 = 0.7;
const MINOR_ARMY_COUNT_MULTIPLIER: f32 = 0.55;
const MAJOR_ARMY_COUNT_MULTIPLIER: f32 = 1.25;
const SMALL_ARMY_BATCH_GROWTH_INTERVAL_WAVES: u32 = 20;
const MINOR_ARMY_BATCH_GROWTH_INTERVAL_WAVES: u32 = 20;
const MAJOR_ARMY_BATCH_GROWTH_INTERVAL_WAVES: u32 = 20;
const MAX_SMALL_ARMY_BATCHES_PER_WAVE: u32 = 5;
const MAX_MINOR_ARMY_BATCHES_PER_WAVE: u32 = 4;
const MAX_MAJOR_ARMY_BATCHES_PER_WAVE: u32 = 3;
const MINOR_ARMY_STAT_MULTIPLIER: f32 = 1.12;
const MAJOR_ARMY_STAT_MULTIPLIER: f32 = 1.30;
const SMALL_ARMY_LOADOUT_SLOT_COUNT: usize = 3;
const MINOR_ARMY_LOADOUT_SLOT_COUNT: usize = 4;
const MAJOR_ARMY_LOADOUT_SLOT_COUNT: usize = 5;
const ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER: f32 = 200.0;
const ENEMY_SPAWN_ATTEMPTS: u32 = 8;
const DEFAULT_SPAWN_HALF_WIDTH: f32 = 900.0;
const DEFAULT_SPAWN_HALF_HEIGHT: f32 = 700.0;
const ENEMY_LEVEL_PRESSURE_PER_LEVEL: f32 = 0.011;
const ENEMY_RETINUE_PRESSURE_PER_UNIT: f32 = 0.008;
const ENEMY_RETINUE_PRESSURE_EXPONENT: f32 = 0.72;
const ENEMY_PLAYER_PRESSURE_STAT_CAP: f32 = 3.5;
const ENEMY_CROWD_STOP_NEIGHBOR_RADIUS: f32 = 26.0;
const ENEMY_CROWD_STOP_NEIGHBOR_COUNT: usize = 6;
const ENEMY_CROWD_STOP_DISTANCE_FACTOR: f32 = 1.35;
const ENEMY_CROWD_HOLD_SECS: f32 = 0.18;
const ENEMY_CROWD_STUCK_DISTANCE_EPS: f32 = 0.7;
const ENEMY_CROWD_STUCK_MIN_SECS: f32 = 0.22;
const ENEMY_CROWD_STUCK_DECAY_PER_SEC: f32 = 0.65;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaveRuntime>()
            .init_resource::<MajorArmyTracker>()
            .add_event::<MajorArmyDefeatedEvent>()
            .add_systems(Update, reset_waves_on_run_start)
            .add_systems(
                Update,
                (
                    spawn_waves,
                    spawn_pending_enemy_batches,
                    track_major_army_defeats,
                    enemy_chase_targets,
                    update_bandit_visual_states,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            )
            .add_systems(
                Last,
                repel_enemy_overflow_from_formation.run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_waves_on_run_start(
    mut wave_runtime: ResMut<WaveRuntime>,
    mut major_army_tracker: ResMut<MajorArmyTracker>,
    mut start_events: EventReader<StartRunEvent>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}
    let seed = runtime_seed_from_time();
    *wave_runtime = WaveRuntime {
        current_wave: 1,
        spawn_seed: seed,
        spawn_rng_state: rng_state_from_seed(seed ^ 0x4A3C_11D7),
        ..WaveRuntime::default()
    };
    *major_army_tracker = MajorArmyTracker::default();
}

fn spawn_waves(
    time: Res<Time>,
    data: Res<GameData>,
    alive_enemies: Query<(), With<EnemyUnit>>,
    mut wave_runtime: ResMut<WaveRuntime>,
    mut wave_completed_events: EventWriter<WaveCompletedEvent>,
) {
    let dt = time.delta_seconds();
    if dt <= 0.0 {
        return;
    }
    if wave_runtime.current_wave == 0 {
        wave_runtime.current_wave = 1;
    }

    wave_runtime.elapsed += dt;
    if wave_runtime.finished_spawning {
        return;
    }
    let pending_empty = wave_runtime.pending_batches.is_empty();
    let alive_count = alive_enemies.iter().count();
    if !wave_runtime.wave_started {
        let wave_number = wave_runtime.current_wave;
        let base_spawn_count = army_units_to_spawn_for_wave(&data.waves, wave_number);
        let base_stat_scale = wave_stat_multiplier(wave_number);
        for plan in planned_wave_army_batches(base_spawn_count, wave_number, base_stat_scale) {
            enqueue_wave_batch(
                &mut wave_runtime,
                plan.count,
                wave_number,
                plan.stat_scale,
                plan.lane,
            );
        }
        wave_runtime.wave_started = true;
        return;
    }

    if pending_empty && alive_count == 0 {
        let completed_wave = wave_runtime.current_wave.max(1);
        wave_completed_events.send(WaveCompletedEvent {
            wave_number: completed_wave,
        });
        if completed_wave >= MAX_WAVES {
            wave_runtime.finished_spawning = true;
            return;
        }
        wave_runtime.current_wave = completed_wave.saturating_add(1);
        wave_runtime.wave_started = false;
    }
}

fn track_major_army_defeats(
    mut tracker: ResMut<MajorArmyTracker>,
    major_units: Query<(&MajorArmyUnit, &Transform), With<EnemyUnit>>,
    mut defeated_events: EventWriter<MajorArmyDefeatedEvent>,
) {
    let mut major_wave = None;
    let mut major_count = 0usize;
    let mut position_accumulator = Vec2::ZERO;
    for (unit, transform) in &major_units {
        major_wave = Some(unit.wave_number);
        major_count = major_count.saturating_add(1);
        position_accumulator += transform.translation.truncate();
    }

    if major_count > 0 {
        let centroid = position_accumulator / major_count as f32;
        tracker.active_wave = major_wave;
        tracker.last_position = centroid;
        return;
    }

    if let Some(defeated_wave) = tracker.active_wave.take() {
        defeated_events.send(MajorArmyDefeatedEvent {
            wave_number: defeated_wave,
            position: tracker.last_position,
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_pending_enemy_batches(
    mut commands: Commands,
    data: Res<GameData>,
    art: Res<ArtAssets>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    progression: Option<Res<Progression>>,
    roster_economy: Option<Res<RosterEconomy>>,
    bounds: Option<Res<MapBounds>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut wave_runtime: ResMut<WaveRuntime>,
) {
    if wave_runtime.pending_batches.is_empty() {
        return;
    }

    let player_faction = setup_selection
        .as_ref()
        .map(|selection| selection.faction)
        .unwrap_or(PlayerFaction::Christian);
    let difficulty = setup_selection
        .as_ref()
        .map(|selection| selection.difficulty)
        .unwrap_or(GameDifficulty::Recruit);
    let difficulty_mods = data.difficulties.for_difficulty(difficulty);
    let spawn_bounds = bounds
        .as_deref()
        .copied()
        .map(playable_bounds)
        .unwrap_or_else(default_spawn_bounds);
    let commander_position = commanders
        .get_single()
        .map(|transform| transform.translation.truncate())
        .unwrap_or(Vec2::ZERO);
    let commander_level = progression.as_ref().map(|value| value.level).unwrap_or(1);
    let retinue_count = roster_economy
        .as_ref()
        .map(|value| value.total_retinue_count)
        .unwrap_or(0);
    let player_pressure_multiplier =
        enemy_player_pressure_multiplier(commander_level, retinue_count);
    let current_time = wave_runtime.elapsed;
    let mut spawn_sequence = wave_runtime.spawn_sequence;
    let mut spawn_rng_state = wave_runtime.spawn_rng_state;
    let mut pending_batches = std::mem::take(&mut wave_runtime.pending_batches);
    let mut role_mix = std::mem::take(&mut wave_runtime.role_mix);
    let mut remaining = Vec::with_capacity(pending_batches.len());
    for mut batch in pending_batches.drain(..) {
        if current_time + f32::EPSILON < batch.next_spawn_time {
            remaining.push(batch);
            continue;
        }

        let spawn_now = batch_size_for_wave(batch.wave_number).min(batch.remaining);
        let army_progression = enemy_army_progression_profile(
            difficulty,
            commander_level,
            batch.wave_number,
            batch.lane,
            player_faction.opposing(),
            wave_runtime.spawn_seed,
        );
        spawn_enemy_batch(
            &mut commands,
            spawn_now,
            &data,
            &art,
            player_faction,
            difficulty,
            spawn_bounds,
            commander_position,
            batch.wave_number,
            batch.stat_scale
                * player_pressure_multiplier
                * army_progression.upgrade_pressure_multiplier,
            difficulty_mods,
            army_progression,
            batch.lane,
            wave_runtime.spawn_seed,
            &mut spawn_sequence,
            &mut spawn_rng_state,
            &mut role_mix,
        );
        batch.remaining = batch.remaining.saturating_sub(spawn_now);
        if batch.remaining > 0 {
            batch.next_spawn_time = current_time + batch_interval_secs(batch.wave_number);
            remaining.push(batch);
        }
    }
    wave_runtime.spawn_sequence = spawn_sequence;
    wave_runtime.spawn_rng_state = spawn_rng_state;
    wave_runtime.pending_batches = remaining;
    wave_runtime.role_mix = role_mix;
}

#[allow(clippy::too_many_arguments)]
fn spawn_enemy_batch(
    commands: &mut Commands,
    count: u32,
    data: &GameData,
    art: &ArtAssets,
    player_faction: PlayerFaction,
    difficulty: GameDifficulty,
    bounds: MapBounds,
    commander_position: Vec2,
    wave_number: u32,
    stat_scale: f32,
    difficulty_mods: &DifficultyGameplayConfig,
    army_progression: EnemyArmyProgressionProfile,
    lane: EnemyArmyLane,
    spawn_seed: u32,
    spawn_sequence: &mut u32,
    spawn_rng_state: &mut u64,
    role_mix: &mut HashMap<u32, EnemyRoleCounts>,
) {
    let enemy_faction = player_faction.opposing();
    let enemy_pool_roles = build_enemy_pool_roles_for_wave(
        &data.enemy_tier_pools,
        enemy_faction,
        wave_number,
        lane,
        difficulty,
    );
    if enemy_pool_roles.is_empty() {
        return;
    }
    let has_melee = enemy_pool_roles
        .iter()
        .any(|(_, role)| *role == EnemySpawnRole::Melee);
    let has_ranged = enemy_pool_roles
        .iter()
        .any(|(_, role)| *role == EnemySpawnRole::Ranged);
    let has_support = enemy_pool_roles
        .iter()
        .any(|(_, role)| *role == EnemySpawnRole::Support);
    let major_preview_tier = major_wave_preview_tier(wave_number, lane);
    let tier_mix = enemy_tier_mix_for_wave(wave_number, lane, difficulty);
    let major_preview_target_count = if major_preview_tier.is_some() {
        major_wave_preview_target_count_for_batch(count, tier_mix[1].weight_percent)
    } else {
        0
    };
    let mut major_preview_spawned_count = 0u32;
    let fallback_enemy_kind =
        UnitKind::from_faction_and_unit_id(enemy_faction, "peasant_infantry", false)
            .or_else(|| {
                data.enemies
                    .opposing_enemy_pool(player_faction)
                    .first()
                    .copied()
            })
            .expect("enemy fallback kind should resolve");

    for _ in 0..count {
        let seq = *spawn_sequence;
        *spawn_sequence = spawn_sequence.saturating_add(1);
        let counters = role_mix.get(&wave_number).copied().unwrap_or_default();
        let spawn_role = pick_next_spawn_role(
            counters,
            has_melee,
            has_ranged,
            has_support,
            seq ^ spawn_seed ^ 0x5F9D_A5C7,
        );
        let force_preview_this_spawn = major_preview_tier.is_some()
            && major_preview_spawned_count < major_preview_target_count;
        let enemy_kind = if force_preview_this_spawn {
            let preview_tier = major_preview_tier.expect("checked above");
            choose_enemy_kind_for_tier_and_role(
                &data.enemy_tier_pools,
                enemy_faction,
                preview_tier,
                spawn_role,
                hash_seed(wave_number ^ spawn_seed, seq ^ spawn_seed, 0xD13F_0A1D),
            )
            .unwrap_or_else(|| {
                choose_enemy_kind_for_role(
                    &enemy_pool_roles,
                    spawn_role,
                    hash_seed(wave_number ^ spawn_seed, seq ^ spawn_seed, 0xC55A_A5AA),
                    fallback_enemy_kind,
                )
            })
        } else {
            choose_enemy_kind_for_role(
                &enemy_pool_roles,
                spawn_role,
                hash_seed(wave_number ^ spawn_seed, seq ^ spawn_seed, 0xC55A_A5AA),
                fallback_enemy_kind,
            )
        };
        if force_preview_this_spawn
            && major_preview_tier
                .map(|preview_tier| enemy_kind.tier_hint() == Some(preview_tier))
                .unwrap_or(false)
        {
            major_preview_spawned_count = major_preview_spawned_count.saturating_add(1);
        }
        let Some(cfg) = data.enemies.enemy_profile_for_kind(enemy_kind) else {
            continue;
        };
        role_mix
            .entry(wave_number)
            .or_default()
            .register(spawn_role);
        let resolved_enemy_faction = enemy_kind.faction().unwrap_or(enemy_faction);
        let faction_mods = data.factions.for_faction(resolved_enemy_faction);
        let equipment_bonuses = army_progression
            .equipment
            .bonuses_for_spawn_role(spawn_role);
        let role_damage_bonus = match spawn_role {
            EnemySpawnRole::Ranged => equipment_bonuses.ranged_damage_multiplier,
            EnemySpawnRole::Melee | EnemySpawnRole::Support => {
                equipment_bonuses.melee_damage_multiplier
            }
        };
        let equipment_power_multiplier = army_progression.equipment.power_multiplier;
        let role_attack_speed_multiplier =
            (1.0 + equipment_bonuses.attack_speed_multiplier).clamp(0.25, 3.0);
        let hp = (cfg.max_hp
            * combined_enemy_stat_multiplier(
                stat_scale,
                faction_mods.enemy_health_multiplier,
                difficulty_mods.enemy_health_multiplier,
            )
            * equipment_power_multiplier
            + equipment_bonuses.health_bonus)
            .max(1.0);
        let armor = cfg.armor
            + (stat_scale - 1.0) * 2.0
            + army_progression.equipment_fill_ratio * 1.8
            + army_progression.equipment.filled_slots as f32 * 0.22
            + army_progression.equipment.template_ids.len() as f32 * 0.08
            + army_progression.major_upgrades as f32 * 0.04
            + army_progression.minor_upgrades as f32 * 0.01
            + equipment_bonuses.armor_bonus;
        let damage = cfg.damage
            * combined_enemy_stat_multiplier(
                stat_scale,
                faction_mods.enemy_damage_multiplier,
                difficulty_mods.enemy_damage_multiplier,
            )
            * equipment_power_multiplier
            * (1.0 + role_damage_bonus).clamp(0.2, 3.0);
        let base_cooldown = (cfg.attack_cooldown_secs
            / (1.0 + (stat_scale - 1.0) * 0.15)
            / role_attack_speed_multiplier)
            .clamp(0.2, cfg.attack_cooldown_secs);
        let attack_cooldown_secs = scale_enemy_attack_cooldown(
            base_cooldown,
            faction_mods.enemy_attack_speed_multiplier
                * difficulty_mods.enemy_attack_speed_multiplier,
        );
        let ranged_cooldown_secs = if cfg.ranged_attack_damage > 0.0 {
            let base_ranged_cooldown = (cfg.ranged_attack_cooldown_secs
                / (1.0 + (stat_scale - 1.0) * 0.15))
                / role_attack_speed_multiplier;
            let base_ranged_cooldown =
                base_ranged_cooldown.clamp(0.15, cfg.ranged_attack_cooldown_secs);
            Some(scale_enemy_attack_cooldown(
                base_ranged_cooldown,
                faction_mods.enemy_attack_speed_multiplier
                    * difficulty_mods.enemy_attack_speed_multiplier,
            ))
        } else {
            None
        };
        let move_speed = enemy_move_speed(
            (cfg.move_speed
                * combined_enemy_speed_multiplier(
                    faction_mods.enemy_move_speed_multiplier,
                    difficulty_mods.enemy_move_speed_multiplier,
                )
                + equipment_bonuses.move_speed_bonus)
                .max(20.0),
        );
        let morale = (cfg.morale
            * combined_enemy_stat_multiplier(
                1.0,
                faction_mods.enemy_morale_multiplier,
                difficulty_mods.enemy_morale_multiplier,
            ))
        .max(1.0);
        let texture = enemy_texture_for_kind(art, enemy_kind);
        let pos = random_spawn_position_from_rng(bounds, commander_position, spawn_rng_state);
        let mut entity = commands.spawn((
            Unit {
                team: Team::Enemy,
                kind: enemy_kind,
                level: army_progression.army_level.max(1),
            },
            EnemyUnit,
            EnemySpawnSource {
                lane: enemy_spawn_lane_for_army_lane(lane),
            },
            BanditVisualRuntime {
                last_position: pos,
                state: BanditVisualState::Idle,
            },
            EnemyMovementState {
                moving: true,
                crowd_hold_secs: 0.0,
                last_position: pos,
                stuck_secs: 0.0,
            },
            Health::new(hp),
            Morale::new(morale),
            Armor(armor),
            AttackProfile {
                damage,
                range: cfg.attack_range,
                cooldown_secs: attack_cooldown_secs,
            },
            AttackCooldown(Timer::from_seconds(
                attack_cooldown_secs,
                TimerMode::Repeating,
            )),
            MoveSpeed(move_speed),
            ColliderRadius(cfg.collision_radius),
            SpriteBundle {
                texture,
                sprite: Sprite {
                    color: Color::srgb(1.0, 0.85, 0.85),
                    custom_size: Some(Vec2::splat(32.0)),
                    ..default()
                },
                transform: Transform::from_xyz(pos.x, pos.y, 5.0),
                ..default()
            },
        ));
        if let Some(cooldown_secs) = ranged_cooldown_secs {
            entity.insert((
                RangedAttackProfile {
                    damage: cfg.ranged_attack_damage
                        * equipment_power_multiplier
                        * (1.0 + equipment_bonuses.ranged_damage_multiplier).clamp(0.2, 3.0),
                    range: (cfg.ranged_attack_range + equipment_bonuses.ranged_range_bonus)
                        .max(cfg.attack_range),
                    projectile_speed: cfg.ranged_projectile_speed,
                    projectile_max_distance: (cfg.ranged_projectile_max_distance
                        + equipment_bonuses.ranged_range_bonus)
                        .max(cfg.ranged_projectile_max_distance),
                },
                RangedAttackCooldown(Timer::from_seconds(cooldown_secs, TimerMode::Repeating)),
            ));
        }
        if enemy_kind.is_priest() {
            entity.insert(PriestSupportCaster { cooldown: 20.0 });
        }
        if matches!(lane, EnemyArmyLane::Major) {
            entity.insert((MajorArmyUnit { wave_number }, Name::new("MajorArmyUnit")));
        }
    }
}

fn enemy_spawn_lane_for_army_lane(lane: EnemyArmyLane) -> EnemySpawnLane {
    match lane {
        EnemyArmyLane::Small => EnemySpawnLane::Small,
        EnemyArmyLane::Minor => EnemySpawnLane::Minor,
        EnemyArmyLane::Major => EnemySpawnLane::Major,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EnemyTierWeight {
    tier: u8,
    weight_percent: u8,
}

fn unlocked_enemy_tier_for_regular_wave(wave_number: u32) -> u8 {
    (wave_number.saturating_sub(1) / 10).min(5) as u8
}

fn major_army_next_tier_preview_percent(difficulty: GameDifficulty) -> u8 {
    match difficulty {
        GameDifficulty::Recruit => 20,
        GameDifficulty::Experienced => 35,
        GameDifficulty::AloneAgainstTheInfidels => 50,
    }
}

fn major_wave_preview_target_count_for_batch(count: u32, preview_percent: u8) -> u32 {
    if count == 0 || preview_percent == 0 {
        return 0;
    }
    ((count.saturating_mul(preview_percent as u32) + 50) / 100)
        .max(1)
        .min(count)
}

fn enemy_tier_mix_for_wave(
    wave_number: u32,
    lane: EnemyArmyLane,
    difficulty: GameDifficulty,
) -> [EnemyTierWeight; 2] {
    match lane {
        EnemyArmyLane::Major => {
            let base_tier = unlocked_enemy_tier_for_regular_wave(wave_number);
            if base_tier >= 5 {
                return [
                    EnemyTierWeight {
                        tier: 5,
                        weight_percent: 100,
                    },
                    EnemyTierWeight {
                        tier: 5,
                        weight_percent: 0,
                    },
                ];
            }
            let next_tier = (base_tier + 1).min(5);
            let next_share = major_army_next_tier_preview_percent(difficulty);
            [
                EnemyTierWeight {
                    tier: base_tier,
                    weight_percent: 100u8.saturating_sub(next_share),
                },
                EnemyTierWeight {
                    tier: next_tier,
                    weight_percent: next_share,
                },
            ]
        }
        EnemyArmyLane::Small | EnemyArmyLane::Minor => {
            let unlocked_tier = unlocked_enemy_tier_for_regular_wave(wave_number);
            if unlocked_tier == 0 {
                return [
                    EnemyTierWeight {
                        tier: 0,
                        weight_percent: 100,
                    },
                    EnemyTierWeight {
                        tier: 0,
                        weight_percent: 0,
                    },
                ];
            }
            let window_start_wave = unlocked_tier as u32 * 10;
            let progress_steps = wave_number.saturating_sub(window_start_wave).clamp(0, 10);
            let unlocked_share = ((progress_steps * 10).min(100)) as u8;
            [
                EnemyTierWeight {
                    tier: unlocked_tier - 1,
                    weight_percent: 100u8.saturating_sub(unlocked_share),
                },
                EnemyTierWeight {
                    tier: unlocked_tier,
                    weight_percent: unlocked_share,
                },
            ]
        }
    }
}

fn major_wave_preview_tier(wave_number: u32, lane: EnemyArmyLane) -> Option<u8> {
    if !matches!(lane, EnemyArmyLane::Major) {
        return None;
    }
    let base_tier = unlocked_enemy_tier_for_regular_wave(wave_number);
    if base_tier >= 5 {
        None
    } else {
        Some((base_tier + 1).min(5))
    }
}

fn enemy_unit_ids_for_tier_and_role(
    pools: &EnemyTierPoolsConfigFile,
    tier: u8,
    role: EnemySpawnRole,
) -> Option<&[String]> {
    let archetype = match role {
        EnemySpawnRole::Melee => crate::model::RecruitArchetype::Infantry,
        EnemySpawnRole::Ranged => crate::model::RecruitArchetype::Archer,
        EnemySpawnRole::Support => crate::model::RecruitArchetype::Priest,
    };
    pools.unit_ids_for_tier_and_archetype(tier, archetype)
}

fn choose_enemy_kind_for_tier_and_role(
    pools: &EnemyTierPoolsConfigFile,
    faction: PlayerFaction,
    tier: u8,
    role: EnemySpawnRole,
    seed: u32,
) -> Option<UnitKind> {
    let ids = enemy_unit_ids_for_tier_and_role(pools, tier, role)?;
    if ids.is_empty() {
        return None;
    }
    let pick = ids[(seed as usize) % ids.len()].as_str();
    UnitKind::from_faction_and_unit_id(faction, pick, false)
}

fn build_enemy_pool_roles_for_wave(
    pools: &EnemyTierPoolsConfigFile,
    enemy_faction: PlayerFaction,
    wave_number: u32,
    lane: EnemyArmyLane,
    difficulty: GameDifficulty,
) -> Vec<(UnitKind, EnemySpawnRole)> {
    let tier_mix = enemy_tier_mix_for_wave(wave_number, lane, difficulty);
    let mut pool = Vec::new();
    for weight in tier_mix {
        if weight.weight_percent == 0 {
            continue;
        }
        for role in [
            EnemySpawnRole::Melee,
            EnemySpawnRole::Ranged,
            EnemySpawnRole::Support,
        ] {
            if let Some(unit_ids) = enemy_unit_ids_for_tier_and_role(pools, weight.tier, role) {
                for unit_id in unit_ids {
                    let Some(kind) =
                        UnitKind::from_faction_and_unit_id(enemy_faction, unit_id, false)
                    else {
                        continue;
                    };
                    for _ in 0..weight.weight_percent {
                        pool.push((kind, role));
                    }
                }
            }
        }
    }
    if pool.is_empty() {
        for role in [
            EnemySpawnRole::Melee,
            EnemySpawnRole::Ranged,
            EnemySpawnRole::Support,
        ] {
            if let Some(unit_ids) = enemy_unit_ids_for_tier_and_role(pools, 0, role)
                && let Some(unit_id) = unit_ids.first()
                && let Some(kind) =
                    UnitKind::from_faction_and_unit_id(enemy_faction, unit_id, false)
            {
                pool.push((kind, role));
            }
        }
    }
    pool
}

fn choose_enemy_kind_for_role(
    enemy_pool_roles: &[(UnitKind, EnemySpawnRole)],
    preferred_role: EnemySpawnRole,
    seed: u32,
    fallback_kind: UnitKind,
) -> UnitKind {
    if enemy_pool_roles.is_empty() {
        return fallback_kind;
    }
    let matching_count = enemy_pool_roles
        .iter()
        .filter(|(_, role)| *role == preferred_role)
        .count();
    if matching_count == 0 {
        return enemy_pool_roles[(seed as usize) % enemy_pool_roles.len()].0;
    }
    let target_match_index = (seed as usize) % matching_count;
    let mut seen = 0usize;
    for (kind, role) in enemy_pool_roles {
        if *role != preferred_role {
            continue;
        }
        if seen == target_match_index {
            return *kind;
        }
        seen = seen.saturating_add(1);
    }
    enemy_pool_roles[0].0
}

fn pick_next_spawn_role(
    counts: EnemyRoleCounts,
    has_melee: bool,
    has_ranged: bool,
    has_support: bool,
    tie_seed: u32,
) -> EnemySpawnRole {
    let next_total = counts.total.saturating_add(1);
    let support_cap = next_total / 4;
    let support_allowed = has_support && counts.support < support_cap;

    let mut candidates = [EnemySpawnRole::Melee; 3];
    let mut candidate_count = 0usize;
    if has_melee {
        candidates[candidate_count] = EnemySpawnRole::Melee;
        candidate_count += 1;
    }
    if has_ranged {
        candidates[candidate_count] = EnemySpawnRole::Ranged;
        candidate_count += 1;
    }
    if support_allowed {
        candidates[candidate_count] = EnemySpawnRole::Support;
        candidate_count += 1;
    }

    if candidate_count == 0 {
        if has_melee {
            candidates[candidate_count] = EnemySpawnRole::Melee;
            candidate_count += 1;
        }
        if has_ranged {
            candidates[candidate_count] = EnemySpawnRole::Ranged;
            candidate_count += 1;
        }
        if has_support {
            candidates[candidate_count] = EnemySpawnRole::Support;
            candidate_count += 1;
        }
    }
    if candidate_count == 0 {
        return EnemySpawnRole::Melee;
    }

    let mut best_deficit = f32::MIN;
    let mut best = [EnemySpawnRole::Melee; 3];
    let mut best_count = 0usize;
    let non_support_role_count = has_melee as u32 + has_ranged as u32;
    for role in candidates.iter().copied().take(candidate_count) {
        let deficit = target_role_count(role, next_total, has_support, non_support_role_count)
            - counts.count_for(role) as f32;
        if deficit > best_deficit + f32::EPSILON {
            best_deficit = deficit;
            best[0] = role;
            best_count = 1;
            continue;
        }
        if (deficit - best_deficit).abs() <= f32::EPSILON {
            best[best_count] = role;
            best_count += 1;
        }
    }

    best[(tie_seed as usize) % best_count]
}

fn target_role_count(
    role: EnemySpawnRole,
    next_total: u32,
    has_support: bool,
    non_support_role_count: u32,
) -> f32 {
    let ratio = match role {
        EnemySpawnRole::Support => {
            if !has_support {
                0.0
            } else if non_support_role_count == 0 {
                1.0
            } else {
                0.25
            }
        }
        EnemySpawnRole::Melee | EnemySpawnRole::Ranged => {
            if non_support_role_count == 0 {
                0.0
            } else {
                let support_ratio = if has_support { 0.25 } else { 0.0 };
                (1.0 - support_ratio) / non_support_role_count as f32
            }
        }
    };
    next_total as f32 * ratio
}

fn scale_enemy_attack_cooldown(base_cooldown: f32, speed_multiplier: f32) -> f32 {
    (base_cooldown / speed_multiplier.max(0.01)).max(0.08)
}

pub fn is_minor_army_wave(wave_number: u32) -> bool {
    wave_number > 0 && wave_number.is_multiple_of(2)
}

pub fn is_major_army_wave(wave_number: u32) -> bool {
    wave_number > 0 && wave_number.is_multiple_of(10)
}

fn planned_wave_army_batches(
    base_count: u32,
    wave_number: u32,
    base_stat_scale: f32,
) -> Vec<WaveArmyBatchPlan> {
    let mut plans = Vec::new();
    append_lane_batches(
        &mut plans,
        EnemyArmyLane::Small,
        base_count.max(1),
        base_stat_scale,
        small_army_batch_count_for_wave(wave_number),
    );
    if is_minor_army_wave(wave_number) {
        append_lane_batches(
            &mut plans,
            EnemyArmyLane::Minor,
            scaled_lane_count(base_count, MINOR_ARMY_COUNT_MULTIPLIER),
            base_stat_scale * MINOR_ARMY_STAT_MULTIPLIER,
            minor_army_batch_count_for_wave(wave_number),
        );
    }
    if is_major_army_wave(wave_number) {
        append_lane_batches(
            &mut plans,
            EnemyArmyLane::Major,
            scaled_lane_count(base_count, MAJOR_ARMY_COUNT_MULTIPLIER),
            base_stat_scale * MAJOR_ARMY_STAT_MULTIPLIER,
            major_army_batch_count_for_wave(wave_number),
        );
    }
    plans
}

fn scaled_lane_count(base_count: u32, multiplier: f32) -> u32 {
    ((base_count.max(1) as f32) * multiplier).round().max(1.0) as u32
}

fn append_lane_batches(
    plans: &mut Vec<WaveArmyBatchPlan>,
    lane: EnemyArmyLane,
    total_count: u32,
    stat_scale: f32,
    batch_count: u32,
) {
    if total_count == 0 {
        return;
    }
    let batch_count = batch_count.max(1);
    let base_batch_size = total_count / batch_count;
    let remainder = total_count % batch_count;
    for batch_index in 0..batch_count {
        let count = base_batch_size + u32::from(batch_index < remainder);
        if count == 0 {
            continue;
        }
        plans.push(WaveArmyBatchPlan {
            lane,
            count,
            stat_scale,
        });
    }
}

fn scaled_batch_count(wave_number: u32, growth_interval: u32, max_batches: u32) -> u32 {
    (1 + wave_number / growth_interval).clamp(1, max_batches.max(1))
}

fn small_army_batch_count_for_wave(wave_number: u32) -> u32 {
    scaled_batch_count(
        wave_number.saturating_sub(1),
        SMALL_ARMY_BATCH_GROWTH_INTERVAL_WAVES,
        MAX_SMALL_ARMY_BATCHES_PER_WAVE,
    )
}

fn minor_army_batch_count_for_wave(wave_number: u32) -> u32 {
    scaled_batch_count(
        wave_number.saturating_sub(2),
        MINOR_ARMY_BATCH_GROWTH_INTERVAL_WAVES,
        MAX_MINOR_ARMY_BATCHES_PER_WAVE,
    )
}

fn major_army_batch_count_for_wave(wave_number: u32) -> u32 {
    scaled_batch_count(
        wave_number.saturating_sub(10),
        MAJOR_ARMY_BATCH_GROWTH_INTERVAL_WAVES,
        MAX_MAJOR_ARMY_BATCHES_PER_WAVE,
    )
}

pub fn enemy_army_level_for_difficulty(difficulty: GameDifficulty, player_level: u32) -> u32 {
    let level = player_level.max(1);
    match difficulty {
        GameDifficulty::Recruit => (level / 2).max(1),
        GameDifficulty::Experienced | GameDifficulty::AloneAgainstTheInfidels => level,
    }
}

fn enemy_equipment_fill_ratio_for_difficulty(difficulty: GameDifficulty) -> f32 {
    match difficulty {
        GameDifficulty::Recruit => 1.0 / 3.0,
        GameDifficulty::Experienced => 0.5,
        GameDifficulty::AloneAgainstTheInfidels => 2.0 / 3.0,
    }
}

fn enemy_equipment_rarity_bonus_for_difficulty(difficulty: GameDifficulty) -> f32 {
    match difficulty {
        GameDifficulty::Recruit => 0.0,
        GameDifficulty::Experienced => 0.18,
        GameDifficulty::AloneAgainstTheInfidels => 0.33,
    }
}

fn enemy_loadout_slot_count_for_lane(lane: EnemyArmyLane) -> usize {
    match lane {
        EnemyArmyLane::Small => SMALL_ARMY_LOADOUT_SLOT_COUNT,
        EnemyArmyLane::Minor => MINOR_ARMY_LOADOUT_SLOT_COUNT,
        EnemyArmyLane::Major => MAJOR_ARMY_LOADOUT_SLOT_COUNT,
    }
}

fn enemy_army_progression_profile(
    difficulty: GameDifficulty,
    player_level: u32,
    wave_number: u32,
    lane: EnemyArmyLane,
    enemy_faction: PlayerFaction,
    spawn_seed: u32,
) -> EnemyArmyProgressionProfile {
    let army_level = enemy_army_level_for_difficulty(difficulty, player_level);
    let (major_upgrades, minor_upgrades) = major_minor_reward_counts_for_level(army_level);
    let equipment_fill_ratio = enemy_equipment_fill_ratio_for_difficulty(difficulty);
    let equipment = enemy_equipment_loadout_for_army(
        difficulty,
        lane,
        enemy_faction,
        wave_number,
        army_level,
        equipment_fill_ratio,
        spawn_seed,
    );
    let upgrade_pressure_multiplier = enemy_upgrade_pressure_multiplier(
        major_upgrades,
        minor_upgrades,
        difficulty,
        wave_number,
        spawn_seed,
    );
    EnemyArmyProgressionProfile {
        army_level,
        major_upgrades,
        minor_upgrades,
        equipment_fill_ratio,
        upgrade_pressure_multiplier,
        equipment,
    }
}

fn enemy_equipment_loadout_for_army(
    difficulty: GameDifficulty,
    lane: EnemyArmyLane,
    enemy_faction: PlayerFaction,
    wave_number: u32,
    army_level: u32,
    equipment_fill_ratio: f32,
    spawn_seed: u32,
) -> EnemyEquipmentLoadoutProfile {
    let slot_count = enemy_loadout_slot_count_for_lane(lane);
    let filled_slots = enemy_loadout_slot_fill_count(
        difficulty,
        lane,
        wave_number,
        slot_count,
        equipment_fill_ratio,
        spawn_seed,
    );
    if filled_slots == 0 {
        return EnemyEquipmentLoadoutProfile::default();
    }

    let seed = ((hash_seed(
        spawn_seed ^ lane_seed_salt(lane),
        wave_number ^ (army_level << 8) ^ difficulty_seed_salt(difficulty),
        filled_slots as u32,
    ) as u64)
        << 32)
        | hash_seed(
            spawn_seed ^ 0x74AD_1E93,
            wave_number ^ lane_seed_salt(lane),
            difficulty_seed_salt(difficulty),
        ) as u64;
    let items = roll_chest_items_from_seed(
        seed,
        enemy_faction,
        filled_slots,
        enemy_equipment_rarity_bonus_for_difficulty(difficulty),
    );
    let melee_bonuses = aggregate_item_bonuses_for_role(&items, UnitCombatRole::Melee);
    let ranged_bonuses = aggregate_item_bonuses_for_role(&items, UnitCombatRole::Ranged);
    let support_bonuses = aggregate_item_bonuses_for_role(&items, UnitCombatRole::Support);
    let power_multiplier = role_loadout_power_multiplier(melee_bonuses, filled_slots)
        .max(role_loadout_power_multiplier(ranged_bonuses, filled_slots))
        .max(role_loadout_power_multiplier(support_bonuses, filled_slots));

    EnemyEquipmentLoadoutProfile {
        filled_slots,
        template_ids: items.into_iter().map(|item| item.template_id).collect(),
        melee_bonuses,
        ranged_bonuses,
        support_bonuses,
        power_multiplier,
    }
}

fn enemy_loadout_slot_fill_count(
    difficulty: GameDifficulty,
    lane: EnemyArmyLane,
    wave_number: u32,
    slot_count: usize,
    equipment_fill_ratio: f32,
    spawn_seed: u32,
) -> usize {
    if slot_count == 0 || equipment_fill_ratio <= 0.0 {
        return 0;
    }
    let mut filled = 0usize;
    for slot_index in 0..slot_count {
        let roll = normalized_seed(hash_seed(
            spawn_seed ^ lane_seed_salt(lane),
            wave_number ^ ((slot_index as u32) << 4),
            difficulty_seed_salt(difficulty),
        ));
        if roll < equipment_fill_ratio {
            filled += 1;
        }
    }
    filled.clamp(1, slot_count)
}

fn role_loadout_power_multiplier(bonuses: UnitEquipmentBonuses, filled_slots: usize) -> f32 {
    let offensive_bonus = bonuses
        .melee_damage_multiplier
        .max(bonuses.ranged_damage_multiplier)
        + bonuses.attack_speed_multiplier * 0.6;
    let defense_bonus = bonuses.health_bonus / 120.0 + bonuses.armor_bonus * 0.025;
    let utility_bonus = bonuses.move_speed_bonus / 220.0 + bonuses.ranged_range_bonus / 1_000.0;
    let slot_bonus = filled_slots as f32 * 0.018;
    (1.0 + offensive_bonus * 0.35 + defense_bonus + utility_bonus + slot_bonus).clamp(1.0, 1.8)
}

fn difficulty_seed_salt(difficulty: GameDifficulty) -> u32 {
    match difficulty {
        GameDifficulty::Recruit => 0x1A77_91C3,
        GameDifficulty::Experienced => 0x4B62_D4AF,
        GameDifficulty::AloneAgainstTheInfidels => 0x73D3_2AA1,
    }
}

fn lane_seed_salt(lane: EnemyArmyLane) -> u32 {
    match lane {
        EnemyArmyLane::Small => 0x15F8_2B6D,
        EnemyArmyLane::Minor => 0x42C5_B9F0,
        EnemyArmyLane::Major => 0x9BA2_11CE,
    }
}

fn enemy_upgrade_pressure_multiplier(
    major_upgrades: u32,
    minor_upgrades: u32,
    difficulty: GameDifficulty,
    wave_number: u32,
    spawn_seed: u32,
) -> f32 {
    let base = 1.0 + major_upgrades as f32 * 0.03 + minor_upgrades as f32 * 0.004;
    let spread = match difficulty {
        GameDifficulty::Recruit => 0.03,
        GameDifficulty::Experienced => 0.05,
        GameDifficulty::AloneAgainstTheInfidels => 0.07,
    };
    let roll = normalized_seed(hash_seed(
        spawn_seed ^ 0xA5A5_1F1F,
        wave_number ^ (major_upgrades << 16),
        minor_upgrades,
    ));
    let variance = 1.0 + (roll * 2.0 - 1.0) * spread;
    (base * variance).clamp(1.0, 4.0)
}

fn combined_enemy_stat_multiplier(
    stat_scale: f32,
    faction_multiplier: f32,
    difficulty_multiplier: f32,
) -> f32 {
    stat_scale * faction_multiplier * difficulty_multiplier
}

fn combined_enemy_speed_multiplier(faction_multiplier: f32, difficulty_multiplier: f32) -> f32 {
    faction_multiplier * difficulty_multiplier
}

fn enqueue_wave_batch(
    wave_runtime: &mut WaveRuntime,
    count: u32,
    wave_number: u32,
    stat_scale: f32,
    lane: EnemyArmyLane,
) {
    if count == 0 {
        return;
    }
    wave_runtime.pending_batches.push(PendingEnemyBatch {
        remaining: count,
        wave_number,
        stat_scale,
        lane,
        next_spawn_time: wave_runtime.elapsed,
    });
}

pub fn wave_duration_secs() -> f32 {
    WAVE_DURATION_SECS
}

pub fn enemy_move_speed(base_speed: f32) -> f32 {
    base_speed * ENEMY_BASE_SPEED_MULTIPLIER
}

fn wave_base_count(config: &WavesConfigFile, wave_number: u32) -> f32 {
    if config.waves.is_empty() {
        return 1.0;
    }
    let index = wave_number.saturating_sub(1) as usize;
    if let Some(wave) = config.waves.get(index) {
        return wave.count as f32;
    }
    let last_count = config.waves.last().map(|wave| wave.count).unwrap_or(1) as f32;
    let extra_waves = index.saturating_sub(config.waves.len().saturating_sub(1)) as f32;
    last_count * POST_SCRIPTED_WAVE_COUNT_GROWTH.powf(extra_waves)
}

pub fn units_per_second_for_wave(config: &WavesConfigFile, wave_number: u32) -> f32 {
    let base_count = wave_base_count(config, wave_number);
    let wave_total = (base_count * WAVE_UNITS_MULTIPLIER).clamp(1.0, MAX_NON_ARMY_ENEMIES_PER_WAVE);
    wave_total / WAVE_DURATION_SECS
}

pub fn units_to_spawn_for_wave(config: &WavesConfigFile, wave_number: u32) -> u32 {
    let base_count = wave_base_count(config, wave_number);
    (base_count * WAVE_UNITS_MULTIPLIER)
        .clamp(1.0, MAX_NON_ARMY_ENEMIES_PER_WAVE)
        .round() as u32
}

pub fn army_units_to_spawn_for_wave(config: &WavesConfigFile, wave_number: u32) -> u32 {
    let base_count = wave_base_count(config, wave_number);
    (base_count * WAVE_UNITS_MULTIPLIER)
        .clamp(1.0, MAX_ARMY_ENEMIES_PER_WAVE)
        .round() as u32
}

pub fn wave_stat_multiplier(wave_number: u32) -> f32 {
    1.0 + wave_number.saturating_sub(1) as f32 * WAVE_STAT_GROWTH_PER_WAVE
}

pub fn enemy_player_pressure_multiplier(commander_level: u32, retinue_count: u32) -> f32 {
    let level_multiplier =
        1.0 + commander_level.saturating_sub(1) as f32 * ENEMY_LEVEL_PRESSURE_PER_LEVEL;
    let retinue_multiplier = 1.0
        + (retinue_count as f32).powf(ENEMY_RETINUE_PRESSURE_EXPONENT)
            * ENEMY_RETINUE_PRESSURE_PER_UNIT;
    (level_multiplier * retinue_multiplier).clamp(1.0, ENEMY_PLAYER_PRESSURE_STAT_CAP)
}

fn default_spawn_bounds() -> MapBounds {
    MapBounds {
        half_width: DEFAULT_SPAWN_HALF_WIDTH,
        half_height: DEFAULT_SPAWN_HALF_HEIGHT,
    }
}

fn batch_size_for_wave(wave_number: u32) -> u32 {
    (WAVE_BATCH_SIZE + wave_number / 4).clamp(WAVE_BATCH_SIZE, 22)
}

fn batch_interval_secs(wave_number: u32) -> f32 {
    (WAVE_BATCH_INTERVAL_SECS - wave_number as f32 * 0.01).clamp(0.24, WAVE_BATCH_INTERVAL_SECS)
}

pub fn should_trigger_victory(runtime: &WaveRuntime, alive_enemy_count: usize) -> bool {
    runtime.finished_spawning
        && runtime.current_wave >= MAX_WAVES
        && runtime.pending_batches.is_empty()
        && alive_enemy_count == 0
}

pub fn random_spawn_position(
    bounds: MapBounds,
    commander_position: Vec2,
    wave_seed: u32,
    spawn_sequence: u32,
) -> Vec2 {
    let min_distance_sq =
        ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER * ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER;
    let mut fallback = commander_position;
    for attempt in 0..ENEMY_SPAWN_ATTEMPTS {
        let seed = hash_seed(wave_seed, spawn_sequence, attempt);
        let x = normalized_seed(seed);
        let y = normalized_seed(seed ^ 0x9E37_79B9);
        let candidate = Vec2::new(
            lerp(-bounds.half_width, bounds.half_width, x),
            lerp(-bounds.half_height, bounds.half_height, y),
        );
        fallback = candidate;
        if candidate.distance_squared(commander_position) >= min_distance_sq {
            return candidate;
        }
    }
    fallback
}

fn random_spawn_position_from_rng(
    bounds: MapBounds,
    commander_position: Vec2,
    rng_state: &mut u64,
) -> Vec2 {
    let min_distance_sq =
        ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER * ENEMY_SPAWN_MIN_DISTANCE_FROM_COMMANDER;
    let mut fallback = commander_position;
    for attempt in 0..ENEMY_SPAWN_ATTEMPTS {
        let candidate = Vec2::new(
            lerp(
                -bounds.half_width,
                bounds.half_width,
                next_random_f32(rng_state),
            ),
            lerp(
                -bounds.half_height,
                bounds.half_height,
                next_random_f32(rng_state),
            ),
        );
        fallback = candidate;
        if candidate.distance_squared(commander_position) >= min_distance_sq
            || attempt == ENEMY_SPAWN_ATTEMPTS - 1
        {
            return candidate;
        }
    }
    fallback
}

fn hash_seed(wave_seed: u32, spawn_sequence: u32, attempt: u32) -> u32 {
    let mut value = wave_seed
        .wrapping_mul(1_103_515_245)
        .wrapping_add(spawn_sequence.wrapping_mul(747_796_405))
        .wrapping_add(attempt.wrapping_mul(2_891_336_453))
        .wrapping_add(0x9E37_79B9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^ (value >> 16)
}

fn normalized_seed(seed: u32) -> f32 {
    seed as f32 / u32::MAX as f32
}

fn rng_state_from_seed(seed: u32) -> u64 {
    let mixed = (seed as u64) ^ 0x9E37_79B9_7F4A_7C15;
    if mixed == 0 {
        0x9D4C_6F82_11B5_A7D3
    } else {
        mixed
    }
}

fn next_random_u32(rng_state: &mut u64) -> u32 {
    *rng_state = rng_state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*rng_state >> 32) as u32
}

fn next_random_f32(rng_state: &mut u64) -> f32 {
    next_random_u32(rng_state) as f32 / u32::MAX as f32
}

fn lerp(min: f32, max: f32, t: f32) -> f32 {
    min + (max - min) * t
}

fn runtime_seed_from_time() -> u32 {
    runtime_entropy_seed_u32()
}

#[allow(clippy::type_complexity)]
fn enemy_chase_targets(
    time: Res<Time>,
    data: Res<GameData>,
    setup_selection: Option<Res<MatchSetupSelection>>,
    mut enemy_sets: ParamSet<(
        Query<
            (
                Entity,
                &Unit,
                &MoveSpeed,
                Option<&Morale>,
                &AttackProfile,
                Option<&RangedAttackProfile>,
                &mut EnemyMovementState,
                &mut Transform,
            ),
            (With<EnemyUnit>, Without<FriendlyUnit>),
        >,
        Query<(Entity, &Transform, &Unit), (With<EnemyUnit>, Without<FriendlyUnit>)>,
    )>,
    friendlies: Query<
        (&Transform, Option<&CommanderUnit>),
        (With<FriendlyUnit>, Without<EnemyUnit>),
    >,
) {
    let delta_seconds = time.delta_seconds().max(0.0);
    let all_friendlies: Vec<(Vec2, bool)> = friendlies
        .iter()
        .map(|(transform, commander)| (transform.translation.truncate(), commander.is_some()))
        .collect();
    let difficulty = setup_selection
        .as_ref()
        .map(|selection| selection.difficulty)
        .unwrap_or(GameDifficulty::Recruit);
    let difficulty_mods = data.difficulties.for_difficulty(difficulty);
    let targets = chase_target_positions(&all_friendlies);
    if targets.is_empty() {
        return;
    }

    let all_enemy_positions: Vec<(Entity, Vec2, bool)> = enemy_sets
        .p1()
        .iter()
        .map(|(entity, transform, unit)| {
            (
                entity,
                transform.translation.truncate(),
                unit.kind.is_priest(),
            )
        })
        .collect();
    let non_support_follow_targets: Vec<(Entity, Vec2)> = all_enemy_positions
        .iter()
        .filter(|(_, _, is_priest)| !*is_priest)
        .map(|(entity, position, _)| (*entity, *position))
        .collect();
    let fallback_follow_targets: Vec<(Entity, Vec2)> = all_enemy_positions
        .iter()
        .map(|(entity, position, _)| (*entity, *position))
        .collect();

    for (
        enemy_entity,
        unit,
        move_speed,
        morale,
        attack_profile,
        ranged_profile,
        mut movement_state,
        mut enemy_transform,
    ) in &mut enemy_sets.p0()
    {
        let enemy_position = enemy_transform.translation.truncate();
        let frame_displacement = enemy_position.distance(movement_state.last_position);
        movement_state.last_position = enemy_position;
        movement_state.stuck_secs = next_enemy_stuck_secs(
            movement_state.moving,
            frame_displacement,
            movement_state.stuck_secs,
            delta_seconds,
        );

        if movement_state.crowd_hold_secs > 0.0 {
            movement_state.crowd_hold_secs =
                (movement_state.crowd_hold_secs - delta_seconds).max(0.0);
        }
        let target = if unit.kind.is_priest() {
            choose_support_follow_target(
                enemy_entity,
                enemy_position,
                &non_support_follow_targets,
                &fallback_follow_targets,
            )
        } else {
            choose_nearest(enemy_position, &targets)
        };
        if let Some(target) = target {
            let delta = target - enemy_position;
            let distance = delta.length();
            let prefers_spacing =
                enemy_prefers_ranged_spacing(unit.kind, attack_profile, ranged_profile);
            let desired_range = enemy_desired_engagement_range(
                attack_profile.range,
                ranged_profile,
                prefers_spacing && difficulty_mods.ranged_support_avoid_melee,
            );
            let stop_distance = (desired_range * STOP_FACTOR).max(10.0);
            let resume_distance = (desired_range * RESUME_FACTOR).max(stop_distance + 3.0);
            let crowded = crowded_enemy_neighbor_count(
                enemy_entity,
                enemy_position,
                &all_enemy_positions,
                ENEMY_CROWD_STOP_NEIGHBOR_RADIUS,
            ) >= ENEMY_CROWD_STOP_NEIGHBOR_COUNT;
            if should_start_crowd_hold(
                crowded,
                distance,
                desired_range,
                movement_state.crowd_hold_secs,
                movement_state.stuck_secs,
            ) {
                movement_state.crowd_hold_secs = ENEMY_CROWD_HOLD_SECS;
            }
            if movement_state.crowd_hold_secs > 0.0
                && distance <= (desired_range * ENEMY_CROWD_STOP_DISTANCE_FACTOR)
            {
                movement_state.moving = false;
                continue;
            }

            if difficulty_mods.ranged_support_avoid_melee && prefers_spacing {
                let avoid_trigger_distance = enemy_ranged_support_avoid_trigger_distance(
                    attack_profile.range,
                    ranged_profile,
                );
                if distance <= avoid_trigger_distance && distance > 0.001 {
                    let morale_speed_multiplier = morale
                        .copied()
                        .map(|value| morale_movement_multiplier(value.ratio()))
                        .unwrap_or(1.0);
                    let retreat_step = move_speed.0 * morale_speed_multiplier * delta_seconds;
                    if retreat_step > 0.0 {
                        let retreat_direction = (enemy_position - target).normalize();
                        enemy_transform.translation.x += retreat_direction.x * retreat_step;
                        enemy_transform.translation.y += retreat_direction.y * retreat_step;
                        movement_state.moving = true;
                        continue;
                    }
                }
            }

            movement_state.moving = should_move_towards_target(
                movement_state.moving,
                distance,
                stop_distance,
                resume_distance,
            );
            if movement_state.moving && distance > 0.001 {
                let morale_speed_multiplier = morale
                    .copied()
                    .map(|value| morale_movement_multiplier(value.ratio()))
                    .unwrap_or(1.0);
                let step_distance = chase_step_distance(
                    distance,
                    stop_distance,
                    move_speed.0 * morale_speed_multiplier * time.delta_seconds(),
                );
                if step_distance <= 0.0 {
                    continue;
                }
                let step = delta.normalize() * step_distance;
                enemy_transform.translation.x += step.x;
                enemy_transform.translation.y += step.y;
            }
        } else {
            movement_state.moving = false;
            movement_state.stuck_secs = 0.0;
        }
    }
}

fn next_enemy_stuck_secs(
    was_moving: bool,
    frame_displacement: f32,
    current_stuck_secs: f32,
    delta_seconds: f32,
) -> f32 {
    if delta_seconds <= 0.0 {
        return current_stuck_secs.max(0.0);
    }
    if was_moving && frame_displacement <= ENEMY_CROWD_STUCK_DISTANCE_EPS {
        return (current_stuck_secs + delta_seconds).max(0.0);
    }
    (current_stuck_secs - delta_seconds * ENEMY_CROWD_STUCK_DECAY_PER_SEC).max(0.0)
}

fn should_start_crowd_hold(
    crowded: bool,
    distance: f32,
    desired_range: f32,
    crowd_hold_secs: f32,
    stuck_secs: f32,
) -> bool {
    crowded
        && crowd_hold_secs <= 0.0
        && stuck_secs >= ENEMY_CROWD_STUCK_MIN_SECS
        && distance <= (desired_range * ENEMY_CROWD_STOP_DISTANCE_FACTOR)
}

fn crowded_enemy_neighbor_count(
    enemy_entity: Entity,
    enemy_position: Vec2,
    all_enemy_positions: &[(Entity, Vec2, bool)],
    radius: f32,
) -> usize {
    let radius_sq = radius * radius;
    all_enemy_positions
        .iter()
        .filter(|(other_entity, other_position, _)| {
            *other_entity != enemy_entity
                && enemy_position.distance_squared(*other_position) <= radius_sq
        })
        .count()
}

pub fn enemy_engagement_range(
    melee_range: f32,
    ranged_profile: Option<&RangedAttackProfile>,
) -> f32 {
    match ranged_profile {
        Some(profile) if profile.range > melee_range && profile.damage > 0.0 => profile.range,
        _ => melee_range,
    }
}

fn enemy_prefers_ranged_spacing(
    kind: UnitKind,
    attack_profile: &AttackProfile,
    ranged_profile: Option<&RangedAttackProfile>,
) -> bool {
    if kind.is_priest() {
        return true;
    }
    ranged_profile
        .map(|profile| profile.range > attack_profile.range && profile.damage > 0.0)
        .unwrap_or(false)
}

fn enemy_ranged_support_avoid_trigger_distance(
    melee_range: f32,
    ranged_profile: Option<&RangedAttackProfile>,
) -> f32 {
    let ranged_range = enemy_engagement_range(melee_range, ranged_profile);
    (ranged_range * 0.70).max(melee_range + 28.0)
}

fn enemy_desired_engagement_range(
    melee_range: f32,
    ranged_profile: Option<&RangedAttackProfile>,
    avoid_melee: bool,
) -> f32 {
    let base = enemy_engagement_range(melee_range, ranged_profile);
    if !avoid_melee {
        return base;
    }
    base.max(enemy_ranged_support_avoid_trigger_distance(
        melee_range,
        ranged_profile,
    ))
}

#[derive(Clone, Copy, Debug)]
struct InsideEnemySample {
    entity: Entity,
    position: Vec2,
    distance_sq: f32,
}

#[allow(clippy::type_complexity)]
fn repel_enemy_overflow_from_formation(
    data: Res<GameData>,
    active_formation: Res<ActiveFormation>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    recruits: Query<Entity, (With<FriendlyUnit>, Without<CommanderUnit>)>,
    mut enemies: Query<(Entity, &mut Transform), (With<EnemyUnit>, Without<CommanderUnit>)>,
) {
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };
    let recruit_count = recruits.iter().count();
    if recruit_count == 0 {
        return;
    }
    let commander_position = commander_transform.translation.truncate();
    let formation_cfg = active_formation_config(&data, *active_formation);
    let slot_spacing = formation_cfg.slot_spacing;
    if slot_spacing <= 0.0 {
        return;
    }

    let inside_cap = max_inside_enemy_count_for_formation(
        recruit_count,
        formation_anti_entry_enabled(&data, *active_formation),
        formation_allows_unlimited_enemy_inside(&data, *active_formation),
    );
    if inside_cap == usize::MAX {
        return;
    }
    let mut inside_samples = Vec::new();
    for (entity, transform) in &mut enemies {
        let enemy_position = transform.translation.truncate();
        if formation_contains_position(
            *active_formation,
            commander_position,
            enemy_position,
            recruit_count,
            slot_spacing,
            ENEMY_INSIDE_FORMATION_PADDING_SLOTS,
        ) {
            inside_samples.push(InsideEnemySample {
                entity,
                position: enemy_position,
                distance_sq: commander_position.distance_squared(enemy_position),
            });
        }
    }
    if inside_samples.len() <= inside_cap {
        return;
    }

    let distances: Vec<f32> = inside_samples
        .iter()
        .map(|sample| sample.distance_sq)
        .collect();
    let overflow_indices = overflow_indices_by_distance(&distances, inside_cap);
    for overflow_index in overflow_indices {
        let sample = inside_samples[overflow_index];
        let target = formation_overflow_repel_target(
            *active_formation,
            commander_position,
            sample.position,
            recruit_count,
            slot_spacing,
        );
        if let Ok((_, mut transform)) = enemies.get_mut(sample.entity) {
            transform.translation.x = target.x;
            transform.translation.y = target.y;
        }
    }
}

pub fn max_inside_enemy_count_for_formation(
    recruit_count: usize,
    anti_entry: bool,
    allow_unlimited_enemy_inside: bool,
) -> usize {
    if recruit_count == 0 {
        return 0;
    }
    if allow_unlimited_enemy_inside {
        return usize::MAX;
    }
    if anti_entry {
        return 0;
    }
    recruit_count / 4
}

pub fn overflow_indices_by_distance(distances_sq: &[f32], cap: usize) -> Vec<usize> {
    if distances_sq.len() <= cap {
        return Vec::new();
    }
    let mut sorted: Vec<(usize, f32)> = distances_sq.iter().copied().enumerate().collect();
    sorted.sort_by(|a, b| {
        a.1.partial_cmp(&b.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });
    let overflow_count = sorted.len().saturating_sub(cap);
    sorted
        .into_iter()
        .take(overflow_count)
        .map(|(index, _)| index)
        .collect()
}

pub fn formation_perimeter_target(
    active_formation: ActiveFormation,
    commander_position: Vec2,
    enemy_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
) -> Vec2 {
    if recruit_count == 0 || slot_spacing <= 0.0 {
        return enemy_position;
    }
    let half_extent = formation_half_extent(
        recruit_count,
        slot_spacing,
        ENEMY_INSIDE_FORMATION_PADDING_SLOTS,
    );
    let delta = enemy_position - commander_position;
    let local = formation_shape_perimeter_target(active_formation, delta, half_extent);
    commander_position + local
}

fn formation_overflow_repel_target(
    active_formation: ActiveFormation,
    commander_position: Vec2,
    enemy_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
) -> Vec2 {
    if recruit_count == 0 || slot_spacing <= 0.0 {
        return enemy_position;
    }
    let half_extent = formation_half_extent(
        recruit_count,
        slot_spacing,
        ENEMY_INSIDE_FORMATION_PADDING_SLOTS + ENEMY_FORMATION_REPEL_MARGIN_SLOTS,
    );
    let delta = enemy_position - commander_position;
    let local = formation_shape_perimeter_target(active_formation, delta, half_extent);
    commander_position + local
}

#[allow(clippy::type_complexity)]
fn update_bandit_visual_states(
    art: Res<ArtAssets>,
    mut enemies: Query<
        (
            &Unit,
            &Health,
            &AttackProfile,
            &AttackCooldown,
            &Transform,
            &mut BanditVisualRuntime,
            &mut Handle<Image>,
        ),
        With<EnemyUnit>,
    >,
) {
    for (unit, health, attack, attack_cd, transform, mut runtime, mut texture) in &mut enemies {
        let position = transform.translation.truncate();
        let moved_distance_sq = runtime.last_position.distance_squared(position);
        let next_state = decide_bandit_visual_state(
            moved_distance_sq,
            attack_cd.0.elapsed_secs(),
            attack.cooldown_secs,
            health.current,
            health.max,
        );

        if runtime.state != next_state {
            *texture = enemy_texture_for_state(&art, unit.kind, next_state);
            runtime.state = next_state;
        }
        runtime.last_position = position;
    }
}

fn enemy_texture_for_state(
    art: &ArtAssets,
    kind: UnitKind,
    _state: BanditVisualState,
) -> Handle<Image> {
    enemy_texture_for_kind(art, kind)
}

fn enemy_texture_for_kind(art: &ArtAssets, kind: UnitKind) -> Handle<Image> {
    let family = enemy_sprite_family_for_kind(kind);
    let Some(faction) = kind.faction() else {
        return art.enemy_bandit_raider_idle.clone();
    };

    match (faction, family) {
        (_, EnemySpriteFamily::Fallback) => art.enemy_bandit_raider_idle.clone(),
        (PlayerFaction::Christian, EnemySpriteFamily::Infantry) => {
            art.friendly_peasant_infantry_idle.clone()
        }
        (PlayerFaction::Christian, EnemySpriteFamily::Archer) => {
            art.friendly_peasant_archer_idle.clone()
        }
        (PlayerFaction::Christian, EnemySpriteFamily::Priest) => {
            art.friendly_peasant_priest_idle.clone()
        }
        (PlayerFaction::Muslim, EnemySpriteFamily::Infantry) => {
            art.muslim_peasant_infantry_idle.clone()
        }
        (PlayerFaction::Muslim, EnemySpriteFamily::Archer) => {
            art.muslim_peasant_archer_idle.clone()
        }
        (PlayerFaction::Muslim, EnemySpriteFamily::Priest) => {
            art.muslim_peasant_priest_idle.clone()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnemySpriteFamily {
    Infantry,
    Archer,
    Priest,
    Fallback,
}

fn enemy_sprite_family_for_kind(kind: UnitKind) -> EnemySpriteFamily {
    if kind == UnitKind::Commander || kind.is_rescuable_variant() {
        return EnemySpriteFamily::Fallback;
    }
    if kind.is_archer_line() {
        EnemySpriteFamily::Archer
    } else if kind.is_priest_family_line() {
        EnemySpriteFamily::Priest
    } else {
        EnemySpriteFamily::Infantry
    }
}

pub fn decide_bandit_visual_state(
    moved_distance_sq: f32,
    cooldown_elapsed_secs: f32,
    attack_cooldown_secs: f32,
    current_hp: f32,
    max_hp: f32,
) -> BanditVisualState {
    if current_hp <= 0.0 {
        return BanditVisualState::Dead;
    }
    let hp_ratio = (current_hp / max_hp).clamp(0.0, 1.0);
    if hp_ratio <= 0.35 {
        return BanditVisualState::Hit;
    }
    let attack_window = (attack_cooldown_secs * 0.2).clamp(0.06, 0.2);
    if cooldown_elapsed_secs <= attack_window {
        return BanditVisualState::Attack;
    }
    if moved_distance_sq > 1.0 {
        BanditVisualState::Move
    } else {
        BanditVisualState::Idle
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{Entity, Vec2};
    use std::path::Path;

    use crate::ai::{
        chase_step_distance, chase_target_positions, choose_nearest, should_move_towards_target,
    };
    use crate::combat::RangedAttackProfile;
    use crate::data::{GameData, WaveConfig, WavesConfigFile};
    use crate::enemies::{
        BanditVisualState, EnemyRoleCounts, EnemySpawnRole, army_units_to_spawn_for_wave,
        batch_interval_secs, batch_size_for_wave, build_enemy_pool_roles_for_wave,
        choose_enemy_kind_for_role, combined_enemy_speed_multiplier,
        combined_enemy_stat_multiplier, crowded_enemy_neighbor_count, decide_bandit_visual_state,
        enemy_army_level_for_difficulty, enemy_army_progression_profile,
        enemy_desired_engagement_range, enemy_engagement_range, enemy_move_speed,
        enemy_player_pressure_multiplier, enemy_prefers_ranged_spacing,
        enemy_ranged_support_avoid_trigger_distance, enemy_tier_mix_for_wave,
        formation_overflow_repel_target, formation_perimeter_target, is_major_army_wave,
        is_minor_army_wave, major_wave_preview_target_count_for_batch,
        max_inside_enemy_count_for_formation, next_enemy_stuck_secs, overflow_indices_by_distance,
        pick_next_spawn_role, planned_wave_army_batches, random_spawn_position,
        should_start_crowd_hold, units_per_second_for_wave, units_to_spawn_for_wave,
        wave_duration_secs, wave_stat_multiplier,
    };
    use crate::formation::{ActiveFormation, formation_contains_position};
    use crate::map::MapBounds;
    use crate::model::{GameDifficulty, PlayerFaction, UnitKind};

    #[test]
    fn chooses_nearest_target() {
        let origin = Vec2::new(0.0, 0.0);
        let targets = [
            Vec2::new(5.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(10.0, 0.0),
        ];
        let nearest = choose_nearest(origin, &targets).expect("target");
        assert_eq!(nearest, Vec2::new(2.0, 0.0));
    }

    #[test]
    fn no_targets_returns_none() {
        assert_eq!(choose_nearest(Vec2::ZERO, &[]), None);
    }

    #[test]
    fn bandit_visual_state_priority_is_dead_then_hit_then_attack_then_move_idle() {
        assert_eq!(
            decide_bandit_visual_state(0.0, 0.1, 1.0, 0.0, 10.0),
            BanditVisualState::Dead
        );
        assert_eq!(
            decide_bandit_visual_state(10.0, 0.1, 1.0, 3.0, 10.0),
            BanditVisualState::Hit
        );
        assert_eq!(
            decide_bandit_visual_state(10.0, 0.05, 1.0, 9.0, 10.0),
            BanditVisualState::Attack
        );
        assert_eq!(
            decide_bandit_visual_state(2.0, 0.8, 1.0, 9.0, 10.0),
            BanditVisualState::Move
        );
        assert_eq!(
            decide_bandit_visual_state(0.1, 0.8, 1.0, 9.0, 10.0),
            BanditVisualState::Idle
        );
    }

    #[test]
    fn enemy_sprite_family_uses_generic_unit_ids() {
        assert_eq!(
            super::enemy_sprite_family_for_kind(UnitKind::ChristianTracker),
            super::EnemySpriteFamily::Archer
        );
        assert_eq!(
            super::enemy_sprite_family_for_kind(UnitKind::MuslimTracker),
            super::EnemySpriteFamily::Archer
        );
        assert_eq!(
            super::enemy_sprite_family_for_kind(UnitKind::ChristianFanatic),
            super::EnemySpriteFamily::Priest
        );
        assert_eq!(
            super::enemy_sprite_family_for_kind(UnitKind::MuslimFanatic),
            super::EnemySpriteFamily::Priest
        );
        assert_eq!(
            super::enemy_sprite_family_for_kind(UnitKind::ChristianMenAtArms),
            super::EnemySpriteFamily::Infantry
        );
    }

    #[test]
    fn wave_progression_scales_spawn_rate_and_stats() {
        let config = WavesConfigFile {
            waves: vec![
                WaveConfig {
                    time_secs: 0.0,
                    count: 8,
                },
                WaveConfig {
                    time_secs: 30.0,
                    count: 12,
                },
            ],
        };
        let first = units_per_second_for_wave(&config, 1);
        let second = units_per_second_for_wave(&config, 2);
        let late = units_per_second_for_wave(&config, 10);
        assert!(second >= first);
        assert!(late > second);
        assert_eq!(wave_duration_secs(), 30.0);
        assert!((wave_stat_multiplier(1) - 1.0).abs() < 0.001);
        assert!((wave_stat_multiplier(2) - 1.102).abs() < 0.001);
        assert!(wave_stat_multiplier(8) > wave_stat_multiplier(3));
        assert!(enemy_move_speed(100.0) < 100.0);
    }

    #[test]
    fn wave_batch_planner_layers_small_minor_and_major_armies() {
        let wave_1 = planned_wave_army_batches(20, 1, 1.0);
        assert_eq!(wave_1.len(), 1);
        assert_eq!(wave_1[0].lane, super::EnemyArmyLane::Small);

        let wave_2 = planned_wave_army_batches(20, 2, 1.0);
        assert_eq!(wave_2.len(), 2);
        assert!(
            wave_2
                .iter()
                .any(|entry| entry.lane == super::EnemyArmyLane::Small)
        );
        assert!(
            wave_2
                .iter()
                .any(|entry| entry.lane == super::EnemyArmyLane::Minor)
        );
        assert!(
            !wave_2
                .iter()
                .any(|entry| entry.lane == super::EnemyArmyLane::Major)
        );

        let wave_10 = planned_wave_army_batches(20, 10, 1.0);
        assert_eq!(wave_10.len(), 3);
        assert!(
            wave_10
                .iter()
                .any(|entry| entry.lane == super::EnemyArmyLane::Small)
        );
        assert!(
            wave_10
                .iter()
                .any(|entry| entry.lane == super::EnemyArmyLane::Minor)
        );
        assert!(
            wave_10
                .iter()
                .any(|entry| entry.lane == super::EnemyArmyLane::Major)
        );
        assert!(is_minor_army_wave(10));
        assert!(is_major_army_wave(10));
    }

    #[test]
    fn wave_batch_planner_increases_lane_batch_count_over_time() {
        let wave_21 = planned_wave_army_batches(120, 21, 1.0);
        let wave_22 = planned_wave_army_batches(120, 22, 1.0);
        let wave_30 = planned_wave_army_batches(120, 30, 1.0);

        let wave_21_small = wave_21
            .iter()
            .filter(|entry| entry.lane == super::EnemyArmyLane::Small)
            .count();
        assert!(
            wave_21_small >= 2,
            "small lane should split into multiple army batches after wave 20"
        );

        let wave_22_minor = wave_22
            .iter()
            .filter(|entry| entry.lane == super::EnemyArmyLane::Minor)
            .count();
        assert!(
            wave_22_minor >= 2,
            "minor lane should split into multiple army batches after wave 20"
        );

        let wave_30_major = wave_30
            .iter()
            .filter(|entry| entry.lane == super::EnemyArmyLane::Major)
            .count();
        assert!(
            wave_30_major >= 2,
            "major lane should split into multiple army batches by wave 30"
        );
    }

    #[test]
    fn army_wave_spawn_count_uses_separate_cap_from_non_army_flow() {
        let config = WavesConfigFile {
            waves: vec![
                WaveConfig {
                    time_secs: 0.0,
                    count: 8,
                },
                WaveConfig {
                    time_secs: 30.0,
                    count: 12,
                },
            ],
        };

        let non_army = units_to_spawn_for_wave(&config, 40);
        let army = army_units_to_spawn_for_wave(&config, 40);
        assert!(non_army <= 200);
        assert!(army > non_army);
    }

    #[test]
    fn wave_tier_mix_ramps_between_unlock_milestones() {
        let wave_1 =
            enemy_tier_mix_for_wave(1, super::EnemyArmyLane::Small, GameDifficulty::Recruit);
        assert_eq!(
            wave_1,
            [
                super::EnemyTierWeight {
                    tier: 0,
                    weight_percent: 100,
                },
                super::EnemyTierWeight {
                    tier: 0,
                    weight_percent: 0,
                },
            ]
        );

        let wave_11 =
            enemy_tier_mix_for_wave(11, super::EnemyArmyLane::Small, GameDifficulty::Recruit);
        assert_eq!(wave_11[0].tier, 0);
        assert_eq!(wave_11[0].weight_percent, 90);
        assert_eq!(wave_11[1].tier, 1);
        assert_eq!(wave_11[1].weight_percent, 10);

        let wave_20 =
            enemy_tier_mix_for_wave(20, super::EnemyArmyLane::Small, GameDifficulty::Recruit);
        assert_eq!(wave_20[0].tier, 0);
        assert_eq!(wave_20[0].weight_percent, 0);
        assert_eq!(wave_20[1].tier, 1);
        assert_eq!(wave_20[1].weight_percent, 100);

        let wave_21 =
            enemy_tier_mix_for_wave(21, super::EnemyArmyLane::Small, GameDifficulty::Recruit);
        assert_eq!(wave_21[0].tier, 1);
        assert_eq!(wave_21[0].weight_percent, 90);
        assert_eq!(wave_21[1].tier, 2);
        assert_eq!(wave_21[1].weight_percent, 10);
    }

    #[test]
    fn major_wave_tier_mix_previews_next_tier_by_difficulty() {
        let recruit =
            enemy_tier_mix_for_wave(20, super::EnemyArmyLane::Major, GameDifficulty::Recruit);
        assert_eq!(recruit[0].tier, 1);
        assert_eq!(recruit[0].weight_percent, 80);
        assert_eq!(recruit[1].tier, 2);
        assert_eq!(recruit[1].weight_percent, 20);

        let experienced =
            enemy_tier_mix_for_wave(20, super::EnemyArmyLane::Major, GameDifficulty::Experienced);
        assert_eq!(experienced[0].weight_percent, 65);
        assert_eq!(experienced[1].weight_percent, 35);

        let hard = enemy_tier_mix_for_wave(
            20,
            super::EnemyArmyLane::Major,
            GameDifficulty::AloneAgainstTheInfidels,
        );
        assert_eq!(hard[0].weight_percent, 50);
        assert_eq!(hard[1].weight_percent, 50);

        let capped =
            enemy_tier_mix_for_wave(60, super::EnemyArmyLane::Major, GameDifficulty::Recruit);
        assert_eq!(capped[0].tier, 5);
        assert_eq!(capped[0].weight_percent, 100);
        assert_eq!(capped[1].weight_percent, 0);
    }

    #[test]
    fn major_preview_target_count_tracks_configured_share() {
        assert_eq!(major_wave_preview_target_count_for_batch(0, 20), 0);
        assert_eq!(major_wave_preview_target_count_for_batch(1, 20), 1);
        assert_eq!(major_wave_preview_target_count_for_batch(3, 20), 1);
        assert_eq!(major_wave_preview_target_count_for_batch(10, 20), 2);
        assert_eq!(major_wave_preview_target_count_for_batch(9, 35), 3);
        assert_eq!(major_wave_preview_target_count_for_batch(20, 50), 10);
    }

    #[test]
    fn wave_enemy_pool_builder_uses_expected_tier_bands() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("data");
        let wave_20_pool = build_enemy_pool_roles_for_wave(
            &data.enemy_tier_pools,
            PlayerFaction::Muslim,
            20,
            super::EnemyArmyLane::Small,
            GameDifficulty::Recruit,
        );
        assert!(
            wave_20_pool
                .iter()
                .all(|(kind, _)| kind.tier_hint() == Some(1)),
            "wave 20 regular should be fully tier-1"
        );

        let wave_21_pool = build_enemy_pool_roles_for_wave(
            &data.enemy_tier_pools,
            PlayerFaction::Muslim,
            21,
            super::EnemyArmyLane::Small,
            GameDifficulty::Recruit,
        );
        assert!(
            wave_21_pool
                .iter()
                .any(|(kind, _)| kind.tier_hint() == Some(1))
        );
        assert!(
            wave_21_pool
                .iter()
                .any(|(kind, _)| kind.tier_hint() == Some(2))
        );

        let wave_20_major_pool = build_enemy_pool_roles_for_wave(
            &data.enemy_tier_pools,
            PlayerFaction::Muslim,
            20,
            super::EnemyArmyLane::Major,
            GameDifficulty::Recruit,
        );
        assert!(
            wave_20_major_pool
                .iter()
                .any(|(kind, _)| kind.tier_hint() == Some(2)),
            "major wave should include next-tier preview"
        );
    }

    #[test]
    fn enemy_army_level_depends_on_difficulty() {
        assert_eq!(
            enemy_army_level_for_difficulty(GameDifficulty::Recruit, 30),
            15
        );
        assert_eq!(
            enemy_army_level_for_difficulty(GameDifficulty::Experienced, 30),
            30
        );
        assert_eq!(
            enemy_army_level_for_difficulty(GameDifficulty::AloneAgainstTheInfidels, 30),
            30
        );
    }

    #[test]
    fn enemy_army_profile_uses_major_minor_parity_formula() {
        let recruit = enemy_army_progression_profile(
            GameDifficulty::Recruit,
            30,
            10,
            super::EnemyArmyLane::Major,
            PlayerFaction::Muslim,
            123,
        );
        assert_eq!(recruit.army_level, 15);
        assert_eq!(recruit.major_upgrades, 3);
        assert_eq!(recruit.minor_upgrades, 12);
        assert!(recruit.equipment.filled_slots > 0);
        assert_eq!(
            recruit.equipment.template_ids.len(),
            recruit.equipment.filled_slots
        );

        let experienced = enemy_army_progression_profile(
            GameDifficulty::Experienced,
            30,
            10,
            super::EnemyArmyLane::Major,
            PlayerFaction::Muslim,
            123,
        );
        assert_eq!(experienced.army_level, 30);
        assert_eq!(experienced.major_upgrades, 6);
        assert_eq!(experienced.minor_upgrades, 24);
        assert!(experienced.upgrade_pressure_multiplier >= 1.0);
        assert!(experienced.equipment.power_multiplier >= 1.0);
    }

    #[test]
    fn enemy_loadout_fill_and_seed_are_deterministic_per_difficulty() {
        let recruit_a = enemy_army_progression_profile(
            GameDifficulty::Recruit,
            40,
            20,
            super::EnemyArmyLane::Major,
            PlayerFaction::Christian,
            991,
        );
        let recruit_b = enemy_army_progression_profile(
            GameDifficulty::Recruit,
            40,
            20,
            super::EnemyArmyLane::Major,
            PlayerFaction::Christian,
            991,
        );
        assert_eq!(
            recruit_a.equipment.template_ids,
            recruit_b.equipment.template_ids
        );
        assert_eq!(
            recruit_a.equipment.filled_slots,
            recruit_b.equipment.filled_slots
        );

        let experienced = enemy_army_progression_profile(
            GameDifficulty::Experienced,
            40,
            20,
            super::EnemyArmyLane::Major,
            PlayerFaction::Christian,
            991,
        );
        assert!(experienced.equipment.filled_slots >= recruit_a.equipment.filled_slots);
        assert!(experienced.equipment.power_multiplier >= recruit_a.equipment.power_multiplier);
    }

    #[test]
    fn player_pressure_multiplier_increases_with_level_and_retinue() {
        let baseline = enemy_player_pressure_multiplier(1, 0);
        let mid = enemy_player_pressure_multiplier(30, 20);
        let late = enemy_player_pressure_multiplier(100, 120);
        assert!((baseline - 1.0).abs() < 0.001);
        assert!(mid > baseline);
        assert!(late > mid);
    }

    #[test]
    fn player_pressure_multiplier_outpaces_passive_level_scaling_with_retinue() {
        let commander_level: u32 = 60;
        let retinue_count: u32 = 40;
        let passive_friendly = 1.0 + commander_level.saturating_sub(1) as f32 * 0.01;
        let enemy_pressure = enemy_player_pressure_multiplier(commander_level, retinue_count);
        assert!(enemy_pressure > passive_friendly);
    }

    #[test]
    fn combined_enemy_multipliers_include_difficulty_layer() {
        let stat = combined_enemy_stat_multiplier(1.5, 1.1, 1.2);
        assert!((stat - 1.98).abs() < 0.001);

        let speed = combined_enemy_speed_multiplier(1.05, 1.15);
        assert!((speed - 1.2075).abs() < 0.001);
    }

    #[test]
    fn units_per_second_is_capped_to_two_hundred_enemies_per_wave() {
        let config = WavesConfigFile {
            waves: vec![WaveConfig {
                time_secs: 0.0,
                count: 700,
            }],
        };
        let rate = units_per_second_for_wave(&config, 1);
        assert!((rate - (200.0 / 30.0)).abs() < 0.001);
    }

    #[test]
    fn wave_role_picker_enforces_support_cap_and_target_mix() {
        let mut counts = EnemyRoleCounts::default();
        for seed in 0..40 {
            let role = pick_next_spawn_role(counts, true, true, true, seed);
            counts.register(role);
            assert!(counts.support <= counts.total / 4);
        }
        assert_eq!(counts.total, 40);
        assert_eq!(counts.support, 10);
        assert_eq!(counts.melee + counts.ranged, 30);
        assert!((counts.melee as i32 - counts.ranged as i32).abs() <= 1);
    }

    #[test]
    fn wave_role_picker_falls_back_when_support_is_only_available_role() {
        let mut counts = EnemyRoleCounts::default();
        for seed in 0..7 {
            let role = pick_next_spawn_role(counts, false, false, true, seed);
            assert_eq!(role, EnemySpawnRole::Support);
            counts.register(role);
        }
        assert_eq!(counts.total, 7);
        assert_eq!(counts.support, 7);
        assert_eq!(counts.melee, 0);
        assert_eq!(counts.ranged, 0);
    }

    #[test]
    fn enemy_kind_picker_uses_faction_aware_fallback_when_pool_is_empty() {
        let fallback =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Muslim, "peasant_infantry", false)
                .expect("fallback kind should resolve");
        let selected = choose_enemy_kind_for_role(&[], EnemySpawnRole::Melee, 42, fallback);
        assert_eq!(selected, UnitKind::MuslimPeasantInfantry);
    }

    #[test]
    fn inside_formation_cap_scales_with_roster_size() {
        assert_eq!(max_inside_enemy_count_for_formation(0, false, false), 0);
        assert_eq!(max_inside_enemy_count_for_formation(3, false, false), 0);
        assert_eq!(max_inside_enemy_count_for_formation(4, false, false), 1);
        assert_eq!(max_inside_enemy_count_for_formation(40, false, false), 10);
        assert_eq!(max_inside_enemy_count_for_formation(180, false, false), 45);
        assert_eq!(max_inside_enemy_count_for_formation(40, true, false), 0);
        assert_eq!(
            max_inside_enemy_count_for_formation(40, false, true),
            usize::MAX
        );
    }

    #[test]
    fn overflow_selection_keeps_outermost_inside_formation_cap() {
        let overflow = overflow_indices_by_distance(&[16.0, 4.0, 9.0, 1.0, 25.0], 2);
        assert_eq!(overflow.len(), 3);
        assert!(overflow.contains(&1));
        assert!(overflow.contains(&2));
        assert!(overflow.contains(&3));
    }

    #[test]
    fn square_perimeter_target_projects_to_boundary() {
        let target = formation_perimeter_target(
            ActiveFormation::Square,
            Vec2::ZERO,
            Vec2::new(10.0, 5.0),
            16,
            30.0,
        );
        assert!(target.x.abs() > target.y.abs());
        assert!(target.x.abs() > 20.0);
    }

    #[test]
    fn diamond_perimeter_target_projects_to_boundary() {
        let target = formation_perimeter_target(
            ActiveFormation::Diamond,
            Vec2::ZERO,
            Vec2::new(7.0, 4.0),
            16,
            30.0,
        );
        let diamond_radius = ((16.0f32 + 1.0).sqrt().ceil() - 1.0) * 0.5 + 0.35;
        let expected = diamond_radius * 30.0 * std::f32::consts::SQRT_2;
        assert!(((target.x.abs() + target.y.abs()) - expected).abs() < 0.2);
    }

    #[test]
    fn circle_perimeter_target_projects_to_boundary() {
        let target = formation_perimeter_target(
            ActiveFormation::Circle,
            Vec2::ZERO,
            Vec2::new(7.0, 4.0),
            16,
            30.0,
        );
        let radius =
            (((16.0f32 + 1.0).sqrt().ceil() - 1.0) * 0.5 + 0.35) * 30.0 * std::f32::consts::SQRT_2;
        assert!((target.length() - radius).abs() < 0.25);
    }

    #[test]
    fn overflow_repel_target_is_outside_formation_footprint() {
        let commander = Vec2::ZERO;
        let recruit_count = 40;
        let spacing = 30.0;
        let sample = Vec2::new(10.0, 10.0);

        let square_target = formation_overflow_repel_target(
            ActiveFormation::Square,
            commander,
            sample,
            recruit_count,
            spacing,
        );
        assert!(!formation_contains_position(
            ActiveFormation::Square,
            commander,
            square_target,
            recruit_count,
            spacing,
            0.35,
        ));

        let diamond_target = formation_overflow_repel_target(
            ActiveFormation::Diamond,
            commander,
            sample,
            recruit_count,
            spacing,
        );
        assert!(!formation_contains_position(
            ActiveFormation::Diamond,
            commander,
            diamond_target,
            recruit_count,
            spacing,
            0.35,
        ));
    }

    #[test]
    fn chase_targets_exclude_commander_when_retinue_exists() {
        let commander = (Vec2::new(0.0, 0.0), true);
        let retinue = (Vec2::new(10.0, 0.0), false);
        let targets = chase_target_positions(&[commander, retinue]);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], retinue.0);

        let commander_only = chase_target_positions(&[commander]);
        assert_eq!(commander_only, vec![commander.0]);
    }

    #[test]
    fn movement_hysteresis_prevents_stop_resume_jitter() {
        assert!(!should_move_towards_target(true, 20.0, 22.0, 26.0));
        assert!(!should_move_towards_target(false, 24.0, 22.0, 26.0));
        assert!(should_move_towards_target(false, 30.0, 22.0, 26.0));
    }

    #[test]
    fn chase_step_distance_prevents_overshoot_into_stop_range() {
        let step = chase_step_distance(24.0, 20.0, 10.0);
        assert!((step - 4.0).abs() < 0.001);
        assert_eq!(chase_step_distance(19.5, 20.0, 10.0), 0.0);
        assert_eq!(chase_step_distance(30.0, 20.0, 0.0), 0.0);
    }

    #[test]
    fn engagement_range_prefers_valid_ranged_profile() {
        let ranged = RangedAttackProfile {
            damage: 8.0,
            range: 260.0,
            projectile_speed: 200.0,
            projectile_max_distance: 300.0,
        };
        assert!((enemy_engagement_range(30.0, Some(&ranged)) - 260.0).abs() < 0.001);
        assert!((enemy_engagement_range(30.0, None) - 30.0).abs() < 0.001);
    }

    #[test]
    fn ranged_and_priest_units_prefer_spacing_when_enabled() {
        let melee_profile = crate::model::AttackProfile {
            damage: 4.0,
            range: 34.0,
            cooldown_secs: 1.0,
        };
        let ranged_profile = RangedAttackProfile {
            damage: 8.0,
            range: 260.0,
            projectile_speed: 200.0,
            projectile_max_distance: 320.0,
        };
        assert!(enemy_prefers_ranged_spacing(
            crate::model::UnitKind::MuslimPeasantArcher,
            &melee_profile,
            Some(&ranged_profile),
        ));
        assert!(enemy_prefers_ranged_spacing(
            crate::model::UnitKind::MuslimPeasantPriest,
            &melee_profile,
            None,
        ));
        assert!(!enemy_prefers_ranged_spacing(
            crate::model::UnitKind::MuslimPeasantInfantry,
            &melee_profile,
            None,
        ));
    }

    #[test]
    fn spacing_helpers_expand_engagement_window_for_avoidance_mode() {
        let ranged_profile = RangedAttackProfile {
            damage: 8.0,
            range: 240.0,
            projectile_speed: 200.0,
            projectile_max_distance: 300.0,
        };
        let trigger = enemy_ranged_support_avoid_trigger_distance(30.0, Some(&ranged_profile));
        let desired = enemy_desired_engagement_range(30.0, Some(&ranged_profile), true);
        let baseline = enemy_desired_engagement_range(30.0, Some(&ranged_profile), false);
        assert!(trigger > 30.0);
        assert!(desired >= trigger);
        assert!(desired >= baseline);
    }

    #[test]
    fn wave_batches_scale_size_and_reduce_interval_over_time() {
        assert!(batch_size_for_wave(0) >= 7);
        assert!(batch_size_for_wave(18) > batch_size_for_wave(2));
        assert!(batch_interval_secs(10) < batch_interval_secs(0));
    }

    #[test]
    fn random_spawn_positions_stay_inside_bounds_and_off_commander() {
        let bounds = MapBounds {
            half_width: 500.0,
            half_height: 350.0,
        };
        let commander = Vec2::ZERO;
        for sequence in 0..80 {
            let point = random_spawn_position(bounds, commander, 4, sequence);
            assert!(point.x >= -500.0 && point.x <= 500.0);
            assert!(point.y >= -350.0 && point.y <= 350.0);
            assert!(point.length() >= 170.0);
        }
    }

    #[test]
    fn crowded_neighbor_count_ignores_self_and_out_of_radius_entities() {
        let entries = vec![
            (Entity::from_raw(1), Vec2::new(0.0, 0.0), false),
            (Entity::from_raw(2), Vec2::new(10.0, 0.0), false),
            (Entity::from_raw(3), Vec2::new(15.0, 0.0), true),
            (Entity::from_raw(4), Vec2::new(50.0, 0.0), false),
        ];
        let count =
            crowded_enemy_neighbor_count(Entity::from_raw(1), Vec2::new(0.0, 0.0), &entries, 20.0);
        assert_eq!(count, 2);
    }

    #[test]
    fn stuck_seconds_accumulate_while_moving_but_not_progressing_and_decay_otherwise() {
        let growing = next_enemy_stuck_secs(true, 0.2, 0.1, 0.16);
        assert!(growing > 0.24);
        let decaying = next_enemy_stuck_secs(false, 2.0, growing, 0.16);
        assert!(decaying < growing);
        assert!(decaying >= 0.0);
    }

    #[test]
    fn crowd_hold_requires_crowd_range_and_stuck_threshold() {
        assert!(should_start_crowd_hold(true, 24.0, 20.0, 0.0, 0.3));
        assert!(!should_start_crowd_hold(true, 24.0, 20.0, 0.1, 0.3));
        assert!(!should_start_crowd_hold(false, 24.0, 20.0, 0.0, 0.3));
        assert!(!should_start_crowd_hold(true, 40.0, 20.0, 0.0, 0.3));
        assert!(!should_start_crowd_hold(true, 24.0, 20.0, 0.0, 0.05));
    }
}
