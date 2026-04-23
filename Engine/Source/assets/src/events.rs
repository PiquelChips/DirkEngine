//! Public and internal events emitted by the asset subsystem.
//!
//! The asset system communicates with the rest of the engine (primarily the
//! renderer) through three events:
//!
//! | Event | Visibility | When fired |
//! |-------|-----------|------------|
//! | [`InternalAssetUnloaded`] | `pub(crate)` | A [`Handle`] ref-count hits zero |
//! | [`AssetLoaded<T>`] | `pub` | An asset has been fully loaded into a [`Handle`] |
//! | [`AssetUnloaded`] | `pub` | An asset's data has been fully freed |
//!
//! # Typical renderer workflow
//!
//! ```text
//! AssetLoaded<ModelData> fires
//!     └─► renderer calls handle.consume()
//!             └─► uploads buffers / images to GPU
//!                     └─► Handle is dropped (ref-count → 0)
//!                             └─► InternalAssetUnloaded fires (crate-internal)
//!                                     └─► AssetRegistry::tick() re-emits it as
//!                                             └─► AssetUnloaded fires
//!                                                     └─► renderer frees GPU resources
//! ```
//!
//! Subscribe to [`AssetLoaded<T>`] to *acquire* GPU resources, and to
//! [`AssetUnloaded`] to *release* them.
//!
//! [`Handle`]: crate::Handle

use crate::{Asset, AssetHandle, Handle};
use events::Event;

/// **Internal** event fired by an [`AssetRef`] when its owning [`Handle`]
/// ref-count drops to zero.
///
/// This event is private to the crate. [`AssetRegistry::tick`] consumes it
/// and re-emits the public [`AssetUnloaded`] event, giving the registry a
/// chance to do any internal bookkeeping before notifying external systems.
///
/// Wraps the [`AssetHandle`] of the asset that was unloaded.
///
/// [`AssetRef`]: crate::handle::AssetRef
/// [`AssetRegistry::tick`]: crate::AssetRegistry::tick
#[derive(Event, Clone, Debug)]
#[event("unload asset {0}")]
pub(crate) struct InternalAssetUnloaded(pub AssetHandle);

/// Fired by [`AssetRegistry`] when an asset of type `T` has been fully loaded
/// and is ready to be consumed.
///
/// Subscribe to this event to know when a specific asset type is ready for
/// GPU upload or other subsystem initialisation. The enclosed [`Handle`]
/// grants access to the asset data via [`Handle::consume`].
///
/// # Type parameter
///
/// `T` is the concrete [`Asset`] implementation (e.g. [`ModelData`]).
/// Subscriptions are type-specific — a consumer of
/// `AssetLoaded<ModelData>` will not receive audio or texture load events.
///
/// # Example
///
/// ```rust
/// use assets::{AssetLoaded, Handle, Model};
/// use events::{Consumer, EventManager};
///
/// # let event_manager = EventManager::new();
/// let consumer: Consumer<AssetLoaded<Model>> = event_manager.subscribe();
///
/// // Once per frame:
/// for event in consumer.consume_all() {
///     let model_data = event.handle.consume().expect("should not be consumed yet");
///     // ... gpu.upload_model(model_data);
/// }
/// ```
///
/// [`AssetRegistry`]: crate::AssetRegistry
/// [`Handle::consume`]: crate::Handle::consume
/// [`ModelData`]: crate::assets::model::ModelData
#[derive(Event, Clone, Debug)]
#[event("asset {handle:?} unloaded")]
pub struct AssetLoaded<T: Asset> {
    /// A cloneable handle to the loaded asset.
    ///
    /// Call [`Handle::consume`] to take ownership of the underlying data.
    /// In release builds the data can only be consumed once; subsequent
    /// calls on any clone of this handle return [`Error::AlreadyConsumed`].
    ///
    /// [`Handle::consume`]: crate::Handle::consume
    /// [`Error::AlreadyConsumed`]: crate::Error::AlreadyConsumed
    pub handle: Handle<T>,
}

/// Fired by [`AssetRegistry`] when an asset's CPU-side data has been fully
/// freed (i.e. the last [`Handle`] was dropped and the internal cleanup has
/// run).
///
/// Subscribe to this event to know when to release GPU-side resources that
/// were created in response to the corresponding [`AssetLoaded`] event.
///
/// Unlike [`AssetLoaded`], this event is not generic — it carries only the
/// [`AssetHandle`] identifier, because by this point the typed data is gone.
///
/// # Delivery guarantee
///
/// This event is dispatched from [`AssetRegistry::tick`], which must be
/// called once per frame. There is therefore a maximum one-frame delay
/// between the last [`Handle`] being dropped and this event being visible to
/// consumers.
///
/// # Example
///
/// ```rust
/// use assets::AssetUnloaded;
/// use events::Consumer;
///
/// # let event_manager = ::events::EventManager::new();
/// let consumer: Consumer<AssetUnloaded> = event_manager.subscribe();
///
/// // Once per frame:
/// for AssetUnloaded { handle } in consumer.consume_all() {
///     // ... GPU destroyes the model with [handle]
/// }
/// ```
///
/// [`AssetRegistry::tick`]: crate::AssetRegistry::tick
/// [`Handle`]: crate::Handle
#[derive(Event, Clone, Debug)]
#[event("asset {handle} unloaded")]
pub struct AssetUnloaded {
    /// The identifier of the asset that was unloaded.
    ///
    /// Use this to correlate with the handle stored when the corresponding
    /// [`AssetLoaded`] event was received.
    pub handle: AssetHandle,
}
