use ash::vk;

/// This scene is created from a [world::World].
/// It should then be updated whenever the world is updated.
///
/// Handles rendering all the [world::components::Renderable] objects
/// of the world.
pub struct Scene {
    /// Each scene has its own command pool.
    command_pool: vk::CommandPool,
    /// View matrix calculated from camera position.
    view: glam::Vec4,
    /// Projection matrix calculated from screen settings.
    proj: glam::Vec4,
}

/// A renderable entity's representation for the renderer.
/// Owned by [Scene], constructed from [world::components::Renderable] and
/// [world::components::Transform].
pub struct SceneProxy {
    /// The name of the model. Used to request a [crate::model::Model] from the
    /// renderer at render time.
    model: String,
    /// The model matrix used for rendering. Constructed from the
    /// [world::components::Transform] of the entity.
    model_matrix: glam::Vec4,
}
