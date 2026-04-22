use crate::{Asset, AssetHandle, Handle};
use events::Event;

/// Fired internally when the last [`Handle`] to an asset is dropped.
/// The [`AssetManager`] listens for this, cleans up, then fires [`AssetUnloaded`].
#[derive(Event, Clone, Debug)]
#[event("unload asset {0}")]
pub(crate) struct InternalAssetUnloaded(pub AssetHandle);

/// Public event dispatched by [`AssetManager`] when an asset has been fully
/// loaded. The renderer (or any other system) should use this to create
/// GPU-side resources.
#[derive(Event, Clone, Debug)]
#[event("asset {handle:?} unloaded")]
pub struct AssetLoaded<T: Asset> {
    pub handle: Handle<T>,
}

/// Public event dispatched by [`AssetManager`] when an asset has been fully
/// unloaded. The renderer (or any other system) should use this to clean up
/// GPU-side resources.
#[derive(Event, Clone, Debug)]
#[event("asset {handle} unloaded")]
pub struct AssetUnloaded {
    pub handle: AssetHandle,
}
