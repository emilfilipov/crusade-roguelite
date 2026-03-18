# ASSET_SOURCES.md

## Purpose
Track externally sourced art used by the project, including source URLs, local paths, license notes, and current runtime mapping.

## Active Asset Packs

### Kenney - Tiny Dungeon
- URL: https://kenney.nl/assets/tiny-dungeon
- Local path: `assets/third_party/kenney_tiny-dungeon_1.0`
- License: Creative Commons CC0 (as listed on source page and pack metadata)
- Runtime usage:
  - Commander sprite
  - Friendly knight sprite
  - Rescuable variant sprite
  - `bandit_raider` idle/move/attack/hit/dead states

### Kenney - Desert Shooter Pack
- URL: https://kenney.nl/assets/desert-shooter-pack
- Local path: `assets/third_party/kenney_desert-shooter-pack_1.0`
- License: Kenney License (CC0/public-domain style usage as provided in pack `License.txt`)
- Runtime usage:
  - Banner upright/dropped sprites
  - Desert ground tile

### Kenney - Sketch Desert
- URL: https://kenney.nl/assets/sketch-desert
- Local path: `assets/third_party/kenney_sketch-desert_1.0`
- License: Creative Commons CC0 (as listed on source page and pack metadata)
- Runtime usage:
  - Oasis placeholder tile handle (`oasis_water_core`) is loaded, but oasis gameplay is currently disabled.

### Kenney - Roguelike/RPG Pack
- URL: https://kenney.nl/assets/roguelike-rpg-pack
- Local path: `assets/third_party/kenney_roguelike-rpg-pack`
- License: Kenney License (CC0/public-domain style usage as provided in pack `License.txt`)
- Runtime usage:
  - Source pack for pickup icon extraction.

## Local Curated Sprites
- `assets/sprites/pickups/xp_coin_stack.png`
  - Used as runtime XP pack sprite.
  - Curated local sprite based on the Kenney roguelike/RPG asset set.

## Downloaded Candidate Packs (Not Wired in Runtime)

### OpenGameArt - Top-Down Asset Pack 1.0
- URL: https://opengameart.org/content/top-down-asset-pack-10
- Local path: `assets/third_party/oga_ishtar_top-down-pack_1.1`
- License: CC0 (per source page)

### OpenGameArt - Pixel FX Pack
- URL: https://opengameart.org/content/pixel-fx-pack
- Local path: `assets/third_party/oga_pixel_fx_pack`
- License: CC0 (per source page)

### OpenGameArt - Top Down Asset Pack 1 (CTATZ)
- URL: https://opengameart.org/content/top-down-asset-pack-1-ctatz
- Local path: `assets/third_party/oga_ctatz_top-down-pack_1`
- License: CC0 (per source page)

## Runtime Mapping (Current)
Configured in `src/visuals.rs`:
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
- `oasis_water_core` -> `third_party/kenney_sketch-desert_1.0/Tiles/water_center_N.png`
- `exp_pack_coin_stack` -> `sprites/pickups/xp_coin_stack.png`

## Notes
- Keep original pack `License.txt` files in the repository whenever assets from that pack are used.
- Update this file in the same commit whenever runtime mappings or source packs change.
