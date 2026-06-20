//! [`EditorCommand`]s. Menus can submit commands that will be run by the editor.

use crate::editor::EditorWindowId;

/// Sends editor runtime commands from UI capabilities.
pub struct EditorCommandSender<'a> {
    commands: &'a mut Vec<EditorCommand>,
}

impl<'a> EditorCommandSender<'a> {
    pub(crate) fn new(commands: &'a mut Vec<EditorCommand>) -> Self {
        Self { commands }
    }

    /// Requests that an editor window be opened.
    pub fn open_window(&mut self, id: EditorWindowId) {
        self.commands.push(EditorCommand::OpenWindow(id));
    }
}

pub(crate) enum EditorCommand {
    OpenWindow(EditorWindowId),
}
