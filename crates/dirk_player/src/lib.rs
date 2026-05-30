#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign},
};

use dirk_universe::components::Component;

pub mod events;

pub mod viewport;
use viewport::Viewport;

// PLAYER ID

/// A light identifier for [`PlayerHandle`]s.
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

// PLAYER HANDLE

/// This is the internal engine representation of a player.
///
/// It receives input, owns its viewport of the screen, ...
/// [`PlayerId`] is used as a handle.
pub struct PlayerHandle {
    id: PlayerId,
}

impl PlayerHandle {
    /// Returns the [`PlayerId`] of `self`.
    #[must_use]
    pub fn id(&self) -> PlayerId {
        self.id
    }
}

// PLAYER MANAGER

/// This manages all the players in the game.
#[derive(Default)]
pub struct PlayerManager {
    // TODO: setup generation based player allocation
    next_player_id: PlayerId,
    players: HashMap<PlayerId, PlayerHandle>,
    viewports: HashMap<PlayerId, Viewport>,
}

impl PlayerManager {
    /// Create a new empty [`PlayerManager`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Create a new player.
    ///
    /// This does not spawn the player in the world.
    pub fn new_player(&mut self) -> PlayerId {
        let id = self.allocate_new_player();
        self.players.insert(id, PlayerHandle { id });
        id
    }

    /// Returns a reference to the player with the specified ID
    ///
    /// Returns `None` if the player does note exist.
    #[must_use]
    pub fn get_player(&self, id: PlayerId) -> Option<&PlayerHandle> {
        self.players.get(&id)
    }

    #[must_use]
    fn allocate_new_player(&mut self) -> PlayerId {
        let id = self.next_player_id;
        self.next_player_id += 1;
        id
    }
}
