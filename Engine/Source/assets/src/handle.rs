use std::sync::Arc;

use events::Dispatcher;
use parking_lot::Mutex;

use crate::{AssetHandle, Error, Result, events::InternalAssetUnloaded};

/// Private inner state. Holds the asset data and fires the unload event on drop.
///
/// Wrapped in `Arc<Mutex<>>` so that all [`Handle<T>`] clones share it —
/// the unload event is fired only when the *last* handle is dropped.
pub(crate) struct AssetRef<T> {
    pub(crate) asset_handle: AssetHandle,
    /// `None` after `consume()` is called in a release build.
    data: Option<T>,
    /// Each `AssetRef` gets its own dispatcher to dispath the unload
    /// even in the [`Drop`] implementation.
    unload_dispatcher: Dispatcher<InternalAssetUnloaded>,
}

impl<T> AssetRef<T> {
    pub(crate) fn new(
        asset_handle: AssetHandle,
        data: T,
        unload_dispatcher: Dispatcher<InternalAssetUnloaded>,
    ) -> Self {
        Self {
            asset_handle,
            data: Some(data),
            unload_dispatcher,
        }
    }
}

impl<T> Drop for AssetRef<T> {
    fn drop(&mut self) {
        // The last Handle<T> has been dropped. Signal the AssetManager so it
        // can dispatch the public AssetUnloaded event to e.g. the renderer.
        self.unload_dispatcher
            .dispatch(InternalAssetUnloaded(self.asset_handle.clone()));
    }
}

/// A reference-counted, cheaply-cloneable public handle to a loaded asset.
#[derive(Clone)]
pub struct Handle<T>(Arc<Mutex<AssetRef<T>>>);

impl<T: Clone> Handle<T> {
    pub(crate) fn new(asset_ref: AssetRef<T>) -> Self {
        Self(Arc::new(Mutex::new(asset_ref)))
    }

    /// Returns the loaded asset data.
    ///
    /// - **Editor builds**: data is cloned and kept alive so it can be
    ///   inspected repeatedly.
    /// - **Release builds**: data is moved out and freed after the first
    ///   call. Subsequent calls return [`Error::AlreadyConsumed`].
    pub fn consume(&self) -> Result<T> {
        let inner = self.0.lock();

        #[cfg(editor)]
        let result = inner.data.clone().ok_or(Error::AlreadyConsumed);
        #[cfg(not(editor))]
        let result = inner.data.take().ok_or(Error::AlreadyConsumed);

        result
    }
}

impl<T> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("asset", &self.0.lock().asset_handle.raw())
            .finish_non_exhaustive()
    }
}
