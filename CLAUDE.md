# voro

A fast, minimal CLI tool to save, organize, and run favorite commands instantly.

## Commands

```bash
cargo build           # Build the project
cargo build --release # Release build
cargo run             # Run the project (use `--` to pass args)
cargo test            # Run tests
cargo fmt             # Format code
cargo clippy          # Lint code
```

## Architecture

**Type**: Rust CLI tool with TUI interface

**Entry Point**: `vo` command

**Tech Stack**:
- `clap` - CLI argument parsing
- `ratatui` + `crossterm` - Terminal UI
- `rusqlite` - SQLite database
- `serde` - Serialization
- `dirs` - Config directory resolution
- `anyhow` - Error handling

## Directory Structure

```
src/
├── main.rs      # Entry point
├── cli.rs       # clap CLI definitions
├── command.rs   # Command execution logic
├── db.rs        # SQLite operations
├── model.rs     # Data structures (Command, History)
├── tui/         # Terminal UI module
│   ├── mod.rs
│   ├── app.rs   # TUI state machine
│   ├── ui.rs    # Rendering
│   └── event.rs # Input handling
└── utils.rs
```

## CLI Syntax

### Execution (default subcommand)
```bash
vo <name>              # Run saved command
vo <name> -- <args...> # Run with arguments passed through
```

### Management
```bash
vo add <name> <command> [--desc <desc>] [--cat <category>] [--tags <t1,t2>] [--confirm]
vo edit <name>
vo del <name>
vo get <name>
vo ls [--cat <category>] [--tag <tag>]
vo search <keyword>
```

### Favorites & History
```bash
vo fav <name>    # Mark as favorite
vo unfav <name>  # Remove from favorites
vo fav           # List all favorites
vo recent        # Show recent commands
```

### TUI
```bash
vo               # No args = launch TUI
```

## Key Implementation Details

### Shell Execution
Commands are executed via `sh -c` to support environment variables, pipes, and redirects:

```rust
Command::new("sh")
    .arg("-c")
    .arg(full_command)
    .status();
```

### Argument Passthrough
The `--` separator passes arguments to the saved command:
```
vo oco_zh -- "summarize this"
# executes: OCO_LANGUAGE=zh_CN oco "summarize this"
```

### Dangerous Commands
Commands added with `--confirm` flag require confirmation before execution:
```
vo add rm_all "rm -rf /" --confirm
# On run: ⚠️ Dangerous command, continue? (y/n)
```

### Database Schema

**commands table**:
- `id`, `name` (unique), `command`, `description`, `category`, `tags` (comma-separated)
- `favorite`, `confirm`, `passthrough` (flags)
- `created_at`, `updated_at`, `last_used_at`, `use_count`

**history table**:
- `id`, `command_name`, `args`, `exit_code`, `duration_ms`, `created_at`

### TUI Layout
- Upper 90%: Command list (name, command, category, tags, description)
- Lower 10%: Input area with `/` commands: `/add`, `/del`, `/edit`, `/search`, `/fav`, `/unfav`

## Data Location

Database stored in platform-specific config directory (via `dirs` crate):
- macOS: `~/Library/Application Support/voro/`
- Linux: `~/.config/voro/`
- Windows: `%APPDATA%\voro\`
