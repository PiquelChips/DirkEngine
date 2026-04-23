//! Error types for the asset loading pipeline.
//!
//! All fallible operations in this crate return [`Result<T>`], which is an
//! alias for `std::result::Result<T, `[`Error`]`>`.
//!
//! # Error hierarchy
//!
//! ```text
//! Error
//! ├── IoError            — filesystem access failed
//! ├── SerialisationError — malformed .dirkasset JSON
//! ├── AlreadyConsumed    — Handle::consume() called twice in release mode
//! ├── NotFound           — asset handle not in registry
//! ├── TypeMismatch       — handle refers to a different asset type
//! └── AssetLoadError     — type-specific load failure (e.g. bad glTF)
//! ```

use thiserror::Error;

/// Convenience alias — every fallible function in this crate returns this.
pub type Result<T> = std::result::Result<T, Error>;

/// All errors that can occur within the asset subsystem.
///
/// Variants are coarse-grained by *phase*: file I/O, JSON deserialisation,
/// asset lifetime, registry lookup, and type-level contract failures each get
/// their own variant so callers can match precisely on what went wrong.
#[derive(Debug, Error)]
pub enum Error {
    /// A filesystem operation failed while scanning or reading asset files.
    ///
    /// This is the `From<std::io::Error>` conversion target, so the `?`
    /// operator on any [`std::io`] call will automatically wrap the error.
    ///
    /// # Common causes
    /// - The `ASSETS_PATH` environment variable points to a non-existent
    ///   directory.
    /// - A `.dirkasset` file was deleted between directory scan and read.
    /// - Insufficient filesystem permissions.
    #[error("IO error while loading assets: {0}")]
    IoError(#[from] std::io::Error),

    /// A `.dirkasset` file contained malformed JSON, or a config struct could
    /// not be serialised for the internal serde round-trip in
    /// [`AssetRegistry::asset_config`].
    ///
    /// This is the `From<serde_json::Error>` conversion target.
    ///
    /// # Common causes
    /// - Typo or missing field in a hand-written `.dirkasset` file.
    /// - A new required field was added to a config struct without updating
    ///   existing asset files.
    #[error("error during .dirkasset JSON serialisation: {0}")]
    SerialisationError(#[from] serde_json::Error),

    /// [`Handle::consume`] was called on an asset that has already been
    /// consumed in a **release** build.
    ///
    /// In release builds, [`Handle::consume`] moves the asset data out of the
    /// handle on first call, freeing the memory immediately. Any subsequent
    /// call on the same (or a clone of the same) handle returns this error.
    ///
    /// In **editor** builds this error is never returned — the data is cloned
    /// and kept alive for repeated inspection.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use assets::{Model, AssetType, AssetHandle};
    /// # fn test() -> anyhow::Result<()> {
    /// # let events = ::events::EventManager::new();
    /// # let mut registry = assets::AssetRegistry::init(&events)?;
    /// # let asset_handle = AssetHandle::from_raw("", AssetType::Model);
    /// // release build only
    /// let handle = registry.load_asset::<Model>(asset_handle)?;
    /// let _data  = handle.consume()?;          // OK — data moved out
    /// let err    = handle.consume().unwrap_err(); // AlreadyConsumed
    /// # Ok(()) }
    /// ```
    #[error("asset data has already been consumed")]
    AlreadyConsumed,

    /// The requested [`AssetHandle`] was not found in the registry.
    ///
    /// The inner `String` is the raw handle path (e.g.
    /// `"models/hero.dirkasset"`) and is included in the error message for
    /// easy diagnosis.
    ///
    /// # Common causes
    /// - The handle string was constructed manually and contains a typo.
    /// - The corresponding `.dirkasset` file failed validation and was pruned
    ///   from the registry at startup.
    /// - The asset file was added after the registry was initialised (hot
    ///   reload is not yet supported).
    #[error("asset not found in registry: {0}")]
    NotFound(String),

    /// A [`Handle`] of the wrong asset type was passed to
    /// [`AssetRegistry::load_asset`].
    ///
    /// For example, passing a handle whose `asset_type` is
    /// [`AssetType::Model`] to a `load_asset::<AudioClip>()` call would
    /// produce this error.
    ///
    /// The inner `String` is the raw handle path.
    #[error("asset {0} has wrong type for the requested load")]
    TypeMismatch(String),

    /// The asset-type-specific loader returned an error.
    ///
    /// Wraps an [`anyhow::Error`] so that loaders can attach rich context
    /// without having to define their own error enums. The source chain is
    /// preserved and printed by `{0}`.
    ///
    /// # Common causes
    /// - A glTF file referenced by a `ModelConfig` is missing or corrupt.
    /// - A required buffer or image embedded in a glTF is inaccessible.
    #[error("Asset load error: {0}")]
    AssetLoadError(#[source] anyhow::Error),
}
