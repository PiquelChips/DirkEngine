use std::path::PathBuf;

use tracing::warn;

use crate::{ASSETS_PATH, AssetConfig, AssetType, Metadata, ModelConfig};

impl AssetConfig {
    pub fn validate(&self) -> bool {
        match self.meta.asset_type {
            AssetType::Unknown => {
                warn!(
                    "asset {} cannot be of type Unknown, please specify an asset_type",
                    self.meta.handle.raw()
                );
                false
            }
            AssetType::Model => {
                if let Some(model_config) = &self.model {
                    model_config.validate(&self.meta)
                } else {
                    warn!(
                        "asset {} must specify a [model] section when asset_type = Model",
                        self.meta.handle.raw()
                    );
                    false
                }
            }
        }
    }
}

impl ModelConfig {
    fn validate(&self, meta: &Metadata) -> bool {
        // TODO: relateiive to asset path not ASSETS_PATH
        let path = PathBuf::from(ASSETS_PATH).join(&self.gltf);
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
