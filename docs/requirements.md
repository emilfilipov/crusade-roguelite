# requirements.md

## Purpose
Define the minimum required **visual art assets** for the MVP.
UI element art is intentionally excluded for now (basic programmatic shapes are acceptable).

## Scope Assumptions (MVP)
- Top-down or slightly angled 2D battlefield.
- One commander at run start (`Baldiun`).
- Two recruitable soldier types (`Christian Peasant Infantry`, `Christian Peasant Archer`).
- One enemy type (melee infantry).
- One formation in use (`Square`).
- One map biome (desert).
- Oasis POI assets are optional/deferred until oasis gameplay is re-enabled.
- Banner drop/recovery and rescue recruitment are included.

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

### 3) Characters - Enemies
1. `enemy_bandit_raider_idle` (loop)
2. `enemy_bandit_raider_move` (loop)
3. `enemy_bandit_raider_attack`
4. `enemy_bandit_raider_hit_react`
5. `enemy_bandit_raider_death`

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
5. `xp_coin_stack_pickup`

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
- Fully animated unit sets: 4 total (commander, friendly peasant infantry, friendly peasant archer, enemy bandit raider).
- Terrain/foliage sprites: 14 base pieces (enough for visual variation without overproduction).
- Gameplay object sprites: 5.
- VFX sprites/flipbooks: 9.
- Decals/background: 5.

## Not Required for MVP (Do Not Produce Yet)
- UI frame art, buttons, portraits, inventory panels.
- Additional enemy families (archer/cavalry/elites).
- Additional recruit classes (spearman/support).
- Additional map biomes.
- Formation-specific art variants beyond square-first gameplay.

## Expansion Hooks (Post-MVP)
- Add per-faction material swaps for units.
- Add additional enemy silhouettes before detail pass.
- Add biome-specific foliage/props with shared naming conventions.
- Add commander skin variants only after core readability is proven.
