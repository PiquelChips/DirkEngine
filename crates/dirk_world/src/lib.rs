#![doc = include_str!("../README.md")]

use dirk_assets::AssetRegistry;
use dirk_universe::{Universe, UniverseBuilder};

pub mod components;
use components::ModelUploadSystem;

/// Registers world-specific ECS systems.
pub struct WorldPlugin;

impl dirk_engine::EnginePlugin for WorldPlugin {
    fn name(&self) -> &'static str {
        "world"
    }

    fn build(&self, builder: &mut dirk_engine::EngineBuilder) -> anyhow::Result<()> {
        builder.with_plugin(dirk_assets::AssetsPlugin)?;

        builder.add_subsystem(|ctx| {
            let assets = ctx.resource::<AssetRegistry>()?;
            ctx.extend_universe(
                Universe::builder().with_component_system(ModelUploadSystem::new(assets)),
            );
            Ok(WorldSubsystem)
        });

        Ok(())
    }
}

struct WorldSubsystem;

impl dirk_engine::Subsystem for WorldSubsystem {
    fn name(&self) -> &'static str {
        "world"
    }
}

/// Creates a [`UniverseBuilder`] with all the systems used by the various
/// utilities & types in this crate.
// TODO: remove after updating engine
#[must_use]
pub fn universe_builder(assets: dirk_assets::AssetRegistry) -> UniverseBuilder {
    Universe::builder().with_component_system(ModelUploadSystem::new(assets))
}
