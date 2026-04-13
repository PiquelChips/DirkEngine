use std::f32::consts::PI;

use platform::WindowId;
use world::{Entity, World, WorldId};

#[derive(derive_getters::Getters)]
pub struct Player {
    world: WorldId,
    entity: Entity,
    window: Option<WindowId>,
}

impl Player {
    pub fn spawn(world: &mut World) -> Self {
        use world::components;

        let player = world.spawn();
        world.insert(
            player,
            components::Transform {
                location: glam::vec3(0., 1000., 1000.),
                rotation: glam::vec3(-PI / 4., 0., 0.),
                scale: glam::Vec3::splat(1.),
            },
        );
        world.insert(
            player,
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
            entity: player,
            window: None,
        }
    }
}
