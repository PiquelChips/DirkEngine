use events::Event;
use macros::Event;
use winit::window::WindowId;

#[derive(Debug, Clone, Event)]
#[event("App Exit with code {0}")]
pub struct AppExit(pub i32);

/// All platform events the engine may need to react to.
#[derive(Debug, Clone, Event)]
pub enum PlatformEvent {
    /// The OS is asking us to close this window.
    WindowCloseRequested { id: WindowId },
    /// The window surface was resized (e.g. user dragged the edge).
    WindowResized {
        id: WindowId,
        width: u32,
        height: u32,
    },
    /// The window gained or lost focus.
    WindowFocusChanged { id: WindowId, focused: bool },
    /// The window is hidden or fully covered.
    WindowOccluded { id: WindowId, occluded: bool },
}
