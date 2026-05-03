//! Reference-counted asset handles.
//!
//! A [`Handle<T>`] is the public token granted to callers of
//! [`AssetRegistry::load_asset`]. It is cheaply cloneable, `Send`-able, and
//! ref-counted — the underlying asset data is freed only when **all** clones
//! of the handle have been dropped.
//!
//! # Ownership model
//!
//! ```text
//!  AssetRegistry::load_asset()
//!       │
//!       └─► Handle<T>  (Arc ref-count = 1)
//!               │
//!               ├── clone() ──► Handle<T>  (ref-count = 2)
//!               │
//!               └── drop()                  (ref-count = 1)
//!                       └── clone() drops   (ref-count = 0)
//!                               └─► AssetRef::drop fires InternalAssetUnloaded
//!                                       └─► AssetRegistry::tick() emits AssetUnloaded
//! ```
//!
//! # Editor vs. release behaviour
//!
//! [`Handle::consume`] behaves differently depending on the build profile:
//!
//! | Build | Behaviour |
//! |-------|-----------|
//! | `editor` | Data is **cloned** and kept alive; consume can be called any number of times. |
//! | release | Data is **moved out** on first call; subsequent calls return [`Error::AlreadyConsumed`]. |
//!
//! [`AssetRegistry::load_asset`]: crate::AssetRegistry::load_asset
//! [`Error::AlreadyConsumed`]: crate::Error::AlreadyConsumed

use std::sync::Arc;

use events::Dispatcher;
use parking_lot::Mutex;

use crate::{Asset, AssetHandle, Error, Result, events::InternalAssetUnloaded};

/// Private inner state shared by all clones of a [`Handle<T>`].
///
/// Wrapped in `Arc<Mutex<>>` so that:
/// - All [`Handle<T>`] clones share the same data without copying.
/// - The [`Drop`] impl fires [`InternalAssetUnloaded`] exactly once — when
///   the last `Arc` reference is released (i.e. when the last `Handle` clone
///   is dropped).
///
/// # Why `Mutex` and not `RwLock`?
///
/// In release builds, [`Handle::consume`] calls `Option::take` which requires
/// exclusive (`&mut`) access. A `Mutex` is the natural fit; the lock is held
/// only for the duration of the take, which is negligible.
pub(crate) struct AssetRef<T: Asset> {
    /// The identifier of this asset; carried along so the drop event can
    /// include the handle for registry bookkeeping.
    pub(crate) asset_handle: AssetHandle,

    /// The loaded asset data.
    ///
    /// - Starts as `Some(data)` after loading.
    /// - Becomes `None` in release builds after the first [`Handle::consume`]
    ///   call (data is moved out).
    /// - Remains `Some` in editor builds (data is cloned on consume).
    data: Option<T>,

    /// Dedicated dispatcher used in [`Drop::drop`] to fire the unload event.
    ///
    /// Each `AssetRef` owns its own dispatcher so the event is sent even
    /// when the [`AssetRegistry`] itself does not hold a reference to this
    /// particular asset.
    ///
    /// [`AssetRegistry`]: crate::AssetRegistry
    unload_dispatcher: Dispatcher<InternalAssetUnloaded>,
}

impl<T: Asset> AssetRef<T> {
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

impl<T: Asset> Drop for AssetRef<T> {
    /// Fires [`InternalAssetUnloaded`] when the last [`Handle<T>`] is dropped.
    ///
    /// The [`AssetRegistry`] listens for this internal event in
    /// [`AssetRegistry::tick`] and re-emits the public [`AssetUnloaded`]
    /// event so that downstream systems (renderer, audio, …) can free their
    /// own resources.
    ///
    /// [`AssetRegistry::tick`]: crate::AssetRegistry::tick
    /// [`AssetUnloaded`]: crate::AssetUnloaded
    fn drop(&mut self) {
        self.unload_dispatcher
            .dispatch(InternalAssetUnloaded(self.asset_handle.clone()));
    }
}

/// A reference-counted, cheaply-cloneable public handle to a loaded asset.
///
/// Obtain one via [`AssetRegistry::load_asset`]. Cloning a handle does not
/// copy the underlying data — all clones share the same [`AssetRef`] inside
/// an `Arc`. The asset's CPU memory is freed only when **all** clones are
/// dropped.
///
/// # Consuming the data
///
/// Call [`consume`] to access the inner `T`. The semantics depend on the
/// build profile — see the [module-level docs](self) for the comparison
/// table.
///
/// # Sending across threads
///
/// `Handle<T>` is [`Send`] whenever `T: Send`, which is required by the
/// [`Asset`] bound. You can hand a handle to a worker thread for background
/// GPU upload and drop it there safely.
///
/// # Debug formatting
///
/// The `Debug` impl prints the raw asset path without locking the inner
/// `Mutex` for data access:
///
/// ```rust
/// # let handle = 0;
/// // Output: Handle { asset: "models/hero.dirkasset", .. }
/// println!("{:?}", handle);
/// ```
///
/// [`AssetRegistry::load_asset`]: crate::AssetRegistry::load_asset
/// [`consume`]: Handle::consume
#[derive(Clone)]
pub struct Handle<T: Asset>(Arc<Mutex<AssetRef<T>>>);

impl<T: Asset> Handle<T> {
    pub(crate) fn new(asset_ref: AssetRef<T>) -> Self {
        Self(Arc::new(Mutex::new(asset_ref)))
    }

    /// Returns the loaded asset data.
    /// This function performs a full copy of the asset data.
    /// Beware of the performance cost.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyTaken`] in release builds if the data has
    /// already been taken by a prior call.
    pub fn get(&self) -> Result<T> {
        let inner = self.0.lock();
        inner.data.clone().ok_or(Error::AlreadyTaken)
    }

    /// Returns the loaded asset data & frees the copy stored by the handle.
    /// This function performs a full copy of the asset data.
    /// Beware of the performance cost.
    ///
    /// # Errors
    ///
    /// Returns [`Error::AlreadyTaken`] in release builds if the data has
    /// already been taken by a prior call.
    pub fn take(&self) -> Result<T> {
        let mut inner = self.0.lock();
        inner.data.take().ok_or(Error::AlreadyTaken)
    }
}

impl<T: Asset> std::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle")
            .field("asset", &self.0.lock().asset_handle.raw())
            .finish_non_exhaustive()
    }
}
