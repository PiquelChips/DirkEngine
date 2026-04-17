//! This crate handles all the asset stuff.

mod errors;
pub use crate::errors::{Error, Result};

mod validation;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

const ASSETS_PATH: &str = env!("ASSETS_PATH");

/// The type that is passed around to access assets.
#[derive(Default)]
pub struct AssetHandle {
    /// All assets are identified by their path.
    handle: String,
    /// Used for internal validation.
    asset_type: AssetType,
}

impl AssetHandle {
    pub fn new() {
        todo!()
    }
    pub fn path(&self) -> String {
        format!("{ASSETS_PATH}/{}", self.handle)
    }
    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }
    pub fn raw(&self) -> &str {
        &self.handle
    }
}

/// Every possible asset type.
#[derive(Default, Clone, Copy, Serialize, Deserialize)]
pub enum AssetType {
    #[default]
    Unknown,
    Model,
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

#[derive(Default)]
pub struct AssetRegistry {
    assets: HashMap<AssetHandle, AssetConfig>,
}

impl AssetRegistry {
    pub fn init() -> Result<Self> {
        let mut registry = Self::default();

        registry.load(ASSETS_PATH)?;
        registry.validate();

        Ok(registry)
    }

    /// Will recursively load assets from a specific dir.
    fn load(&mut self, dir: &str) -> Result<()> {
        let dirs = std::fs::read_dir(dir)?
            .filter_map(|result| {
                let entry = result.ok()?;
                if entry.metadata().ok()?.is_dir() {
                    return Some(entry.path());
                }

                // TODO: load the file if it is a .dirkasset

                None
            })
            .collect::<Vec<_>>();

        dirs.iter()
            .try_for_each(|dir| self.load(&dir.as_os_str().to_string_lossy()))?;
        Ok(())
    }

    fn validate(&mut self) {
        self.assets.retain(|_, conf| conf.validate());
    }
}
