#![doc = include_str!("../README.md")]

use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Add, AddAssign},
    sync::Arc,
};

use dirk_engine::{EngineBuilder, EngineHandle, EnginePlugin, Subsystem};
use dirk_events::{Dispatcher, EventManager};
use dirk_platform::{InputEvent, WindowId};
use dirk_universe::{UniverseBuilder, components::Component};
use input::InputContext;
use movement::PlayerMovementSystem;
use parking_lot::RwLock;

mod events;
pub mod input;
mod movement;
pub use events::{PlayerDespawned, PlayerSpawned};
pub use movement::{DEFAULT_PLAYER_LOOK_SENSITIVITY, DEFAULT_PLAYER_MOVE_SPEED};

/// Registers player management as an engine subsystem.
pub struct PlayerPlugin;

impl EnginePlugin for PlayerPlugin {
    fn name(&self) -> &'static str {
        "player"
    }

    fn build(&self, builder: &mut EngineBuilder) -> anyhow::Result<()> {
        builder.add_subsystem(|ctx| {
            let players = PlayerManager::new(ctx.events());
            ctx.extend_universe(players.universe_builder());
            Ok(players)
        });
        Ok(())
    }
}

impl Subsystem for PlayerManager {
    fn name(&self) -> &'static str {
        "player"
    }

    fn tick(
        &mut self,
        _delta_time: f64,
        _handle: &EngineHandle,
        _universe: &mut dirk_universe::Universe,
    ) -> anyhow::Result<()> {
        self.tick();
        Ok(())
    }
}

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
    input: InputContext,
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

    /// Returns the current hard-coded movement input for this player.
    ///
    /// The vector is in local camera movement space: `x` is right/left, `y` is
    /// up/down, and `z` is forward/back.
    #[must_use]
    pub fn movement_input(&self) -> glam::Vec3 {
        self.input.movement_input()
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
// TODO: make private when removing engine
pub struct PlayerManager {
    // TODO: setup generation based player allocation
    next_id: PlayerId,
    players: HashMap<PlayerId, PlayerHandle>,
    input_state: PlayerInputState,
    input_consumer: dirk_events::Consumer<InputEvent>,

    spawned_dispatcher: Dispatcher<PlayerSpawned>,
    despawned_dispatcher: Dispatcher<PlayerDespawned>,
}

impl PlayerManager {
    /// Creates a new [`PlayerManager`], registering its event channels with
    /// `events`.
    #[must_use]
    // TODO: make private after engine update
    pub fn new(events: &EventManager) -> Self {
        Self {
            next_id: PlayerId::default(),
            players: HashMap::new(),
            input_state: PlayerInputState::default(),
            input_consumer: events.subscribe(),
            spawned_dispatcher: events.register(),
            despawned_dispatcher: events.register(),
        }
    }

    /// Returns a [`UniverseBuilder`] with player-related ECS systems.
    #[must_use]
    // TODO: remove function after engine update
    pub fn universe_builder(&self) -> UniverseBuilder {
        dirk_universe::Universe::builder()
            .with_ticking_system(PlayerMovementSystem::new(self.input_state.clone()))
    }

    /// Returns a shared read handle for systems that consume player input.
    #[must_use]
    pub fn input_state(&self) -> PlayerInputState {
        self.input_state.clone()
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
        self.players.insert(
            id,
            PlayerHandle {
                id,
                window,
                input: InputContext::new(),
            },
        );
        self.input_state.set(
            id,
            PlayerInputFrame {
                movement: glam::Vec3::ZERO,
                look: glam::DVec2::ZERO,
            },
        );
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
            self.input_state.remove_player(id);
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
    // TODO: merge with Subsystem::tick
    pub fn tick(&mut self) {
        let events = self.input_consumer.consume_all().collect::<Vec<_>>();
        for event in events {
            self.players
                .values_mut()
                .filter(|p| p.window == *event.id())
                .for_each(|p| p.input.handle_event(&event));
        }
        for player in self.players.values() {
            self.input_state.set(
                player.id,
                PlayerInputFrame {
                    movement: player.movement_input(),
                    look: player.input.look_input(),
                },
            );
        }
        for player in self.players.values_mut() {
            player.input.clear_frame_state();
        }
    }

    fn allocate_id(&mut self) -> PlayerId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

/// Shared per-player input values consumed by universe systems.
#[derive(Clone, Default)]
pub struct PlayerInputState {
    frames: Arc<RwLock<HashMap<PlayerId, PlayerInputFrame>>>,
}

/// Per-frame input values for one player.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInputFrame {
    /// Movement intent in local player space.
    pub movement: glam::Vec3,
    /// Pointer-look delta in physical pixels.
    pub look: glam::DVec2,
}

impl PlayerInputState {
    pub(crate) fn get(&self, player: PlayerId) -> PlayerInputFrame {
        self.frames.read().get(&player).copied().unwrap_or_default()
    }

    fn set(&self, player: PlayerId, frame: PlayerInputFrame) {
        self.frames.write().insert(player, frame);
    }

    fn remove_player(&self, player: PlayerId) {
        self.frames.write().remove(&player);
    }
}
