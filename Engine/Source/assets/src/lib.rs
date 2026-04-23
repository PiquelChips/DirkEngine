//! Asset loading, lifetime management, and registry.
//!
//! This crate is the central hub for all on-disk assets consumed by the
//! engine. It provides:
//!
//! - **[`AssetHandle`]** — a lightweight identifier (path + type tag) that
//!   locates an asset inside the asset tree.
//! - **[`Handle<T>`]** — a ref-counted, typed wrapper around loaded asset
//!   data; the last drop triggers an [`AssetUnloaded`] event.
//! - **[`AssetRegistry`]** — scans the asset directory at startup, validates
//!   every `.dirkasset` descriptor, and vends [`Handle<T>`]s on demand.
//!
//! # Asset file format
//!
//! Every asset on disk is described by a JSON file with the `.dirkasset`
//! extension. The file lives alongside (or near) the source data it
//! references. Example for a model:
//!
//! ```json
//! {
//!   "meta": { "asset_type": "Model" },
//!   "model": { "gltf": "hero.gltf" }
//! }
//! ```
//!
//! The `meta.asset_type` field determines which config section is used. Each
//! asset type has its own optional config object (currently only `"model"`).
//!
//! # Quick start
//!
//! ```rust
//! use assets::{AssetRegistry, AssetHandle, AssetType, AssetLoaded, AssetUnloaded, Model};
//! use ::events::EventManager;
//!
//! # fn test() -> anyhow::Result<()> {
//! // 1. Initialise — scans ASSETS_PATH and validates all .dirkasset files.
//! let events = EventManager::new();
//! let mut registry = AssetRegistry::init(&events)?;
//!
//! // 2. Load an asset by its handle string (path relative to ASSETS_PATH).
//! let handle = registry.load_asset::<Model>(
//!     AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model))?;
//!
//! // 3. Subscribe to receive load events for future loads.
//! let loaded_consumer = events.subscribe::<AssetLoaded<Model>>();
//! let unloaded_consumer = events.subscribe::<AssetUnloaded>();
//!
//! // 4. Game loop — call once per frame.
//! loop {
//!     events.dispatch_all();
//!     registry.tick();
//!
//!     for event in loaded_consumer.consume_all() {
//!         let data = event.handle.consume()?;
//!         // ... upload data to GPU
//!     }
//!     for AssetUnloaded { handle } in unloaded_consumer.consume_all() {
//!         // ... remove data from the GPU
//!     }
//! }
//! # Ok(()) }
//! ```
//!
//! # Environment variables
//!
//! | Variable | Set by | Purpose |
//! |----------|--------|---------|
//! | `ASSETS_PATH` | `build.rs` via the `build` crate | Absolute path to the root assets directory; baked into the binary at compile time. |
//!
//! # Feature flags / build profiles
//!
//! The crate is aware of a single build-profile flag:
//!
//! | Flag | Effect |
//! |------|--------|
//! | `--cfg editor` | [`Handle::consume`] clones data instead of moving it out, enabling repeated inspection by editor tools. |
//!
//! [`Handle::consume`]: Handle::consume

mod errors;
pub use errors::{Error, Result};

mod events;
pub use events::{AssetLoaded, AssetUnloaded};

mod assets;
pub use assets::*;

mod handle;
use handle::AssetRef;
pub use handle::Handle;

mod validation;

use ::events::{Consumer, Dispatcher, EventManager};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Display,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::events::InternalAssetUnloaded;

/// Extension used for all asset descriptor files.
pub(crate) const DIRK_ASSET_EXT: &str = "dirkasset";

/// Absolute path to the root of the asset tree, baked in at compile time by
/// `build.rs`.
pub(crate) const ASSETS_PATH: &str = std::env!("ASSETS_PATH");

/// A lightweight, cheaply-cloneable identifier for a single asset.
///
/// An `AssetHandle` consists of two parts:
/// - A **path string** relative to `ASSETS_PATH` (e.g.
///   `"models/hero.dirkasset"`).
/// - An [`AssetType`] discriminant embedded at load time from the asset's
///   `meta.asset_type` field, used to catch type mismatches before attempting
///   a load.
///
/// Handles are created internally by [`AssetRegistry`] during directory
/// scanning and are not normally constructed by hand. They are serialisable
/// so they can be stored in scene files or editor state.
///
/// # Cloning cost
///
/// Cloning an `AssetHandle` allocates a new `String`. For hot paths, prefer
/// passing `&AssetHandle` or working with the [`Handle<T>`] directly.
///
/// # Display
///
/// The `Display` impl prints the raw handle path:
///
/// ```rust
/// # use assets::{AssetHandle, AssetType};
/// // Construct a minimal handle for illustration:
/// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
/// assert_eq!(handle.to_string(), "models/hero.dirkasset");
/// ```
///
/// # Serialisation
///
/// `AssetHandle` round-trips through JSON:
///
/// ```rust
/// # use assets::{AssetHandle, AssetType};
/// let handle = AssetHandle::from_raw("textures/dirt.dirkasset", AssetType::Unknown);
/// let json   = serde_json::to_string(&handle).unwrap();
/// let back: AssetHandle = serde_json::from_str(&json).unwrap();
/// assert_eq!(handle.raw(), back.raw());
/// assert_eq!(handle.asset_type(), back.asset_type());
/// ```
#[derive(Default, PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct AssetHandle {
    /// Path relative to `ASSETS_PATH`, including the `.dirkasset` extension.
    handle: String,
    /// Runtime type tag, validated against the requested `T` in
    /// [`AssetRegistry::load_asset`].
    asset_type: AssetType,
}

impl Display for AssetHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.handle)
    }
}

impl AssetHandle {
    /// Constructs an `AssetHandle` from a raw path string and type tag.
    ///
    /// Intended primarily for tests and editor tooling. In production code,
    /// handles are issued by [`AssetRegistry`] during scanning.
    ///
    /// ```rust
    /// # use assets::{AssetHandle, AssetType};
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// assert_eq!(handle.raw(), "models/hero.dirkasset");
    /// assert_eq!(handle.asset_type(), AssetType::Model);
    /// ```
    pub fn from_raw(path: impl Into<String>, asset_type: AssetType) -> Self {
        Self {
            handle: path.into(),
            asset_type,
        }
    }

    /// Returns the **absolute** path to the `.dirkasset` file on disk.
    ///
    /// Constructed as `ASSETS_PATH + "/" + self.handle`.
    ///
    /// ```rust
    /// # use assets::{AssetHandle, AssetType};
    /// # // ASSETS_PATH is baked in at compile time; we can only check the suffix.
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// assert!(handle.path().ends_with("models/hero.dirkasset"));
    /// ```
    pub fn path(&self) -> PathBuf {
        PathBuf::from(format!("{ASSETS_PATH}/{}", self.handle))
    }

    /// Returns the **absolute** path to the directory containing the
    /// `.dirkasset` file.
    ///
    /// This is the base path used by asset loaders to resolve relative file
    /// references (e.g. a glTF path in a [`ModelConfig`]).
    ///
    /// Returns `ASSETS_PATH` if [`path`] has no parent component (which
    /// should not occur in practice for well-formed handles).
    ///
    /// ```rust
    /// # use assets::{AssetHandle, AssetType};
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// assert!(handle.dir().ends_with("models"));
    /// ```
    ///
    /// [`path`]: AssetHandle::path
    pub fn dir(&self) -> PathBuf {
        self.path()
            .parent()
            .unwrap_or_else(|| std::path::Path::new(ASSETS_PATH))
            .to_path_buf()
    }

    /// Returns the file name component of the asset path (including the
    /// `.dirkasset` extension).
    ///
    /// ```rust
    /// # use assets::{AssetHandle, AssetType};
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// assert_eq!(handle.name(), "hero.dirkasset");
    /// ```
    pub fn name(&self) -> String {
        self.path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }

    /// Returns the [`AssetType`] discriminant embedded in this handle.
    ///
    /// ```rust
    /// # use assets::{AssetHandle, AssetType};
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// assert_eq!(handle.asset_type(), AssetType::Model);
    /// ```
    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    /// Returns the raw handle string (the path relative to `ASSETS_PATH`).
    ///
    /// ```rust
    /// # use assets::{AssetHandle, AssetType};
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// assert_eq!(handle.raw(), "models/hero.dirkasset");
    /// ```
    pub fn raw(&self) -> &str {
        &self.handle
    }
}

/// Metadata common to every `.dirkasset` file.
///
/// Serialised as the `"meta"` object in the JSON:
///
/// ```json
/// { "meta": { "asset_type": "Model" } }
/// ```
///
/// The `handle` field is populated at load time (after the file is read and
/// its path is known) and is intentionally skipped by serde — it is not
/// stored on disk.
#[derive(Serialize, Deserialize)]
pub struct Metadata {
    /// The kind of asset this descriptor represents.
    pub asset_type: AssetType,

    /// Back-reference to the handle that owns this metadata.
    ///
    /// Not serialised. Populated by [`AssetRegistry::load`] immediately after
    /// deserialisation so that validation and loading code can access path
    /// helpers via [`AssetHandle`].
    #[serde(skip)]
    handle: AssetHandle,
}

/// The complete in-memory representation of a `.dirkasset` file.
///
/// Deserialised from JSON by [`AssetRegistry::load`]. Fields for asset types
/// that are not present in a given file are deserialised as `None`.
///
/// # Adding a new asset type
///
/// Add an `Option<YourConfig>` field here, add a corresponding JSON key to
/// asset files of that type, and wire the new arm into both
/// [`AssetRegistry::asset_config`] and [`DirkAsset::validate`].
///
/// [`DirkAsset::validate`]: validation
#[derive(Serialize, Deserialize)]
struct DirkAsset {
    /// Common fields (type tag, back-reference handle).
    pub meta: Metadata,
    /// Configuration for [`AssetType::Model`] assets. `None` for all other
    /// asset types.
    pub model: Option<ModelConfig>,
}

/// Central asset management system.
///
/// `AssetRegistry` is responsible for the full lifecycle of asset descriptors:
///
/// 1. **Scan** — walks `ASSETS_PATH` recursively at startup, discovering all
///    `.dirkasset` files.
/// 2. **Deserialise** — parses each file's JSON into a [`DirkAsset`].
/// 3. **Validate** — prunes structurally invalid assets (missing config
///    sections, missing source files) and logs warnings.
/// 4. **Vend** — on [`load_asset`] calls, locates the descriptor, resolves the
///    typed config, invokes the [`Asset::load`] implementation, wraps the data
///    in a [`Handle<T>`], and fires an [`AssetLoaded<T>`] event.
/// 5. **Clean up** — in [`tick`], converts internal drop notifications into
///    the public [`AssetUnloaded`] event.
///
/// # Thread safety
///
/// `AssetRegistry` is not `Sync`. It is intended to be owned and driven from
/// the main game loop thread. Event dispatchers and consumers can be cloned and
/// sent to other threads.
///
/// # Initialisation
///
/// ```rust
/// use assets::AssetRegistry;
/// use events::EventManager;
///
/// # fn test() -> anyhow::Result<()> {
/// let events = EventManager::new();
/// let mut registry = AssetRegistry::init(&events)?;
/// # Ok(()) }
/// ```
///
/// # Per-frame update
///
/// ```rust
/// # fn test() -> anyhow::Result<()> {
/// # let events = ::events::EventManager::new();
/// # let mut registry = assets::AssetRegistry::init(&events).unwrap();
/// // Must be called once per frame, *after* EventManager::dispatch_all.
/// registry.tick();
/// # Ok(()) }
/// ```
///
/// [`load_asset`]: AssetRegistry::load_asset
/// [`tick`]: AssetRegistry::tick
/// [`AssetLoaded<T>`]: events::AssetLoaded
pub struct AssetRegistry {
    /// All validated asset descriptors, keyed by their handle.
    assets: HashMap<AssetHandle, DirkAsset>,

    /// Shared event bus used to clone dispatchers for new asset types.
    event_manager: EventManager,

    /// Receives [`InternalAssetUnloaded`] from every live `AssetRef` when its
    /// ref-count reaches zero.
    internal_unload_consumer: Consumer<InternalAssetUnloaded>,

    /// Emits the public [`AssetUnloaded`] event consumed by e.g. the renderer.
    unload_dispatcher: Dispatcher<AssetUnloaded>,

    /// One [`Dispatcher<AssetLoaded<T>>`] per concrete asset type `T`,
    /// keyed by `TypeId::of::<T>()`.
    ///
    /// Lazily populated on the first [`load_asset::<T>`] call for each `T`.
    ///
    /// [`load_asset::<T>`]: AssetRegistry::load_asset
    load_dispatchers: HashMap<TypeId, Box<dyn Any>>,
}

impl AssetRegistry {
    /// Creates and fully initialises the registry.
    ///
    /// This is the only constructor. It:
    ///
    /// 1. Registers internal event channels with `event_manager`.
    /// 2. Canonicalises `ASSETS_PATH` and walks it recursively.
    /// 3. Deserialises every `.dirkasset` file found.
    /// 4. Validates all descriptors; invalid ones are pruned and warned about.
    ///
    /// # Errors
    ///
    /// Returns [`Error::IoError`] if `ASSETS_PATH` does not exist or cannot
    /// be read, or if any `.dirkasset` file cannot be opened.
    ///
    /// Returns [`Error::SerialisationError`] if a `.dirkasset` file contains
    /// malformed JSON.
    ///
    /// # Panics
    ///
    /// Does not panic. All error conditions are returned as `Result`.
    ///
    /// [`Error::IoError`]: crate::Error::IoError
    /// [`Error::SerialisationError`]: crate::Error::SerialisationError
    pub fn init(event_manager: &EventManager) -> Result<Self> {
        let mut registry = Self {
            assets: HashMap::new(),

            unload_dispatcher: event_manager.register(),
            internal_unload_consumer: event_manager.subscribe(),
            event_manager: event_manager.clone(),

            load_dispatchers: HashMap::new(),
        };

        let assets_path = PathBuf::from(ASSETS_PATH).canonicalize()?;
        registry.load(&assets_path, &assets_path)?;
        registry.validate();

        Ok(registry)
    }

    /// Processes deferred asset-unload notifications and emits public events.
    ///
    /// Must be called **exactly once per frame**, after
    /// [`EventManager::dispatch_all`]. Skipping this call means
    /// [`AssetUnloaded`] events are never delivered and downstream systems
    /// (e.g. the renderer) cannot free GPU resources.
    ///
    /// # Delivery sequence within a frame
    ///
    /// ```text
    /// 1. EventManager::dispatch_all()     ← forward InternalAssetUnloaded to this registry
    /// 2. AssetRegistry::tick()            ← convert them to public AssetUnloaded events
    /// 3. EventManager::dispatch_all()     ← (next frame) forward AssetUnloaded to renderer
    /// ```
    ///
    /// Note the one-frame lag between a handle being dropped and the renderer
    /// receiving [`AssetUnloaded`].
    pub fn tick(&self) {
        for InternalAssetUnloaded(handle) in self.internal_unload_consumer.consume_all() {
            self.unload_dispatcher.dispatch(AssetUnloaded { handle });
        }
    }

    /// Recursively scans `dir` for `.dirkasset` files and deserialises them.
    ///
    /// `base` is the `ASSETS_PATH` root, used to compute relative paths for
    /// handle strings. All handles are stored as paths relative to `base` so
    /// that the registry is portable across machines.
    ///
    /// Logs each discovered asset at the `debug` level.
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
                let mut config: DirkAsset = serde_json::from_slice(&data)?;

                let handle = AssetHandle {
                    handle: relative_path.display().to_string(),
                    asset_type: config.meta.asset_type,
                };
                config.meta.handle = handle.clone();

                self.assets.insert(handle, config);
            }
        }
        Ok(())
    }

    /// Prunes all invalid assets from the registry in-place.
    ///
    /// Delegates per-asset validation to [`DirkAsset::validate`]; anything
    /// that returns `false` is removed. Warnings are emitted inside
    /// `validate` so this method produces no output of its own.
    fn validate(&mut self) {
        self.assets.retain(|_, conf| conf.validate());
    }

    /// Resolves and deserialises the typed config for a given handle.
    ///
    /// Returns `None` if the handle is not in the registry or if the required
    /// config section is absent on the [`DirkAsset`].
    ///
    /// # Implementation note — the serde round-trip
    ///
    /// Because `DirkAsset` stores each config type as a concrete `Option<T>`
    /// (e.g. `Option<ModelConfig>`), but this method must return a generic
    /// `T::Config`, it uses a two-step serde conversion:
    ///
    /// 1. Serialise the concrete `Option<ModelConfig>` → `serde_json::Value`.
    /// 2. Deserialise the `Value` → `T::Config`.
    ///
    /// Both steps are guaranteed to succeed for any well-formed config because
    /// [`AssetConfig`] bounds `Serialize + DeserializeOwned`. The conversion
    /// happens once per [`load_asset`] call (not per frame), so the overhead
    /// is negligible.
    ///
    /// A future refactor could replace this with a type-erased config map to
    /// avoid the round-trip entirely.
    ///
    /// [`load_asset`]: AssetRegistry::load_asset
    /// [`AssetConfig`]: assets::AssetConfig
    fn asset_config<T: Asset>(&self, handle: &AssetHandle) -> Option<T::Config> {
        let asset = self.assets.get(handle)?;

        let raw = match T::asset_type() {
            AssetType::Unknown => return None,
            AssetType::Model => serde_json::to_value(asset.model.as_ref()?).ok()?,
        };

        serde_json::from_value(raw).ok()
    }

    /// Loads an asset of type `T` and returns a reference-counted [`Handle<T>`].
    ///
    /// # Steps
    ///
    /// 1. Validates that `handle.asset_type() == T::asset_type()`.
    /// 2. Resolves the typed config via [`asset_config`].
    /// 3. Calls [`T::load`] to read the asset data from disk.
    /// 4. Wraps the data in a [`Handle<T>`] backed by an `Arc<Mutex<AssetRef<T>>>`.
    /// 5. Dispatches an [`AssetLoaded<T>`] event so the renderer (or other
    ///    systems) can react immediately.
    ///
    /// # Errors
    ///
    /// | Condition | Error |
    /// |-----------|-------|
    /// | `handle.asset_type() != T::asset_type()` | [`Error::TypeMismatch`] |
    /// | Handle not found in registry | [`Error::NotFound`] |
    /// | Asset-type-specific load failure | [`Error::AssetLoadError`] |
    ///
    /// # Example
    ///
    /// ```rust
    /// # fn test() -> anyhow::Result<()> {
    /// # let events = ::events::EventManager::new();
    /// # let mut registry = assets::AssetRegistry::init(&events).unwrap();
    /// use assets::{AssetHandle, AssetRegistry, AssetType};
    /// use assets::Model;
    ///
    /// let handle = AssetHandle::from_raw("models/hero.dirkasset", AssetType::Model);
    /// let typed_handle = registry.load_asset::<Model>(handle)?;
    /// # Ok(()) }
    /// ```
    ///
    /// # Dispatcher lazy-initialisation
    ///
    /// `load_dispatchers` is a `HashMap<TypeId, Box<dyn Any>>` that is
    /// populated on the first `load_asset::<T>` call for each distinct `T`.
    /// This avoids requiring callers to pre-register every asset type at
    /// startup, at the cost of a `HashMap` lookup per load.
    ///
    /// [`asset_config`]: AssetRegistry::asset_config
    /// [`T::load`]: Asset::load
    /// [`AssetLoaded<T>`]: events::AssetLoaded
    /// [`Error::TypeMismatch`]: crate::Error::TypeMismatch
    /// [`Error::NotFound`]: crate::Error::NotFound
    /// [`Error::AssetLoadError`]: crate::Error::AssetLoadError
    pub fn load_asset<T: Asset>(&mut self, handle: AssetHandle) -> Result<Handle<T>> {
        let type_id = TypeId::of::<T>();
        if handle.asset_type() != T::asset_type() {
            return Err(Error::TypeMismatch(handle.raw().to_owned()));
        }

        let config = self
            .asset_config::<T>(&handle)
            .ok_or_else(|| Error::NotFound(handle.raw().to_owned()))?;

        // Give this AssetRef its own dispatcher clone so it can fire
        // InternalAssetUnloaded from inside its Drop impl, independent of the
        // registry's own lifetime.
        let typed_handle = Handle::new(AssetRef::new(
            handle.clone(),
            T::load(&config, handle)?,
            self.event_manager.register(),
        ));

        // Lazily create and cache a Dispatcher<AssetLoaded<T>> for this type.
        let dispatcher = self
            .load_dispatchers
            .entry(type_id)
            .or_insert(Box::new(self.event_manager.register::<AssetLoaded<T>>()))
            .downcast_ref::<Dispatcher<AssetLoaded<T>>>()
            .expect("dispatcher type invariant violated: TypeId key must match Dispatcher<T>");

        dispatcher.dispatch(AssetLoaded {
            handle: typed_handle.clone(),
        });

        Ok(typed_handle)
    }
}
