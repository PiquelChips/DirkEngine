//! Player movement systems.

use dirk_universe::{
    CommandBuffer, Entity, Universe,
    query::Query,
    systems::{System, TickingSystem},
};

use crate::{PlayerId, PlayerInputState};

/// Default movement speed in world units per second.
pub const DEFAULT_PLAYER_MOVE_SPEED: f64 = 350.0;

/// Applies player movement input to entities that have a [`PlayerId`] and
/// [`dirk_world::components::Transform`].
#[derive(System)]
pub struct PlayerMovementSystem {
    input_state: PlayerInputState,
    speed: f64,
}

impl PlayerMovementSystem {
    /// Creates a movement system using the default movement speed.
    #[must_use]
    pub fn new(input_state: PlayerInputState) -> Self {
        Self {
            input_state,
            speed: DEFAULT_PLAYER_MOVE_SPEED,
        }
    }

    fn update_entity(
        &self,
        cmd: &mut CommandBuffer,
        universe: &Universe,
        delta_time: f64,
        entity: Entity,
    ) {
        let Some(player) = universe.component::<PlayerId>(entity).copied() else {
            return;
        };
        let input = self.input_state.movement(player);
        if input == glam::Vec3::ZERO {
            return;
        }

        let Some(transform) = universe.component::<dirk_world::components::Transform>(entity)
        else {
            return;
        };

        let mut transform = transform.clone();
        let movement = transform.movement_direction(input);
        if movement == glam::Vec3::ZERO {
            return;
        }

        #[allow(clippy::cast_possible_truncation)]
        let distance = (self.speed * delta_time) as f32;
        transform.location += movement * distance;
        cmd.set_component(entity, transform);
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
            self.update_entity(cmd, universe, delta_time, entity);
        }
    }

    fn query(&self) -> Query {
        Query::empty()
            .with_component::<PlayerId>()
            .with_component::<dirk_world::components::Transform>()
    }
}
