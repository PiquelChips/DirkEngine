use events::Event;
use macros::Event;
use platform::WindowId;

use crate::{
    Entity, WorldId,
    player::{Player, PlayerId, PlayerRegion},
};

#[derive(Debug, Clone, Event)]
pub enum WorldEvent {
    #[event("World created with id {0}")]
    Created(WorldId),
    #[event("World destroyed with id {0}")]
    Destroyed(WorldId),
    #[event("Entity spawned in world {world} with id {entity}")]
    EntitySpawn { world: WorldId, entity: Entity },
    /// Entity update is fired when a component is added, removed or
    /// gotten as mutable.
    #[event("Entity updated in world {world} with id {entity}")]
    EntityUpdate { world: WorldId, entity: Entity },
    #[event("Entity despawned in world {world} with id {entity}")]
    EntityDespawn { world: WorldId, entity: Entity },
}

impl WorldEvent {
    pub fn world(&self) -> WorldId {
        match &self {
            Self::Created(id) => *id,
            Self::Destroyed(id) => *id,
            Self::EntitySpawn { world, .. } => *world,
            Self::EntityUpdate { world, .. } => *world,
            Self::EntityDespawn { world, .. } => *world,
        }
    }
}

/// Events emitted by the player system.
///
/// These describe changes to the *player handle* itself (spawn, despawn,
/// per-tick state change). Component-level mutations (Transform, Camera) are
/// separately covered by [`world::events::WorldEvent`].
///
/// TODO: refactor player events: only care about change in world, entity,
/// window, region, ...
#[derive(Debug, Clone, Event)]
pub struct PlayerUpdateEvent {
    id: PlayerId,
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: PlayerRegion,
    update_type: PlayerUpdateType,
}

#[derive(Clone, Debug)]
pub enum PlayerUpdateType {
    Spawned,
    Updated,
    Despawned,
}

impl PlayerUpdateEvent {
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
