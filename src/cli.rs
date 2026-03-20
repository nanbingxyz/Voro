use clap::{Parser, Subcommand};

/// A fast, minimal CLI tool to save, organize, and run favorite commands
#[derive(Parser)]
#[command(name = "vo", author, version, about, long_about = None)]
pub struct Cli {
    /// Command name to execute (when no subcommand is given)
    #[arg(value_name = "NAME")]
    pub name: Option<String>,

    /// Arguments to pass to the command (after --)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Subcommand)]
pub enum Commands {
    /// Add a new command
    Add {
        /// Name for the command
        name: String,

        /// The command to save
        command: String,

        /// Description of the command
        #[arg(short, long)]
        desc: Option<String>,

        /// Category for organizing commands
        #[arg(short, long)]
        cat: Option<String>,

        /// Tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Require confirmation before execution
        #[arg(short = 'C', long)]
        confirm: bool,

        /// Allow argument passthrough (append args after --)
        #[arg(short = 'P', long)]
        passthrough: bool,
    },

    /// Edit an existing command
    Edit {
        /// Name of the command to edit
        name: String,

        /// New command string
        #[arg(short, long)]
        command: Option<String>,

        /// New description
        #[arg(short, long)]
        desc: Option<String>,

        /// New category
        #[arg(short, long)]
        cat: Option<String>,

        /// New tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,

        /// Open in $EDITOR instead of flags
        #[arg(short, long)]
        editor: bool,
    },

    /// Delete a command
    #[clap(alias = "rm")]
    Del {
        /// Name of the command to delete
        name: String,
    },

    /// Get details of a command
    Get {
        /// Name of the command
        name: String,
    },

    /// List all commands
    #[clap(alias = "list")]
    Ls {
        /// Filter by category
        #[arg(short, long)]
        cat: Option<String>,

        /// Filter by tag
        #[arg(short, long)]
        tag: Option<String>,
    },

    /// Search commands by keyword
    Search {
        /// Keyword to search for
        keyword: String,
    },

    /// Mark a command as favorite (or list favorites)
    Fav {
        /// Name of the command to favorite (omit to list all favorites)
        name: Option<String>,
    },

    /// Remove a command from favorites
    Unfav {
        /// Name of the command
        name: String,
    },

    /// Show recently used commands
    Recent {
        /// Maximum number of commands to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show execution history
    History {
        /// Maximum number of entries to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_add() {
        let cli = Cli::try_parse_from(["vo", "add", "test", "echo hello"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Add { name, command, .. }) => {
                assert_eq!(name, "test");
                assert_eq!(command, "echo hello");
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_parse_add_with_options() {
        let cli = Cli::try_parse_from([
            "vo", "add", "test", "echo hello",
            "--desc", "A test command",
            "--cat", "testing",
            "--tags", "demo,example",
            "--confirm",
            "--passthrough",
        ]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Add { name, command, desc, cat, tags, confirm, passthrough }) => {
                assert_eq!(name, "test");
                assert_eq!(command, "echo hello");
                assert_eq!(desc, Some("A test command".to_string()));
                assert_eq!(cat, Some("testing".to_string()));
                assert_eq!(tags, Some("demo,example".to_string()));
                assert!(confirm);
                assert!(passthrough);
            }
            _ => panic!("Expected Add command"),
        }
    }

    #[test]
    fn test_cli_parse_execute() {
        let cli = Cli::try_parse_from(["vo", "test", "arg1", "arg2"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        assert_eq!(cli.name, Some("test".to_string()));
        assert_eq!(cli.args, vec!["arg1", "arg2"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_parse_ls() {
        let cli = Cli::try_parse_from(["vo", "ls", "--cat", "git"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Ls { cat, tag }) => {
                assert_eq!(cat, Some("git".to_string()));
                assert_eq!(tag, None);
            }
            _ => panic!("Expected Ls command"),
        }
    }

    #[test]
    fn test_cli_parse_search() {
        let cli = Cli::try_parse_from(["vo", "search", "docker"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Search { keyword }) => {
                assert_eq!(keyword, "docker");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_cli_parse_fav_list() {
        let cli = Cli::try_parse_from(["vo", "fav"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Fav { name }) => {
                assert!(name.is_none());
            }
            _ => panic!("Expected Fav command"),
        }
    }

    #[test]
    fn test_cli_parse_fav_add() {
        let cli = Cli::try_parse_from(["vo", "fav", "test"]);
        assert!(cli.is_ok());
        let cli = cli.unwrap();
        match cli.command {
            Some(Commands::Fav { name }) => {
                assert_eq!(name, Some("test".to_string()));
            }
            _ => panic!("Expected Fav command"),
        }
    }
}
