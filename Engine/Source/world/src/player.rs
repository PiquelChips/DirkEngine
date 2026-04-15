//! This module handles everything to do with players.

use std::f32::consts::PI;

use events::{Dispatcher, EventManager};
use platform::WindowId;

use crate::{Entity, World, WorldId, events::PlayerEvent};

pub type PlayerId = u32;

/// A player's rectangular slice of a window, expressed in normalised
/// `[0, 1] × [0, 1]` window coordinates.
///
/// A single-player game uses the default full-screen region. For split-screen,
/// set up two regions whose offsets and sizes tile the window without overlap.
///
/// ```text
/// ┌─────────────────────┐
/// │  Player 1           │  offset=(0.0, 0.0), size=(0.5, 1.0)
/// │           │         │
/// │           │  P2     │  offset=(0.5, 0.0), size=(0.5, 1.0)
/// └─────────────────────┘
/// ```
pub struct PlayerRegion {
    /// Top-left corner in normalised window space.
    pub offset: glam::Vec2,
    /// Width and height in normalised window space.
    pub size: glam::Vec2,
}

impl Default for PlayerRegion {
    fn default() -> Self {
        Self {
            offset: glam::Vec2::ZERO,
            size: glam::Vec2::ONE,
        }
    }
}

impl PlayerRegion {
    /// Returns `true` if `norm_pos` (in `[0,1]²` window space) lies inside
    /// this region.
    pub fn contains(&self, norm_pos: glam::Vec2) -> bool {
        let max = self.offset + self.size;
        norm_pos.cmpge(self.offset).all() && norm_pos.cmplt(max).all()
    }

    /// Maps a normalised window position to a normalised position *within*
    /// this region (`[0, 1]²` in region space).
    ///
    /// Returns `Vec2::ZERO` if the region has zero area.
    pub fn to_local(&self, norm_pos: glam::Vec2) -> glam::Vec2 {
        if self.size.x == 0.0 || self.size.y == 0.0 {
            return glam::Vec2::ZERO;
        }
        (norm_pos - self.offset) / self.size
    }
}

pub struct Player {
    id: PlayerId,
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: PlayerRegion,
    dispatcher: Dispatcher<PlayerEvent>,
}

impl Player {
    /// Spawns a new player entity into `world`, attaches a default
    /// [`world::components::Transform`] and [`world::components::Camera`],
    /// and returns the [`Player`] handle.
    ///
    /// Fires [`PlayerEvent::Spawned`].
    pub fn spawn(
        id: PlayerId,
        world: &mut World,
        window: WindowId,
        event_manager: &EventManager,
    ) -> Self {
        use crate::components;
        let entity = world.spawn();
        world.insert(
            entity,
            components::Transform {
                location: glam::vec3(0.0, 1000.0, 1000.0),
                rotation: glam::vec3(-PI / 4.0, 0.0, 0.0),
                scale: glam::Vec3::ONE,
            },
        );
        world.insert(
            entity,
            components::Camera {
                fov: 45_f32.to_radians(),
                near_clip: 0.1,
                far_clip: 100_000.0,
                width: 100.0,
                height: 100.0,
            },
        );

        let dispatcher = event_manager.register();
        dispatcher.dispatch(PlayerEvent::Spawned(id));

        Self {
            id,
            world: world.id(),
            entity,
            window,
            region: PlayerRegion::default(),
            dispatcher,
        }
    }

    /// Removes the player entity from the world and fires
    /// [`PlayerEvent::Despawned`].
    ///
    /// Consumes `self` so the handle cannot be used after despawning.
    pub fn despawn(self, world: &mut World) {
        world.despawn(self.entity);
        self.dispatcher.dispatch(PlayerEvent::Despawned(self.id));
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }
    pub fn world(&self) -> WorldId {
        self.world
    }
    pub fn entity(&self) -> Entity {
        self.entity
    }
    pub fn window(&self) -> WindowId {
        self.window
    }
    pub fn region(&self) -> &PlayerRegion {
        &self.region
    }
    pub fn set_region(&mut self, region: PlayerRegion) {
        self.region = region;
    }
}
