# ASSET_SOURCES.md

## Purpose
Track external art sources, licenses, and current runtime mapping.

## Active Runtime Sources

### Kenney - Tiny Dungeon
- URL: https://kenney.nl/assets/tiny-dungeon
- Local path: `assets/third_party/kenney_tiny-dungeon_1.0`
- License: CC0 (per source page/pack metadata)
- Runtime usage:
  - Commander sprite
  - Friendly peasant infantry sprite
  - Friendly peasant archer sprite
  - Friendly peasant priest sprite
  - Rescuable peasant variants
  - `bandit_raider` state sprites (idle/move/attack/hit/dead)
  - Several upgrade icons
  - In-run utility bar icons (inventory/stats/skill book/archive/unit upgrade)

### Kenney - Desert Shooter Pack
- URL: https://kenney.nl/assets/desert-shooter-pack
- Local path: `assets/third_party/kenney_desert-shooter-pack_1.0`
- License: Kenney License (`License.txt` in pack)
- Runtime usage:
  - Commander arrow projectile sprite
  - Movement-speed upgrade icon

### OGA - Ishtar Top-Down Pack 1.1
- URL: https://opengameart.org/content/top-down-asset-pack-1
- Local path: `assets/third_party/oga_ishtar_top-down-pack_1.1`
- License: See bundled `License.txt`
- Runtime usage:
  - Banner upright/dropped sprites
  - Desert base/foliage/wall terrain tiles

### Local Curated Sprite
- `assets/sprites/pickups/xp_coin_stack.png`
- `assets/sprites/pickups/magnet_cross.png`
- `assets/sprites/pickups/magnet_crescent.png`
- Runtime usage: XP pack pickup sprite (`exp_pack_coin_stack`)
  - plus wave magnet pickup symbols (`magnet_cross_pickup`, `magnet_crescent_pickup`)

### Local Generated Formation Icons
- `assets/sprites/skills/formation_square.png`
- `assets/sprites/skills/formation_diamond.png`
- Runtime usage:
  - formation skillbar slot icons
  - formation unlock card icons

## Installed Candidate Packs (Not Active in Runtime)
- Kenney Roguelike/RPG Pack: `assets/third_party/kenney_roguelike-rpg-pack`
- Kenney Sketch Desert: `assets/third_party/kenney_sketch-desert_1.0`
- OGA Top-Down Asset Pack 1.0: `assets/third_party/oga_ishtar_top-down-pack_1.1`
- OGA Pixel FX Pack: `assets/third_party/oga_pixel_fx_pack`
- OGA Top Down Asset Pack 1 (CTATZ): `assets/third_party/oga_ctatz_top-down-pack_1`

## Runtime Mapping (`src/visuals.rs`)
- `commander_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0097.png`
- `friendly_peasant_infantry_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0111.png`
- `friendly_peasant_infantry_rescuable_variant` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0111.png`
- `friendly_peasant_archer_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0112.png`
- `friendly_peasant_archer_rescuable_variant` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0112.png`
- `friendly_peasant_priest_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0109.png`
- `enemy_bandit_raider_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0105.png`
- `enemy_bandit_raider_move` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0100.png`
- `enemy_bandit_raider_attack` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0099.png`
- `enemy_bandit_raider_hit` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0098.png`
- `enemy_bandit_raider_dead` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0120.png`
- `banner_upright` -> `third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 24.png`
- `banner_dropped` -> `third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 16.png`
- `terrain_desert_base_tile_a` -> `third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 66.png`
- `terrain_desert_foliage_tile_a` -> `third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 70.png`
- `terrain_boundary_wall_tile_a` -> `third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 76.png`
- `exp_pack_coin_stack` -> `sprites/pickups/xp_coin_stack.png`
- `magnet_cross_pickup` -> `sprites/pickups/magnet_cross.png`
- `magnet_crescent_pickup` -> `sprites/pickups/magnet_crescent.png`
- `arrow_projectile` -> `third_party/kenney_desert-shooter-pack_1.0/PNG/Weapons/Tiles/tile_0018.png`
- `formation_square_icon` -> `sprites/skills/formation_square.png`
- `formation_diamond_icon` -> `sprites/skills/formation_diamond.png`

## Notes
- Keep source pack license files in-repo for any active runtime pack.
- Update this file in the same commit as runtime asset mapping changes.
