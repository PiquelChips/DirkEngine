use events::Event;
use macros::Event;

pub type PlayerId = u32;

/// Events emitted by the player system.
///
/// These describe changes to the *player handle* itself (spawn, despawn,
/// per-tick state change). Component-level mutations (Transform, Camera) are
/// separately covered by [`world::events::WorldEvent`].
///
/// TODO: refactor player events: only care about change in world, entity,
/// window, region, ...
#[derive(Debug, Clone, Event)]
pub enum PlayerEvent {
    #[event("Player {0} spawned")]
    Spawned(PlayerId),
    #[event("Player {0} despawned")]
    Despawned(PlayerId),
    /// Fired at the end of a tick in which the player's transform actually
    /// changed (movement applied or camera rotated). Not fired on idle ticks.
    #[event("Player {0} updated")]
    Updated(PlayerId),
}
