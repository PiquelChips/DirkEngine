use crate::{Asset, AssetHandle, AssetType, Error, Result, assets::AssetConfig};

use serde::{Deserialize, Serialize};

use anyhow::Context;

/// Type to configure a model.
#[derive(Serialize, Deserialize, Clone, AssetConfig)]
pub struct ModelConfig {
    /// Path to .gltf. Relative to asset dir
    pub gltf: String,
}

/// Raw glTF bytes for a model asset. The renderer is responsible for
/// uploading this to the GPU after calling [`Handle::consume`].
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
