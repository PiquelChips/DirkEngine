//! This crate handles all the asset stuff.

mod validation;

use serde::{Deserialize, Serialize};

const ASSET_PATH: &str = env!("ASSETS_PATH");

/// The type that is passed around to access assets.
#[derive(Default)]
pub struct AssetHandle {
    /// All assets are identified by their path.
    pub handle: String,
    /// Used for internal validation.
    pub asset_type: AssetType,
}

impl AssetHandle {
    pub fn new() {
        todo!()
    }
    pub fn path(&self) -> String {
        format!("{ASSET_PATH}/{}", self.handle)
    }
    pub fn raw(&self) -> &str {
        &self.handle
    }
}

/// Every possible asset type.
#[derive(Default, Serialize, Deserialize)]
pub enum AssetType {
    #[default]
    Unknown,
    Model,
    Sound,
}

/// Asset metadata, all assets have this.
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    pub asset_type: AssetType,
    /// Should be populated at load time
    #[serde(skip)]
    handle: AssetHandle,
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
