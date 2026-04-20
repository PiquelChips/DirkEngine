//! Asset loading, lifetime management, and registry.

mod errors;
pub use errors::{Error, Result};

mod events;
pub use events::AssetUnloaded;

mod assets;
pub use assets::{Asset, AssetType};

mod handle;
use handle::AssetRef;
pub use handle::Handle;

mod validation;

use ::events::{Consumer, Dispatcher, EventManager};
use core::{clone::Clone, todo};
use std::{
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::events::InternalAssetUnloaded;

pub(crate) const DIRK_ASSET_EXT: &str = "dirkasset";
pub(crate) const ASSETS_PATH: &str = std::env!("ASSETS_PATH");

/// Identifies an asset. Cheap to clone — just two heap strings.
// TODO: should be serialisable for saving & stuff
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
pub struct AssetHandle {
    /// All assets are identified by their path.
    handle: String,
    /// Used for internal validation.
    asset_type: AssetType,
}

impl Display for AssetHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.handle)
    }
}

impl AssetHandle {
    /// Returns the absolute path to the asset file
    pub fn path(&self) -> PathBuf {
        PathBuf::from(format!("{ASSETS_PATH}/{}", self.handle))
    }
    pub fn dir(&self) -> PathBuf {
        todo!("return the directory that the asset is in")
    }
    pub fn name(&self) -> String {
        todo!("return just the name of the asset not its full path handle")
    }
    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }
    pub fn raw(&self) -> &str {
        &self.handle
    }
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
#[derive(Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    /// Path to .gltf. Relative to asset dir
    pub gltf: String,
}

pub struct AssetRegistry {
    assets: HashMap<AssetHandle, AssetConfig>,

    event_manager: EventManager,
    /// Receives drop notifications from every `AssetRef`.
    internal_unload_consumer: Consumer<InternalAssetUnloaded>,
    /// Public event — subscribe to this to know when to clean up GPU resources.
    unload_dispatcher: Dispatcher<AssetUnloaded>,
}

impl AssetRegistry {
    pub fn init(event_manager: &EventManager) -> Result<Self> {
        let mut registry = Self {
            assets: HashMap::new(),

            unload_dispatcher: event_manager.register(),
            internal_unload_consumer: event_manager.subscribe(),
            event_manager: event_manager.clone(),
        };

        let assets_path = PathBuf::from(ASSETS_PATH).canonicalize()?;
        registry.load(&assets_path, &assets_path)?;
        registry.validate();

        Ok(registry)
    }

    /// Must be called once per frame/tick.
    ///
    /// Converts internal `AssetRef` drop notifications into the public
    /// `AssetUnloaded` event that other systems (e.g. the renderer) consume.
    pub fn tick(&self) {
        for InternalAssetUnloaded(handle) in self.internal_unload_consumer.consume_all() {
            // TODO: see about how to also store some kind of reference on which
            // assets are still loaded.
            self.unload_dispatcher.dispatch(AssetUnloaded { handle });
        }
    }

    /// Returns the `ModelConfig` for the given handle, if it exists.
    pub(crate) fn model_config(&self, handle: &AssetHandle) -> Option<&ModelConfig> {
        self.assets.get(handle)?.model.as_ref()
    }

    fn load(&mut self, base: &Path, dir: &Path) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path().canonicalize()?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                self.load(base, &path)?;
            } else if metadata.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some(DIRK_ASSET_EXT)
            {
                let relative_path =
                    path.strip_prefix(base)
                        .map(|p| p.to_path_buf())
                        .map_err(|_| {
                            std::io::Error::new(
                                std::io::ErrorKind::InvalidInput,
                                format!(
                                    "path '{}' is not relative to base '{}'",
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
        Ok(())
    }

    fn validate(&mut self) {
        self.assets.retain(|_, conf| conf.validate());
    }

    // TODO: combine with load_asset function. It should be generic the whole
    // way to avoid needing to specify asset types in registry itself.
    pub fn load_model(&self, handle: AssetHandle) -> Result<Handle<ModelData>> {
        if handle.asset_type() != AssetType::Model {
            return Err(Error::TypeMismatch(handle.raw().to_owned()));
        }
        let config = self
            .model_config(&handle)
            .ok_or_else(|| Error::NotFound(handle.raw().to_owned()))?;

        self.load_asset::<ModelData>(handle, config)
    }

    /// Generic core: loads data and wraps it in a [`Handle<T>`].
    fn load_asset<T: Asset>(
        &self,
        asset_handle: AssetHandle,
        config: &T::Config,
    ) -> Result<Handle<T>> {
        // Clone the dispatcher so this AssetRef has its own sender.
        // Cloning a Dispatcher registers a fresh producer in the EventManager,
        // which is exactly what we want — one producer per live asset.
        Ok(Handle::new(AssetRef::new(
            asset_handle.clone(),
            T::load(config, asset_handle)?,
            self.event_manager.register(),
        )))
    }
}
