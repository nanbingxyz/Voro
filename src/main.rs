mod cli;
mod command;
mod db;
mod model;
mod tui;
mod utils;

use anyhow::{bail, Result};
use clap::Parser;
use crossterm::{
    event::{KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

use crate::cli::{Cli, Commands};
use crate::db::Database;
use crate::model::Command;
use crate::tui::{App, EventHandler, InputMode};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::new()?;

    match cli.command {
        Some(cmd) => handle_command(cmd, &db),
        None => {
            if cli.name.is_some() {
                // Execute command by name
                handle_execute(&cli, &db)
            } else {
                // Launch TUI
                run_tui(db)
            }
        }
    }
}

/// Handle executing a command by name
fn handle_execute(cli: &Cli, db: &Database) -> Result<()> {
    let name = cli.name.as_ref().unwrap();

    let cmd = db.get_command(name)?
        .ok_or_else(|| anyhow::anyhow!("Command '{}' not found", name))?;

    // Check if passthrough is enabled and args are provided
    let args = if cmd.passthrough {
        cli.args.clone()
    } else {
        vec![]
    };

    let exit_code = command::execute_command(&cmd, &args, db)?;
    std::process::exit(exit_code);
}

/// Handle CLI subcommands
fn handle_command(cmd: Commands, db: &Database) -> Result<()> {
    match cmd {
        Commands::Add { name, command, desc, cat, tags, confirm, passthrough } => {
            let mut cmd = Command::new(name, command);
            cmd.description = desc;
            cmd.category = cat;
            cmd.tags = tags;
            cmd.confirm = confirm;
            cmd.passthrough = passthrough;

            db.add_command(&cmd)?;
            println!("Added command: {}", cmd.name);
        }

        Commands::Edit { name, command, desc, cat, tags, editor } => {
            let existing = db.get_command(&name)?
                .ok_or_else(|| anyhow::anyhow!("Command '{}' not found", name))?;

            if editor {
                // Open in editor
                let editor_cmd = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
                let temp_file = std::env::temp_dir().join("voro_edit.txt");

                // Write current command to temp file
                let content = format!(
                    "# Edit command: {}\n# Lines starting with # are ignored\n\n# Command:\n{}\n\n# Description:\n{}\n\n# Category:\n{}\n\n# Tags (comma-separated):\n{}",
                    name,
                    existing.command,
                    existing.description.as_deref().unwrap_or(""),
                    existing.category.as_deref().unwrap_or(""),
                    existing.tags.as_deref().unwrap_or("")
                );
                std::fs::write(&temp_file, content)?;

                // Open editor
                let status = std::process::Command::new(&editor_cmd)
                    .arg(&temp_file)
                    .status()?;

                if !status.success() {
                    bail!("Editor exited with non-zero status");
                }

                // Parse edited file
                let edited = std::fs::read_to_string(&temp_file)?;
                let mut lines: Vec<String> = edited
                    .lines()
                    .filter(|l| !l.starts_with('#'))
                    .map(String::from)
                    .collect();

                // Remove empty lines from the end
                while lines.last().map_or(false, |l| l.trim().is_empty()) {
                    lines.pop();
                }

                // Parse sections
                let mut new_cmd = existing.clone();
                let mut current_section = None;
                let mut cmd_str = String::new();
                let mut desc_str = String::new();
                let mut cat_str = String::new();
                let mut tags_str = String::new();

                for line in &lines {
                    if line.starts_with("Command:") {
                        current_section = Some("command");
                        continue;
                    } else if line.starts_with("Description:") {
                        current_section = Some("description");
                        continue;
                    } else if line.starts_with("Category:") {
                        current_section = Some("category");
                        continue;
                    } else if line.starts_with("Tags") {
                        current_section = Some("tags");
                        continue;
                    }

                    match current_section {
                        Some("command") => {
                            if !cmd_str.is_empty() { cmd_str.push(' '); }
                            cmd_str.push_str(line.trim());
                        }
                        Some("description") => {
                            if !desc_str.is_empty() { desc_str.push(' '); }
                            desc_str.push_str(line.trim());
                        }
                        Some("category") => {
                            cat_str = line.trim().to_string();
                        }
                        Some("tags") => {
                            tags_str = line.trim().to_string();
                        }
                        _ => {}
                    }
                }

                if !cmd_str.is_empty() {
                    new_cmd.command = cmd_str;
                }
                new_cmd.description = if desc_str.is_empty() { None } else { Some(desc_str) };
                new_cmd.category = if cat_str.is_empty() { None } else { Some(cat_str) };
                new_cmd.tags = if tags_str.is_empty() { None } else { Some(tags_str) };

                db.update_command(&new_cmd)?;
                println!("Updated command: {}", name);

                // Clean up
                std::fs::remove_file(&temp_file).ok();
            } else {
                // Flag mode
                let mut updated = existing.clone();

                if let Some(c) = command {
                    updated.command = c;
                }
                if desc.is_some() {
                    updated.description = desc;
                }
                if cat.is_some() {
                    updated.category = cat;
                }
                if tags.is_some() {
                    updated.tags = tags;
                }

                db.update_command(&updated)?;
                println!("Updated command: {}", name);
            }
        }

        Commands::Del { name } => {
            let deleted = db.delete_command(&name)?;
            if deleted {
                println!("Deleted command: {}", name);
            } else {
                bail!("Command '{}' not found", name);
            }
        }

        Commands::Get { name } => {
            let cmd = db.get_command(&name)?
                .ok_or_else(|| anyhow::anyhow!("Command '{}' not found", name))?;

            println!("Name: {}", cmd.name);
            println!("Command: {}", cmd.command);
            if let Some(ref desc) = cmd.description {
                println!("Description: {}", desc);
            }
            if let Some(ref cat) = cmd.category {
                println!("Category: {}", cat);
            }
            if let Some(ref tags) = cmd.tags {
                println!("Tags: {}", tags);
            }
            println!("Favorite: {}", if cmd.favorite { "Yes" } else { "No" });
            println!("Confirm: {}", if cmd.confirm { "Yes" } else { "No" });
            println!("Passthrough: {}", if cmd.passthrough { "Yes" } else { "No" });
            println!("Use count: {}", cmd.use_count);
            if let Some(ref last_used) = cmd.last_used_at {
                println!("Last used: {}", last_used);
            }
        }

        Commands::Ls { cat, tag } => {
            let commands = if let Some(category) = cat {
                db.list_by_category(&category)?
            } else if let Some(t) = tag {
                db.list_by_tag(&t)?
            } else {
                db.list_commands()?
            };

            if commands.is_empty() {
                println!("No commands found.");
                return Ok(());
            }

            // Print header
            println!("{:2} {:15} {:30} {:12} {:15} {}", " ", "NAME", "COMMAND", "CATEGORY", "TAGS", "DESCRIPTION");
            println!("{}", "-".repeat(90));

            for cmd in &commands {
                let favorite = if cmd.favorite { "★" } else { " " };
                let name = utils::truncate_string(&cmd.name, 15);
                let command = utils::truncate_string(&cmd.command, 30);
                let category = utils::truncate_string(cmd.category.as_deref().unwrap_or("-"), 12);
                let tags = utils::truncate_string(cmd.tags.as_deref().unwrap_or("-"), 15);
                let desc = utils::truncate_string(cmd.description.as_deref().unwrap_or("-"), 25);

                println!("{} {:15} {:30} {:12} {:15} {}", favorite, name, command, category, tags, desc);
            }

            println!();
            println!("Total: {} command(s)", commands.len());
        }

        Commands::Search { keyword } => {
            let commands = db.search_commands(&keyword)?;

            if commands.is_empty() {
                println!("No commands matching '{}' found.", keyword);
                return Ok(());
            }

            println!("Commands matching '{}':", keyword);
            println!("{}", "-".repeat(90));

            for cmd in &commands {
                let favorite = if cmd.favorite { "★" } else { " " };
                let name = utils::truncate_string(&cmd.name, 15);
                let command = utils::truncate_string(&cmd.command, 30);

                println!("{} {:15} {}", favorite, name, command);
            }

            println!();
            println!("Found: {} command(s)", commands.len());
        }

        Commands::Fav { name } => {
            match name {
                Some(n) => {
                    db.toggle_favorite(&n, true)?;
                    println!("Favorited command: {}", n);
                }
                None => {
                    let favorites = db.get_favorites()?;
                    if favorites.is_empty() {
                        println!("No favorite commands.");
                    } else {
                        println!("Favorite commands:");
                        for cmd in favorites {
                            println!("  ★ {} - {}", cmd.name, cmd.command);
                        }
                    }
                }
            }
        }

        Commands::Unfav { name } => {
            db.toggle_favorite(&name, false)?;
            println!("Unfavorited command: {}", name);
        }

        Commands::Recent { limit } => {
            let commands = db.get_recent(limit)?;

            if commands.is_empty() {
                println!("No recent commands.");
                return Ok(());
            }

            println!("Recent commands:");
            for cmd in commands {
                let favorite = if cmd.favorite { "★" } else { " " };
                let last_used = cmd.last_used_at.as_deref().unwrap_or("never");
                println!("  {} {} (used {} times, last: {})", favorite, cmd.name, cmd.use_count, last_used);
            }
        }

        Commands::History { limit } => {
            let history = db.get_history(limit)?;

            if history.is_empty() {
                println!("No execution history.");
                return Ok(());
            }

            println!("Execution history:");
            println!("{:5} {:15} {:8} {:12} {}", "ID", "COMMAND", "EXIT", "DURATION", "TIME");
            println!("{}", "-".repeat(60));

            for entry in history {
                let duration = utils::format_duration(entry.duration_ms);
                let time = entry.created_at.as_deref().unwrap_or("-");
                println!("{:5} {:15} {:8} {:12} {}", entry.id.unwrap_or(0), entry.command_name, entry.exit_code, duration, time);
            }
        }
    }

    Ok(())
}

/// Run the TUI application
fn run_tui(db: Database) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and event handler
    let mut app = App::new(db)?;
    let events = EventHandler::default();
    let mut show_help = false;

    // Main loop
    loop {
        // Draw
        terminal.draw(|f| {
            if show_help {
                tui::render_help(f);
            } else {
                tui::render(f, &app);
            }
        })?;

        // Handle events
        if let Ok(event) = events.next() {
            match event {
                tui::Event::Key(key) => {
                    if show_help {
                        show_help = false;
                        app.message = None;
                        continue;
                    }

                    // Clear message on any key
                    app.message = None;

                    match app.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => app.quit(),
                                KeyCode::Char('?') => show_help = true,
                                KeyCode::Char('/') => app.enter_command_mode(),
                                KeyCode::Char('i') => app.enter_insert_mode(),
                                KeyCode::Char('j') | KeyCode::Down => app.select_next(),
                                KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
                                KeyCode::Char('g') => app.select_first(),
                                KeyCode::Char('G') => app.select_last(),
                                KeyCode::Enter => {
                                    if let Some(exit_code) = app.execute_selected()? {
                                        // Command executed, show message
                                        app.message = Some(format!("Command exited with code: {}", exit_code));
                                    }
                                }
                                KeyCode::Esc => app.clear_filter(),
                                _ => {}
                            }
                        }
                        InputMode::Insert | InputMode::Command => {
                            match key.code {
                                KeyCode::Esc => app.exit_input_mode(),
                                KeyCode::Enter => app.execute_command()?,
                                KeyCode::Backspace => app.handle_backspace(),
                                KeyCode::Char(c) => {
                                    // Handle Ctrl+C
                                    if c == 'c' && key.modifiers.contains(KeyModifiers::CONTROL) {
                                        app.quit();
                                    } else {
                                        app.handle_char(c);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                tui::Event::Tick => {
                    // Tick event - could be used for periodic updates
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
