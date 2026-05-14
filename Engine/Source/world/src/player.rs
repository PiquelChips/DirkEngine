//! This module handles everything to do with players.
//!
//! # Overview
//!
//! A [`Player`] is the bridge between a window, a logical game world, and the
//! ECS entity that represents the player inside that world.  Creating a player
//! via [`Player::spawn`] automatically:
//!
//! * allocates a new [`Entity`] in the target [`World`],
//! * attaches default [`Transform`] and [`Camera`] components.
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
//!
//! [`Transform`]: crate::components::Transform
//! [`Camera`]: crate::components::Camera

use std::f32::consts::PI;

use events::{Consumer, Dispatcher, Event, EventManager};
use platform::{WindowEvent, WindowId};

use crate::components::{Camera, Transform};
use universe::{Entity, Universe, WorldId};

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
    #[must_use]
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
    #[must_use]
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
    platform_consumer: Consumer<WindowEvent>,
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
        universe: &mut Universe,
        world: WorldId,
        window: WindowId,
        event_manager: &EventManager,
    ) -> Self {
        let builder = Entity::builder()
            .with_component(Transform {
                location: glam::vec3(0.0, 500.0, 500.0),
                rotation: glam::vec3(-PI / 4.0, 0.0, 0.0),
                scale: glam::Vec3::ONE,
            })
            .with_component(Camera {
                fov: 45_f32.to_radians(),
                near_clip: 0.1,
                far_clip: 100_000.0,
                width: 100.0,
                height: 100.0,
            });

        let player = Self {
            id,
            world,
            entity: universe
                .spawn_entity(world, builder)
                .expect("the world should exist"),
            window,
            region: PlayerRegion::default(),
            dispatcher: event_manager.register(),
            platform_consumer: event_manager.subscribe(),
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
    pub fn despawn(self, universe: &mut Universe) {
        let mut cmd = Universe::new_command_buffer();
        cmd.despawn(self.entity);
        universe.submit_buffer(cmd);
        self.dispatcher.dispatch(PlayerUpdateEvent::from_player(
            &self,
            PlayerUpdateType::Despawned,
        ));
    }

    /// Returns the player's unique [`PlayerId`].
    #[must_use]
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Returns the [`WorldId`] of the world this player lives in.
    #[must_use]
    pub fn world(&self) -> WorldId {
        self.world
    }

    /// Returns the ECS [`Entity`] associated with this player.
    #[must_use]
    pub fn entity(&self) -> Entity {
        self.entity
    }

    /// Returns the [`WindowId`] of the window this player renders into.
    #[must_use]
    pub fn window(&self) -> WindowId {
        self.window
    }

    /// Returns a shared reference to the player's current [`PlayerRegion`].
    #[must_use]
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

    /// Updates the player's information. This mainly listens for
    /// [`WindowEvent::Resized`] & updates the camera accordingly.
    ///
    /// # Panics
    ///
    /// Will panic if the player entity does not have a [`Camera`] component.
    // the window size will never get close to 2^23
    #[allow(clippy::cast_precision_loss)]
    pub fn tick(&mut self, universe: &mut Universe) {
        for event in self.platform_consumer.consume_all() {
            let WindowEvent::Resized { id, width, height } = event else {
                continue;
            };
            if id != self.window {
                continue;
            }

            let mut camera = universe
                .component::<Camera>(self.entity)
                .cloned()
                .expect("player should have his own camera");

            camera.width = width as f32;
            camera.height = height as f32;

            let mut cmd = Universe::new_command_buffer();
            cmd.set_component(self.entity, camera);
            universe.submit_buffer(cmd);
        }
    }

    fn dispatch_update(&self) {
        self.dispatcher.dispatch(PlayerUpdateEvent::from_player(
            self,
            PlayerUpdateType::Updated,
        ));
    }
}

/// A snapshot of a player's observable state at the moment a change occurred.
///
/// Emitted by [`Player`] on spawn, every call to
/// [`Player::set_region`](crate::player::Player::set_region), and on despawn.
/// Because the event is a *value snapshot* (not a reference), listeners can
/// safely store it or send it across threads without holding a lock on the
/// player.
///
/// # Fields
///
/// | Field | Description |
/// |-------|-------------|
/// | `id`          | The player's unique [`PlayerId`]. |
/// | `world`       | The [`WorldId`] the player lives in. |
/// | `entity`      | The ECS [`Entity`] for this player. |
/// | `window`      | The [`WindowId`] the player renders into. |
/// | `region`      | A clone of the player's [`PlayerRegion`] at the time of the event. |
/// | `update_type` | Why the event was fired — see [`PlayerUpdateType`]. |
///
/// # Examples
///
/// ```rust
/// # use world::player::{PlayerUpdateEvent, PlayerUpdateType};
/// # fn example(evt: PlayerUpdateEvent) {
/// match evt.update_type {
///     PlayerUpdateType::Spawned   => { /* initialise per-player state */ }
///     PlayerUpdateType::Updated   => { /* refresh cached region / camera */ }
///     PlayerUpdateType::Despawned => { /* free per-player resources */ }
/// }
/// # }
/// ```
#[derive(Clone, Debug, Event)]
pub struct PlayerUpdateEvent {
    /// The player's ID
    pub id: PlayerId,
    /// The world the player currently is in
    pub world: WorldId,
    /// The entity that the player possesses in the world
    pub entity: Entity,
    /// The window the player's viewport is being drawn to
    pub window: WindowId,
    /// The region of the window that the player's viewport is being draw to
    pub region: PlayerRegion,
    /// The kind of update that triggered this event. See [`PlayerUpdateType`]
    pub update_type: PlayerUpdateType,
}

/// The reason a [`PlayerUpdateEvent`] was fired.
///
/// Variants are ordered chronologically: a player is first `Spawned`, may be
/// `Updated` zero or more times, and is finally `Despawned`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerUpdateType {
    /// The player was just created and its entity inserted into the world.
    Spawned,
    /// Some player state changed (currently: the viewport region).
    Updated,
    /// The player's entity was removed from the world.
    Despawned,
}

impl PlayerUpdateEvent {
    /// Constructs a [`PlayerUpdateEvent`] by snapshotting the relevant fields
    /// from `player`.
    ///
    /// This is the canonical constructor; it is called internally by [`Player`]
    /// and is exposed so that test harnesses or mock dispatchers can create
    /// events without going through the full `Player` machinery.
    ///
    /// # Arguments
    ///
    /// * `player`      — the player whose state should be snapshotted.
    /// * `update_type` — the reason for the event.
    #[must_use]
    pub fn from_player(player: &Player, update_type: PlayerUpdateType) -> Self {
        Self {
            id: player.id(),
            world: player.world(),
            entity: player.entity(),
            window: player.window(),
            region: player.region().clone(),
            update_type,
        }
    }
}
