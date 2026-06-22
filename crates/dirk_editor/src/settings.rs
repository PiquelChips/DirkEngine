use crate::{
    EDITOR_CATEGORY, EditorMenu, EditorMenuContext, EditorMenuDescriptor, EditorServices,
    EditorUiContext, EditorWindow, EditorWindowDescriptor, EditorWindowId,
};

pub fn register_capabilities(services: &EditorServices) {
    let editor_settings = services.add_window(EditorSettingsWindow);
    services.add_menu(SettingsMenu {
        settings_windows: vec![editor_settings],
    });
}

struct SettingsMenu {
    settings_windows: Vec<EditorWindowId>,
}

impl EditorMenu for SettingsMenu {
    fn descriptor(&self) -> EditorMenuDescriptor {
        EditorMenuDescriptor {
            title: "Settings".to_owned(),
        }
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _context: &mut EditorUiContext<'_>,
        editor: &mut EditorMenuContext<'_>,
    ) -> anyhow::Result<()> {
        for window_id in &self.settings_windows {
            let Some(window) = editor.window(*window_id) else {
                continue;
            };

            if ui.button(&window.title).clicked() {
                editor.open_window(window.id);
                ui.close();
            }
        }
        Ok(())
    }
}

struct EditorSettingsWindow;

impl EditorWindow for EditorSettingsWindow {
    fn descriptor(&self) -> EditorWindowDescriptor {
        EditorWindowDescriptor {
            title: "Settings".to_owned(),
            category: EDITOR_CATEGORY.to_owned(),
            default_open: false,
            show_in_list: false,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _context: &mut EditorUiContext<'_>) -> anyhow::Result<()> {
        ui.label("Settings");
        Ok(())
    }
}
