//! Engine-owned editor subsystem and capability API.

use std::{
    any::type_name,
    collections::{BTreeMap, HashMap},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::Context as _;
use parking_lot::Mutex;
use tracing::warn;

use crate::{EngineBuildContext, EngineHandle, errors::Error};

/// Editor lifecycle subsystem owned by the engine.
pub trait EditorSubsystem: Send + 'static {
    /// The subsystem name used for diagnostics.
    fn name(&self) -> &'static str;

    /// Starts the editor subsystem after regular subsystems have started.
    ///
    /// # Errors
    ///
    /// Returns an error if startup fails.
    fn start(&mut self, _context: &mut EditorStartContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Advances the editor subsystem by one engine tick.
    ///
    /// # Errors
    ///
    /// Returns an error if ticking fails.
    fn tick(&mut self, _context: &mut EditorTickContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Shuts the editor subsystem down.
    ///
    /// # Errors
    ///
    /// Returns an error if shutdown fails.
    fn shutdown(&mut self, _context: &mut EditorShutdownContext<'_>) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Build-time context for editor subsystem factories.
pub struct EditorBuildContext<'a> {
    /// Mutable engine build context for core services and typed resources.
    pub engine: &'a mut EngineBuildContext,
    /// Shared editor services available to editor subsystems.
    pub editor: &'a EditorServices,
}

/// Context passed to editor subsystem startup.
pub struct EditorStartContext<'a> {
    /// Shared engine handle.
    pub engine: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

/// Context passed to editor subsystem ticks.
pub struct EditorTickContext<'a> {
    /// Seconds elapsed since the previous engine tick.
    delta_time: f64,
    /// Shared engine handle.
    pub engine: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

impl EditorTickContext<'_> {
    /// Returns the seconds elapsed since the previous engine tick.
    #[must_use]
    pub fn delta_time(&self) -> f64 {
        self.delta_time
    }
}

/// Context passed to editor subsystem shutdown.
pub struct EditorShutdownContext<'a> {
    /// Shared engine handle.
    pub engine: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

/// Per-frame context passed to editor rendering.
pub struct EditorRenderContext<'a> {
    /// Seconds elapsed since the previous engine tick.
    delta_time: f64,
    /// Shared engine handle.
    pub handle: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
}

impl<'a> EditorRenderContext<'a> {
    /// Creates a context for rendering editor UI.
    #[must_use]
    pub fn new(
        delta_time: f64,
        handle: &'a EngineHandle,
        universe: &'a dirk_universe::Universe,
        editor: &'a EditorServices,
    ) -> Self {
        Self {
            delta_time,
            handle,
            universe,
            editor,
        }
    }

    /// Returns the seconds elapsed since the previous engine tick.
    #[must_use]
    pub fn delta_time(&self) -> f64 {
        self.delta_time
    }
}

/// Per-frame context passed to editor UI capabilities.
pub struct EditorUiContext<'a> {
    /// Seconds elapsed since the previous engine tick.
    delta_time: f64,
    /// Shared engine handle.
    pub handle: &'a EngineHandle,
    /// Read-only ECS universe.
    pub universe: &'a dirk_universe::Universe,
    /// Shared editor services.
    pub editor: &'a EditorServices,
    commands: EditorCommandSender<'a>,
    window_menu_entries: Option<&'a WindowMenuEntries>,
}

impl<'a> EditorUiContext<'a> {
    /// Returns the seconds elapsed since the previous engine tick.
    #[must_use]
    pub fn delta_time(&self) -> f64 {
        self.delta_time
    }

    /// Requests that an editor window be opened.
    pub fn open_window(&mut self, id: EditorWindowId) {
        self.commands.open_window(id);
    }

    /// Returns the editor command sender for this UI pass.
    pub fn commands(&mut self) -> &mut EditorCommandSender<'a> {
        &mut self.commands
    }

    /// Draws the built-in categorized window list menu.
    pub fn windows_menu_ui(&mut self, ui: &mut egui::Ui) {
        let Some(window_menu_entries) = self.window_menu_entries else {
            warn!("attempted to draw window menu. no window menu entries found");
            return;
        };
        let commands = &mut self.commands;

        for (category, windows) in window_menu_entries {
            ui.menu_button(category, |ui| {
                for (id, title) in windows {
                    if ui.button(title).clicked() {
                        commands.open_window(*id);
                        ui.close();
                    }
                }
            });
        }
    }
}

/// Static metadata for an editor-native window capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorWindowDescriptor {
    /// Window title.
    pub title: String,
    /// Window category used by the built-in Windows menu.
    pub category: String,
    /// Whether the window starts open after registration.
    pub default_open: bool,
}

/// Static metadata for an editor menu capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorMenuDescriptor {
    /// Menu title shown in the editor menu bar.
    pub title: String,
}

/// Stable editor window identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EditorWindowId(u64);

impl EditorWindowId {
    /// Returns the raw numeric identifier.
    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Stable editor menu identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct EditorMenuId(u64);

impl EditorMenuId {
    /// Returns the raw numeric identifier.
    #[must_use]
    pub fn raw(self) -> u64 {
        self.0
    }
}

