use bevy::prelude::*;

use crate::banner::BannerMovementPenalty;
use crate::data::{FormationConfig, GameData};
use crate::model::{CommanderUnit, FriendlyUnit, GameState, StartRunEvent, Unit, UnitKind};
use crate::squad::OutOfFormation;

pub const SKILL_BAR_CAPACITY: usize = 10;

#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActiveFormation {
    #[default]
    Square,
    Diamond,
}

impl ActiveFormation {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Diamond => "diamond",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Square => "Square Formation",
            Self::Diamond => "Diamond Formation",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "square" => Some(Self::Square),
            "diamond" => Some(Self::Diamond),
            _ => None,
        }
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
    data: Res<GameData>,
) {
    *formation = ActiveFormation::Square;
    skillbar.reset_to_default();
    sync_modifiers_for_active(&mut modifiers, &data, *formation);
}

fn reset_formation_state_on_run_start(
    mut start_events: EventReader<StartRunEvent>,
    mut formation: ResMut<ActiveFormation>,
    mut modifiers: ResMut<FormationModifiers>,
    mut skillbar: ResMut<FormationSkillBar>,
    data: Res<GameData>,
) {
    if start_events.is_empty() {
        return;
    }
    for _ in start_events.read() {}

    *formation = ActiveFormation::Square;
    skillbar.reset_to_default();
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
        ActiveFormation::Diamond => &data.formations.diamond,
    }
}

pub fn skill_for_formation(formation: ActiveFormation) -> SkillBarSkill {
    SkillBarSkill {
        id: format!("formation_{}", formation.id()),
        label: formation.display_name().to_string(),
        kind: SkillBarSkillKind::Formation(formation),
    }
}

#[allow(clippy::type_complexity)]
fn apply_active_formation(
    time: Res<Time>,
    data: Res<GameData>,
    formation: Res<ActiveFormation>,
    banner_penalty: Option<Res<BannerMovementPenalty>>,
    commanders: Query<&Transform, With<CommanderUnit>>,
    mut friendlies: Query<
        (Entity, &Unit, &mut Transform),
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
    let Ok(commander_transform) = commanders.get_single() else {
        return;
    };

    let config = active_formation_config(&data, *formation);
    let spacing = config.slot_spacing;
    let mut members: Vec<(Entity, UnitKind, Mut<Transform>)> = friendlies
        .iter_mut()
        .map(|(entity, unit, transform)| (entity, unit.kind, transform))
        .collect();
    let total_recruits = members.len() + out_of_formation.iter().count();
    let offsets = offsets_for_formation(*formation, total_recruits, spacing);
    let ordered_offsets = role_ordered_offsets(offsets);
    members.sort_by_key(|(entity, kind, _)| (formation_slot_role_priority(*kind), entity.index()));
    let speed_multiplier = banner_penalty
        .as_ref()
        .map(|penalty| penalty.friendly_speed_multiplier)
        .unwrap_or(1.0);

    for ((_, _, mut transform), offset) in members.into_iter().zip(ordered_offsets.into_iter()) {
        let target = commander_transform.translation.truncate() + offset;
        let current = transform.translation.truncate();
        let smooth =
            (time.delta_seconds() * 10.0 * speed_multiplier * config.move_speed_multiplier)
                .clamp(0.0, 1.0);
        let next = current.lerp(target, smooth);
        transform.translation.x = next.x;
        transform.translation.y = next.y;
    }
}

fn role_ordered_offsets(mut offsets: Vec<Vec2>) -> Vec<Vec2> {
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

fn formation_slot_role_priority(kind: UnitKind) -> u8 {
    match kind {
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
        | UnitKind::ChristianScout
        | UnitKind::ChristianMountedScout
        | UnitKind::ChristianShockCavalry
        | UnitKind::ChristianCitadelGuard
        | UnitKind::ChristianArmoredHalberdier
        | UnitKind::ChristianEliteHeavyKnight
        | UnitKind::ChristianEliteShockCavalry
        | UnitKind::ChristianFanatic
        | UnitKind::ChristianFlagellant
        | UnitKind::ChristianEliteFlagellant
        | UnitKind::ChristianDivineJudge
        | UnitKind::MuslimPeasantInfantry
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
        | UnitKind::MuslimScout
        | UnitKind::MuslimMountedScout
        | UnitKind::MuslimShockCavalry
        | UnitKind::MuslimCitadelGuard
        | UnitKind::MuslimArmoredHalberdier
        | UnitKind::MuslimEliteHeavyKnight
        | UnitKind::MuslimEliteShockCavalry
        | UnitKind::MuslimFanatic
        | UnitKind::MuslimFlagellant
        | UnitKind::MuslimEliteFlagellant
        | UnitKind::MuslimDivineJudge => 0, // outer frontline
        UnitKind::ChristianPeasantArcher
        | UnitKind::ChristianBowman
        | UnitKind::ChristianExperiencedBowman
        | UnitKind::ChristianEliteBowman
        | UnitKind::ChristianLongbowman
        | UnitKind::ChristianEliteLongbowman
        | UnitKind::ChristianCrossbowman
        | UnitKind::ChristianEliteCrossbowman
        | UnitKind::ChristianSiegeCrossbowman
        | UnitKind::ChristianTracker
        | UnitKind::ChristianArmoredCrossbowman
        | UnitKind::ChristianPathfinder
        | UnitKind::ChristianHoundmaster
        | UnitKind::ChristianEliteHoundmaster
        | UnitKind::MuslimPeasantArcher
        | UnitKind::MuslimBowman
        | UnitKind::MuslimExperiencedBowman
        | UnitKind::MuslimCrossbowman
        | UnitKind::MuslimEliteBowman
        | UnitKind::MuslimLongbowman
        | UnitKind::MuslimEliteLongbowman
        | UnitKind::MuslimArmoredCrossbowman
        | UnitKind::MuslimEliteCrossbowman
        | UnitKind::MuslimSiegeCrossbowman
        | UnitKind::MuslimTracker
        | UnitKind::MuslimPathfinder
        | UnitKind::MuslimHoundmaster
        | UnitKind::MuslimEliteHoundmaster => 1, // middle ring
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
        | UnitKind::MuslimPeasantPriest
        | UnitKind::MuslimDevoted
        | UnitKind::MuslimSquire
        | UnitKind::MuslimBannerman
        | UnitKind::MuslimEliteBannerman
        | UnitKind::MuslimGodsChosen
        | UnitKind::MuslimDevotedOne
        | UnitKind::MuslimCardinal
        | UnitKind::MuslimEliteCardinal
        | UnitKind::MuslimDivineSpeaker => 2, // innermost support
        _ => 1,
    }
}

fn offsets_for_formation(
    formation: ActiveFormation,
    recruit_count: usize,
    spacing: f32,
) -> Vec<Vec2> {
    match formation {
        ActiveFormation::Square => square_offsets_excluding_commander_slot(recruit_count, spacing),
        ActiveFormation::Diamond => {
            diamond_offsets_excluding_commander_slot(recruit_count, spacing)
        }
    }
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
    let side = ((recruit_count + 1) as f32).sqrt().ceil();
    let half_extent = ((side - 1.0) * 0.5 + padding_slots) * slot_spacing;
    let delta = target_position - commander_position;

    match formation {
        ActiveFormation::Square => delta.x.abs() <= half_extent && delta.y.abs() <= half_extent,
        ActiveFormation::Diamond => {
            delta.x.abs() + delta.y.abs() <= half_extent * std::f32::consts::SQRT_2
        }
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

    use bevy::prelude::Vec2;

    use crate::model::UnitKind;

    use crate::formation::{
        ActiveFormation, FormationSkillBar, SKILL_BAR_CAPACITY, depth_z_for_world_y,
        formation_contains_position, square_offsets,
    };

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
    fn role_priority_prefers_melee_outer_and_support_inner() {
        let offsets = vec![Vec2::ZERO, Vec2::new(30.0, 0.0), Vec2::new(-30.0, 0.0)];
        let ordered_offsets = super::role_ordered_offsets(offsets);
        assert!(ordered_offsets[0].length_squared() >= ordered_offsets[2].length_squared());

        let mut members = [
            (2u32, UnitKind::ChristianPeasantPriest),
            (1u32, UnitKind::ChristianPeasantArcher),
            (0u32, UnitKind::ChristianPeasantInfantry),
        ];
        members.sort_by_key(|(stable_id, kind)| {
            (super::formation_slot_role_priority(*kind), *stable_id)
        });

        assert_eq!(members[0].1, UnitKind::ChristianPeasantInfantry);
        assert_eq!(members[1].1, UnitKind::ChristianPeasantArcher);
        assert_eq!(members[2].1, UnitKind::ChristianPeasantPriest);
    }
}
