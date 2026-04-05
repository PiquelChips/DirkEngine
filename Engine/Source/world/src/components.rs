/// Holds the visual description of an entity.
#[derive(Debug, Clone)]
// TODO: actually renderable
pub struct Renderable;

#[derive(Debug, Clone)]
pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Vec3,
    pub scale: glam::Vec3,
}
