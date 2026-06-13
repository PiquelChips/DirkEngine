//! A simple [`Subsystem`] that creates a demo world.

use std::f32::consts::PI;

use anyhow::Context;
use dirk_engine::{EngineHandle, Subsystem};
use dirk_universe::{Entity, Universe, World};

/// A basic demo [`Subsystem`].
pub struct Demo {
    players: dirk_player::PlayerRegistry,
    windows: dirk_platform::PlatformWindows,
    started: bool,
}

impl Demo {
    /// Creates a new [`Demo`] [`Subsystem`].
    #[must_use]
    pub fn new(
        players: dirk_player::PlayerRegistry,
        windows: dirk_platform::PlatformWindows,
    ) -> Self {
        Self {
            players,
            windows,
            started: false,
        }
    }
}

impl Subsystem for Demo {
    fn name(&self) -> &'static str {
        "demo-startup"
    }

    fn start(&mut self, _handle: &EngineHandle, universe: &mut Universe) -> anyhow::Result<()> {
        if self.started {
            return Ok(());
        }

        let world_id = create_test_world(universe).context("create test world")?;
        let player = self.players.new_player(self.windows.main_window().id());

        universe.spawn_entity(
            world_id,
            Entity::builder().with_component(player).with_component(
                dirk_world::components::Transform {
                    location: glam::vec3(0.0, 500.0, 500.0),
                    rotation: glam::Quat::from_rotation_x(-PI / 4.0),
                    scale: glam::Vec3::ONE,
                },
            ),
        );

        self.started = true;
        Ok(())
    }
}

fn create_test_world(universe: &mut Universe) -> anyhow::Result<dirk_universe::WorldId> {
    use dirk_world::components::{Renderable, Transform};

    let duck_model = dirk_assets::AssetHandle::from_raw(
        "models/Duck/Duck.dirkasset",
        dirk_assets::AssetType::Model,
    );
    let shrek_model = dirk_assets::AssetHandle::from_raw(
        "models/Shrek/Shrek.dirkasset",
        dirk_assets::AssetType::Model,
    );

    let shrek_builder = Entity::builder()
        .with_component(Transform {
            location: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::splat(1.),
        })
        .with_component(Renderable::new(shrek_model));

    let duck_builder = Entity::builder()
        .with_component(Transform {
            location: glam::vec3(100., 0., 0.),
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::splat(1.),
        })
        .with_component(Renderable::new(duck_model));

    let world_builder = World::builder("test world")
        .with_entity(shrek_builder)
        .with_entity(duck_builder);

    universe
        .create_world(world_builder)
        .context("world creation returned no id")
}
