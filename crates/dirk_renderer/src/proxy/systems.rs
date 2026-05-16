//! ECS systems for proxy creation and synchrnozation

use dirk_universe::{
    CommandBuffer,
    systems::{ComponentSystem, System, UniverseSystem},
};
use dirk_world::components::{Camera, Renderable, Transform};

use crate::{Error, render_commands::RenderCommandSender};

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
    fn tick(&self, _: &mut CommandBuffer, _: &dirk_universe::Universe, _: f64) {}

    fn entity_spawned(
        &self,
        _: &mut CommandBuffer,
        universe: &dirk_universe::Universe,
        entity: dirk_universe::Entity,
    ) {
        let world = universe
            .get_world(entity)
            .expect("entity should be in world");

        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.create_proxy(entity, world)?;
            Ok(())
        });
    }
    fn entity_sent(
        &self,
        _: &mut CommandBuffer,
        _: &dirk_universe::Universe,
        entity: dirk_universe::Entity,
        _: dirk_universe::WorldId,
        new: dirk_universe::WorldId,
    ) {
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.send_proxy(entity, new)?;
            Ok(())
        });
    }

    fn entity_despawned(
        &self,
        _: &mut CommandBuffer,
        _: &dirk_universe::Universe,
        entity: dirk_universe::Entity,
    ) {
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.destroy_proxy(entity)?;
            Ok(())
        });
    }

    fn world_created(
        &self,
        _: &mut CommandBuffer,
        _: &dirk_universe::Universe,
        world: &dirk_universe::World,
    ) {
        let world = world.id();
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.create_scene(world)?;
            Ok(())
        });
    }
    fn world_destroyed(
        &self,
        _: &mut CommandBuffer,
        _: &dirk_universe::Universe,
        world: &dirk_universe::World,
    ) {
        let world = world.id();
        self.sender.enqueue_command(move |renderer| {
            let manager = &mut renderer.scene_manager;
            manager.destroy_scene(world);
            Ok(())
        });
    }
}

#[derive(System)]
pub struct RendererMeshSystem {
    sender: RenderCommandSender,
}

impl ComponentSystem for RendererMeshSystem {
    type Component = Renderable;

    fn added(
        &self,
        _: &mut CommandBuffer,
        entity: dirk_universe::Entity,
        component: &Self::Component,
    ) {
        let model = component.model.clone();
        self.sender.enqueue_command(move |renderer| {
            let proxy = renderer
                .scene_manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;
            proxy.set_model(Some(model));
            Ok(())
        });
    }

    fn updated(
        &self,
        cmd: &mut CommandBuffer,
        entity: dirk_universe::Entity,
        _: &Self::Component,
        new: &Self::Component,
    ) {
        self.added(cmd, entity, new);
    }

    fn removed(&self, _: &mut CommandBuffer, entity: dirk_universe::Entity, _: &Self::Component) {
        self.sender.enqueue_command(move |renderer| {
            let proxy = renderer
                .scene_manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;
            proxy.set_model(None);
            Ok(())
        });
    }
}

impl RendererMeshSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

#[derive(System)]
pub struct RendererTransformSystem {
    sender: RenderCommandSender,
}

impl ComponentSystem for RendererTransformSystem {
    type Component = Transform;

    fn added(
        &self,
        _: &mut CommandBuffer,
        entity: dirk_universe::Entity,
        component: &Self::Component,
    ) {
        let model = component.matrix();
        let view = component.view();
        self.sender.enqueue_command(move |renderer| {
            let proxy = renderer
                .scene_manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;
            proxy.set_model_matrix(Some(model));
            proxy.set_view(Some(view));
            Ok(())
        });
    }

    fn updated(
        &self,
        cmd: &mut CommandBuffer,
        entity: dirk_universe::Entity,
        _: &Self::Component,
        new: &Self::Component,
    ) {
        self.added(cmd, entity, new);
    }

    fn removed(&self, _: &mut CommandBuffer, entity: dirk_universe::Entity, _: &Self::Component) {
        self.sender.enqueue_command(move |renderer| {
            let proxy = renderer
                .scene_manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;
            proxy.set_model_matrix(None);
            proxy.set_view(None);
            Ok(())
        });
    }
}

impl RendererTransformSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}

#[derive(System)]
pub struct RendererCameraSystem {
    sender: RenderCommandSender,
}

impl ComponentSystem for RendererCameraSystem {
    type Component = Camera;

    fn added(
        &self,
        _: &mut CommandBuffer,
        entity: dirk_universe::Entity,
        component: &Self::Component,
    ) {
        let proj = component.projection();
        self.sender.enqueue_command(move |renderer| {
            let proxy = renderer
                .scene_manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;
            proxy.set_proj(Some(proj));
            Ok(())
        });
    }

    fn updated(
        &self,
        cmd: &mut CommandBuffer,
        entity: dirk_universe::Entity,
        _: &Self::Component,
        new: &Self::Component,
    ) {
        self.added(cmd, entity, new);
    }

    fn removed(&self, _: &mut CommandBuffer, entity: dirk_universe::Entity, _: &Self::Component) {
        self.sender.enqueue_command(move |renderer| {
            let proxy = renderer
                .scene_manager
                .get_proxy_mut(entity)
                .ok_or(Error::EntityDoesNotExist(entity))?;
            proxy.set_proj(None);
            Ok(())
        });
    }
}

impl RendererCameraSystem {
    pub fn new(sender: RenderCommandSender) -> Self {
        Self { sender }
    }
}
