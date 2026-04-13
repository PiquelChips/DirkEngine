use events::Event;
use macros::Event;

use crate::{Entity, WorldId};

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
