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
    pub exp_pack_coin_stack: Handle<Image>,
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
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 41.png");
    assets.terrain_desert_foliage_tile_a = asset_server
        .load("third_party/oga_ishtar_top-down-pack_1.1/top-down-pack-1/tiles/Slice 47.png");
    assets.exp_pack_coin_stack = asset_server.load("sprites/pickups/xp_coin_stack.png");
}
