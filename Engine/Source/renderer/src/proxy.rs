//! This module holds proxies for various engine objects

use platform::WindowId;
use universe::{Entity, WorldId};
use universe::{
    query::Query,
    systems::{EntitySystem, System, UniverseSystem},
};
use world::{components, player::PlayerId};

use crate::render_commands::RenderCommandSender;

pub struct CameraProxy {
    /// View matrix calculated from camera position.
    pub view: glam::Mat4,
    /// Projection matrix calculated from camera settings.
    pub proj: glam::Mat4,
}

pub struct PlayerProxy {
    #[allow(unused)]
    pub id: PlayerId,
    pub world: WorldId,
    pub entity: Entity,
    pub window: WindowId,
    // TODO: render to a specific region of the window
    // pub region: PlayerRegion,
}

impl From<world::player::PlayerUpdateEvent> for PlayerProxy {
    fn from(event: world::player::PlayerUpdateEvent) -> Self {
        Self {
            id: event.id,
            world: event.world,
            entity: event.entity,
            window: event.window,
        }
    }
}

#[derive(System)]
pub struct RendererEntitySynchronizationSystem {
    sender: RenderCommandSender,
}

impl RendererEntitySynchronizationSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

// ECS SYSTEMS FOR SYNCHRONIZATION

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
