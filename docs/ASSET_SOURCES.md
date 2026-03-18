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
  - Friendly knight sprite
  - Rescuable variant sprite
  - `bandit_raider` state sprites (idle/move/attack/hit/dead)

### Kenney - Desert Shooter Pack
- URL: https://kenney.nl/assets/desert-shooter-pack
- Local path: `assets/third_party/kenney_desert-shooter-pack_1.0`
- License: Kenney License (`License.txt` in pack)
- Runtime usage:
  - Banner upright/dropped sprites
  - Desert terrain base tile

### Local Curated Sprite
- `assets/sprites/pickups/xp_coin_stack.png`
- Runtime usage: XP pack pickup sprite (`exp_pack_coin_stack`)

## Installed Candidate Packs (Not Active in Runtime)
- Kenney Roguelike/RPG Pack: `assets/third_party/kenney_roguelike-rpg-pack`
- Kenney Sketch Desert: `assets/third_party/kenney_sketch-desert_1.0`
- OGA Top-Down Asset Pack 1.0: `assets/third_party/oga_ishtar_top-down-pack_1.1`
- OGA Pixel FX Pack: `assets/third_party/oga_pixel_fx_pack`
- OGA Top Down Asset Pack 1 (CTATZ): `assets/third_party/oga_ctatz_top-down-pack_1`

## Runtime Mapping (`src/visuals.rs`)
- `commander_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0097.png`
- `friendly_knight_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0096.png`
- `friendly_knight_rescuable_variant` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0096.png`
- `enemy_bandit_raider_idle` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0105.png`
- `enemy_bandit_raider_move` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0100.png`
- `enemy_bandit_raider_attack` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0099.png`
- `enemy_bandit_raider_hit` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0098.png`
- `enemy_bandit_raider_dead` -> `third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0120.png`
- `banner_upright` -> `third_party/kenney_desert-shooter-pack_1.0/PNG/Weapons/Tiles/tile_0018.png`
- `banner_dropped` -> `third_party/kenney_desert-shooter-pack_1.0/PNG/Weapons/Tiles/tile_0003.png`
- `terrain_desert_base_tile_a` -> `third_party/kenney_desert-shooter-pack_1.0/PNG/Tiles/Tiles/tile_0000.png`
- `exp_pack_coin_stack` -> `sprites/pickups/xp_coin_stack.png`

## Notes
- Keep source pack license files in-repo for any active runtime pack.
- Update this file in the same commit as runtime asset mapping changes.
