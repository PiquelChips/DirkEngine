use glam::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct Renderable {
    /// The name of the model to be rendered for this entity
    pub model: String,
}

#[derive(Debug, Clone)]
pub struct Transform {
    pub location: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl From<Transform> for glam::Mat4 {
    fn from(transform: Transform) -> Self {
        let translation = Mat4::from_translation(transform.location);
        let scale = Mat4::from_scale(transform.scale);
        let rot_x = Mat4::from_rotation_y(transform.rotation.x.to_radians());
        let rot_y = Mat4::from_rotation_x(transform.rotation.y.to_radians());
        let rot_z = Mat4::from_rotation_z(transform.rotation.z.to_radians());

        translation * scale * rot_x * rot_y * rot_z
    }
}
