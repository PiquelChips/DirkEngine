//! Post-load validation for `.dirkasset` files.
//!
//! After the registry scans and deserialises all `.dirkasset` files, it calls
//! [`DirkAsset::validate`] on each entry. Assets that fail validation are
//! **silently pruned** from the registry and a [`tracing::warn`] message is
//! emitted. They will not appear in any subsequent [`AssetRegistry::load_asset`]
//! call.
//!
//! Validation is intentionally kept cheap — it checks structural consistency
//! (right config section present, referenced files exist on disk) rather than
//! parsing full asset content. Full content errors surface later at
//! [`Asset::load`] time.
//!
//! # What is validated
//!
//! - `asset_type != Unknown`
//! - `[model]` section is present if `AssetType::Model`
//! - run [`AssetConfig::validate`] on all asset configurations
//!
//! [`AssetRegistry::load_asset`]: crate::AssetRegistry::load_asset
//! [`Asset::load`]: crate::Asset::load

use tracing::warn;

use crate::{AssetType, DirkAsset, assets::AssetConfig};

impl DirkAsset {
    /// Validates the deserialised asset descriptor.
    ///
    /// Returns `true` if the asset is structurally sound and should be kept
    /// in the registry; `false` if it should be pruned. A warning is logged
    /// for every failure condition.
    ///
    /// # Failure conditions
    ///
    /// - `asset_type` is [`AssetType::Unknown`] — the `.dirkasset` file is
    ///   missing or has an unrecognised `asset_type` field.
    /// - `asset_type` is [`AssetType::Model`] but the `"model"` JSON section
    ///   is absent.
    /// - The [`AssetConfig::validate`] failes on all configurations.
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
