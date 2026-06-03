//! This module contains all the shared shader types.

/// The Uniform buffer for scene specific data
#[derive(Clone, Copy)]
pub struct SceneUbo {
    /// The view matrix to render the scene
    pub view: glam::Mat4,
    /// The projection matrix to render the scene
    pub proj: glam::Mat4,
}

/// The Uniform buffer for proxy specific data
#[derive(Clone, Copy)]
pub struct ProxyUbo {
    /// The model matrix of the proxy
    pub model: glam::Mat4,
}
