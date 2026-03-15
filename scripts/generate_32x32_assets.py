from __future__ import annotations

from pathlib import Path
import hashlib
import random

from PIL import Image, ImageDraw


ROOT = Path(__file__).resolve().parents[1]
SPRITES = ROOT / "assets" / "sprites"
SIZE = 32


ASSET_PATHS = [
    ("characters", "commander_baldiun_idle"),
    ("characters", "commander_baldiun_move"),
    ("characters", "commander_baldiun_attack_melee"),
    ("characters", "commander_baldiun_hit_react"),
    ("characters", "commander_baldiun_death"),
    ("characters", "commander_baldiun_battle_cry_cast"),
    ("characters", "friendly_infantry_knight_idle"),
    ("characters", "friendly_infantry_knight_move"),
    ("characters", "friendly_infantry_knight_attack_melee"),
    ("characters", "friendly_infantry_knight_hit_react"),
    ("characters", "friendly_infantry_knight_death"),
    ("characters", "friendly_infantry_knight_rescuable_variant"),
    ("characters", "enemy_infantry_melee_idle"),
    ("characters", "enemy_infantry_melee_move"),
    ("characters", "enemy_infantry_melee_attack"),
    ("characters", "enemy_infantry_melee_hit_react"),
    ("characters", "enemy_infantry_melee_death"),
    ("characters", "unit_shadow_blob_small"),
    ("characters", "unit_shadow_blob_medium"),
    ("characters", "selection_ring_friendly"),
    ("characters", "selection_ring_enemy"),
    ("props", "banner_upright"),
    ("props", "banner_dropped"),
    ("props", "banner_recover_fx_marker"),
    ("props", "rescue_marker_neutral"),
    ("props", "xp_shard_pickup"),
    ("environment", "terrain_desert_base_tile_a"),
    ("environment", "terrain_desert_base_tile_b"),
    ("environment", "terrain_desert_base_tile_c"),
    ("environment", "terrain_dune_overlay_a"),
    ("environment", "terrain_dune_overlay_b"),
    ("environment", "rock_cluster_small_a"),
    ("environment", "rock_cluster_small_b"),
    ("environment", "rock_cluster_medium_a"),
    ("environment", "dry_bush_a"),
    ("environment", "dry_bush_b"),
    ("environment", "scrub_grass_patch_a"),
    ("environment", "scrub_grass_patch_b"),
    ("environment", "palm_tree_a"),
    ("environment", "palm_tree_b"),
    ("environment", "oasis_water_core"),
    ("environment", "oasis_shore_edge"),
    ("environment", "oasis_reeds_patch"),
    ("environment", "oasis_small_rock_border"),
    ("vfx", "vfx_slash_arc_light"),
    ("vfx", "vfx_hit_spark_small"),
    ("vfx", "vfx_hit_spark_medium"),
    ("vfx", "vfx_dust_step_puff"),
    ("vfx", "vfx_dust_impact_puff"),
    ("vfx", "vfx_death_fade_puff"),
    ("vfx", "vfx_commander_aura_ring"),
    ("vfx", "vfx_battle_cry_wave"),
    ("vfx", "vfx_rescue_channel_ring"),
    ("decals", "decal_body_fade_small"),
    ("decals", "decal_weapon_drop_small"),
    ("decals", "decal_scorch_or_dust_mark"),
    ("background", "bg_far_dune_strip"),
    ("background", "bg_haze_gradient"),
]


def seeded_rng(name: str) -> random.Random:
    digest = hashlib.sha256(name.encode("utf-8")).digest()
    return random.Random(int.from_bytes(digest[:8], "little"))


def new_canvas(opaque: bool = False) -> Image.Image:
    if opaque:
        return Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 255))
    return Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))


def draw_humanoid(name: str, palette: tuple[int, int, int], accent: tuple[int, int, int]) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    body = palette + (255,)
    accent_color = accent + (255,)
    skin = (215, 178, 140, 255)
    outline = (30, 24, 20, 255)
    draw.ellipse((12, 5, 20, 13), fill=skin, outline=outline)
    draw.rectangle((10, 14, 22, 26), fill=body, outline=outline)
    draw.rectangle((8, 15, 10, 24), fill=body, outline=outline)
    draw.rectangle((22, 15, 24, 24), fill=body, outline=outline)
    draw.rectangle((11, 26, 14, 31), fill=body, outline=outline)
    draw.rectangle((18, 26, 21, 31), fill=body, outline=outline)
    draw.rectangle((12, 15, 20, 17), fill=accent_color, outline=outline)

    if "attack" in name:
        draw.line((24, 19, 30, 15), fill=(190, 190, 200, 255), width=2)
    elif "battle_cry" in name:
        draw.arc((1, 8, 31, 30), start=290, end=70, fill=(240, 210, 110, 255), width=2)
    elif "hit" in name:
        draw.line((6, 7, 9, 10), fill=(255, 90, 90, 255), width=2)
        draw.line((9, 7, 6, 10), fill=(255, 90, 90, 255), width=2)
    elif "death" in name:
        draw.rectangle((6, 22, 26, 28), fill=body, outline=outline)
    elif "move" in name:
        draw.line((10, 29, 6, 31), fill=outline, width=2)
        draw.line((20, 29, 24, 31), fill=outline, width=2)

    return img


def draw_shadow(radius: int) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    draw.ellipse((16 - radius, 16 - radius // 2, 16 + radius, 16 + radius // 2), fill=(0, 0, 0, 120))
    return img


def draw_ring(color: tuple[int, int, int]) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    rgba = color + (220,)
    draw.ellipse((4, 4, 27, 27), outline=rgba, width=2)
    return img


def draw_banner(name: str) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    pole = (120, 90, 60, 255)
    cloth = (190, 40, 40, 255)
    if "dropped" in name:
        draw.rectangle((6, 22, 27, 24), fill=pole)
        draw.polygon([(10, 20), (24, 20), (20, 28), (8, 28)], fill=cloth)
    else:
        draw.rectangle((14, 4, 17, 30), fill=pole)
        draw.polygon([(17, 6), (29, 8), (24, 16), (17, 14)], fill=cloth)
    return img


def draw_marker(color: tuple[int, int, int]) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    rgba = color + (255,)
    draw.polygon([(16, 3), (29, 16), (16, 29), (3, 16)], outline=rgba, fill=(0, 0, 0, 0), width=2)
    draw.ellipse((11, 11, 21, 21), fill=rgba)
    return img


def draw_terrain(name: str) -> Image.Image:
    rng = seeded_rng(name)
    img = new_canvas(opaque=True)
    draw = ImageDraw.Draw(img)
    base = (186 + rng.randint(-10, 10), 157 + rng.randint(-10, 10), 112 + rng.randint(-10, 10), 255)
    draw.rectangle((0, 0, SIZE, SIZE), fill=base)
    for _ in range(30):
        x = rng.randint(0, SIZE - 1)
        y = rng.randint(0, SIZE - 1)
        shade = (base[0] - rng.randint(6, 24), base[1] - rng.randint(6, 24), base[2] - rng.randint(6, 24), 255)
        draw.point((x, y), fill=shade)
    if "dune_overlay" in name:
        draw.arc((2, 8, 30, 28), 180, 350, fill=(210, 186, 140, 210), width=2)
        draw.arc((0, 6, 28, 24), 180, 350, fill=(160, 130, 90, 150), width=1)
    return img


def draw_rocks(name: str) -> Image.Image:
    rng = seeded_rng(name)
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    count = 3 if "small" in name else 5
    for _ in range(count):
        x = rng.randint(5, 21)
        y = rng.randint(8, 24)
        w = rng.randint(5, 10)
        h = rng.randint(4, 8)
        draw.ellipse((x, y, x + w, y + h), fill=(110, 104, 96, 255), outline=(60, 56, 50, 255))
    return img


def draw_plant(name: str) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    if "palm_tree" in name:
        draw.rectangle((15, 12, 17, 31), fill=(118, 85, 45, 255))
        for offset in (-8, -4, 0, 4, 8):
            draw.polygon([(16, 8), (16 + offset, 2), (16 + offset // 2, 10)], fill=(68, 116, 62, 255))
    elif "dry_bush" in name:
        draw.ellipse((8, 14, 24, 30), fill=(134, 120, 72, 255), outline=(83, 74, 45, 255))
    elif "scrub_grass" in name:
        for x in range(6, 27, 3):
            draw.line((x, 30, x + (x % 2) * 2 - 1, 20), fill=(109, 129, 73, 255), width=2)
    elif "oasis_reeds" in name:
        for x in range(8, 24, 2):
            draw.line((x, 30, x, 16), fill=(66, 122, 79, 255), width=1)
    return img


def draw_oasis(name: str) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    if "water_core" in name:
        draw.ellipse((4, 4, 28, 28), fill=(44, 124, 162, 220), outline=(24, 86, 118, 255))
    elif "shore_edge" in name:
        draw.arc((3, 3, 29, 29), 170, 350, fill=(210, 188, 141, 255), width=4)
    elif "rock_border" in name:
        for x in range(4, 28, 6):
            draw.ellipse((x, 20, x + 6, 26), fill=(110, 104, 96, 255))
    return img


def draw_vfx(name: str) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    if "slash_arc" in name:
        draw.arc((4, 4, 28, 28), 220, 320, fill=(230, 230, 240, 255), width=3)
    elif "hit_spark" in name:
        color = (255, 210, 90, 255)
        if "medium" in name:
            points = [(16, 3), (21, 12), (31, 16), (21, 20), (16, 29), (11, 20), (1, 16), (11, 12)]
        else:
            points = [(16, 7), (20, 13), (26, 16), (20, 19), (16, 25), (12, 19), (6, 16), (12, 13)]
        draw.polygon(points, fill=color)
    elif "dust" in name:
        draw.ellipse((8, 16, 24, 30), fill=(173, 150, 112, 170))
        draw.ellipse((5, 18, 15, 28), fill=(173, 150, 112, 130))
        draw.ellipse((17, 18, 27, 28), fill=(173, 150, 112, 130))
    elif "death_fade" in name:
        draw.ellipse((7, 10, 25, 28), outline=(190, 190, 190, 180), width=2)
    elif "aura_ring" in name:
        draw.ellipse((3, 3, 28, 28), outline=(128, 204, 210, 220), width=3)
    elif "battle_cry_wave" in name:
        draw.arc((2, 8, 30, 30), 240, 60, fill=(245, 212, 114, 255), width=2)
        draw.arc((6, 10, 26, 26), 240, 60, fill=(245, 212, 114, 180), width=2)
    elif "rescue_channel_ring" in name:
        draw.ellipse((5, 5, 27, 27), outline=(120, 220, 140, 230), width=2)
        draw.arc((7, 7, 25, 25), 270, 30, fill=(120, 220, 140, 230), width=2)
    return img


def draw_decal(name: str) -> Image.Image:
    img = new_canvas()
    draw = ImageDraw.Draw(img)
    if "body_fade" in name:
        draw.ellipse((8, 18, 24, 28), fill=(75, 56, 46, 120))
    elif "weapon_drop" in name:
        draw.line((10, 24, 22, 16), fill=(140, 140, 150, 220), width=2)
        draw.rectangle((8, 24, 12, 26), fill=(92, 62, 42, 220))
    elif "scorch" in name:
        draw.ellipse((6, 12, 26, 26), fill=(60, 50, 44, 140))
    return img


def draw_background(name: str) -> Image.Image:
    img = new_canvas(opaque=True)
    draw = ImageDraw.Draw(img)
    if "haze" in name:
        for y in range(SIZE):
            alpha = int(90 + (y / (SIZE - 1)) * 165)
            draw.line((0, y, SIZE, y), fill=(201, 187, 158, alpha))
    else:
        draw.rectangle((0, 0, SIZE, SIZE), fill=(195, 168, 128, 255))
        draw.arc((0, 8, 22, 30), 180, 360, fill=(170, 142, 102, 255), width=3)
        draw.arc((10, 4, 32, 28), 180, 360, fill=(182, 153, 112, 255), width=3)
    return img


def render_sprite(group: str, name: str) -> Image.Image:
    if "commander" in name:
        return draw_humanoid(name, (142, 156, 178), (224, 196, 112))
    if "friendly_infantry_knight" in name:
        return draw_humanoid(name, (116, 146, 178), (204, 196, 170))
    if "enemy_infantry" in name:
        return draw_humanoid(name, (162, 92, 80), (120, 64, 56))
    if "shadow_blob" in name:
        return draw_shadow(9 if "medium" in name else 6)
    if "selection_ring_friendly" in name:
        return draw_ring((80, 180, 255))
    if "selection_ring_enemy" in name:
        return draw_ring((230, 90, 90))
    if "banner_" in name:
        return draw_banner(name)
    if "marker" in name or "xp_shard" in name:
        color = (140, 230, 150) if "rescue" in name else (120, 200, 230)
        if "xp_shard" in name:
            color = (140, 220, 250)
        if "recover" in name:
            color = (240, 210, 110)
        return draw_marker(color)
    if group == "environment":
        if "terrain_" in name:
            return draw_terrain(name)
        if "rock_cluster" in name:
            return draw_rocks(name)
        if "dry_bush" in name or "scrub_grass" in name or "palm_tree" in name or "reeds" in name:
            return draw_plant(name)
        if "oasis_" in name:
            return draw_oasis(name)
    if group == "vfx":
        return draw_vfx(name)
    if group == "decals":
        return draw_decal(name)
    if group == "background":
        return draw_background(name)
    return new_canvas()


def main() -> None:
    generated = 0
    for group, name in ASSET_PATHS:
        folder = SPRITES / group
        folder.mkdir(parents=True, exist_ok=True)
        img = render_sprite(group, name)
        img.save(folder / f"{name}.png")
        generated += 1
    print(f"Generated {generated} assets in {SPRITES}")


if __name__ == "__main__":
    main()
