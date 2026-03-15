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

    assets.commander_idle = asset_server.load("sprites/characters/commander_baldiun_idle.png");
    assets.friendly_knight_idle =
        asset_server.load("sprites/characters/friendly_infantry_knight_idle.png");
    assets.friendly_knight_rescuable_variant =
        asset_server.load("sprites/characters/friendly_infantry_knight_rescuable_variant.png");
    assets.enemy_infantry_idle =
        asset_server.load("sprites/characters/enemy_infantry_melee_idle.png");
    assets.banner_upright = asset_server.load("sprites/props/banner_upright.png");
    assets.banner_dropped = asset_server.load("sprites/props/banner_dropped.png");
    assets.oasis_water_core = asset_server.load("sprites/environment/oasis_water_core.png");
    assets.terrain_desert_base_tile_a =
        asset_server.load("sprites/environment/terrain_desert_base_tile_a.png");
}
