//! This module has a bunch of frequently used and central [`Component`]s
//!
//! [`Component`]: universe::components::Component

use dirk_assets::{Handle, LoadAsset, Model};
use dirk_events::Dispatcher;
use dirk_universe::{
    CommandBuffer, Entity,
    components::Component,
    systems::{ComponentSystem, System},
};
use glam::{Mat4, Vec3};
use tracing::warn;

/// Marks an entity as having a renderable mesh.
///
/// The `model` field is resolved at render time against the engine's asset
/// registry. If no matching asset is found the entity is silently skipped.
///
/// # Examples
/// ```
/// # use dirk_world::components::Renderable;
/// use dirk_assets::{AssetHandle, AssetType};
/// let r = Renderable::new(AssetHandle::from_raw("meshes/cube.glb", AssetType::Model));
/// assert_eq!(r.model.raw(), "meshes/cube.glb");
/// ```
#[derive(Debug, Clone, Component)]
pub struct Renderable {
    /// Asset-registry key for the mesh to render (e.g. `"meshes/cube.glb"`).
    pub model: dirk_assets::AssetHandle,
    handle: Option<Handle<Model>>,
}

impl Renderable {
    /// Creates a new [`Renderable`] component from an [`AssetHandle`].
    ///
    /// [`AssetHandle`]: assets::AssetHandle
    #[must_use]
    pub fn new(model: dirk_assets::AssetHandle) -> Self {
        Self {
            model,
            handle: None,
        }
    }
}

/// A [`universe`] system that will automatically load a model
/// when a [`Renderable`] is added to an [`universe::Entity`].
#[derive(System)]
pub struct ModelUploadSystem {
    dispatcher: Dispatcher<LoadAsset>,
}

impl ModelUploadSystem {
    /// Creates a new [`ModelUploadSystem`] creating a [`Dispatcher`] with
    /// the provided [`EventManager`].
    ///
    /// [`EventManager`]: events::EventManager
    #[must_use]
    pub fn new(event_manager: &dirk_events::EventManager) -> Self {
        Self {
            dispatcher: event_manager.register(),
        }
    }
}

impl ComponentSystem for ModelUploadSystem {
    type Component = Renderable;
    fn added(&self, _cmd: &mut CommandBuffer, _: Entity, component: &Self::Component) {
        if component.handle.is_some() {
            return;
        }

        // TODO: load asset from registry & set handle
        self.dispatcher.dispatch(LoadAsset(component.model.clone()));
    }
    fn updated(
        &self,
        cmd: &mut CommandBuffer,
        entity: Entity,
        _: &Self::Component,
        new: &Self::Component,
    ) {
        self.added(cmd, entity, new);
    }
    /// Nothing happens when this component is removed. The asset will be unloaded
    /// automatically when it is no longer used.
    fn removed(&self, _: &mut CommandBuffer, _: Entity, _: &Self::Component) {}
}

/// Spatial transform for an entity: position, orientation, and scale.
///
/// Rotation is stored as **Euler angles in radians** using the **YXZ** convention
/// (yaw → pitch → roll), which matches a typical first-person camera setup.
///
/// # Examples
/// ```
/// # use dirk_world::components::Transform;
/// # use glam::Vec3;
/// let t = Transform {
///     location: Vec3::new(1.0, 0.0, 0.0),
///     rotation: Vec3::ZERO,
///     scale:    Vec3::ONE,
/// };
/// // The forward vector of an un-rotated transform points along the engine's
/// // canonical forward axis.
/// let fwd = t.forward();
/// assert!((fwd.length() - 1.0).abs() < 1e-5);
/// ```
#[derive(Debug, Clone, Component)]
pub struct Transform {
    /// World-space position.
    pub location: Vec3,
    /// Euler angles **in radians**, applied in YXZ order (yaw, pitch, roll).
    pub rotation: Vec3,
    /// Per-axis scale factor. `Vec3::ONE` is the identity scale.
    pub scale: Vec3,
}

impl Default for Transform {
    /// Returns the identity transform: origin, no rotation, unit scale.
    fn default() -> Self {
        Self {
            location: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    /// Builds the full model matrix (`T × S × R`) for this transform.
    ///
    /// Rotation is applied in **YXZ** order (yaw around Y, then pitch around X,
    /// then roll around Z).
    #[must_use]
    pub fn matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(self.location);
        let scale = Mat4::from_scale(self.scale);
        let rot_x = Mat4::from_rotation_x(self.rotation.x);
        let rot_y = Mat4::from_rotation_y(self.rotation.y);
        let rot_z = Mat4::from_rotation_z(self.rotation.z);

        translation * scale * rot_y * rot_x * rot_z
    }

    /// Returns the orientation as a unit quaternion (YXZ Euler decomposition).
    #[must_use]
    pub fn rotation_quat(&self) -> glam::Quat {
        glam::Quat::from_euler(
            glam::EulerRot::YXZ,
            self.rotation.y,
            self.rotation.x,
            self.rotation.z,
        )
    }

    /// Returns the unit vector pointing "forward" from this transform.
    ///
    /// The result is obtained by rotating the engine's canonical forward
    /// direction ([`utils::FORWARD_DIRECTION`]) by the current orientation.
    #[must_use]
    pub fn forward(&self) -> Vec3 {
        self.rotation_quat() * dirk_utils::FORWARD_DIRECTION
    }

    /// Builds a **left-handed** view matrix for a camera placed at this
    /// transform, looking in the [`forward`](Self::forward) direction.
    pub fn view(&self) -> Mat4 {
        let forward = self.forward();
        if forward.cross(dirk_utils::UP_DIRECTION).length() < 1e-4 {
            warn!(
                "camera forward {:?} is parallel to UP — view matrix will be NaN",
                forward
            );
        }
        Mat4::look_at_lh(
            self.location,
            self.location + forward,
            dirk_utils::UP_DIRECTION,
        )
    }
}

impl From<Transform> for Mat4 {
    /// Converts the transform to its model matrix via [`Transform::matrix`].
    fn from(transform: Transform) -> Self {
        transform.matrix()
    }
}
