# ASSET_SOURCES.md

## Purpose
Track all externally sourced art used by the project, including source URLs and license terms.

## Active Asset Packs

### Kenney - Desert Shooter Pack
- URL: https://kenney.nl/assets/desert-shooter-pack
- Local path: `assets/third_party/kenney_desert-shooter-pack_1.0`
- License: Kenney License (CC0 / public domain style usage as provided in pack `License.txt`)
- Usage in game runtime:
  - Commander sprite source
  - Friendly knight sprite source
  - Enemy infantry sprite source
  - Banner upright/dropped sprite sources
  - Oasis tile source
  - Background terrain tile source

### Kenney - Roguelike/RPG Pack
- URL: https://kenney.nl/assets/roguelike-rpg-pack
- Local path: `assets/third_party/kenney_roguelike-rpg-pack`
- License: Kenney License (CC0 / public domain style usage as provided in pack `License.txt`)
- Usage in project:
  - Imported as an additional medieval-compatible source pack for future swaps/expansion.

## Runtime Mapping (Current)
Configured in `src/visuals.rs`:
- `commander_idle` -> `.../PNG/Players/Tiles/tile_0000.png`
- `friendly_knight_idle` -> `.../PNG/Players/Tiles/tile_0008.png`
- `friendly_knight_rescuable_variant` -> `.../PNG/Players/Tiles/tile_0001.png`
- `enemy_infantry_idle` -> `.../PNG/Enemies/Tiles/tile_0012.png`
- `banner_upright` -> `.../PNG/Weapons/Tiles/tile_0018.png`
- `banner_dropped` -> `.../PNG/Weapons/Tiles/tile_0003.png`
- `oasis_water_core` -> `.../PNG/Tiles/Tiles/tile_0006.png`
- `terrain_desert_base_tile_a` -> `.../PNG/Tiles/Tiles/tile_0000.png`

## Notes
- Keep each pack's original `License.txt` in the repository when assets from that pack are used.
- If new packs are added, update this file in the same commit that introduces them.
