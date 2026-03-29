# requirements.md

## Purpose
Define the minimum required **visual art assets** for the MVP.
UI element art is intentionally excluded for now (basic programmatic shapes are acceptable).

## Scope Assumptions (MVP)
- Top-down or slightly angled 2D battlefield.
- One commander at run start, selected by faction (`Baldiun` or `Saladin`).
- Six recruitable soldier variants across two factions (`Peasant Infantry`, `Peasant Archer`, `Peasant Priest` per faction).
- Three enemy archetypes (infantry, archer, priest) mirrored across factions.
- Formation roster in active use: `Square`, `Circle`, `Skean`, `Diamond`, `Shield Wall`, `Loose`.
- One map biome (desert).
- Oasis POI assets are optional/deferred until oasis gameplay is re-enabled.
- Banner drop/recovery and rescue recruitment are included.

## Runtime Identity Requirements (Refactor Baseline)
- Runtime-facing identity should be generic-first:
  - units resolve to shared `unit_id` values (for example `peasant_infantry`) with faction context carried separately,
  - heroes/items follow the same pattern (`id + faction override context`) instead of duplicating definitions per faction.
- Canonical identity payloads are:
  - `UnitRef { faction_id, unit_id, rescuable }`,
  - `HeroRef { faction_id, hero_id }`,
  - `ItemRef { faction_id, item_id }`.
- Content ownership must be split into base catalogs + faction overrides:
  - base owns default stats/abilities/visuals and generic progression metadata,
  - faction overrides own presentation/stats/ability tweaks, promotion exclusions, hero pools, item drop-table tuning, and icon swaps.
- Merge/fallback contract:
  - unresolved override fields fall back to base values,
  - unresolved override IDs or unknown faction keys fail fast during config load.
- Faction differentiation should default to data overrides (display, stats, abilities, visuals, drops), not hardcoded branch-specific names in UI/runtime labels.
- Item catalogs must allow faction-scoped entries with the same base `item_id` to support override-style authoring.
- Legacy faction-duplicated schema keys are not part of the supported runtime contract after cutover.

## Counter Role Contract (Gameplay Baseline)
- Unit counter logic must be tag-driven and faction-agnostic.
- Canonical role tags:
  - `frontline`
  - `anti_cavalry`
  - `cavalry`
  - `anti_armor`
  - `skirmisher`
  - `support`
  - `hero_doctrine`
- Canonical armor classes:
  - `unarmored`
  - `light`
  - `armored`
  - `heavy`
- Counter modifiers must be bounded (explicit clamp) and deterministic.
- Runtime must not depend on faction-name branch logic for matchup effects; it should resolve through shared unit IDs and tag/armor metadata.

## Formation Lane Policy Contract
- Formation slot assignment must use three canonical lanes:
  - `outer`
  - `middle`
  - `inner`
- Lane assignment rules must be tag/trait-driven and faction-agnostic.
- Default lane intent:
  - `frontline`, `anti_cavalry`, and `shielded` lines prefer `outer -> middle -> inner`,
  - `support` lines prefer `inner -> middle -> outer`,
  - ranged anti-armor lines prefer `middle -> inner -> outer`,
  - `cavalry` and `skirmisher` lines prefer:
    - `middle -> outer -> inner` in `Square`,
    - `outer -> middle -> inner` in `Diamond`.
- Resolver must enforce deterministic tie-breaking under equal priority.
- Resolver must apply lane quotas and prevent support over-crowding in `outer` when middle/inner slots are available.

## Formation Roster Contract
- `Square` is the baseline/default formation.
- `Circle`:
  - defensive shell profile (higher defense, lower mobility/offense),
  - no anti-entry lock.
- `Skean`:
  - charge profile (higher move speed and moving offense),
  - persistent defense penalty.
- `Diamond` (rework):
  - high-mobility moving-melee pressure profile,
  - hard anti-entry (`0` enemies allowed inside footprint).
- `Shield Wall`:
  - low mobility, high defense profile,
  - hard anti-entry (`0` enemies allowed inside footprint),
  - `Shielded` units gain block bonus,
  - reflects a fixed share of post-mitigation melee-hit damage back to the attacker.
- `Loose`:
  - expanded spacing profile,
  - unlimited enemies may occupy interior footprint (no inside-cap repel).
- Reflect semantics:
  - reflected amount derives from post-armor/post-block incoming damage,
  - source-hit critical impact is preserved in reflected amount,
  - reflected packet itself cannot crit.

## Upgrade Audit Contract (`CRU-260`)
- Every active upgrade in `assets/data/upgrades.json` must have an explicit disposition (`keep`, `merge/rework`, `deprecate`) tracked in `docs/SYSTEMS_REFERENCE.md`.
- Duplicate variant IDs are allowed only as temporary migration state; permanent roster should converge to consolidated schema-driven families.
- Any deprecated upgrade ID must have:
  - a blocking validator in the data loader/runtime path,
  - a mapped replacement path documented in tasks/docs before removal.
- Follow-up refactor/balance tickets must exist for each merge/rework/deprecation cluster before schema cutover.

## Global Art Specs
- Format: `PNG` with transparency for sprites/decals, `PNG` tiles for terrain.
- Style: grounded, dusty, readable silhouettes, low detail noise.
- Palette: desert earth tones with high contrast accents for faction readability.
- Camera readability target: unit roles recognizable at gameplay zoom.
- Placeholder-friendly first pass is acceptable; polish pass can happen post-MVP.

## Required Asset List (MVP)

### 1) Characters - Commander
1. `commander_baldiun_idle` (loop)
2. `commander_baldiun_move` (loop)
3. `commander_baldiun_attack_melee`
4. `commander_baldiun_hit_react`
5. `commander_baldiun_death`
6. `commander_baldiun_battle_cry_cast` (simple cast/readability pose)
7. `commander_saladin_idle` (loop)
8. `commander_saladin_move` (loop)
9. `commander_saladin_attack_melee`
10. `commander_saladin_hit_react`
11. `commander_saladin_death`
12. `commander_saladin_battle_cry_cast` (simple cast/readability pose)

### 2) Characters - Friendly Units
1. `friendly_christian_peasant_infantry_idle` (loop)
2. `friendly_christian_peasant_infantry_move` (loop)
3. `friendly_christian_peasant_infantry_attack_melee`
4. `friendly_christian_peasant_infantry_hit_react`
5. `friendly_christian_peasant_infantry_death`
6. `friendly_christian_peasant_infantry_rescuable_variant`
7. `friendly_christian_peasant_archer_idle` (loop)
8. `friendly_christian_peasant_archer_move` (loop)
9. `friendly_christian_peasant_archer_attack_ranged`
10. `friendly_christian_peasant_archer_hit_react`
11. `friendly_christian_peasant_archer_death`
12. `friendly_christian_peasant_archer_rescuable_variant`
13. `friendly_christian_peasant_priest_idle` (loop)
14. `friendly_christian_peasant_priest_move` (loop)
15. `friendly_christian_peasant_priest_cast_support`
16. `friendly_christian_peasant_priest_hit_react`
17. `friendly_christian_peasant_priest_death`
18. `friendly_christian_peasant_priest_rescuable_variant`
19. `friendly_muslim_peasant_infantry_idle` (loop)
20. `friendly_muslim_peasant_archer_idle` (loop)
21. `friendly_muslim_peasant_priest_idle` (loop)
22. `friendly_muslim_rescuable_variants` (infantry/archer/priest)

### 3) Characters - Enemies
1. `enemy_peasant_infantry_idle` (loop)
2. `enemy_peasant_infantry_move` (loop)
3. `enemy_peasant_infantry_attack`
4. `enemy_peasant_archer_idle` (loop)
5. `enemy_peasant_archer_move` (loop)
6. `enemy_peasant_archer_attack_ranged`
7. `enemy_peasant_priest_idle` (loop)
8. `enemy_peasant_priest_move` (loop)
9. `enemy_peasant_priest_cast_support`
10. `enemy_shared_hit_react`
11. `enemy_shared_death`

### 4) Shared Character Support
1. `unit_shadow_blob_small`
2. `unit_shadow_blob_medium`
3. `selection_ring_friendly` (optional if done by shader/shape)
4. `selection_ring_enemy` (optional if done by shader/shape)

### 5) Gameplay Objects and Props
1. `banner_upright`
2. `banner_dropped`
3. `banner_recover_fx_marker` (simple marker for interaction readability)
4. `rescue_marker_neutral` (icon/beacon above rescuable unit)
5. `gold_coin_stack_pickup`
6. `wave_magnet_pickup` (cross/crescent variants)
7. `equipment_chest_drop_closed`

### 6) Environment - Terrain and Foliage
1. `terrain_desert_base_tile_a`
2. `terrain_desert_base_tile_b`
3. `terrain_desert_base_tile_c`
4. `terrain_dune_overlay_a`
5. `terrain_dune_overlay_b`
6. `rock_cluster_small_a`
7. `rock_cluster_small_b`
8. `rock_cluster_medium_a`
9. `dry_bush_a`
10. `dry_bush_b`
11. `scrub_grass_patch_a`
12. `scrub_grass_patch_b`
13. `palm_tree_a`
14. `palm_tree_b`

### 7) Environment - Oasis POI (Deferred/Optional for Current Runtime)
1. `oasis_water_core`
2. `oasis_shore_edge`
3. `oasis_reeds_patch`
4. `oasis_small_rock_border`

### 8) Combat and Gameplay VFX (2D sprites/flipbooks)
1. `vfx_slash_arc_light`
2. `vfx_hit_spark_small`
3. `vfx_hit_spark_medium`
4. `vfx_dust_step_puff`
5. `vfx_dust_impact_puff`
6. `vfx_death_fade_puff`
7. `vfx_commander_aura_ring`
8. `vfx_battle_cry_wave`
9. `vfx_rescue_channel_ring`

### 9) Decals
1. `decal_body_fade_small` (or non-gore "fallen cloth" marker)
2. `decal_weapon_drop_small` (optional readability prop)
3. `decal_scorch_or_dust_mark`

### 10) Background Layer
1. `bg_far_dune_strip`
2. `bg_haze_gradient`

## Recommended First-Pass Quantities
- Fully animated unit sets: 8 total (2 commanders, 3 friendly archetypes, 3 enemy archetypes; faction variants can start as palette/gear swaps).
- Terrain/foliage sprites: 14 base pieces (enough for visual variation without overproduction).
- Gameplay object sprites: 7 (include magnet and chest pickups).
- VFX sprites/flipbooks: 9.
- Decals/background: 5.

## Not Required for MVP (Do Not Produce Yet)
- UI frame art, buttons, portraits, inventory panels.
- Additional enemy families beyond current infantry/archer/priest (for example cavalry/elites).
- Additional recruit classes beyond current peasant infantry/archer/priest (for example spearman, engineer).
- Additional map biomes.
- Formation-specific art variants beyond current six-formation gameplay set.

## Expansion Hooks (Post-MVP)
- Add per-faction material swaps for units.
- Add additional enemy silhouettes before detail pass.
- Add biome-specific foliage/props with shared naming conventions.
- Add commander skin variants only after core readability is proven.
