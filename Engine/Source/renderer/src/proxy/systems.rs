//! ECS systems for proxy creation and synchrnozation

use universe::{
    query::Query,
    systems::{EntitySystem, System, UniverseSystem},
};
use world::components;

use crate::render_commands::RenderCommandSender;

#[derive(System)]
pub struct RendererEntitySynchronizationSystem {
    sender: RenderCommandSender,
}

impl RendererEntitySynchronizationSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

impl EntitySystem for RendererEntitySynchronizationSystem {
    fn spawned(&self, universe: &universe::Universe, entity: universe::Entity) {
        self.sender.enqueue_command(|renderer| {
            // TODO: create proxy
            todo!("create proxy")
        });
    }
    fn sent(
        &self,
        universe: &universe::Universe,
        entity: universe::Entity,
        old: universe::WorldId,
        new: universe::WorldId,
    ) {
        self.sender.enqueue_command(|renderer| {
            // TODO: move the entity from a Scene to another
            todo!("move the proxy")
        });
    }
    fn despawned(&self, universe: &universe::Universe, entity: universe::Entity) {
        self.sender.enqueue_command(|renderer| {
            // TODO: delete the proxy & entity references
            todo!("remove the proxy")
        });
    }
    fn query(&self) -> Option<Query> {
        Some(
            Query::new()
                .with_component::<components::Renderable>()
                .with_component::<components::Transform>(),
        )
    }
}

#[derive(System)]
pub struct RendererUniverseSynchronizationSystem {
    sender: RenderCommandSender,
}

impl RendererUniverseSynchronizationSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

impl UniverseSystem for RendererUniverseSynchronizationSystem {
    // these functions aren't needed
    fn tick(&self, _: &universe::Universe, _: f32) {}
    fn entity_spawned(&self, _: &universe::Universe, _: universe::Entity) {}
    fn entity_despawned(&self, _: &universe::Universe, _: universe::Entity) {}

    fn world_created(&self, universe: &universe::Universe, world: &universe::World) {
        self.sender.enqueue_command(|renderer| {
            // TODO: create new scene proxy
            todo!("create scene proxy")
        });
    }
    fn world_destroyed(&self, universe: &universe::Universe, world: &universe::World) {
        self.sender.enqueue_command(|renderer| {
            // TODO: delete the scene proxy
            todo!("delete scene proxy")
        });
    }
}
