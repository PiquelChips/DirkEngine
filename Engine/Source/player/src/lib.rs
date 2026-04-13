//! This crate handles everything to do with players.
//! This includes the input system.

pub mod input;

use std::f32::consts::PI;

use platform::WindowId;
use world::{Entity, World, WorldId};

#[derive(derive_getters::Getters)]
pub struct Player {
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: glam::Vec2,
}

impl Player {
    pub fn spawn(world: &mut World, window: WindowId) -> Self {
        use world::components;

        let entity = world.spawn();
        world.insert(
            entity,
            components::Transform {
                location: glam::vec3(0., 1000., 1000.),
                rotation: glam::vec3(-PI / 4., 0., 0.),
                scale: glam::Vec3::splat(1.),
            },
        );
        world.insert(
            entity,
            components::Camera {
                fov: (45_f32).to_radians(),
                near_clip: 0.1,
                far_clip: 100000.,
                width: 100.,
                height: 100.,
            },
        );

        Self {
            world: world.id(),
            entity,
            window,
            // TODO: handle regions
            region: glam::Vec2::splat(1.),
        }
    }
    fn handle_input_event(&self, _event: platform::InputEvent) {
        // TODO: actually do something with input
        // TODO: hard code basic movement when right key down
    }
}
