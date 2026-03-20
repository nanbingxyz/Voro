use crate::tui::app::{App, InputMode};
use crate::utils::truncate_string;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

/// Render the TUI
pub fn render(f: &mut Frame, app: &App) {
    // Create main layout: 90% list, 10% input
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(90),
            Constraint::Min(3),
        ])
        .split(f.size());

    // Render command list
    render_command_list(f, app, chunks[0]);

    // Render input area
    render_input_area(f, app, chunks[1]);

    // Render message overlay if present
    if app.message.is_some() {
        render_message(f, app);
    }
}

/// Render the command list
fn render_command_list(f: &mut Frame, app: &App, area: Rect) {
    let commands = app.get_commands();

    let items: Vec<ListItem> = commands
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let favorite = if cmd.favorite { "★ " } else { "  " };
            let name = truncate_string(&cmd.name, 15);
            let command = truncate_string(&cmd.command, 30);
            let category = truncate_string(cmd.category.as_deref().unwrap_or("-"), 12);
            let tags = truncate_string(cmd.tags.as_deref().unwrap_or("-"), 15);
            let desc = truncate_string(cmd.description.as_deref().unwrap_or("-"), 25);

            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if cmd.favorite {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };

            let content = format!(
                "{}{:15} {:30} {:12} {:15} {}",
                favorite, name, command, category, tags, desc
            );

            ListItem::new(content).style(style)
        })
        .collect();

    let title = if app.is_filtered {
        format!("Commands (filtered: {})", commands.len())
    } else {
        format!("Commands ({})", commands.len())
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(list, area);
}

/// Render the input area
fn render_input_area(f: &mut Frame, app: &App, area: Rect) {
    let style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::Gray),
        InputMode::Insert => Style::default().fg(Color::Green),
        InputMode::Command => Style::default().fg(Color::Yellow),
    };

    let mode_text = match app.input_mode {
        InputMode::Normal => "NORMAL",
        InputMode::Insert => "INSERT",
        InputMode::Command => "COMMAND",
    };

    let title = format!("[{}] Press ? for help, / for command mode", mode_text);

    let paragraph = Paragraph::new(app.input_buffer.as_str())
        .style(style)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(style),
        );

    f.render_widget(paragraph, area);

    // Show cursor in input mode
    if app.input_mode != InputMode::Normal {
        let cursor_x = area.x + app.input_buffer.len() as u16 + 1;
        let cursor_y = area.y + 1;
        f.set_cursor(cursor_x, cursor_y);
    }
}

/// Render a message overlay
fn render_message(f: &mut Frame, app: &App) {
    if let Some(ref msg) = app.message {
        // Calculate message area
        let area = centered_rect(60, 20, f.size());

        // Clear the area first
        f.render_widget(Clear, area);

        // Create message box
        let paragraph = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::White))
            .block(
                Block::default()
                    .title("Message")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .style(Style::default().bg(Color::DarkGray)),
            );

        f.render_widget(paragraph, area);
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Render help overlay
#[allow(dead_code)]
pub fn render_help(f: &mut Frame) {
    let area = centered_rect(70, 60, f.size());
    f.render_widget(Clear, area);

    let help_text = r#"
voro - Command Organizer

Navigation:
  j/↓     Move down
  k/↑     Move up
  g       Go to first
  G       Go to last
  Enter   Execute selected command

Commands (/):
  /add <name> <cmd>   Add a new command
  /del                Delete selected command
  /edit               Edit selected command (via CLI)
  /search <term>      Search commands
  /fav                Favorite selected command
  /unfav              Unfavorite selected command
  /q                  Quit

Other:
  /       Enter command mode
  Esc     Exit to normal mode
  q       Quit
  ?       Show this help

Press any key to close
"#;

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .style(Style::default().bg(Color::DarkGray)),
        );

    f.render_widget(paragraph, area);
}
