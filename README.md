# voro

**Your commands, remembered.**

A fast, minimal CLI tool to save, organize, and run your favorite commands instantly.

## Why voro?

Tired of:
- Scrolling through bash history trying to find that one command?
- Keeping complex commands in random notes files?
- Alias clutter in your shell config?

**voro** is your personal command library. Save it once, run it anywhere.

## Features

- **Quick Save** - Add commands in seconds with descriptions, categories, and tags
- **Instant Run** - Execute saved commands with a single keystroke
- **Template Parameters** - Define `{param}` placeholders and fill them at runtime
- **TUI Interface** - Beautiful terminal UI for browsing and managing commands
- **Argument Passthrough** - Pass extra arguments with `--`
- **Smart Organization** - Categories, tags, favorites, and search
- **Execution History** - Track what you ran and when
- **Self-Updating** - `vo update` handles everything

## Installation

### One-liner (macOS & Linux)

```bash
curl -sSL https://voro.sh/install | sh
```

### Homebrew

```bash
brew tap nanbingxyz/voro
brew install voro
```

### Build from Source

```bash
git clone https://github.com/nanbingxyz/voro.git
cd voro
cargo build --release
sudo cp target/release/vo /usr/local/bin/
```

### Prebuilt Binaries

Download from [Releases](https://github.com/nanbingxyz/voro/releases) for your platform:

| Platform | Architecture | Binary |
|----------|-------------|--------|
| macOS | Apple Silicon | `voro-*-darwin-arm64.tar.gz` |
| macOS | Intel | `voro-*-darwin-x64.tar.gz` |
| Linux | x64 | `voro-*-linux-x64.tar.gz` |
| Windows | x64 | `voro-*-windows-x64.zip` |

## Quick Start

```bash
# Save a command
vo add deploy "npm run build && rsync -avz dist/ server:/var/www"
vo add gcp "git commit -m '{message}' && git push" --desc "Commit and push"
vo add rm-node "rm -rf node_modules" --confirm

# Run it
vo deploy

# Launch TUI
vo
```

## Usage

### Running Commands

```bash
vo <name>                  # Run a saved command
vo <name> -- <args>        # Run with additional arguments
```

### Managing Commands

```bash
# Add commands
vo add <name> <command> [options]

  Options:
    --desc <text>         Description
    --cat <category>      Category for organization
    --tags <t1,t2>        Comma-separated tags
    --confirm             Require confirmation before execution
    --passthrough         Allow argument passthrough with --

# Examples
vo add g "git"
vo add gs "git status" --cat git --tags "quick,status"
vo add deploy "npm run deploy" --desc "Deploy to production" --confirm
vo add serve "python -m http.server" --passthrough
vo serve -- 8080                    # Runs: python -m http.server 8080

# Edit commands
vo edit <name> [--command <cmd>] [--desc <text>] [--cat <cat>] [--tags <tags>]
vo edit <name> --editor             # Open in $EDITOR

# Delete commands
vo del <name>                       # Alias: vo rm

# View command details
vo get <name>
```

### Template Parameters

Use `{param}` syntax for dynamic values:

```bash
vo add commit "git commit -m '{message}'"
vo commit
# Prompts: Enter value for 'message':
# Then runs: git commit -m 'your input'

vo add api "curl {url} -H 'Authorization: Bearer {token}'"
vo api
# Prompts for url, then token, then executes
```

Works with `--confirm` - parameters are collected first, then confirmation is requested.

### Listing & Searching

```bash
vo ls                               # List all commands
vo ls --cat git                     # Filter by category
vo ls --tag docker                  # Filter by tag
vo search <keyword>                 # Search name, command, description
```

### Favorites

```bash
vo fav <name>                       # Mark as favorite
vo unfav <name>                     # Remove from favorites
vo fav                              # List all favorites
```

### History & Recent

```bash
vo recent                           # Recently used commands
vo recent --limit 20
vo history                          # Execution history with exit codes
vo history --limit 50
```

### Update

```bash
vo update                           # Check and install updates
```

### TUI Mode

Run without arguments to launch the interactive terminal UI:

```bash
vo
```

**Keybindings:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Enter` | Execute selected |
| `/` | Command mode |
| `i` | Insert mode |
| `?` | Help |
| `q` | Quit |

**TUI Commands:**
- `/add <name> <command>` - Add command
- `/del <name>` - Delete command
- `/edit <name>` - Edit command
- `/search <term>` - Search commands
- `/fav <name>` - Toggle favorite

## Data Storage

Commands are stored in a local SQLite database:

- **macOS**: `~/Library/Application Support/voro/`
- **Linux**: `~/.config/voro/`
- **Windows**: `%APPDATA%\voro\`

## Contributing

Contributions are welcome! Here's how to get started:

### Development Setup

```bash
git clone https://github.com/nanbingxyz/voro.git
cd voro
cargo build
cargo test
```

### Project Structure

```
src/
├── main.rs      # Entry point & command routing
├── cli.rs       # CLI definitions (clap)
├── command.rs   # Command execution logic
├── db.rs        # SQLite operations
├── model.rs     # Data structures
├── update.rs    # Self-update functionality
├── tui/         # Terminal UI module
│   ├── mod.rs
│   ├── app.rs   # TUI state machine
│   ├── ui.rs    # Rendering
│   └── event.rs # Input handling
└── utils.rs     # Helper functions
```

### Guidelines

1. **Code Style** - Run `cargo fmt` before committing
2. **Linting** - Ensure `cargo clippy` passes
3. **Tests** - Add tests for new functionality
4. **Commits** - Write clear, descriptive commit messages

### Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Format code (`cargo fmt`)
6. Commit changes (`git commit -m 'Add amazing feature'`)
7. Push to branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built with:
- [clap](https://github.com/clap-rs/clap) - CLI argument parsing
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI
- [rusqlite](https://github.com/rusqlite/rusqlite) - SQLite bindings

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/nanbingxyz">nanbingxyz</a>
</p>
