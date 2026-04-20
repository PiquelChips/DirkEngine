//! This module has the main [`Asset`] trait and is the parent
//! of all asset types declared in this crate.

use serde::{Deserialize, Serialize};

pub mod model;

use crate::{AssetHandle, Result};

/// Implemented by every concrete asset data type.
pub trait Asset: Clone + Sized + Send + 'static {
    /// The config section from the `.dirkasset` file for this type.
    type Config;

    /// Fully loads the asset into memory from disk.
    fn load(config: &Self::Config, handle: AssetHandle) -> Result<Self>;
    /// Get what kind of asset this is
    fn asset_type() -> AssetType;
}

/// Every possible asset type.
#[derive(Default, PartialEq, Eq, Hash, Clone, Copy, Serialize, Deserialize, Debug)]
pub enum AssetType {
    #[default]
    Unknown,
    Model,
}

/// Marker trait for every asset configuration struct.
pub trait AssetConfig<'a>: Serialize + Deserialize<'a> {}
