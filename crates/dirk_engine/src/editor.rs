//! Engine-owned editor subsystem and capability API.

use std::{
    any::type_name,
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow::Context as _;
use parking_lot::Mutex;

use crate::{EngineBuildContext, EngineHandle, Error, Result};

pub mod commands;
use commands::{EditorCommand, EditorCommandSender};

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
}

impl<'a> EditorUiContext<'a> {
    /// Returns the seconds elapsed since the previous engine tick.
    #[must_use]
    pub fn delta_time(&self) -> f64 {
        self.delta_time
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

/// Snapshot of a registered editor window.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditorWindowInfo {
    /// Stable window identifier.
    pub id: EditorWindowId,
    /// Window title.
    pub title: String,
    /// Window category.
    pub category: String,
    /// Whether the window is currently open.
    pub open: bool,
}

/// Global editor state snapshot and controls available to menu capabilities.
pub struct EditorMenuContext<'a> {
    windows: &'a [EditorWindowInfo],
    commands: EditorCommandSender<'a>,
}

impl<'a> EditorMenuContext<'a> {
    fn new(windows: &'a [EditorWindowInfo], commands: EditorCommandSender<'a>) -> Self {
        Self { windows, commands }
    }

    /// Returns all registered windows in registration order.
    #[must_use]
    pub fn windows(&self) -> &[EditorWindowInfo] {
        self.windows
    }

    /// Returns a registered window by id.
    #[must_use]
    pub fn window(&self, id: EditorWindowId) -> Option<&EditorWindowInfo> {
        self.windows.iter().find(|window| window.id == id)
    }

    /// Returns whether a registered window is currently open.
    #[must_use]
    pub fn is_open(&self, id: EditorWindowId) -> Option<bool> {
        self.window(id).map(|window| window.open)
    }

    /// Requests that an editor window be opened.
    pub fn open_window(&mut self, id: EditorWindowId) {
        self.commands.open_window(id);
    }

    /// Returns the editor command sender for this UI pass.
    pub fn commands(&mut self) -> &mut EditorCommandSender<'a> {
        &mut self.commands
    }
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

/// Editor window.
pub trait EditorWindow: Send + 'static {
    /// Returns this window's descriptor.
    fn descriptor(&self) -> EditorWindowDescriptor;

    /// Draws this window's contents.
    ///
    /// # Errors
    ///
    /// Returns an error when the window cannot complete drawing.
    fn ui(&mut self, ui: &mut egui::Ui, context: &mut EditorUiContext<'_>) -> anyhow::Result<()>;
}

/// Editor menu.
pub trait EditorMenu: Send + 'static {
    /// Returns this menu's descriptor.
    fn descriptor(&self) -> EditorMenuDescriptor;

    /// Draws this menu's contents.
    ///
    /// # Errors
    ///
    /// Returns an error when the menu cannot complete drawing.
    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        context: &mut EditorUiContext<'_>,
        editor: &mut EditorMenuContext<'_>,
    ) -> anyhow::Result<()>;
}

/// Shared state with editor services.
///
/// Services are extensions of the editor. Notably, windows & menus.
#[derive(Clone)]
pub struct EditorServices {
    state: Arc<Mutex<EditorServicesState>>,
    next_window_id: Arc<AtomicU64>,
    next_menu_id: Arc<AtomicU64>,
}

impl EditorServices {
    /// Creates empty editor services.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(EditorServicesState::new())),
            next_window_id: Arc::new(AtomicU64::new(0)),
            next_menu_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Registers an editor-native window.
    pub fn add_window<W>(&self, window: W) -> EditorWindowId
    where
        W: EditorWindow,
    {
        let descriptor = window.descriptor();
        let default_open = descriptor.default_open;
        let id = EditorWindowId(self.next_window_id.fetch_add(1, Ordering::Relaxed));
        let mut state = self.state.lock();
        state.windows.push(RegisteredWindow {
            id,
            descriptor,
            window: Box::new(window),
        });
        state
            .window_states
            .insert(id, WindowState { open: default_open });
        id
    }

    /// Registers an editor-native window from a callback.
    pub fn add_window_fn<F>(&self, descriptor: EditorWindowDescriptor, ui: F) -> EditorWindowId
    where
        F: FnMut(&mut egui::Ui, &mut EditorUiContext<'_>) -> anyhow::Result<()> + Send + 'static,
    {
        self.add_window(FnEditorWindow { descriptor, ui })
    }

    /// Registers an editor menu capability.
    pub fn add_menu<M>(&self, menu: M) -> EditorMenuId
    where
        M: EditorMenu,
    {
        let descriptor = menu.descriptor();
        let id = EditorMenuId(self.next_menu_id.fetch_add(1, Ordering::Relaxed));
        self.state.lock().menus.push(RegisteredMenu {
            id,
            title: descriptor.title,
            menu: Box::new(menu),
        });
        id
    }

    /// Registers an editor menu capability from a callback.
    pub fn add_menu_fn<F>(&self, descriptor: EditorMenuDescriptor, ui: F) -> EditorMenuId
    where
        F: FnMut(
                &mut egui::Ui,
                &mut EditorUiContext<'_>,
                &mut EditorMenuContext<'_>,
            ) -> anyhow::Result<()>
            + Send
            + 'static,
    {
        self.add_menu(FnEditorMenu { descriptor, ui })
    }

    /// Updates a registered window's open state.
    pub fn set_open(&self, id: EditorWindowId, open: bool) {
        if let Some(state) = self.state.lock().window_states.get_mut(&id) {
            state.open = open;
        }
    }

    /// Returns whether a registered window is currently open.
    #[must_use]
    pub fn is_open(&self, id: EditorWindowId) -> Option<bool> {
        self.state
            .lock()
            .window_states
            .get(&id)
            .map(|state| state.open)
    }

    /// Returns registered windows in registration order.
    #[must_use]
    pub fn windows(&self) -> Vec<EditorWindowInfo> {
        self.state.lock().windows()
    }

    /// Returns a registered window by id.
    #[must_use]
    pub fn window(&self, id: EditorWindowId) -> Option<EditorWindowInfo> {
        self.state
            .lock()
            .windows()
            .into_iter()
            .find(|window| window.id == id)
    }

    /// Returns the registered window count.
    #[must_use]
    pub fn window_count(&self) -> usize {
        self.state.lock().windows.len()
    }

    /// Returns the registered menu count.
    #[must_use]
    pub fn menu_count(&self) -> usize {
        self.state.lock().menus.len()
    }

    /// Returns registered window titles in registration order.
    #[must_use]
    pub fn window_titles(&self) -> Vec<String> {
        self.state
            .lock()
            .windows
            .iter()
            .map(|window| window.descriptor.title.clone())
            .collect()
    }

    /// Returns registered menu titles in registration order.
    #[must_use]
    pub fn menu_titles(&self) -> Vec<String> {
        self.state
            .lock()
            .menus
            .iter()
            .map(|menu| menu.title.clone())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn open_window_for_tests(&self, id: EditorWindowId) {
        self.state
            .lock()
            .apply_commands(vec![EditorCommand::OpenWindow(id)]);
    }

    /// Renders editor menus and windows.
    ///
    /// # Errors
    ///
    /// Returns the first error emitted by a registered capability callback.
    pub fn render_ui(
        &self,
        ctx: &egui::Context,
        context: &EditorRenderContext<'_>,
    ) -> anyhow::Result<()> {
        self.state.lock().render(ctx, context)
    }
}

impl Default for EditorServices {
    fn default() -> Self {
        Self::new()
    }
}

struct EditorServicesState {
    windows: Vec<RegisteredWindow>,
    window_states: HashMap<EditorWindowId, WindowState>,
    menus: Vec<RegisteredMenu>,
}

impl EditorServicesState {
    fn new() -> Self {
        Self {
            windows: Vec::new(),
            window_states: HashMap::new(),
            menus: Vec::new(),
        }
    }

    fn render(
        &mut self,
        ctx: &egui::Context,
        context: &EditorRenderContext<'_>,
    ) -> anyhow::Result<()> {
        let mut editor_commands = Vec::new();
        self.render_menus(ctx, context, &mut editor_commands)?;
        self.apply_commands(std::mem::take(&mut editor_commands));
        self.render_windows(ctx, context)?;
        self.apply_commands(editor_commands);
        Ok(())
    }

    fn render_menus(
        &mut self,
        ctx: &egui::Context,
        context: &EditorRenderContext<'_>,
        editor_commands: &mut Vec<EditorCommand>,
    ) -> anyhow::Result<()> {
        if self.menus.is_empty() {
            return Ok(());
        }

        let windows = self.windows();
        let mut menu_context =
            EditorMenuContext::new(&windows, EditorCommandSender::new(editor_commands));
        let mut context = EditorUiContext {
            delta_time: context.delta_time(),
            handle: context.handle,
            universe: context.universe,
            editor: context.editor,
        };

        let mut result = Ok(());
        egui::TopBottomPanel::top("dirk_editor_menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                for menu in &mut self.menus {
                    ui.menu_button(menu.title.clone(), |ui| {
                        if result.is_ok() {
                            result = menu
                                .menu
                                .ui(ui, &mut context, &mut menu_context)
                                .with_context(|| format!("menu `{}` failed to render", menu.title));
                        }
                    });
                }
            });
        });

        result
    }
    fn render_windows(
        &mut self,
        ctx: &egui::Context,
        context: &EditorRenderContext<'_>,
    ) -> anyhow::Result<()> {
        for window in &mut self.windows {
            if !self
                .window_states
                .get(&window.id)
                .is_some_and(|state| state.open)
            {
                continue;
            }

            let mut open = true;
            let title = window.descriptor.title.clone();
            let mut result = Ok(());
            let mut context = EditorUiContext {
                delta_time: context.delta_time(),
                handle: context.handle,
                universe: context.universe,
                editor: context.editor,
            };

            egui::Window::new(title.clone())
                .open(&mut open)
                .show(ctx, |ui| {
                    result = window
                        .window
                        .ui(ui, &mut context)
                        .with_context(|| format!("window `{title}` failed to render"));
                });

            if let Some(state) = self.window_states.get_mut(&window.id) {
                state.open = open;
            }

            result?;
        }
        Ok(())
    }

    fn windows(&self) -> Vec<EditorWindowInfo> {
        self.windows
            .iter()
            .map(|window| EditorWindowInfo {
                id: window.id,
                title: window.descriptor.title.clone(),
                category: window.descriptor.category.clone(),
                open: self
                    .window_states
                    .get(&window.id)
                    .is_some_and(|state| state.open),
            })
            .collect()
    }

    fn apply_commands(&mut self, commands: Vec<EditorCommand>) {
        for command in commands {
            match command {
                EditorCommand::OpenWindow(id) => {
                    if let Some(state) = self.window_states.get_mut(&id) {
                        state.open = true;
                    }
                }
            }
        }
    }
}

struct RegisteredWindow {
    id: EditorWindowId,
    descriptor: EditorWindowDescriptor,
    window: Box<dyn EditorWindow>,
}

struct WindowState {
    open: bool,
}

pub(crate) struct EditorRuntime {
    services: EditorServices,
    subsystems: Vec<Box<dyn EditorSubsystem>>,
}

impl EditorRuntime {
    pub(crate) fn new(services: EditorServices, subsystems: Vec<Box<dyn EditorSubsystem>>) -> Self {
        Self {
            services,
            subsystems,
        }
    }

    #[cfg(test)]
    pub(crate) fn empty_for_tests() -> Self {
        Self::new(EditorServices::new(), Vec::new())
    }

    pub(crate) fn start(
        &mut self,
        engine: &EngineHandle,
        universe: &dirk_universe::Universe,
    ) -> Result<()> {
        for subsystem in &mut self.subsystems {
            let name = subsystem.name();
            let mut context = EditorStartContext {
                engine,
                universe,
                editor: &self.services,
            };
            subsystem
                .start(&mut context)
                .map_err(|source| Error::EditorSubsystemFailedStart { name, source })?;
        }
        Ok(())
    }

    pub(crate) fn tick(
        &mut self,
        delta_time: f64,
        engine: &EngineHandle,
        universe: &dirk_universe::Universe,
    ) -> crate::Result<()> {
        for subsystem in &mut self.subsystems {
            let name = subsystem.name();
            let mut context = EditorTickContext {
                delta_time,
                engine,
                universe,
                editor: &self.services,
            };
            subsystem
                .tick(&mut context)
                .map_err(|source| Error::EditorSubsystemFailedTick { name, source })?;
        }
        Ok(())
    }

    pub(crate) fn shutdown(
        &mut self,
        engine: &EngineHandle,
        universe: &dirk_universe::Universe,
    ) -> crate::Result<()> {
        while let Some(mut subsystem) = self.subsystems.pop() {
            let name = subsystem.name();
            let mut context = EditorShutdownContext {
                engine,
                universe,
                editor: &self.services,
            };
            subsystem
                .shutdown(&mut context)
                .map_err(|source| Error::EditorSubsystemFailedShutdown { name, source })?;
        }
        Ok(())
    }
}

