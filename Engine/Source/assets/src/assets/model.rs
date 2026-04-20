use crate::{Asset, AssetHandle, AssetType, Result, assets::AssetConfig};

use serde::{Deserialize, Serialize};

/// Type to configure a model.
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    /// Path to .gltf. Relative to asset dir
    pub gltf: String,
}
impl AssetConfig<'_> for ModelConfig {}

/// Raw glTF bytes for a model asset. The renderer is responsible for
/// uploading this to the GPU after calling [`Handle::consume`].
// TODO: properly import the glTF file and attach all the data to this function
#[derive(Clone)]
pub struct ModelData {
    pub gltf_bytes: Vec<u8>,
}

impl Asset for ModelData {
    type Config = ModelConfig;

    fn load(config: &Self::Config, handle: AssetHandle) -> Result<Self> {
        let path = handle.dir().join(&config.gltf);
        // TODO: actually load import glTF file using the glTF library
        let gltf_bytes = std::fs::read(path)?;
        Ok(Self { gltf_bytes })
    }
    fn asset_type() -> AssetType {
        AssetType::Model
    }
}
