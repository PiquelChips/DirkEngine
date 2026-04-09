use events::Event;
use macros::Event;
use winit::window::WindowId;

#[derive(Debug, Clone, Event)]
#[event("App Exit with code {0}")]
pub struct AppExit(pub i32);

/// All platform events.
/// These are specific to global platform stuff. No input or window
/// specific events are listed here.
/// The only exeptions are window closing and creating events.
#[derive(Debug, Clone, Event)]
pub enum PlatformEvent {
    /// Window created event.
    WindowCreated { id: WindowId },
    /// The OS is asking us to close this window.
    /// This should be when the renderer window is destroyed.
    WindowCloseRequested { id: WindowId },
    /// The window has been finally destroyed. This event should not be
    /// used as all window related objects should have been destroyed on
    /// [Self::WindowCloseRequested].
    WindowDestroyed { id: WindowId },
}

/// All window specific events.
#[derive(Debug, Clone, Event)]
pub enum WindowEvent {
    Resized {
        id: WindowId,
        width: u32,
        height: u32,
    },
    FocusChanged {
        id: WindowId,
        focused: bool,
    },
    Occluded {
        id: WindowId,
        occluded: bool,
    },
    ThemeChanged {
        id: WindowId,
        theme: winit::window::Theme,
    },
}

impl WindowEvent {
    pub fn id(&self) -> &WindowId {
        match self {
            Self::Resized { id, .. } => id,
            Self::Occluded { id, .. } => id,
            Self::FocusChanged { id, .. } => id,
            Self::ThemeChanged { id, .. } => id,
        }
    }
}
