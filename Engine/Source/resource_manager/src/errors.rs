use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("gltf error: {0}")]
    GltfError(#[from] gltf::Error),
}
