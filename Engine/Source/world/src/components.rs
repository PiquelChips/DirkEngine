//! This module has a bunch of frequently used and central [`Component`]s
//!
//! [`Component`]: universe::components::Component

use assets::{Handle, LoadAsset, Model};
use events::Dispatcher;
use glam::{Mat4, Vec3};
use tracing::warn;
use universe::{
    CommandBuffer, Entity,
    components::Component,
    systems::{ComponentSystem, System},
};

/// Marks an entity as having a renderable mesh.
///
/// The `model` field is resolved at render time against the engine's asset
/// registry. If no matching asset is found the entity is silently skipped.
///
/// # Examples
/// ```
/// # use world::components::Renderable;
/// use assets::{AssetHandle, AssetType};
/// let r = Renderable { model: AssetHandle::from_raw("meshes/cube.glb", AssetType::Model) };
/// assert_eq!(r.model.raw(), "meshes/cube.glb");
/// ```
#[derive(Debug, Clone, Component)]
pub struct Renderable {
    /// Asset-registry key for the mesh to render (e.g. `"meshes/cube.glb"`).
    pub model: assets::AssetHandle,
    handle: Option<Handle<Model>>,
}

impl Renderable {
    /// Creates a new [`Renderable`] component from an [`AssetHandle`].
    ///
    /// [`AssetHandle`]: assets::AssetHandle
    #[must_use]
    pub fn new(model: assets::AssetHandle) -> Self {
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
    pub fn new(event_manager: &events::EventManager) -> Self {
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
/// # use world::components::Transform;
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
        self.rotation_quat() * utils::FORWARD_DIRECTION
    }

    /// Builds a **left-handed** view matrix for a camera placed at this
    /// transform, looking in the [`forward`](Self::forward) direction.
    pub fn view(&self) -> Mat4 {
        let forward = self.forward();
        if forward.cross(utils::UP_DIRECTION).length() < 1e-4 {
            warn!(
                "camera forward {:?} is parallel to UP — view matrix will be NaN",
                forward
            );
        }
        Mat4::look_at_lh(self.location, self.location + forward, utils::UP_DIRECTION)
    }
}

impl From<Transform> for Mat4 {
    /// Converts the transform to its model matrix via [`Transform::matrix`].
    fn from(transform: Transform) -> Self {
        transform.matrix()
    }
}

/// Perspective camera parameters attached to an entity.
///
/// Combine with a [`Transform`] component to obtain a complete view-projection
/// matrix. The projection uses a **right-handed** coordinate system with the
/// Y axis flipped to match Vulkan / wgpu NDC conventions (Y points downward
/// in clip space).
///
/// # Examples
/// ```
/// # use world::components::Camera;
/// let cam = Camera::default();
/// let proj = cam.projection();
/// // The matrix must be finite.
/// assert!(proj.to_cols_array().iter().all(|v| v.is_finite()));
/// ```
#[derive(Debug, Clone, Component)]
pub struct Camera {
    /// Vertical field of view **in radians**.
    pub fov: f32,
    /// Near clip plane distance (must be > 0).
    pub near_clip: f32,
    /// Far clip plane distance (must be > `near_clip`).
    pub far_clip: f32,
    /// Viewport width in pixels, used to compute the aspect ratio.
    pub width: f32,
    /// Viewport height in pixels, used to compute the aspect ratio.
    pub height: f32,
}

impl Camera {
    /// Computes a perspective projection matrix.
    ///
    /// The Y axis of the resulting matrix is negated so that clip-space Y
    /// increases *downward*, matching Vulkan / wgpu NDC conventions.
    #[must_use]
    pub fn projection(&self) -> Mat4 {
        let mut proj = Mat4::perspective_rh(
            self.fov,
            self.width / self.height,
            self.near_clip,
            self.far_clip,
        );
        // Vulkan NDC has Y pointing down; flip the projection accordingly.
        proj.y_axis.y *= -1.0;
        proj
    }

    /// Returns the aspect ratio (`width / height`).
    #[must_use]
    pub fn aspect_ratio(&self) -> f32 {
        self.width / self.height
    }
}

impl Default for Camera {
    /// Returns a camera with a 45° `FoV`, near = 1, far = 100, and a square
    /// 100 × 100 viewport.
    fn default() -> Self {
        Self {
            fov: 45_f32.to_radians(),
            near_clip: 1.0,
            far_clip: 100.0,
            width: 100.0,
            height: 100.0,
        }
    }
}
