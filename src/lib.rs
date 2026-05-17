#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod engine;

#[doc(inline)]
pub use dirk_assets as assets;
#[doc(inline)]
pub use dirk_events as events;
#[doc(inline)]
pub use dirk_logging as logging;
#[doc(inline)]
pub use dirk_platform as platform;
#[doc(inline)]
pub use dirk_player as player;
#[doc(inline)]
pub use dirk_renderer as renderer;
#[doc(inline)]
pub use dirk_universe as universe;
#[doc(inline)]
pub use dirk_utils as utils;
#[doc(inline)]
pub use dirk_world as world;
