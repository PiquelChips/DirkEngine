//! Public and internal events emitted by the asset subsystem.

use crate::{Asset, AssetHandle, Handle};
use dirk_events::Event;

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
/// grants access to the asset data via [`Handle::take`].
///
/// # Type parameter
///
/// `T` is the concrete [`Asset`] implementation (e.g. [`Model`]).
/// Subscriptions are type-specific — a consumer of
/// `AssetLoaded<Model>` will not receive audio or texture load events.
///
/// # Example
///
/// ```rust
/// use dirk_assets::{AssetLoaded, Handle, Model};
/// use dirk_events::{Consumer, EventManager};
///
/// # let events = EventManager::new();
/// let mut consumer: Consumer<AssetLoaded<Model>> = events.subscribe();
///
/// // Once per frame:
/// for event in consumer.consume_all() {
///     let model_data = event.handle.take().expect("should not be consumed yet");
///     // ... gpu.upload_model(model_data);
/// }
/// ```
///
/// [`AssetRegistry`]: crate::AssetRegistry
/// [`Handle::take`]: crate::Handle::take
/// [`Model`]: crate::Model
#[derive(Event, Clone, Debug)]
#[event("asset {handle:?} unloaded")]
pub struct AssetLoaded<T: Asset> {
    /// A cloneable handle to the loaded asset.
    ///
    /// Call [`Handle::take`] to take ownership of the underlying data.
    /// In release builds the data can only be taken once; subsequent
    /// calls on any clone of this handle return [`Error::AlreadyTaken`].
    ///
    /// [`Handle::take`]: crate::Handle::take
    /// [`Error::AlreadyTaken`]: crate::Error::AlreadyTaken
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
/// use dirk_assets::AssetUnloaded;
/// use dirk_events::Consumer;
///
/// # let events = dirk_events::EventManager::new();
/// let mut consumer: Consumer<AssetUnloaded> = events.subscribe();
///
/// // Once per frame:
/// for AssetUnloaded { handle } in consumer.consume_all() {
///     // ... GPU destroyes the model with [handle]
/// }
/// ```
///
/// [`AssetRegistry`]: crate::AssetRegistry
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

/// Any system can fire this event, the [`AssetRegistry`] will respond
/// by loading the requested asset.
#[derive(Event, Clone, Debug)]
pub struct LoadAsset(pub AssetHandle);
