//! Default editor feature package for `DirkEngine`.
//!
//! This also contains loads of utility functions for working with the Editor.

use std::collections::BTreeMap;

use dirk_engine::editor::{
    EditorMenu, EditorMenuContext, EditorMenuDescriptor, EditorServices, EditorUiContext,
    EditorWindow, EditorWindowDescriptor, EditorWindowId, EditorWindowInfo,
};

mod settings;

struct MainMenu;

impl EditorMenu for MainMenu {
    fn descriptor(&self) -> EditorMenuDescriptor {
        EditorMenuDescriptor {
            title: "Main".to_owned(),
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        context: &mut EditorUiContext<'_>,
        _editor: &mut EditorMenuContext<'_>,
    ) -> anyhow::Result<()> {
        if ui.button("Exit").clicked() {
            context.handle.exit();
            ui.close();
        }
        Ok(())
    }
}

/// Window list menu capability for the default editor package.
pub struct WindowListMenu;

impl EditorMenu for WindowListMenu {
    fn descriptor(&self) -> EditorMenuDescriptor {
        EditorMenuDescriptor {
            title: "Windows".to_owned(),
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _context: &mut EditorUiContext<'_>,
        editor: &mut EditorMenuContext<'_>,
    ) -> anyhow::Result<()> {
        for (category, windows) in grouped_window_menu_entries(editor.windows()) {
            ui.menu_button(category, |ui| {
                for window in windows {
                    if ui.button(&window.title).clicked() {
                        editor.open_window(window.id);
                        ui.close();
                    }
                }
            });
        }
        Ok(())
    }
}

fn grouped_window_menu_entries(
    entries: &[EditorWindowInfo],
) -> Vec<(String, Vec<EditorWindowInfo>)> {
    let mut grouped: BTreeMap<String, Vec<EditorWindowInfo>> = BTreeMap::new();
    for entry in entries {
        if !entry.show_in_list {
            continue;
        }
        grouped
            .entry(entry.category.clone())
            .or_default()
            .push(entry.clone());
    }
    for windows in grouped.values_mut() {
        windows.sort_by(|left, right| left.title.cmp(&right.title));
    }
    grouped.into_iter().collect()
}

/// Engine diagnostics window capability for the default editor package.
pub struct EngineDiagnosticsWindow;

impl EditorWindow for EngineDiagnosticsWindow {
    fn descriptor(&self) -> EditorWindowDescriptor {
        EditorWindowDescriptor {
            title: "Engine".to_owned(),
            category: "Editor".to_owned(),
            default_open: true,
            show_in_list: true,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, context: &mut EditorUiContext<'_>) -> anyhow::Result<()> {
        let metadata = context.handle.metadata();

        ui.label(format!(
            "app: {} {}",
            metadata.app_name(),
            metadata.app_version()
        ));
        ui.label(format!(
            "engine: {} {}",
            metadata.engine_name(),
            metadata.engine_version()
        ));
        ui.label(format!("frame: {}", context.handle.frame()));
        ui.label(format!("status: {:?}", context.handle.status()));
        ui.label(format!("delta: {:.2} ms", context.delta_time() * 1_000.0));
        ui.label(format!("fps: {:.0}", 1.0 / context.delta_time()));
        Ok(())
    }
}
