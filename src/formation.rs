use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

use crate::banner::BannerMovementPenalty;
use crate::data::{FormationConfig, GameData};
use crate::model::{
    CommanderUnit, FriendlyUnit, GameState, StartRunEvent, Unit, UnitKind, UnitRoleTag,
};
use crate::squad::OutOfFormation;

pub const SKILL_BAR_CAPACITY: usize = 10;

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActiveFormation {
    #[default]
    Square,
    Circle,
    Skean,
    Diamond,
    ShieldWall,
    Loose,
}

impl ActiveFormation {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Circle => "circle",
            Self::Skean => "skean",
            Self::Diamond => "diamond",
            Self::ShieldWall => "shield_wall",
            Self::Loose => "loose",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Square => "Square Formation",
            Self::Circle => "Circle Formation",
            Self::Skean => "Skean Formation",
            Self::Diamond => "Diamond Formation",
            Self::ShieldWall => "Shield Wall Formation",
            Self::Loose => "Loose Formation",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "square" => Some(Self::Square),
            "circle" => Some(Self::Circle),
            "skean" => Some(Self::Skean),
            "diamond" => Some(Self::Diamond),
            "shield_wall" => Some(Self::ShieldWall),
            "loose" => Some(Self::Loose),
            _ => None,
        }
    }

    pub const fn all() -> [Self; 6] {
        [
            Self::Square,
            Self::Circle,
            Self::Skean,
            Self::Diamond,
            Self::ShieldWall,
            Self::Loose,
        ]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkillBarSkillKind {
    Formation(ActiveFormation),
}

#[derive(Clone, Debug)]
pub struct SkillBarSkill {
    pub id: String,
    pub label: String,
    pub kind: SkillBarSkillKind,
}

#[derive(Resource, Clone, Debug)]
pub struct FormationSkillBar {
    pub slots: Vec<SkillBarSkill>,
    pub active_slot: Option<usize>,
}

impl Default for FormationSkillBar {
    fn default() -> Self {
        Self {
            slots: vec![skill_for_formation(ActiveFormation::Square)],
            active_slot: Some(0),
        }
    }
}

impl FormationSkillBar {
    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }

    pub fn is_full(&self) -> bool {
        self.slots.len() >= SKILL_BAR_CAPACITY
    }

    pub fn has_formation(&self, formation: ActiveFormation) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.kind == SkillBarSkillKind::Formation(formation))
    }

    pub fn try_add_formation(&mut self, formation: ActiveFormation) -> bool {
        if self.is_full() || self.has_formation(formation) {
            return false;
        }
        self.slots.push(skill_for_formation(formation));
        true
    }

    pub fn activate_slot(&mut self, slot_index: usize) -> Option<SkillBarSkillKind> {
        if slot_index >= self.slots.len() {
            return None;
        }
        self.active_slot = Some(slot_index);
        Some(self.slots[slot_index].kind)
    }
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct FormationModifiers {
    pub offense_multiplier: f32,
    pub offense_while_moving_multiplier: f32,
    pub defense_multiplier: f32,
    pub move_speed_multiplier: f32,
}

impl Default for FormationModifiers {
    fn default() -> Self {
        Self {
            offense_multiplier: 1.0,
            offense_while_moving_multiplier: 1.0,
            defense_multiplier: 1.0,
            move_speed_multiplier: 1.0,
        }
    }
}

pub struct FormationPlugin;

impl Plugin for FormationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveFormation>()
            .init_resource::<FormationModifiers>()
            .init_resource::<FormationSkillBar>()
            .init_resource::<FormationLaneSummary>()
            .add_systems(Update, reset_formation_state_on_run_start)
            .add_systems(
                OnEnter(GameState::MainMenu),
                reset_formation_state_on_main_menu,
            )
            .add_systems(
                Update,
                (
                    handle_skillbar_hotkeys,
                    sync_formation_modifiers,
                    apply_active_formation,
                    sync_friendly_depth_sorting,
                )
                    .chain()
                    .run_if(in_state(GameState::InRun)),
            );
    }
}

fn reset_formation_state_on_main_menu(
    mut formation: ResMut<ActiveFormation>,
    mut modifiers: ResMut<FormationModifiers>,
    mut skillbar: ResMut<FormationSkillBar>,
    mut lane_summary: ResMut<FormationLaneSummary>,
    data: Res<GameData>,
) {
    *formation = ActiveFormation::Square;
    skillbar.reset_to_default();
    *lane_summary = FormationLaneSummary::default();
    sync_modifiers_for_active(&mut modifiers, &data, *formation);
}

fn reset_formation_state_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut formation: ResMut<ActiveFormation>,
    mut modifiers: ResMut<FormationModifiers>,
    mut skillbar: ResMut<FormationSkillBar>,
    mut lane_summary: ResMut<FormationLaneSummary>,
    data: Res<GameData>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    *formation = ActiveFormation::Square;
    skillbar.reset_to_default();
    *lane_summary = FormationLaneSummary::default();
    sync_modifiers_for_active(&mut modifiers, &data, *formation);
}

fn handle_skillbar_hotkeys(
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut skillbar: ResMut<FormationSkillBar>,
    mut active_formation: ResMut<ActiveFormation>,
) {
    let Some(keys) = keyboard else {
        return;
    };
    let Some(slot_index) = slot_index_from_hotkey(&keys) else {
        return;
    };

    if let Some(SkillBarSkillKind::Formation(next)) = skillbar.activate_slot(slot_index)
        && *active_formation != next
    {
        *active_formation = next;
    }
}

fn slot_index_from_hotkey(keys: &ButtonInput<KeyCode>) -> Option<usize> {
    if keys.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keys.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keys.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else if keys.just_pressed(KeyCode::Digit4) {
        Some(3)
    } else if keys.just_pressed(KeyCode::Digit5) {
        Some(4)
    } else if keys.just_pressed(KeyCode::Digit6) {
        Some(5)
    } else if keys.just_pressed(KeyCode::Digit7) {
        Some(6)
    } else if keys.just_pressed(KeyCode::Digit8) {
        Some(7)
    } else if keys.just_pressed(KeyCode::Digit9) {
        Some(8)
    } else if keys.just_pressed(KeyCode::Digit0) {
        Some(9)
    } else {
        None
    }
}

fn sync_formation_modifiers(
    mut modifiers: ResMut<FormationModifiers>,
    data: Res<GameData>,
    active: Res<ActiveFormation>,
) {
    sync_modifiers_for_active(&mut modifiers, &data, *active);
}

fn sync_modifiers_for_active(
    modifiers: &mut FormationModifiers,
    data: &GameData,
    active: ActiveFormation,
) {
    let config = active_formation_config(data, active);
    modifiers.offense_multiplier = config.offense_multiplier;
    modifiers.offense_while_moving_multiplier = config.offense_while_moving_multiplier;
    modifiers.defense_multiplier = config.defense_multiplier;
    modifiers.move_speed_multiplier = config.move_speed_multiplier;
}

pub fn active_formation_config(data: &GameData, active: ActiveFormation) -> &FormationConfig {
    match active {
        ActiveFormation::Square => &data.formations.square,
        ActiveFormation::Circle => &data.formations.circle,
        ActiveFormation::Skean => &data.formations.skean,
        ActiveFormation::Diamond => &data.formations.diamond,
        ActiveFormation::ShieldWall => &data.formations.shield_wall,
        ActiveFormation::Loose => &data.formations.loose,
    }
}

pub fn skill_for_formation(formation: ActiveFormation) -> SkillBarSkill {
    SkillBarSkill {
        id: format!("formation_{}", formation.id()),
        label: formation.display_name().to_string(),
        kind: SkillBarSkillKind::Formation(formation),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum FormationLane {
    Outer,
    Middle,
    Inner,
}

impl FormationLane {
    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Outer => "O",
            Self::Middle => "M",
            Self::Inner => "I",
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssignedFormationLane(pub FormationLane);

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FormationLaneSummary {
    pub outer: usize,
    pub middle: usize,
    pub inner: usize,
}

#[derive(Clone, Copy, Debug)]
struct FormationLanePolicy {
    outer_share: f32,
    middle_share: f32,
    support_outer_cap_ratio: f32,
    support_order: [FormationLane; 3],
    frontline_order: [FormationLane; 3],
    ranged_order: [FormationLane; 3],
    skirmisher_order: [FormationLane; 3],
}

fn lane_policy_for_formation(formation: ActiveFormation) -> FormationLanePolicy {
    match formation {
        ActiveFormation::Square => FormationLanePolicy {
            outer_share: 0.45,
            middle_share: 0.35,
            support_outer_cap_ratio: 0.25,
            support_order: [
                FormationLane::Inner,
                FormationLane::Middle,
                FormationLane::Outer,
            ],
            frontline_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
            ranged_order: [
                FormationLane::Middle,
                FormationLane::Inner,
                FormationLane::Outer,
            ],
            skirmisher_order: [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
        },
        ActiveFormation::Circle => FormationLanePolicy {
            outer_share: 0.38,
            middle_share: 0.34,
            support_outer_cap_ratio: 0.25,
            support_order: [
                FormationLane::Inner,
                FormationLane::Middle,
                FormationLane::Outer,
            ],
            frontline_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
            ranged_order: [
                FormationLane::Middle,
                FormationLane::Inner,
                FormationLane::Outer,
            ],
            skirmisher_order: [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
        },
        ActiveFormation::Skean => FormationLanePolicy {
            outer_share: 0.55,
            middle_share: 0.30,
            support_outer_cap_ratio: 0.20,
            support_order: [
                FormationLane::Middle,
                FormationLane::Inner,
                FormationLane::Outer,
            ],
            frontline_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
            ranged_order: [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
            skirmisher_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
        },
        ActiveFormation::Diamond => FormationLanePolicy {
            outer_share: 0.52,
            middle_share: 0.30,
            support_outer_cap_ratio: 0.20,
            support_order: [
                FormationLane::Inner,
                FormationLane::Middle,
                FormationLane::Outer,
            ],
            frontline_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
            ranged_order: [
                FormationLane::Middle,
                FormationLane::Inner,
                FormationLane::Outer,
            ],
            skirmisher_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
        },
        ActiveFormation::ShieldWall => FormationLanePolicy {
            outer_share: 0.62,
            middle_share: 0.28,
            support_outer_cap_ratio: 0.15,
            support_order: [
                FormationLane::Inner,
                FormationLane::Middle,
                FormationLane::Outer,
            ],
            frontline_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
            ranged_order: [
                FormationLane::Inner,
                FormationLane::Middle,
                FormationLane::Outer,
            ],
            skirmisher_order: [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
        },
        ActiveFormation::Loose => FormationLanePolicy {
            outer_share: 0.34,
            middle_share: 0.43,
            support_outer_cap_ratio: 0.33,
            support_order: [
                FormationLane::Inner,
                FormationLane::Middle,
                FormationLane::Outer,
            ],
            frontline_order: [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
            ranged_order: [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
            skirmisher_order: [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
        },
    }
}

#[derive(Debug)]
struct LaneOffsetBuckets {
    outer: VecDeque<Vec2>,
    middle: VecDeque<Vec2>,
    inner: VecDeque<Vec2>,
}

impl LaneOffsetBuckets {
    fn from_offsets(formation: ActiveFormation, offsets: Vec<Vec2>) -> Self {
        let mut ordered_offsets = lane_ordered_offsets(offsets);
        let (outer_count, middle_count, _) =
            lane_slot_counts_for_formation(formation, ordered_offsets.len());
        let mut middle_and_inner =
            ordered_offsets.split_off(outer_count.min(ordered_offsets.len()));
        let inner = middle_and_inner.split_off(middle_count.min(middle_and_inner.len()));
        Self {
            outer: VecDeque::from(ordered_offsets),
            middle: VecDeque::from(middle_and_inner),
            inner: VecDeque::from(inner),
        }
    }

    fn lane_has_slots(&self, lane: FormationLane) -> bool {
        match lane {
            FormationLane::Outer => !self.outer.is_empty(),
            FormationLane::Middle => !self.middle.is_empty(),
            FormationLane::Inner => !self.inner.is_empty(),
        }
    }

    fn pop_lane(&mut self, lane: FormationLane) -> Option<Vec2> {
        match lane {
            FormationLane::Outer => self.outer.pop_front(),
            FormationLane::Middle => self.middle.pop_front(),
            FormationLane::Inner => self.inner.pop_front(),
        }
    }

    fn pop_any(&mut self) -> Option<(FormationLane, Vec2)> {
        if let Some(offset) = self.pop_lane(FormationLane::Outer) {
            return Some((FormationLane::Outer, offset));
        }
        if let Some(offset) = self.pop_lane(FormationLane::Middle) {
            return Some((FormationLane::Middle, offset));
        }
        self.pop_lane(FormationLane::Inner)
            .map(|offset| (FormationLane::Inner, offset))
    }

    fn outer_capacity(&self) -> usize {
        self.outer.len()
    }
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
fn apply_active_formation(
    mut commands: Commands,
    time: Res<Time>,
    data: Res<GameData>,
    formation: Res<ActiveFormation>,
    mut lane_summary: ResMut<FormationLaneSummary>,
    banner_penalty: Option<Res<BannerMovementPenalty>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut friendlies: Query<
        (
            Entity,
            &Unit,
            &mut Transform,
            Option<&mut AssignedFormationLane>,
        ),
        (
            With<FriendlyUnit>,
            Without<CommanderUnit>,
            Without<OutOfFormation>,
        ),
    >,
    out_of_formation: Query<
        Entity,
        (
            With<FriendlyUnit>,
            Without<CommanderUnit>,
            With<OutOfFormation>,
        ),
    >,
) {
    *lane_summary = FormationLaneSummary::default();
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };

    let config = active_formation_config(&data, *formation);
    let spacing = config.slot_spacing;
    let mut members: Vec<(
        Entity,
        UnitKind,
        Mut<Transform>,
        Option<Mut<AssignedFormationLane>>,
    )> = friendlies
        .iter_mut()
        .map(|(entity, unit, transform, assigned_lane)| {
            (entity, unit.kind, transform, assigned_lane)
        })
        .collect();
    let total_recruits = members.len() + out_of_formation.iter().count();
    let offsets = offsets_for_formation(*formation, total_recruits, spacing);
    let assignments = resolve_lane_assignments(
        members
            .iter()
            .map(|(entity, kind, _, _)| (*entity, *kind))
            .collect(),
        *formation,
        offsets,
    );
    let mut slot_lookup: HashMap<Entity, (FormationLane, Vec2)> =
        HashMap::with_capacity(assignments.len());
    for (entity, lane, offset) in assignments {
        match lane {
            FormationLane::Outer => lane_summary.outer = lane_summary.outer.saturating_add(1),
            FormationLane::Middle => lane_summary.middle = lane_summary.middle.saturating_add(1),
            FormationLane::Inner => lane_summary.inner = lane_summary.inner.saturating_add(1),
        }
        slot_lookup.insert(entity, (lane, offset));
    }
    let speed_multiplier = banner_penalty
        .as_ref()
        .map(|penalty| penalty.friendly_speed_multiplier)
        .unwrap_or(1.0);

    for (entity, _, mut transform, assigned_lane_component) in members.drain(..) {
        let Some((lane, offset)) = slot_lookup.get(&entity).copied() else {
            commands.entity(entity).remove::<AssignedFormationLane>();
            continue;
        };
        if let Some(mut assigned_lane) = assigned_lane_component {
            assigned_lane.0 = lane;
        } else {
            commands.entity(entity).insert(AssignedFormationLane(lane));
        }
        let target = commander_transform.translation.truncate() + offset;
        let current = transform.translation.truncate();
        let smooth =
            (time.delta_seconds() * 10.0 * speed_multiplier * config.move_speed_multiplier)
                .clamp(0.0, 1.0);
        let next = current.lerp(target, smooth);
        transform.translation.x = next.x;
        transform.translation.y = next.y;
    }
    for entity in out_of_formation.iter() {
        commands.entity(entity).remove::<AssignedFormationLane>();
    }
}

fn lane_ordered_offsets(mut offsets: Vec<Vec2>) -> Vec<Vec2> {
    offsets.sort_by(offset_outer_first_cmp);
    offsets
}

fn offset_outer_first_cmp(a: &Vec2, b: &Vec2) -> std::cmp::Ordering {
    let radius_cmp = b
        .length_squared()
        .partial_cmp(&a.length_squared())
        .unwrap_or(std::cmp::Ordering::Equal);
    if radius_cmp != std::cmp::Ordering::Equal {
        return radius_cmp;
    }

    let angle_a = diamond_clockwise_angle(*a);
    let angle_b = diamond_clockwise_angle(*b);
    angle_a
        .partial_cmp(&angle_b)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
        .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
}

fn lane_slot_counts_for_formation(
    formation: ActiveFormation,
    total_slots: usize,
) -> (usize, usize, usize) {
    if total_slots == 0 {
        return (0, 0, 0);
    }
    if total_slots == 1 {
        return (1, 0, 0);
    }
    if total_slots == 2 {
        return (1, 1, 0);
    }

    let policy = lane_policy_for_formation(formation);
    let mut outer = ((total_slots as f32) * policy.outer_share).ceil() as usize;
    outer = outer.clamp(1, total_slots.saturating_sub(1));
    let mut middle = ((total_slots as f32) * policy.middle_share).ceil() as usize;
    if outer + middle > total_slots {
        middle = total_slots.saturating_sub(outer);
    }
    middle = middle.min(total_slots.saturating_sub(outer));
    let mut inner = total_slots.saturating_sub(outer + middle);

    // Ensure all three lanes exist once roster size is large enough.
    if inner == 0 {
        if middle > 1 {
            middle -= 1;
            inner = 1;
        } else if outer > 1 {
            outer -= 1;
            inner = 1;
        }
    }

    (outer, middle, inner)
}

fn lane_priority_key(kind: UnitKind, formation: ActiveFormation) -> u8 {
    let preferred = lane_preference_for_unit(kind, formation)[0];
    let preferred_rank = match preferred {
        FormationLane::Outer => 0,
        FormationLane::Middle => 1,
        FormationLane::Inner => 2,
    };
    let role_rank = if kind.has_role_tag(UnitRoleTag::Frontline)
        || kind.has_role_tag(UnitRoleTag::AntiCavalry)
        || kind.has_shielded_trait()
        || kind.is_fanatic_line()
    {
        0
    } else if kind.has_role_tag(UnitRoleTag::Cavalry) || kind.has_role_tag(UnitRoleTag::Skirmisher)
    {
        1
    } else if kind.has_role_tag(UnitRoleTag::Support) {
        3
    } else {
        2
    };
    preferred_rank * 4 + role_rank
}

fn lane_preference_for_unit(kind: UnitKind, formation: ActiveFormation) -> [FormationLane; 3] {
    let policy = lane_policy_for_formation(formation);

    if kind.is_fanatic_line() {
        return policy.frontline_order;
    }
    if kind.is_support_priest_line() {
        return policy.support_order;
    }
    if kind.is_tracker_line() {
        return match formation {
            ActiveFormation::ShieldWall => [
                FormationLane::Middle,
                FormationLane::Inner,
                FormationLane::Outer,
            ],
            ActiveFormation::Skean | ActiveFormation::Diamond => [
                FormationLane::Outer,
                FormationLane::Middle,
                FormationLane::Inner,
            ],
            _ => [
                FormationLane::Middle,
                FormationLane::Outer,
                FormationLane::Inner,
            ],
        };
    }
    if kind.is_scout_line() {
        return policy.skirmisher_order;
    }

    if kind.has_role_tag(UnitRoleTag::Support) {
        return policy.support_order;
    }

    if kind.has_role_tag(UnitRoleTag::Frontline)
        || kind.has_role_tag(UnitRoleTag::AntiCavalry)
        || kind.has_shielded_trait()
        || kind.is_fanatic_line()
    {
        return policy.frontline_order;
    }

    if kind.has_role_tag(UnitRoleTag::Cavalry) || kind.has_role_tag(UnitRoleTag::Skirmisher) {
        return policy.skirmisher_order;
    }

    if kind.is_archer_line() || kind.has_role_tag(UnitRoleTag::AntiArmor) {
        return policy.ranged_order;
    }

    policy.ranged_order
}

fn support_outer_cap(formation: ActiveFormation, outer_capacity: usize) -> usize {
    if outer_capacity == 0 {
        0
    } else {
        let ratio = lane_policy_for_formation(formation).support_outer_cap_ratio;
        ((outer_capacity as f32) * ratio).ceil() as usize
    }
}

fn resolve_lane_assignments(
    members: Vec<(Entity, UnitKind)>,
    formation: ActiveFormation,
    offsets: Vec<Vec2>,
) -> Vec<(Entity, FormationLane, Vec2)> {
    let mut buckets = LaneOffsetBuckets::from_offsets(formation, offsets);
    let mut ordered_members = members;
    ordered_members
        .sort_by_key(|(entity, kind)| (lane_priority_key(*kind, formation), entity.index()));

    let support_outer_limit = support_outer_cap(formation, buckets.outer_capacity());
    let mut support_outer_assigned = 0usize;
    let mut assigned = Vec::with_capacity(ordered_members.len());

    for (entity, kind) in ordered_members {
        let lane_order = lane_preference_for_unit(kind, formation);
        let mut placed: Option<(FormationLane, Vec2)> = None;

        for lane in lane_order {
            if !buckets.lane_has_slots(lane) {
                continue;
            }

            if lane == FormationLane::Outer
                && kind.has_role_tag(UnitRoleTag::Support)
                && support_outer_assigned >= support_outer_limit
                && (buckets.lane_has_slots(FormationLane::Middle)
                    || buckets.lane_has_slots(FormationLane::Inner))
            {
                continue;
            }

            placed = buckets.pop_lane(lane).map(|offset| (lane, offset));
            if placed.is_some() {
                break;
            }
        }

        let Some((lane, offset)) = placed.or_else(|| buckets.pop_any()) else {
            break;
        };
        if lane == FormationLane::Outer && kind.has_role_tag(UnitRoleTag::Support) {
            support_outer_assigned += 1;
        }
        assigned.push((entity, lane, offset));
    }

    assigned
}

fn offsets_for_formation(
    formation: ActiveFormation,
    recruit_count: usize,
    spacing: f32,
) -> Vec<Vec2> {
    match formation {
        ActiveFormation::Square | ActiveFormation::ShieldWall | ActiveFormation::Loose => {
            square_offsets_excluding_commander_slot(recruit_count, spacing)
        }
        ActiveFormation::Circle => circle_offsets_excluding_commander_slot(recruit_count, spacing),
        ActiveFormation::Skean | ActiveFormation::Diamond => {
            diamond_offsets_excluding_commander_slot(recruit_count, spacing)
        }
    }
}

fn circle_offsets_excluding_commander_slot(recruit_count: usize, spacing: f32) -> Vec<Vec2> {
    if recruit_count == 0 {
        return Vec::new();
    }

    let mut offsets = Vec::with_capacity(recruit_count);
    let mut remaining = recruit_count;
    let mut ring = 1usize;
    while remaining > 0 {
        let ring_slots = (8 * ring).max(6);
        let spawn_count = ring_slots.min(remaining);
        let radius = spacing * ring as f32 * 0.75;
        for index in 0..spawn_count {
            let angle = std::f32::consts::TAU * (index as f32 / ring_slots as f32);
            offsets.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
        }
        remaining -= spawn_count;
        ring = ring.saturating_add(1);
    }
    offsets
}

fn diamond_offsets_excluding_commander_slot(recruit_count: usize, spacing: f32) -> Vec<Vec2> {
    let mut offsets: Vec<Vec2> = square_offsets_excluding_commander_slot(recruit_count, spacing)
        .into_iter()
        .map(rotate_45_degrees)
        .collect();
    offsets.sort_by(diamond_slot_cmp);
    offsets
}

fn rotate_45_degrees(offset: Vec2) -> Vec2 {
    let scale = std::f32::consts::FRAC_1_SQRT_2;
    Vec2::new((offset.x - offset.y) * scale, (offset.x + offset.y) * scale)
}

fn diamond_slot_cmp(a: &Vec2, b: &Vec2) -> std::cmp::Ordering {
    let radius_cmp = a
        .length_squared()
        .partial_cmp(&b.length_squared())
        .unwrap_or(std::cmp::Ordering::Equal);
    if radius_cmp != std::cmp::Ordering::Equal {
        return radius_cmp;
    }

    let angle_a = diamond_clockwise_angle(*a);
    let angle_b = diamond_clockwise_angle(*b);
    let angle_cmp = angle_a
        .partial_cmp(&angle_b)
        .unwrap_or(std::cmp::Ordering::Equal);
    if angle_cmp != std::cmp::Ordering::Equal {
        return angle_cmp;
    }

    a.x.partial_cmp(&b.x)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
}

fn diamond_clockwise_angle(offset: Vec2) -> f32 {
    let mut angle = offset.x.atan2(offset.y);
    if angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    angle
}

pub fn formation_half_extent(recruit_count: usize, slot_spacing: f32, padding_slots: f32) -> f32 {
    let side = ((recruit_count + 1) as f32).sqrt().ceil();
    ((side - 1.0) * 0.5 + padding_slots) * slot_spacing
}

fn circle_radius_for_extent(half_extent: f32) -> f32 {
    half_extent * std::f32::consts::SQRT_2
}

fn inside_circle_bounds(delta: Vec2, half_extent: f32) -> bool {
    let radius = circle_radius_for_extent(half_extent);
    delta.length_squared() <= radius * radius
}

fn inside_diamond_bounds(delta: Vec2, half_extent: f32) -> bool {
    delta.x.abs() + delta.y.abs() <= half_extent * std::f32::consts::SQRT_2
}

pub fn formation_anti_entry_enabled(data: &GameData, formation: ActiveFormation) -> bool {
    active_formation_config(data, formation).anti_entry
}

pub fn formation_allows_unlimited_enemy_inside(
    data: &GameData,
    formation: ActiveFormation,
) -> bool {
    active_formation_config(data, formation).allow_unlimited_enemy_inside
}

pub fn formation_shielded_block_bonus(data: &GameData, formation: ActiveFormation) -> f32 {
    active_formation_config(data, formation)
        .shielded_block_bonus
        .clamp(0.0, 0.95)
}

pub fn formation_melee_reflect_ratio(data: &GameData, formation: ActiveFormation) -> f32 {
    active_formation_config(data, formation)
        .melee_reflect_ratio
        .clamp(0.0, 1.0)
}

pub fn formation_shape_perimeter_target(
    formation: ActiveFormation,
    delta: Vec2,
    half_extent: f32,
) -> Vec2 {
    match formation {
        ActiveFormation::Square | ActiveFormation::ShieldWall | ActiveFormation::Loose => {
            project_to_square_perimeter(delta, half_extent)
        }
        ActiveFormation::Skean | ActiveFormation::Diamond => {
            project_to_diamond_perimeter(delta, half_extent)
        }
        ActiveFormation::Circle => project_to_circle_perimeter(delta, half_extent),
    }
}

pub fn project_to_square_perimeter(delta: Vec2, half_extent: f32) -> Vec2 {
    if half_extent <= 0.0 {
        return delta;
    }
    let dominant = delta.x.abs().max(delta.y.abs());
    if dominant <= f32::EPSILON {
        return Vec2::new(half_extent, 0.0);
    }
    delta * (half_extent / dominant)
}

pub fn project_to_diamond_perimeter(delta: Vec2, half_extent: f32) -> Vec2 {
    let diamond_radius = half_extent * std::f32::consts::SQRT_2;
    if diamond_radius <= 0.0 {
        return delta;
    }
    let l1 = delta.x.abs() + delta.y.abs();
    if l1 <= f32::EPSILON {
        return Vec2::new(diamond_radius, 0.0);
    }
    delta * (diamond_radius / l1)
}

pub fn project_to_circle_perimeter(delta: Vec2, half_extent: f32) -> Vec2 {
    let circle_radius = circle_radius_for_extent(half_extent);
    if circle_radius <= 0.0 {
        return delta;
    }
    let length = delta.length();
    if length <= f32::EPSILON {
        return Vec2::new(circle_radius, 0.0);
    }
    delta * (circle_radius / length)
}

pub fn formation_contains_position(
    formation: ActiveFormation,
    commander_position: Vec2,
    target_position: Vec2,
    recruit_count: usize,
    slot_spacing: f32,
    padding_slots: f32,
) -> bool {
    if recruit_count == 0 || slot_spacing <= 0.0 {
        return false;
    }
    let half_extent = formation_half_extent(recruit_count, slot_spacing, padding_slots);
    let delta = target_position - commander_position;

    match formation {
        ActiveFormation::Square | ActiveFormation::ShieldWall | ActiveFormation::Loose => {
            delta.x.abs() <= half_extent && delta.y.abs() <= half_extent
        }
        ActiveFormation::Skean | ActiveFormation::Diamond => {
            inside_diamond_bounds(delta, half_extent)
        }
        ActiveFormation::Circle => inside_circle_bounds(delta, half_extent),
    }
}

#[allow(clippy::type_complexity)]
fn sync_friendly_depth_sorting(
    mut units: Query<&mut Transform, (With<FriendlyUnit>, Without<Camera>)>,
) {
    for mut transform in &mut units {
        transform.translation.z = depth_z_for_world_y(transform.translation.y);
    }
}

fn square_offsets_excluding_commander_slot(recruit_count: usize, spacing: f32) -> Vec<Vec2> {
    if recruit_count == 0 {
        return Vec::new();
    }

    let mut offsets = square_offsets(recruit_count + 1, spacing);
    let commander_slot = offsets
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.length_squared()
                .partial_cmp(&b.length_squared())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let commander_offset = offsets.remove(commander_slot);
    for offset in &mut offsets {
        *offset -= commander_offset;
    }
    offsets
}

pub fn depth_z_for_world_y(y: f32) -> f32 {
    20.0 - y * 0.001
}

pub fn square_offsets(count: usize, spacing: f32) -> Vec<Vec2> {
    if count == 0 {
        return Vec::new();
    }
    let side = (count as f32).sqrt().ceil() as i32;
    let half = (side as f32 - 1.0) * 0.5;
    let mut result = Vec::with_capacity(count);
    for idx in 0..count {
        let row = (idx as i32) / side;
        let col = (idx as i32) % side;
        let x = (col as f32 - half) * spacing;
        let y = (row as f32 - half) * spacing;
        result.push(Vec2::new(x, y));
    }
    result
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use bevy::prelude::{
        App, ButtonInput, Entity, KeyCode, MinimalPlugins, Transform, Update, Vec2,
    };

    use crate::data::GameData;
    use crate::model::{CommanderUnit, FriendlyUnit, PlayerFaction, Unit, UnitKind};

    use crate::formation::{
        ActiveFormation, FormationSkillBar, SKILL_BAR_CAPACITY, depth_z_for_world_y,
        formation_contains_position, square_offsets,
    };

    #[derive(Clone, Copy)]
    struct FormationScenarioWeights {
        offense: f32,
        moving_offense: f32,
        defense: f32,
        anti_cavalry: f32,
        move_speed: f32,
        anti_entry: f32,
        reflect: f32,
        loose_spread: f32,
    }

    fn scenario_score(
        data: &GameData,
        formation: ActiveFormation,
        weights: FormationScenarioWeights,
    ) -> f32 {
        let config = super::active_formation_config(data, formation);
        let anti_entry = if config.anti_entry { 1.0 } else { 0.0 };
        let reflect = config.melee_reflect_ratio;
        let loose_spread = if config.allow_unlimited_enemy_inside {
            1.0
        } else {
            0.0
        };
        config.offense_multiplier * weights.offense
            + config.offense_while_moving_multiplier * weights.moving_offense
            + config.defense_multiplier * weights.defense
            + config.anti_cavalry_multiplier * weights.anti_cavalry
            + config.move_speed_multiplier * weights.move_speed
            + anti_entry * weights.anti_entry
            + reflect * weights.reflect
            + loose_spread * weights.loose_spread
    }

    fn best_formation_for_scenario(
        data: &GameData,
        weights: FormationScenarioWeights,
    ) -> ActiveFormation {
        let mut best = ActiveFormation::Square;
        let mut best_score = f32::MIN;
        for formation in ActiveFormation::all() {
            let score = scenario_score(data, formation, weights);
            if score > best_score {
                best_score = score;
                best = formation;
            }
        }
        best
    }

    #[test]
    fn square_offsets_return_expected_count() {
        let offsets = square_offsets(7, 12.0);
        assert_eq!(offsets.len(), 7);
    }

    #[test]
    fn square_offsets_are_unique() {
        let offsets = square_offsets(9, 10.0);
        let mut set = HashSet::new();
        for offset in offsets {
            set.insert((offset.x as i32, offset.y as i32));
        }
        assert_eq!(set.len(), 9);
    }

    #[test]
    fn zero_count_returns_empty() {
        let offsets = square_offsets(0, 10.0);
        assert_eq!(offsets, Vec::<Vec2>::new());
    }

    #[test]
    fn depth_sorting_places_lower_units_on_top() {
        let upper = depth_z_for_world_y(100.0);
        let lower = depth_z_for_world_y(-100.0);
        assert!(lower > upper);
    }

    #[test]
    fn skillbar_starts_with_square_in_first_slot_active() {
        let skillbar = FormationSkillBar::default();
        assert_eq!(skillbar.slots.len(), 1);
        assert_eq!(skillbar.active_slot, Some(0));
        assert!(skillbar.has_formation(ActiveFormation::Square));
    }

    #[test]
    fn skillbar_add_and_activate_diamond() {
        let mut skillbar = FormationSkillBar::default();
        assert!(skillbar.try_add_formation(ActiveFormation::Diamond));
        let activated = skillbar.activate_slot(1);
        assert_eq!(
            activated,
            Some(crate::formation::SkillBarSkillKind::Formation(
                ActiveFormation::Diamond
            ))
        );
        assert_eq!(skillbar.active_slot, Some(1));
    }

    #[test]
    fn skillbar_rejects_duplicate_or_full_adds() {
        let mut skillbar = FormationSkillBar::default();
        assert!(!skillbar.try_add_formation(ActiveFormation::Square));

        skillbar.slots = (0..SKILL_BAR_CAPACITY)
            .map(|i| crate::formation::SkillBarSkill {
                id: format!("slot_{i}"),
                label: format!("Slot {i}"),
                kind: crate::formation::SkillBarSkillKind::Formation(ActiveFormation::Square),
            })
            .collect();
        assert!(skillbar.is_full());
        assert!(!skillbar.try_add_formation(ActiveFormation::Diamond));
    }

    #[test]
    fn formation_contains_position_handles_square_and_diamond() {
        let commander = Vec2::ZERO;
        let near = Vec2::new(12.0, 10.0);
        assert!(formation_contains_position(
            ActiveFormation::Square,
            commander,
            near,
            9,
            30.0,
            0.35,
        ));
        assert!(formation_contains_position(
            ActiveFormation::Diamond,
            commander,
            near,
            9,
            30.0,
            0.35,
        ));
        assert!(formation_contains_position(
            ActiveFormation::Circle,
            commander,
            near,
            9,
            30.0,
            0.35,
        ));
    }

    #[test]
    fn active_formation_ids_cover_all_runtime_formations() {
        let ids = [
            ("square", ActiveFormation::Square),
            ("circle", ActiveFormation::Circle),
            ("skean", ActiveFormation::Skean),
            ("diamond", ActiveFormation::Diamond),
            ("shield_wall", ActiveFormation::ShieldWall),
            ("loose", ActiveFormation::Loose),
        ];
        for (id, expected) in ids {
            assert_eq!(ActiveFormation::from_id(id), Some(expected));
        }
    }

    #[test]
    fn skillbar_can_add_and_activate_all_formations() {
        let mut skillbar = FormationSkillBar::default();
        for formation in ActiveFormation::all() {
            if formation == ActiveFormation::Square {
                continue;
            }
            assert!(skillbar.try_add_formation(formation));
        }
        assert_eq!(skillbar.slots.len(), ActiveFormation::all().len());
        for (index, formation) in ActiveFormation::all().iter().enumerate() {
            let activated = skillbar.activate_slot(index);
            assert_eq!(
                activated,
                Some(crate::formation::SkillBarSkillKind::Formation(*formation))
            );
        }
    }

    #[test]
    fn skillbar_hotkeys_cover_extended_slots() {
        let mut digit_six = ButtonInput::<KeyCode>::default();
        digit_six.press(KeyCode::Digit6);
        assert_eq!(super::slot_index_from_hotkey(&digit_six), Some(5));

        let mut digit_zero = ButtonInput::<KeyCode>::default();
        digit_zero.press(KeyCode::Digit0);
        assert_eq!(super::slot_index_from_hotkey(&digit_zero), Some(9));

        let no_key = ButtonInput::<KeyCode>::default();
        assert_eq!(super::slot_index_from_hotkey(&no_key), None);
    }

    #[test]
    fn lane_summary_totals_match_assigned_friendlies() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(
            GameData::load_from_dir(std::path::Path::new("assets/data")).expect("load data"),
        );
        app.insert_resource(ActiveFormation::Square);
        app.init_resource::<super::FormationLaneSummary>();
        app.add_systems(Update, super::apply_active_formation);

        app.world_mut().spawn((
            CommanderUnit,
            Unit {
                team: crate::model::Team::Friendly,
                kind: UnitKind::Commander,
                level: 1,
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
        for (index, unit_id) in [
            "peasant_infantry",
            "peasant_infantry",
            "peasant_archer",
            "peasant_archer",
            "peasant_priest",
            "tracker",
        ]
        .iter()
        .enumerate()
        {
            let kind = UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, unit_id, false)
                .expect("unit kind should resolve");
            app.world_mut().spawn((
                FriendlyUnit,
                Unit {
                    team: crate::model::Team::Friendly,
                    kind,
                    level: 1,
                },
                Transform::from_xyz(index as f32 * 8.0, 0.0, 0.0),
            ));
        }

        app.update();

        let summary = *app.world().resource::<super::FormationLaneSummary>();
        let assigned_count = {
            let world = app.world_mut();
            let mut query = world.query::<&super::AssignedFormationLane>();
            query.iter(world).count()
        };
        assert_eq!(
            summary.outer + summary.middle + summary.inner,
            assigned_count
        );
        assert_eq!(assigned_count, 6);
    }

    #[test]
    fn lane_quota_profiles_change_with_formation() {
        let square = super::lane_slot_counts_for_formation(ActiveFormation::Square, 20);
        let shield_wall = super::lane_slot_counts_for_formation(ActiveFormation::ShieldWall, 20);
        let loose = super::lane_slot_counts_for_formation(ActiveFormation::Loose, 20);
        assert!(shield_wall.0 > square.0);
        assert!(loose.1 >= square.1);
    }

    #[test]
    fn diamond_offsets_are_ordered_by_ring_then_clockwise() {
        let offsets = super::diamond_offsets_excluding_commander_slot(8, 20.0);
        assert_eq!(offsets.len(), 8);
        for pair in offsets.windows(2) {
            let a = pair[0];
            let b = pair[1];
            assert!(
                b.length_squared() + 0.0001 >= a.length_squared(),
                "diamond ordering should be non-decreasing by ring distance"
            );
        }
        let topish_exists = offsets
            .iter()
            .take(4)
            .any(|offset| offset.y > offset.x.abs() * 0.5);
        assert!(
            topish_exists,
            "early diamond slots should include a top-side placement"
        );
    }

    #[test]
    fn lane_preferences_are_trait_based_and_faction_agnostic() {
        let fixture_ids = [
            ("peasant_infantry", super::FormationLane::Outer),
            ("peasant_archer", super::FormationLane::Middle),
            ("peasant_priest", super::FormationLane::Inner),
            ("fanatic", super::FormationLane::Outer),
            ("tracker", super::FormationLane::Middle),
            ("scout", super::FormationLane::Middle),
            ("squire", super::FormationLane::Inner),
        ];

        for (unit_id, expected_lane) in fixture_ids {
            let christian =
                UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, unit_id, false)
                    .expect("christian unit should resolve");
            let muslim = UnitKind::from_faction_and_unit_id(PlayerFaction::Muslim, unit_id, false)
                .expect("muslim unit should resolve");
            assert_eq!(
                super::lane_preference_for_unit(christian, ActiveFormation::Square)[0],
                expected_lane,
                "unexpected christian lane for {unit_id}"
            );
            assert_eq!(
                super::lane_preference_for_unit(muslim, ActiveFormation::Square)[0],
                expected_lane,
                "unexpected muslim lane for {unit_id}"
            );
        }
    }

    #[test]
    fn diamond_prefers_skirmisher_outer_while_square_prefers_middle() {
        let scout = UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "scout", false)
            .expect("scout should resolve");
        assert_eq!(
            super::lane_preference_for_unit(scout, ActiveFormation::Square)[0],
            super::FormationLane::Middle
        );
        assert_eq!(
            super::lane_preference_for_unit(scout, ActiveFormation::Diamond)[0],
            super::FormationLane::Outer
        );
    }

    #[test]
    fn tracker_and_support_lane_preferences_vary_by_formation_profile() {
        let tracker =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "tracker", false)
                .expect("tracker should resolve");
        let speaker =
            UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "divine_speaker", false)
                .expect("support hero should resolve");

        assert_eq!(
            super::lane_preference_for_unit(tracker, ActiveFormation::ShieldWall)[0],
            super::FormationLane::Middle
        );
        assert_eq!(
            super::lane_preference_for_unit(tracker, ActiveFormation::Skean)[0],
            super::FormationLane::Outer
        );
        assert_eq!(
            super::lane_preference_for_unit(speaker, ActiveFormation::Skean)[0],
            super::FormationLane::Middle
        );
        assert_eq!(
            super::lane_preference_for_unit(speaker, ActiveFormation::ShieldWall)[0],
            super::FormationLane::Inner
        );
    }

    #[test]
    fn lane_assignment_places_frontline_outer_and_support_inner() {
        let members = vec![
            (
                Entity::from_raw(10),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_infantry",
                    false,
                )
                .expect("infantry should resolve"),
            ),
            (
                Entity::from_raw(11),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_archer",
                    false,
                )
                .expect("archer should resolve"),
            ),
            (
                Entity::from_raw(12),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_priest",
                    false,
                )
                .expect("priest should resolve"),
            ),
        ];
        let offsets = vec![Vec2::ZERO, Vec2::new(30.0, 0.0), Vec2::new(-30.0, 0.0)];
        let assignments =
            super::resolve_lane_assignments(members, ActiveFormation::Square, offsets);
        let assignment_map: std::collections::HashMap<Entity, super::FormationLane> = assignments
            .into_iter()
            .map(|(entity, lane, _)| (entity, lane))
            .collect();

        assert_eq!(
            assignment_map.get(&Entity::from_raw(10)),
            Some(&super::FormationLane::Outer)
        );
        assert_eq!(
            assignment_map.get(&Entity::from_raw(11)),
            Some(&super::FormationLane::Middle)
        );
        assert_eq!(
            assignment_map.get(&Entity::from_raw(12)),
            Some(&super::FormationLane::Inner)
        );
    }

    #[test]
    fn lane_assignment_is_deterministic_for_identical_input() {
        let members = vec![
            (
                Entity::from_raw(1),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_infantry",
                    false,
                )
                .expect("unit should resolve"),
            ),
            (
                Entity::from_raw(2),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_archer",
                    false,
                )
                .expect("unit should resolve"),
            ),
            (
                Entity::from_raw(3),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_priest",
                    false,
                )
                .expect("unit should resolve"),
            ),
            (
                Entity::from_raw(4),
                UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "tracker", false)
                    .expect("unit should resolve"),
            ),
            (
                Entity::from_raw(5),
                UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "squire", false)
                    .expect("unit should resolve"),
            ),
        ];
        let offsets = super::square_offsets_excluding_commander_slot(8, 20.0);

        let first = super::resolve_lane_assignments(
            members.clone(),
            ActiveFormation::Square,
            offsets.clone(),
        );
        let second = super::resolve_lane_assignments(members, ActiveFormation::Square, offsets);
        assert_eq!(first, second);
    }

    #[test]
    fn support_units_do_not_overcrowd_outer_lane_when_alternatives_exist() {
        let mut members: Vec<(Entity, UnitKind)> = Vec::new();
        let mut support_entities: std::collections::HashSet<Entity> =
            std::collections::HashSet::new();
        let mut next_entity = 1u32;
        for _ in 0..3 {
            members.push((
                Entity::from_raw(next_entity),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_infantry",
                    false,
                )
                .expect("unit should resolve"),
            ));
            next_entity += 1;
        }
        for _ in 0..2 {
            members.push((
                Entity::from_raw(next_entity),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_archer",
                    false,
                )
                .expect("unit should resolve"),
            ));
            next_entity += 1;
        }
        for _ in 0..4 {
            let entity = Entity::from_raw(next_entity);
            members.push((
                entity,
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_priest",
                    false,
                )
                .expect("unit should resolve"),
            ));
            support_entities.insert(entity);
            next_entity += 1;
        }

        let offsets = super::square_offsets_excluding_commander_slot(9, 20.0);
        let assignments =
            super::resolve_lane_assignments(members, ActiveFormation::Square, offsets.clone());
        let support_on_outer = assignments
            .iter()
            .filter(|(_, lane, _)| *lane == super::FormationLane::Outer)
            .filter(|(entity, _, _)| support_entities.contains(entity))
            .count();
        let outer_capacity =
            super::lane_slot_counts_for_formation(ActiveFormation::Square, offsets.len()).0;
        let outer_support_cap = super::support_outer_cap(ActiveFormation::Square, outer_capacity);

        assert!(
            support_on_outer <= outer_support_cap,
            "support units should not exceed configured outer-lane cap when alternatives exist"
        );
    }

    #[test]
    fn support_heavy_fixture_keeps_support_majority_off_outer_lane() {
        let mut members: Vec<(Entity, UnitKind)> = Vec::new();
        let mut support_entities: std::collections::HashSet<Entity> =
            std::collections::HashSet::new();
        let mut next_entity = 1u32;
        for _ in 0..4 {
            members.push((
                Entity::from_raw(next_entity),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_infantry",
                    false,
                )
                .expect("frontline should resolve"),
            ));
            next_entity += 1;
        }
        for _ in 0..2 {
            members.push((
                Entity::from_raw(next_entity),
                UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "tracker", false)
                    .expect("tracker should resolve"),
            ));
            next_entity += 1;
        }
        for _ in 0..8 {
            let entity = Entity::from_raw(next_entity);
            members.push((
                entity,
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_priest",
                    false,
                )
                .expect("support should resolve"),
            ));
            support_entities.insert(entity);
            next_entity += 1;
        }

        let offsets = super::square_offsets_excluding_commander_slot(14, 20.0);
        let assignments =
            super::resolve_lane_assignments(members, ActiveFormation::Square, offsets.clone());
        let support_on_outer = assignments
            .iter()
            .filter(|(_, lane, _)| *lane == super::FormationLane::Outer)
            .filter(|(entity, _, _)| support_entities.contains(entity))
            .count();
        let support_on_non_outer = assignments
            .iter()
            .filter(|(_, lane, _)| *lane != super::FormationLane::Outer)
            .filter(|(entity, _, _)| support_entities.contains(entity))
            .count();
        assert!(
            support_on_non_outer > support_on_outer,
            "support-heavy fixture should place most supports away from outer lane"
        );
    }

    #[test]
    fn cavalry_heavy_fixture_pushes_more_skirmishers_outer_in_diamond_than_square() {
        let mut members: Vec<(Entity, UnitKind)> = Vec::new();
        let mut scout_entities: std::collections::HashSet<Entity> =
            std::collections::HashSet::new();
        let mut next_entity = 1u32;
        for _ in 0..10 {
            let entity = Entity::from_raw(next_entity);
            members.push((
                entity,
                UnitKind::from_faction_and_unit_id(PlayerFaction::Christian, "scout", false)
                    .expect("scout should resolve"),
            ));
            scout_entities.insert(entity);
            next_entity += 1;
        }
        for _ in 0..2 {
            members.push((
                Entity::from_raw(next_entity),
                UnitKind::from_faction_and_unit_id(
                    PlayerFaction::Christian,
                    "peasant_priest",
                    false,
                )
                .expect("support should resolve"),
            ));
            next_entity += 1;
        }

        let offsets = super::square_offsets_excluding_commander_slot(12, 20.0);
        let square = super::resolve_lane_assignments(
            members.clone(),
            ActiveFormation::Square,
            offsets.clone(),
        );
        let diamond = super::resolve_lane_assignments(members, ActiveFormation::Diamond, offsets);
        let square_outer_scouts = square
            .iter()
            .filter(|(_, lane, _)| *lane == super::FormationLane::Outer)
            .filter(|(entity, _, _)| scout_entities.contains(entity))
            .count();
        let diamond_outer_scouts = diamond
            .iter()
            .filter(|(_, lane, _)| *lane == super::FormationLane::Outer)
            .filter(|(entity, _, _)| scout_entities.contains(entity))
            .count();
        assert!(
            diamond_outer_scouts > square_outer_scouts,
            "diamond should allocate more skirmisher/scout units to the outer shell in cavalry-heavy fixtures"
        );
    }

    #[test]
    fn formation_counterplay_matrix_has_multiple_profile_winners() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load data");
        let scenarios = [
            (
                "swarm",
                FormationScenarioWeights {
                    offense: 0.10,
                    moving_offense: 0.05,
                    defense: 0.45,
                    anti_cavalry: 0.10,
                    move_speed: 0.05,
                    anti_entry: 0.15,
                    reflect: 0.10,
                    loose_spread: 0.00,
                },
                ActiveFormation::ShieldWall,
            ),
            (
                "cavalry",
                FormationScenarioWeights {
                    offense: 0.10,
                    moving_offense: 0.10,
                    defense: 0.30,
                    anti_cavalry: 0.30,
                    move_speed: 0.05,
                    anti_entry: 0.10,
                    reflect: 0.05,
                    loose_spread: 0.00,
                },
                ActiveFormation::ShieldWall,
            ),
            (
                "ranged",
                FormationScenarioWeights {
                    offense: 0.20,
                    moving_offense: 0.15,
                    defense: 0.15,
                    anti_cavalry: 0.05,
                    move_speed: 0.25,
                    anti_entry: 0.00,
                    reflect: 0.00,
                    loose_spread: 0.20,
                },
                ActiveFormation::Loose,
            ),
            (
                "mixed",
                FormationScenarioWeights {
                    offense: 0.20,
                    moving_offense: 0.15,
                    defense: 0.25,
                    anti_cavalry: 0.15,
                    move_speed: 0.10,
                    anti_entry: 0.10,
                    reflect: 0.05,
                    loose_spread: 0.00,
                },
                ActiveFormation::Diamond,
            ),
        ];

        let mut winners: Vec<ActiveFormation> = Vec::new();
        for (label, weights, expected) in scenarios {
            let winner = best_formation_for_scenario(&data, weights);
            assert_eq!(winner, expected, "unexpected winner for scenario={label}");
            if !winners.contains(&winner) {
                winners.push(winner);
            }
        }
        assert!(
            winners.len() >= 3,
            "expected at least three different winners across scenario matrix"
        );
    }

    #[test]
    fn each_formation_profile_has_explicit_tradeoff_against_square_baseline() {
        let data = GameData::load_from_dir(Path::new("assets/data")).expect("load data");
        let square = super::active_formation_config(&data, ActiveFormation::Square);
        for formation in ActiveFormation::all() {
            if formation == ActiveFormation::Square {
                continue;
            }
            let config = super::active_formation_config(&data, formation);
            let has_upgrade_axis = config.offense_multiplier > square.offense_multiplier
                || config.offense_while_moving_multiplier > square.offense_while_moving_multiplier
                || config.defense_multiplier > square.defense_multiplier
                || config.anti_cavalry_multiplier > square.anti_cavalry_multiplier
                || config.move_speed_multiplier > square.move_speed_multiplier
                || config.anti_entry
                || config.melee_reflect_ratio > 0.0
                || config.allow_unlimited_enemy_inside;
            let has_downgrade_axis = config.offense_multiplier < square.offense_multiplier
                || config.offense_while_moving_multiplier < square.offense_while_moving_multiplier
                || config.defense_multiplier < square.defense_multiplier
                || config.anti_cavalry_multiplier < square.anti_cavalry_multiplier
                || config.move_speed_multiplier < square.move_speed_multiplier;
            assert!(
                has_upgrade_axis && has_downgrade_axis,
                "{formation:?} should have both strengths and weaknesses vs Square baseline"
            );
        }
    }
}
