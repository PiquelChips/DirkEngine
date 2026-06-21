//! Default editor feature package for `DirkEngine`.
//!
//! This also contains loads of utility functions for working with the Editor.

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
