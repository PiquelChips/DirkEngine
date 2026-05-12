//! ECS systems for proxy creation and synchrnozation

use universe::{
    query::Query,
    systems::{ComponentSystem, EntitySystem, System, UniverseSystem},
};
use world::components::{self, Camera, Renderable, Transform};

use crate::{Error, render_commands::RenderCommandSender};

#[derive(System)]
pub struct RendererEntitySystem {
    sender: RenderCommandSender,
}

impl RendererEntitySystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

impl EntitySystem for RendererEntitySystem {
    fn spawned(&self, universe: &universe::Universe, entity: universe::Entity) {
        let world = universe
            .get_world(entity)
            .expect("entity should be in world");

        let renderable = universe
            .component::<Renderable>(entity)
            .cloned()
            .expect("queried for entity with renderable");
        let transform = universe
            .component::<Transform>(entity)
            .cloned()
            .expect("queried for entity with transform");
        let camera = universe.component::<Camera>(entity).cloned();
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.create_proxy(entity, world)?;

            let proxy = manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;

            proxy.set_model(renderable.model);
            proxy.set_model_matrix(transform.matrix());
            proxy.set_view(transform.view());

            if let Some(camera) = camera {
                proxy.set_proj(camera.projection());
            }

            Ok(())
        });
    }
    fn sent(
        &self,
        _: &universe::Universe,
        entity: universe::Entity,
        _: universe::WorldId,
        new: universe::WorldId,
    ) {
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.send_proxy(entity, new)?;
            Ok(())
        });
    }
    fn despawned(&self, _: &universe::Universe, entity: universe::Entity) {
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.destroy_proxy(entity)?;
            Ok(())
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
pub struct RendererUniverseSystem {
    sender: RenderCommandSender,
}

impl RendererUniverseSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

impl UniverseSystem for RendererUniverseSystem {
    // these functions aren't needed
    fn tick(&self, _: &universe::Universe, _: f32) {}
    fn entity_spawned(&self, _: &universe::Universe, _: universe::Entity) {}
    fn entity_despawned(&self, _: &universe::Universe, _: universe::Entity) {}

    fn world_created(&self, _: &universe::Universe, world: &universe::World) {
        let world = world.id();
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.create_scene(world)?;
            Ok(())
        });
    }
    fn world_destroyed(&self, _: &universe::Universe, world: &universe::World) {
        let world = world.id();
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.destroy_scene(world);
            Ok(())
        });
    }
}
