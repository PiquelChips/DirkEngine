use std::any::TypeId;

use crate::{
    Entity, EntityBuilder, WorldBuilder, WorldId,
    components::{AnyComponent, Component},
};

/// A buffer to record edits to the [`Universe`].
pub struct CommandBuffer {
    commands: Vec<Command>,
}

pub(crate) enum Command {
    CreateWorld(WorldBuilder),
    DestroyWorld(WorldId),
    Spawn(WorldId, EntityBuilder),
    Despawn(Entity),
    Send(Entity, WorldId),
    SetComponent(Entity, Box<dyn AnyComponent>),
    RemoveComponent(Entity, TypeId),
}

impl CommandBuffer {
    pub(crate) fn new() -> Self {
        CommandBuffer {
            commands: Vec::new(),
        }
    }
    pub(crate) fn commands(self) -> Vec<Command> {
        self.commands
    }
    /// Returns if the command buffer has had no submitted commands. This is useful to skip
    /// all submission logic when no commands should be run.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    // WORLD MANAGEMENT

    /// Will create a new empty world & return its ID.
    pub fn create_world(&mut self, builder: WorldBuilder) {
        self.commands.push(Command::CreateWorld(builder));
    }
    /// Will destroy the world & call all its destruction systems.
    pub fn destroy_world(&mut self, world: WorldId) {
        self.commands.push(Command::DestroyWorld(world));
    }

    // ENTITY MANAGEMENT

    /// Will spawn a new [`Entity`] using the provided [`EntityBuilder`].
    /// Returns the handle of the new [`Entity`].
    ///
    /// If None, then the [`World`] does not exist.
    pub fn spawn(&mut self, world: WorldId, builder: EntityBuilder) {
        self.commands.push(Command::Spawn(world, builder));
    }
    /// Will despawn the provided [`Entity`].
    ///
    /// Calls [`ComponentSystem::removed`] for every component still attached
    /// to the entity before the components are actually dropped.
    pub fn despawn(&mut self, entity: Entity) {
        self.commands.push(Command::Despawn(entity));
    }
    /// Will send the [`Entity`] to the specified [`WorldId`].
    pub fn send(&mut self, entity: Entity, to: WorldId) {
        self.commands.push(Command::Send(entity, to));
    }

    // COMPONENT MANAGEMENT

    /// Attaches a [`Component`] to [`Entity`], replacing any existing component of
    /// the same type.
    ///
    /// [`ComponentSystem::added`] is called every time.
    ///
    /// When replacing, [`ComponentSystem::removed`] is called.
    ///
    /// [`Entity`]: crate::Entity
    pub fn set_component<C: Component>(&mut self, entity: Entity, component: C) {
        self.commands
            .push(Command::SetComponent(entity, Box::new(component)));
    }
    /// Removes a single component from an entity, calling [`ComponentSystem::removed`]
    /// if the component was present.
    ///
    /// The entity itself is **not** despawned. If the component is not
    /// present this is a no-op.
    pub fn remove_component<C: Component>(&mut self, entity: Entity) {
        self.commands
            .push(Command::RemoveComponent(entity, TypeId::of::<C>()));
    }
}
