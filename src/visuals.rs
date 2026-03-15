use bevy::prelude::*;

#[derive(Resource, Clone, Debug, Default)]
pub struct ArtAssets {
    pub commander_idle: Handle<Image>,
    pub friendly_knight_idle: Handle<Image>,
    pub friendly_knight_rescuable_variant: Handle<Image>,
    pub enemy_infantry_idle: Handle<Image>,
    pub banner_upright: Handle<Image>,
    pub banner_dropped: Handle<Image>,
    pub oasis_water_core: Handle<Image>,
    pub terrain_desert_base_tile_a: Handle<Image>,
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

    assets.commander_idle = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Players/Tiles/tile_0000.png");
    assets.friendly_knight_idle = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Players/Tiles/tile_0008.png");
    assets.friendly_knight_rescuable_variant = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Players/Tiles/tile_0001.png");
    assets.enemy_infantry_idle = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Enemies/Tiles/tile_0012.png");
    assets.banner_upright = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Weapons/Tiles/tile_0018.png");
    assets.banner_dropped = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Weapons/Tiles/tile_0003.png");
    assets.oasis_water_core = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Tiles/Tiles/tile_0006.png");
    assets.terrain_desert_base_tile_a = asset_server
        .load("third_party/kenney_desert-shooter-pack_1.0/PNG/Tiles/Tiles/tile_0000.png");
}
