//! This module handles everything to do with players.
//!
//! # Overview
//!
//! A [`Player`] is the bridge between a window, a logical game world, and the
//! ECS entity that represents the player inside that world.  Creating a player
//! via [`Player::spawn`] automatically:
//!
//! * allocates a new [`Entity`] in the target [`World`],
//! * attaches default [`components::Transform`] and [`components::Camera`]
//!   components to that entity,
//! * registers a [`Dispatcher`] so that any interested system can subscribe to
//!   [`PlayerUpdateEvent`]s for this player.
//!
//! Dropping a player **without** calling [`Player::despawn`] will *not* remove
//! the entity from the world; always prefer the explicit call so that the
//! `Despawned` event is reliably emitted.
//!
//! # Split-screen layout
//!
//! Each player owns a [`PlayerRegion`] that describes which rectangular slice
//! of the window it "sees".  For a single-player game the default full-screen
//! region is used automatically.  For split-screen, tile the window with
//! non-overlapping regions:
//!
//! ```text
//! ┌─────────────────────┐
//! │  Player 1           │  offset=(0.0, 0.0), size=(0.5, 1.0)
//! │           │         │
//! │           │  P2     │  offset=(0.5, 0.0), size=(0.5, 1.0)
//! └─────────────────────┘
//! ```
//!
//! # Event flow
//!
//! ```text
//! Player::spawn  ──► PlayerUpdateEvent { update_type: Spawned  }
//! set_region     ──► PlayerUpdateEvent { update_type: Updated  }
//! Player::despawn──► PlayerUpdateEvent { update_type: Despawned}
//! ```

use std::f32::consts::PI;

use events::{Dispatcher, EventManager};
use platform::WindowId;

use crate::{
    Entity, World, WorldId,
    events::{PlayerUpdateEvent, PlayerUpdateType},
};

/// Opaque identifier for a player, unique within a session.
pub type PlayerId = u32;

/// A player's rectangular slice of a window, expressed in normalised
/// `[0, 1] × [0, 1]` window coordinates.
///
/// * `offset` — top-left corner of the region in window-normalised space.
/// * `size`   — width and height of the region in window-normalised space.
///
/// Both axes run from `0.0` (left / top) to `1.0` (right / bottom).
///
/// # Invariants
///
/// The caller is responsible for ensuring that `offset + size` does not exceed
/// `(1.0, 1.0)`.  The struct itself does not enforce this.
///
/// # Examples
///
/// ```rust
/// # use world::player::PlayerRegion;
/// // Left half of the screen (Player 1 in a horizontal split-screen).
/// let p1 = PlayerRegion {
///     offset: glam::vec2(0.0, 0.0),
///     size:   glam::vec2(0.5, 1.0),
/// };
/// assert!(p1.contains(glam::vec2(0.25, 0.5)));
/// assert!(!p1.contains(glam::vec2(0.75, 0.5)));
/// ```
#[derive(Debug, Clone)]
pub struct PlayerRegion {
    /// Top-left corner in normalised window space.
    pub offset: glam::Vec2,
    /// Width and height in normalised window space.
    pub size: glam::Vec2,
}

impl Default for PlayerRegion {
    /// Returns the full-screen region: `offset = (0,0)`, `size = (1,1)`.
    fn default() -> Self {
        Self {
            offset: glam::Vec2::ZERO,
            size: glam::Vec2::ONE,
        }
    }
}

impl PlayerRegion {
    /// Returns `true` if `norm_pos` (in `[0,1]²` window space) lies **inside**
    /// this region.
    ///
    /// The check is half-open: the left and top edges are inclusive, the right
    /// and bottom edges are exclusive.  This ensures that a position sitting
    /// exactly on a shared boundary between two adjacent regions belongs to
    /// exactly one of them.
    ///
    /// # Arguments
    ///
    /// * `norm_pos` — a point in normalised window space `[0, 1]²`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use world::player::PlayerRegion;
    /// let region = PlayerRegion {
    ///     offset: glam::vec2(0.25, 0.25),
    ///     size:   glam::vec2(0.5,  0.5),
    /// };
    /// assert!(region.contains(glam::vec2(0.5, 0.5)));   // centre — inside
    /// assert!(region.contains(glam::vec2(0.25, 0.25))); // top-left corner — inclusive
    /// assert!(!region.contains(glam::vec2(0.75, 0.75))); // bottom-right corner — exclusive
    /// assert!(!region.contains(glam::vec2(0.1, 0.5)));  // left of region
    /// ```
    pub fn contains(&self, norm_pos: glam::Vec2) -> bool {
        let max = self.offset + self.size;
        norm_pos.cmpge(self.offset).all() && norm_pos.cmplt(max).all()
    }

    /// Maps a normalised **window** position to a normalised position *within*
    /// this region (`[0, 1]²` in region-local space).
    ///
    /// This is useful for passing pointer / cursor coordinates to per-player
    /// UI or camera logic without it needing to know about the global layout.
    ///
    /// Returns `Vec2::ZERO` if the region has zero area (i.e. either dimension
    /// of `size` is `0.0`) to avoid a division by zero.
    ///
    /// # Arguments
    ///
    /// * `norm_pos` — a point in normalised window space `[0, 1]²`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use world::player::PlayerRegion;
    /// // Right half of the screen.
    /// let region = PlayerRegion {
    ///     offset: glam::vec2(0.5, 0.0),
    ///     size:   glam::vec2(0.5, 1.0),
    /// };
    /// // The horizontal centre of the right half maps to 0.5 in local space.
    /// let local = region.to_local(glam::vec2(0.75, 0.5));
    /// assert!((local.x - 0.5).abs() < f32::EPSILON);
    /// assert!((local.y - 0.5).abs() < f32::EPSILON);
    /// ```
    pub fn to_local(&self, norm_pos: glam::Vec2) -> glam::Vec2 {
        if self.size.x == 0.0 || self.size.y == 0.0 {
            return glam::Vec2::ZERO;
        }
        (norm_pos - self.offset) / self.size
    }
}

/// A live player handle — combines an identity, a world, an ECS entity, a
/// window, a viewport region, and an event dispatcher.
///
/// # Lifetime
///
/// A `Player` is created with [`Player::spawn`] and destroyed with
/// [`Player::despawn`].  Both operations emit a [`PlayerUpdateEvent`].
/// Intermediate mutations (currently only [`set_region`](Player::set_region))
/// also emit an `Updated` event so that rendering and network systems can
/// react without polling.
///
/// # Thread safety
///
/// `Player` is **not** `Send` or `Sync` by default; it must be used from the
/// thread that owns the [`World`] it was spawned into.
pub struct Player {
    id: PlayerId,
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: PlayerRegion,
    dispatcher: Dispatcher<PlayerUpdateEvent>,
}

impl Player {
    /// Spawns a new player entity into `world`.
    ///
    /// Attaches the following components with sensible defaults:
    ///
    /// | Component   | Default value |
    /// |-------------|---------------|
    /// | `Transform` | location `(0, 1000, 1000)`, pitch `−45°`, no roll/yaw, scale `1` |
    /// | `Camera`    | FOV `45°`, near `0.1`, far `100 000`, width/height `100` |
    ///
    /// The player starts with the full-screen [`PlayerRegion`] (see
    /// [`PlayerRegion::default`]).
    ///
    /// # Events
    ///
    /// Fires one [`PlayerUpdateEvent`] with `update_type =`
    /// [`PlayerUpdateType::Spawned`].
    ///
    /// # Arguments
    ///
    /// * `id`            — caller-assigned identifier, unique within the session.
    /// * `world`         — mutable reference to the world the player joins.
    /// * `window`        — the window this player's camera renders into.
    /// * `event_manager` — used to register the internal [`Dispatcher`].
    ///
    /// # Panics
    ///
    /// Panics if the underlying ECS `spawn` or `insert` operations fail
    /// (implementation-defined).
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

        let player = Self {
            id,
            world: world.id(),
            entity,
            window,
            region: PlayerRegion::default(),
            dispatcher,
        };
        player.dispatcher.dispatch(PlayerUpdateEvent::from_player(
            &player,
            PlayerUpdateType::Spawned,
        ));
        player
    }

    /// Removes the player entity from the world and fires a `Despawned` event.
    ///
    /// Consumes `self` so the handle cannot be used after despawning.
    ///
    /// # Events
    ///
    /// Fires one [`PlayerUpdateEvent`] with `update_type =`
    /// [`PlayerUpdateType::Despawned`].
    pub fn despawn(self, world: &mut World) {
        world.despawn(self.entity);
        self.dispatcher.dispatch(PlayerUpdateEvent::from_player(
            &self,
            PlayerUpdateType::Despawned,
        ));
    }

    /// Returns the player's unique [`PlayerId`].
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Returns the [`WorldId`] of the world this player lives in.
    pub fn world(&self) -> WorldId {
        self.world
    }

    /// Returns the ECS [`Entity`] associated with this player.
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Returns the [`WindowId`] of the window this player renders into.
    pub fn window(&self) -> WindowId {
        self.window
    }

    /// Returns a shared reference to the player's current [`PlayerRegion`].
    pub fn region(&self) -> &PlayerRegion {
        &self.region
    }

    /// Replaces the player's viewport region and fires an `Updated` event.
    ///
    /// # Events
    ///
    /// Fires one [`PlayerUpdateEvent`] with `update_type =`
    /// [`PlayerUpdateType::Updated`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use world::player::{Player, PlayerRegion};
    /// # fn example(mut player: Player) {
    /// // Assign the left half of the screen to this player.
    /// player.set_region(PlayerRegion {
    ///     offset: glam::vec2(0.0, 0.0),
    ///     size:   glam::vec2(0.5, 1.0),
    /// });
    /// # }
    /// ```
    pub fn set_region(&mut self, region: PlayerRegion) {
        self.region = region;
        self.dispatch_update();
    }

    fn dispatch_update(&self) {
        self.dispatcher.dispatch(PlayerUpdateEvent::from_player(
            self,
            PlayerUpdateType::Updated,
        ));
    }
}
