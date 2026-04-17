use tracing::warn;

use crate::{AssetConfig, AssetType, Metadata, ModelConfig};

impl AssetConfig {
    pub fn validate(&self) -> bool {
        match self.meta.asset_type {
            AssetType::Unknown => {
                warn!(
                    "asset {} can not be of type Unknown, please specify an asset_type",
                    self.meta.handle.raw()
                );
                false
            }
            AssetType::Model => {
                if let Some(model_config) = &self.model {
                    model_config.validate(&self.meta)
                } else {
                    warn!(
                        "asset {} must specify a model configuration if it is of type model",
                        self.meta.handle.raw()
                    );
                    false
                }
            }
        }
    }
}

impl ModelConfig {
    fn validate(&self, _meta: &Metadata) -> bool {
        todo!("check if ModelConfig's glTF file actually exists")
    }
}
