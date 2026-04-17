//! This crate handles all the asset stuff.

mod errors;
pub use crate::errors::{Error, Result};

mod validation;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::debug;

const DIRK_ASSET_EXT: &str = "dirkasset";
const ASSETS_PATH: &str = env!("ASSETS_PATH");

/// The type that is passed around to access assets.
#[derive(Default, PartialEq, Eq, Hash)]
pub struct AssetHandle {
    /// All assets are identified by their path.
    handle: String,
    /// Used for internal validation.
    asset_type: AssetType,
}

impl AssetHandle {
    pub fn new() -> Self {
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
#[derive(Default, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize)]
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

        let assets_path = PathBuf::from(ASSETS_PATH).canonicalize()?;
        registry.load(&assets_path, &assets_path)?;
        registry.validate();

        Ok(registry)
    }

    /// Will recursively load assets from a specific dir.
    fn load(&mut self, base: &Path, dir: &Path) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path().canonicalize()?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                self.load(base, &path)?;
            } else if metadata.is_file() {
                if path.extension().and_then(|ext| ext.to_str()) == Some(DIRK_ASSET_EXT) {
                    let relative_path =
                        path.strip_prefix(base)
                            .map(|p| p.to_path_buf())
                            .map_err(|_| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidInput,
                                    format!(
                                        "Path '{}' is not relative to base '{}'",
                                        path.display(),
                                        base.display()
                                    ),
                                )
                            })?;

                    debug!(
                        "load asset:\n\tpath: {}\n\trelative: {}",
                        path.display(),
                        relative_path.display()
                    );

                    let data = std::fs::read(path)?;
                    let config: AssetConfig = serde_json::from_slice(&data)?;
                    self.assets.insert(
                        AssetHandle {
                            handle: relative_path.display().to_string(),
                            asset_type: config.meta.asset_type,
                        },
                        config,
                    );
                }
            }
        }

        Ok(())
    }

    fn validate(&mut self) {
        self.assets.retain(|_, conf| conf.validate());
    }
}
