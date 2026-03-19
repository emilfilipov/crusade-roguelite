use bevy::prelude::*;

#[derive(Resource, Clone, Debug, Default)]
pub struct ArtAssets {
    pub commander_idle: Handle<Image>,
    pub friendly_knight_idle: Handle<Image>,
    pub friendly_knight_rescuable_variant: Handle<Image>,
    pub enemy_bandit_raider_idle: Handle<Image>,
    pub enemy_bandit_raider_move: Handle<Image>,
    pub enemy_bandit_raider_attack: Handle<Image>,
    pub enemy_bandit_raider_hit: Handle<Image>,
    pub enemy_bandit_raider_dead: Handle<Image>,
    pub banner_upright: Handle<Image>,
    pub banner_dropped: Handle<Image>,
    pub terrain_desert_base_tile_a: Handle<Image>,
    pub terrain_desert_foliage_tile_a: Handle<Image>,
    pub terrain_boundary_wall_tile_a: Handle<Image>,
    pub exp_pack_coin_stack: Handle<Image>,
    pub arrow_projectile: Handle<Image>,
    pub upgrade_damage_icon: Handle<Image>,
    pub upgrade_attack_speed_icon: Handle<Image>,
    pub upgrade_armor_icon: Handle<Image>,
    pub upgrade_pickup_radius_icon: Handle<Image>,
    pub upgrade_aura_radius_icon: Handle<Image>,
    pub upgrade_authority_icon: Handle<Image>,
    pub upgrade_move_speed_icon: Handle<Image>,
    pub upgrade_hospitalier_icon: Handle<Image>,
}

pub struct VisualPlugin;

impl Plugin for VisualPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArtAssets>()
            .add_systems(Startup, load_art_assets);
    }
}

fn load_art_assets(mut assets: ResMut<ArtAssets>, asset_server: Option<Res<AssetServer>>) {
    let Some(asset_server) = asset_server else {
        return;
    };

    assets.commander_idle =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0097.png");
    assets.friendly_knight_idle =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0096.png");
    assets.friendly_knight_rescuable_variant =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0096.png");
    assets.enemy_bandit_raider_idle =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0105.png");
    assets.enemy_bandit_raider_move =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0100.png");
    assets.enemy_bandit_raider_attack =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0099.png");
    assets.enemy_bandit_raider_hit =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0098.png");
    assets.enemy_bandit_raider_dead =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0120.png");
    assets.banner_upright = asset_server
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 24.png");
    assets.banner_dropped = asset_server
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 16.png");
    assets.terrain_desert_base_tile_a = asset_server
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 39.png");
    assets.terrain_desert_foliage_tile_a = asset_server
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 70.png");
    assets.terrain_boundary_wall_tile_a = asset_server
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 76.png");
    assets.exp_pack_coin_stack = asset_server.load("sprites/pickups/xp_coin_stack.png");
    assets.arrow_projectile = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Weapons/Tiles/tile_0018.png");
    assets.upgrade_damage_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0107.png");
    assets.upgrade_attack_speed_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0062.png");
    assets.upgrade_armor_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0102.png");
    assets.upgrade_pickup_radius_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0060.png");
    assets.upgrade_aura_radius_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0056.png");
    assets.upgrade_authority_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0084.png");
    assets.upgrade_move_speed_icon = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Interface/Tiles/tile_0078.png");
    assets.upgrade_hospitalier_icon =
        asset_server.load("third_party/kenney_tiny-dungeon_1.0/Tiles/tile_0115.png");
}
