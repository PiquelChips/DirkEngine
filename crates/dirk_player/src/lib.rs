#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign},
};

use dirk_events::{Dispatcher, EventManager};
use dirk_platform::WindowId;
use dirk_universe::components::Component;

mod events;
pub use events::{PlayerDespawned, PlayerSpawned};

// PlayerId

/// A lightweight, copyable identifier for a player.
///
/// Implements [`Component`] so that ECS entities can declare ownership by
/// a player. This is the canonical link between [`PlayerManager`] and the
/// ECS: the manager never stores entity references; instead, game code attaches
/// a `PlayerId` component when spawning the player's entity.
///
/// # ECS Relationship
///
/// ```text
/// PlayerManager                Universe
///  └─ PlayerHandle(id=1) ◄──── Entity { PlayerId(1), Transform, Camera, ... }
/// ```
///
/// To find a player's entity, query for [`PlayerId`] in the universe and match
/// the value.  To find which player owns an entity, read its `PlayerId` component.
#[derive(Component, Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct PlayerId(u32);

impl Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<u32> for PlayerId {
    type Output = Self;
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u32> for PlayerId {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

// PlayerHandle

/// The engine's representation of a connected player.
///
/// A `PlayerHandle` is intentionally a pure data container: it has no entity,
/// no world reference, and no knowledge of ECS state. The link between a player
/// and their in-world entity is established by the game in response to
/// [`PlayerSpawned`].
///
/// Access handles via [`PlayerManager::get_player`] or
/// [`PlayerManager::get_player_mut`].
pub struct PlayerHandle {
    id: PlayerId,
    window: WindowId,
}

impl PlayerHandle {
    /// Returns this player's [`PlayerId`].
    #[must_use]
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Returns the [`WindowId`] this player is associated with.
    ///
    /// [`Window`]: dirk_platform::Window
    #[must_use]
    pub fn window(&self) -> WindowId {
        self.window
    }
}

// PlayerManager

/// Manages the full lifecycle of all players.
///
/// `PlayerManager` is the single authority on which players exist.  It does not
/// interact with the ECS directly — all coupling goes through events.
///
/// # Lifecycle
///
/// ```text
/// new_player(window)   ──► PlayerSpawned      — game code spawns ECS entity
/// remove_player(id)    ──► PlayerDespawned    — game code despawns ECS entity
/// ```
///
/// This crate currently tracks only player IDs and their associated windows.
/// Input routing, viewport management, and camera updates are handled outside
/// of `dirk_player`.
pub struct PlayerManager {
    // TODO: setup generation based player allocation
    next_id: PlayerId,
    players: HashMap<PlayerId, PlayerHandle>,

    spawned_dispatcher: Dispatcher<PlayerSpawned>,
    despawned_dispatcher: Dispatcher<PlayerDespawned>,
}

impl PlayerManager {
    /// Creates a new [`PlayerManager`], registering its event channels with
    /// `events`.
    #[must_use]
    pub fn new(events: &EventManager) -> Self {
        Self {
            next_id: PlayerId::default(),
            players: HashMap::new(),
            spawned_dispatcher: events.register(),
            despawned_dispatcher: events.register(),
        }
    }

    /// Creates a new player assigned to `window` and fires [`PlayerSpawned`].
    ///
    /// The player is not placed in any world. Game code should respond to
    /// [`PlayerSpawned`] by spawning an ECS entity with a [`PlayerId`]
    /// component (and whatever other components are appropriate — `Transform`,
    /// `Camera`, etc.).
    ///
    /// # Returns
    ///
    /// The [`PlayerId`] of the new player.
    pub fn new_player(&mut self, window: WindowId) -> PlayerId {
        let id = self.allocate_id();
        self.players.insert(id, PlayerHandle { id, window });
        self.spawned_dispatcher
            .dispatch(PlayerSpawned { id, window });
        id
    }

    /// Removes the player with `id` and fires [`PlayerDespawned`].
    ///
    /// If the player does not exist this is a no-op.
    ///
    /// The game code is responsible for despawning the associated ECS entity
    /// in response to [`PlayerDespawned`].
    pub fn remove_player(&mut self, id: PlayerId) {
        if self.players.remove(&id).is_some() {
            self.despawned_dispatcher.dispatch(PlayerDespawned { id });
        }
    }

    /// Returns a reference to the player with `id`, or `None`.
    #[must_use]
    pub fn get_player(&self, id: PlayerId) -> Option<&PlayerHandle> {
        self.players.get(&id)
    }

    /// Returns a mutable reference to the player with `id`, or `None`.
    #[must_use]
    pub fn get_player_mut(&mut self, id: PlayerId) -> Option<&mut PlayerHandle> {
        self.players.get_mut(&id)
    }

    /// Returns an iterator over all live players.
    pub fn players(&self) -> impl Iterator<Item = &PlayerHandle> {
        self.players.values()
    }

    /// Ticks internal player state.
    ///
    /// This is currently a no-op placeholder for future player systems such as
    /// input handling.
    pub fn tick(&mut self) {
        // TODO: input
    }

    fn allocate_id(&mut self) -> PlayerId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}
