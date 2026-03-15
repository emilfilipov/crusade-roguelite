use bevy::prelude::*;

pub trait PlatformService: Send + Sync {
    fn platform_name(&self) -> &'static str;
}

#[derive(Resource)]
pub struct PlatformRuntime {
    pub service: Box<dyn PlatformService>,
}

#[cfg(not(feature = "steam"))]
struct StandaloneService;

#[cfg(not(feature = "steam"))]
impl PlatformService for StandaloneService {
    fn platform_name(&self) -> &'static str {
        "standalone"
    }
}

#[cfg(feature = "steam")]
struct SteamService;

#[cfg(feature = "steam")]
impl PlatformService for SteamService {
    fn platform_name(&self) -> &'static str {
        "steam"
    }
}

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "steam")]
        app.insert_resource(PlatformRuntime {
            service: Box::new(SteamService),
        });

        #[cfg(not(feature = "steam"))]
        app.insert_resource(PlatformRuntime {
            service: Box::new(StandaloneService),
        });
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::App;

    use crate::steam::{PlatformPlugin, PlatformRuntime};

    #[test]
    fn platform_runtime_exists() {
        let mut app = App::new();
        app.add_plugins(PlatformPlugin);
        let runtime = app.world().resource::<PlatformRuntime>();
        assert!(!runtime.service.platform_name().is_empty());
    }
}
