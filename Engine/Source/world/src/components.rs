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

impl Transform {
    pub fn matrix(&self) -> glam::Mat4 {
        let translation = Mat4::from_translation(self.location);
        let scale = Mat4::from_scale(self.scale);
        let rot_x = Mat4::from_rotation_y(self.rotation.x.to_radians());
        let rot_y = Mat4::from_rotation_x(self.rotation.y.to_radians());
        let rot_z = Mat4::from_rotation_z(self.rotation.z.to_radians());

        translation * scale * rot_x * rot_y * rot_z
    }
    pub fn rotation_quat(&self) -> glam::Quat {
        glam::Quat::from_euler(
            glam::EulerRot::YXZ,
            self.rotation.x.to_radians(),
            self.rotation.y.to_radians(),
            self.rotation.z.to_radians(),
        )
    }
    pub fn forward(&self) -> glam::Vec3 {
        self.rotation_quat() * utils::FORWARD_DIRECTION
    }
    /// Gets the view matrix for this transform's position & look-at from rotation.
    pub fn view(&self) -> glam::Mat4 {
        glam::Mat4::look_at_lh(
            self.location,
            self.location + self.forward(),
            utils::UP_DIRECTION,
        )
    }
}

impl From<Transform> for glam::Mat4 {
    fn from(transform: Transform) -> Self {
        transform.matrix()
    }
}

pub struct Camera {
    /// In radians
    pub fov: f32,
    pub near_clip: f32,
    pub far_clip: f32,
    pub width: f32,
    pub height: f32,
}

impl Camera {
    pub fn projection(&self) -> glam::Mat4 {
        let mut proj = glam::Mat4::perspective_rh(
            self.fov,
            self.width / self.height,
            self.near_clip,
            self.far_clip,
        );
        proj.y_axis.y *= -1.;
        proj
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: f32::to_radians(45.),
            near_clip: 1.,
            far_clip: 100.,
            width: 100.,
            height: 100.,
        }
    }
}
