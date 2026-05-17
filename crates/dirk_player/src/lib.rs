#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign},
};

use dirk_platform::WindowId;
use dirk_universe::components::Component;

pub mod region;
use region::PlayerRegion;

pub mod events;

/// A light identifier for [`DirkPlayer`]s.
///
/// [`PlayerId`] implements [`trait@Component`]. It is also used as the
/// [`Universe`] representation of the [`DirkPlayer`].
///
/// [`Universe`]: dirk_universe::Universe
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

/// This is the internal engine representation of a player.
///
/// It receives input, owns its region of the screen, ...
/// [`PlayerId`] is used as a handle.
pub struct DirkPlayer {
    id: PlayerId,
    window: WindowId,
    region: PlayerRegion,
}

impl DirkPlayer {
    /// Returns the [`PlayerId`] of `self`.
    #[must_use]
    pub fn id(&self) -> PlayerId {
        self.id
    }
    /// Returns the [`WindowId`] of the window the player is rendered to.
    #[must_use]
    pub fn window(&self) -> WindowId {
        self.window
    }
    /// Returns the [`PlayerId`] of `self`.
    #[must_use]
    pub fn region(&self) -> &PlayerRegion {
        &self.region
    }
}

/// This manages all the players in the game.
pub struct PlayerManager {
    // TODO: setup generation based player allocation
    next_player_id: PlayerId,
    players: HashMap<PlayerId, DirkPlayer>,
}

impl PlayerManager {
    /// Create a new empty [`PlayerManager`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_player_id: PlayerId(0),
            players: HashMap::new(),
        }
    }
    /// Create a new player.
    ///
    /// This does not spawn the player in the world.
    pub fn new_player(&mut self, window: WindowId, region: PlayerRegion) -> PlayerId {
        let id = self.allocate_new_player();
        self.players.insert(id, DirkPlayer { id, window, region });
        id
    }

    /// Returns a reference to the player with the specified ID
    ///
    /// Returns `None` if the player does note exist.
    #[must_use]
    pub fn get_player(&self, id: PlayerId) -> Option<&DirkPlayer> {
        self.players.get(&id)
    }

    #[must_use]
    fn allocate_new_player(&mut self) -> PlayerId {
        let id = self.next_player_id;
        self.next_player_id += 1;
        id
    }
}
