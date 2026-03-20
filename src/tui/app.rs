use crate::db::Database;
use crate::model::Command;
use anyhow::Result;

/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Normal navigation mode
    Normal,
    /// Insert/edit mode for input
    Insert,
    /// Command mode (after pressing /)
    Command,
}

/// The TUI application state
pub struct App {
    /// All commands in the database
    pub commands: Vec<Command>,
    /// Currently selected command index
    pub selected: usize,
    /// Current input mode
    pub input_mode: InputMode,
    /// Input buffer for command mode
    pub input_buffer: String,
    /// Current message to display
    pub message: Option<String>,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Filtered command list (when searching)
    pub filtered_commands: Vec<Command>,
    /// Whether we're showing filtered results
    pub is_filtered: bool,
    /// Database reference
    db: Database,
}

impl App {
    /// Create a new app instance
    pub fn new(db: Database) -> Result<Self> {
        let commands = db.list_commands()?;
        Ok(Self {
            commands,
            selected: 0,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            message: None,
            should_quit: false,
            filtered_commands: Vec::new(),
            is_filtered: false,
            db,
        })
    }

    /// Get the currently displayed commands
    pub fn get_commands(&self) -> &Vec<Command> {
        if self.is_filtered {
            &self.filtered_commands
        } else {
            &self.commands
        }
    }

    /// Get the currently selected command
    pub fn get_selected_command(&self) -> Option<&Command> {
        self.get_commands().get(self.selected)
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        let len = self.get_commands().len();
        if self.selected < len.saturating_sub(1) {
            self.selected += 1;
        }
    }

    /// Select the first command
    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    /// Select the last command
    pub fn select_last(&mut self) {
        let len = self.get_commands().len();
        if len > 0 {
            self.selected = len - 1;
        }
    }

    /// Enter command mode
    pub fn enter_command_mode(&mut self) {
        self.input_mode = InputMode::Command;
        self.input_buffer = String::from("/");
    }

    /// Enter insert mode
    pub fn enter_insert_mode(&mut self) {
        self.input_mode = InputMode::Insert;
        self.input_buffer.clear();
    }

    /// Exit to normal mode
    pub fn exit_input_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    /// Handle a character input in insert/command mode
    pub fn handle_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    /// Handle backspace in input mode
    pub fn handle_backspace(&mut self) {
        self.input_buffer.pop();
        // If in command mode and buffer is empty (just "/"), exit to normal mode
        if self.input_mode == InputMode::Command && self.input_buffer == "/" {
            self.exit_input_mode();
        }
    }

    /// Execute a TUI command
    pub fn execute_command(&mut self) -> Result<()> {
        let cmd = self.input_buffer.trim().to_string();

        // Parse command
        if cmd.starts_with('/') {
            let cmd_body = cmd[1..].trim();

            if cmd_body.is_empty() {
                self.message = Some("Empty command".to_string());
            } else if cmd_body == "q" || cmd_body == "quit" {
                self.should_quit = true;
            } else if cmd_body == "help" {
                self.message = Some("Commands: /add <name> <cmd>, /del, /edit, /search <term>, /fav, /unfav, /q".to_string());
            } else if let Some(rest) = cmd_body.strip_prefix("add ") {
                self.handle_add_command(rest)?;
            } else if cmd_body == "del" || cmd_body == "delete" {
                self.handle_delete_command()?;
            } else if cmd_body == "edit" {
                self.handle_edit_command()?;
            } else if let Some(rest) = cmd_body.strip_prefix("search ") {
                self.handle_search_command(rest)?;
            } else if cmd_body == "search" {
                self.clear_filter();
                self.message = Some("Cleared search filter".to_string());
            } else if cmd_body == "fav" {
                self.handle_toggle_favorite(true)?;
            } else if cmd_body == "unfav" {
                self.handle_toggle_favorite(false)?;
            } else {
                self.message = Some(format!("Unknown command: {}", cmd_body));
            }
        } else {
            // Search mode - filter commands
            self.handle_search_command(&cmd)?;
        }

        self.exit_input_mode();
        Ok(())
    }

    /// Handle /add command
    fn handle_add_command(&mut self, args: &str) -> Result<()> {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        if parts.len() < 2 {
            self.message = Some("Usage: /add <name> <command>".to_string());
            return Ok(());
        }

        let name = parts[0].to_string();
        let command = parts[1].to_string();

        let cmd = Command::new(name.clone(), command);
        self.db.add_command(&cmd)?;
        self.refresh_commands()?;
        self.message = Some(format!("Added command: {}", name));
        Ok(())
    }

    /// Handle /del command
    fn handle_delete_command(&mut self) -> Result<()> {
        if let Some(cmd) = self.get_selected_command() {
            let name = cmd.name.clone();
            self.db.delete_command(&name)?;
            self.refresh_commands()?;
            self.message = Some(format!("Deleted command: {}", name));
        } else {
            self.message = Some("No command selected".to_string());
        }
        Ok(())
    }

    /// Handle /edit command - opens in editor (simplified: just shows message)
    fn handle_edit_command(&mut self) -> Result<()> {
        if let Some(cmd) = self.get_selected_command() {
            self.message = Some(format!("Edit: {} - Use CLI: vo edit {}", cmd.name, cmd.name));
        } else {
            self.message = Some("No command selected".to_string());
        }
        Ok(())
    }

    /// Handle /search command
    fn handle_search_command(&mut self, term: &str) -> Result<()> {
        let term = term.trim();
        if term.is_empty() {
            self.clear_filter();
            return Ok(());
        }

        self.filtered_commands = self.db.search_commands(term)?;
        self.is_filtered = true;
        self.selected = 0;

        if self.filtered_commands.is_empty() {
            self.message = Some("No matching commands found".to_string());
        } else {
            self.message = Some(format!("Found {} matching commands", self.filtered_commands.len()));
        }
        Ok(())
    }

    /// Handle favorite toggle
    fn handle_toggle_favorite(&mut self, favorite: bool) -> Result<()> {
        if let Some(cmd) = self.get_selected_command() {
            let name = cmd.name.clone();
            self.db.toggle_favorite(&name, favorite)?;
            self.refresh_commands()?;
            let status = if favorite { "favorited" } else { "unfavorited" };
            self.message = Some(format!("Command '{}' {}", name, status));
        } else {
            self.message = Some("No command selected".to_string());
        }
        Ok(())
    }

    /// Clear search filter
    pub fn clear_filter(&mut self) {
        self.is_filtered = false;
        self.filtered_commands.clear();
        self.selected = 0;
    }

    /// Refresh commands from database
    pub fn refresh_commands(&mut self) -> Result<()> {
        self.commands = self.db.list_commands()?;
        if self.is_filtered {
            // Re-apply filter
            let filter = self.input_buffer.trim_start_matches('/').trim().to_string();
            if !filter.is_empty() {
                self.filtered_commands = self.commands
                    .iter()
                    .filter(|c| {
                        c.name.contains(&filter) ||
                        c.command.contains(&filter) ||
                        c.description.as_deref().unwrap_or("").contains(&filter)
                    })
                    .cloned()
                    .collect();
            }
        }

        // Ensure selection is valid
        let len = self.get_commands().len();
        if self.selected >= len && len > 0 {
            self.selected = len - 1;
        }

        Ok(())
    }

    /// Execute the selected command
    pub fn execute_selected(&mut self) -> Result<Option<i32>> {
        if let Some(cmd) = self.get_selected_command() {
            let cmd = cmd.clone();
            let exit_code = crate::command::execute_command(&cmd, &[], &self.db)?;
            self.refresh_commands()?;
            return Ok(Some(exit_code));
        }
        Ok(None)
    }

    /// Quit the application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn create_test_app() -> App {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("test1".to_string(), "echo 1".to_string())).unwrap();
        db.add_command(&Command::new("test2".to_string(), "echo 2".to_string())).unwrap();
        App::new(db).unwrap()
    }

    #[test]
    fn test_app_new() {
        let app = create_test_app();
        assert_eq!(app.commands.len(), 2);
        assert_eq!(app.selected, 0);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn test_navigation() {
        let mut app = create_test_app();

        app.select_next();
        assert_eq!(app.selected, 1);

        app.select_next();
        assert_eq!(app.selected, 1); // Should stay at last item

        app.select_prev();
        assert_eq!(app.selected, 0);

        app.select_prev();
        assert_eq!(app.selected, 0); // Should stay at first item

        app.select_last();
        assert_eq!(app.selected, 1);

        app.select_first();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn test_input_mode() {
        let mut app = create_test_app();

        app.enter_command_mode();
        assert_eq!(app.input_mode, InputMode::Command);
        assert_eq!(app.input_buffer, "/");

        app.handle_char('t');
        app.handle_char('e');
        app.handle_char('s');
        app.handle_char('t');
        assert_eq!(app.input_buffer, "/test");

        app.handle_backspace();
        assert_eq!(app.input_buffer, "/tes");

        app.exit_input_mode();
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn test_get_selected_command() {
        let app = create_test_app();
        let cmd = app.get_selected_command();
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().name, "test1");
    }
}
