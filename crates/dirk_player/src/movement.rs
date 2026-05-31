//! Player movement systems.

use dirk_universe::{
    CommandBuffer, Entity, Universe,
    query::Query,
    systems::{System, TickingSystem},
};

use crate::{PlayerId, PlayerInputState};

/// Default movement speed in world units per second.
pub const DEFAULT_PLAYER_MOVE_SPEED: f64 = 350.0;
/// Default pointer-look sensitivity, in radians per physical pixel.
pub const DEFAULT_PLAYER_LOOK_SENSITIVITY: f32 = 0.0025;

/// Applies player movement input to entities that have a [`PlayerId`] and
/// [`dirk_world::components::Transform`].
#[derive(System)]
pub struct PlayerMovementSystem {
    input_state: PlayerInputState,
    speed: f64,
    look_sensitivity: f32,
}

impl PlayerMovementSystem {
    /// Creates a movement system using the default movement speed.
    #[must_use]
    pub fn new(input_state: PlayerInputState) -> Self {
        Self {
            input_state,
            speed: DEFAULT_PLAYER_MOVE_SPEED,
            look_sensitivity: DEFAULT_PLAYER_LOOK_SENSITIVITY,
        }
    }
}

impl TickingSystem for PlayerMovementSystem {
    fn tick(
        &self,
        cmd: &mut CommandBuffer,
        universe: &Universe,
        delta_time: f64,
        entities: &mut dyn Iterator<Item = Entity>,
    ) {
        for entity in entities {
            let Some(player) = universe.component::<PlayerId>(entity).copied() else {
                return;
            };
            let movement_input = self.input_state.movement(player);
            let look_input = self.input_state.look(player);
            if movement_input == glam::Vec3::ZERO && look_input == glam::DVec2::ZERO {
                return;
            }

            let Some(transform) = universe.component::<dirk_world::components::Transform>(entity)
            else {
                return;
            };

            let mut transform = transform.clone();
            if look_input != glam::DVec2::ZERO {
                transform.rotate_by_pointer_delta(look_input, self.look_sensitivity);
            }

            if movement_input != glam::Vec3::ZERO {
                let movement = transform.movement_direction(movement_input);
                if movement != glam::Vec3::ZERO {
                    #[allow(clippy::cast_possible_truncation)]
                    let distance = (self.speed * delta_time) as f32;
                    transform.location += movement * distance;
                }
            }

            cmd.set_component(entity, transform);
        }
    }

    fn query(&self) -> Query {
        Query::empty()
            .with_component::<PlayerId>()
            .with_component::<dirk_world::components::Transform>()
    }
}
