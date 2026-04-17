//! This crate handles all the asset stuff.

mod validation;

use serde::{Deserialize, Serialize};

/// The type that is passed around to access assets.
pub struct AssetHandle {
    /// All assets are identified by their path.
    pub id: String,
    /// Used for internal validation.
    pub asset_type: AssetType,
}

impl AssetHandle {
    pub fn new() {
        todo!()
    }
    pub fn path(&self) -> String {
        self.id.clone()
    }
}

/// Every possible asset type.
#[derive(Serialize, Deserialize)]
pub enum AssetType {
    Model,
    Sound,
}

/// Asset metadata, all assets have this.
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub asset_type: AssetType,
}

/// Type to be serialised to and from the `.dirkasset` files.
#[derive(Serialize, Deserialize)]
struct AssetConfig {
    pub meta: Metadata,
    pub model: Option<ModelConfig>,
}

/// Type to configure a model.
#[derive(Serialize, Deserialize)]
pub struct ModelConfig {
    /// Path to .gltf
    pub gltf: String,
}

pub struct Model {
    pub meta: Metadata,
    pub config: ModelConfig,
}
