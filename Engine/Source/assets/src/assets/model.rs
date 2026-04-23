use crate::{Asset, AssetHandle, AssetType, Error, Metadata, Result, assets::AssetConfig};

use serde::{Deserialize, Serialize};

use anyhow::Context;

/// Type to configure a model.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    /// Path to .gltf. Relative to asset dir
    pub gltf: String,
}

/// Raw glTF bytes for a model asset. The renderer is responsible for
/// uploading this to the GPU after calling [`Handle::consume`].
impl AssetConfig for ModelConfig {
    /// Validates that the glTF file actually exists on disk.
    ///
    /// Resolves the path as `handle.dir().join(&self.gltf)` — i.e. relative
    /// to the `.dirkasset` file's own directory — and checks [`Path::exists`].
    ///
    /// Logs a warning and returns `false` if the file is absent.
    ///
    /// [`Path::exists`]: std::path::Path::exists
    fn validate(&self, meta: &Metadata) -> bool {
        let path = meta.handle.dir().join(&self.gltf);
        if !path.exists() {
            warn!(
                "asset {}: glTF file not found at '{}'",
                meta.handle.raw(),
                path.display()
            );
            return false;
        }
        true
    }
}

#[derive(Clone)]
pub struct ModelData {
    pub gltf: gltf::Document,
    pub images: Vec<gltf::image::Data>,
    pub buffers: Vec<gltf::buffer::Data>,
}

impl Asset for ModelData {
    type Config = ModelConfig;

    fn load(config: &Self::Config, handle: AssetHandle) -> Result<Self> {
        let path = handle.dir().join(&config.gltf);
        let (gltf, buffers, images) = gltf::import(path)
            .context("loading glTF model")
            .map_err(Error::AssetLoadError)?;

        Ok(Self {
            gltf,
            buffers,
            images,
        })
    }
    fn asset_type() -> AssetType {
        AssetType::Model
    }
}
