#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod engine;

#[doc(inline)]
pub use dirk_assets as assets;
#[cfg(feature = "editor")]
#[doc(inline)]
pub use dirk_editor as editor;
#[doc(inline)]
pub use dirk_events as events;
#[doc(inline)]
pub use dirk_platform as platform;
#[doc(inline)]
pub use dirk_player as player;
#[doc(inline)]
pub use dirk_renderer as renderer;
#[doc(inline)]
pub use dirk_threads as threads;
#[doc(inline)]
pub use dirk_universe as universe;
#[doc(inline)]
pub use dirk_utils as utils;
#[doc(inline)]
pub use dirk_world as world;
