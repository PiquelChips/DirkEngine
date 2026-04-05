use std::collections::HashMap;

pub mod components;
use components::*;

/// An Entity is nothing more than a unique numeric identifier.
pub type Entity = u32;

#[derive(Default)]
struct Components {
    is_player: HashMap<Entity, IsPlayer>,
    is_dead: HashMap<Entity, IsDead>,
    renderables: HashMap<Entity, Renderable>,
}

impl Components {
    /// Remove every component belonging to an entity.
    fn remove_all(&mut self, entity: Entity) {
        self.is_player.remove(&entity);
        self.is_dead.remove(&entity);
        self.renderables.remove(&entity);
    }
}

#[derive(Default)]
pub struct World {
    next_id: Entity,
    alive: Vec<Entity>,
    components: Components,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new entity and return its ID.
    pub fn spawn(&mut self) -> Entity {
        let id = self.next_id;
        self.next_id += 1;
        self.alive.push(id);
        id
    }
    /// Despawn an entity - removes it and all its components.
    pub fn despawn(&mut self, entity: Entity) {
        self.alive.retain(|&e| e != entity);
        self.components.remove_all(entity);
    }
    /// Returns all the spawned actors
    pub fn alive(&self) -> &[Entity] {
        &self.alive
    }
}
