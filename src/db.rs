use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;

use crate::model::{Command, History};

/// Database handler for command storage
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create a new database connection, initializing tables if needed
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// Create an in-memory database (for testing)
    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }

    /// Get platform-specific database path
    pub fn get_db_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("voro").join("voro.db")
    }

    /// Initialize database tables
    fn init_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS commands (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT UNIQUE NOT NULL,
                command TEXT NOT NULL,
                description TEXT,
                category TEXT,
                tags TEXT,
                favorite INTEGER DEFAULT 0,
                confirm INTEGER DEFAULT 0,
                passthrough INTEGER DEFAULT 0,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                last_used_at TEXT,
                use_count INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command_name TEXT NOT NULL,
                args TEXT,
                exit_code INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_commands_name ON commands(name);
            CREATE INDEX IF NOT EXISTS idx_commands_category ON commands(category);
            CREATE INDEX IF NOT EXISTS idx_commands_favorite ON commands(favorite);
            CREATE INDEX IF NOT EXISTS idx_history_command_name ON history(command_name);
            CREATE INDEX IF NOT EXISTS idx_history_created_at ON history(created_at);
            "#,
        )?;
        Ok(())
    }

    /// Add a new command
    pub fn add_command(&self, cmd: &Command) -> Result<()> {
        let now = chrono::Local::now().to_rfc3339();
        self.conn.execute(
            r#"
            INSERT INTO commands (name, command, description, category, tags, favorite, confirm, passthrough, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                cmd.name,
                cmd.command,
                cmd.description,
                cmd.category,
                cmd.tags,
                cmd.favorite as i32,
                cmd.confirm as i32,
                cmd.passthrough as i32,
                now,
                now,
            ],
        )?;
        Ok(())
    }

    /// Get a command by name
    pub fn get_command(&self, name: &str) -> Result<Option<Command>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands WHERE name = ?1
            "#,
        )?;

        let result = stmt.query_row(params![name], |row| {
            Ok(Command {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                command: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                tags: row.get(5)?,
                favorite: row.get::<_, i32>(6)? != 0,
                confirm: row.get::<_, i32>(7)? != 0,
                passthrough: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                last_used_at: row.get(11)?,
                use_count: row.get(12)?,
            })
        });

        match result {
            Ok(cmd) => Ok(Some(cmd)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all commands
    pub fn list_commands(&self) -> Result<Vec<Command>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands ORDER BY favorite DESC, name ASC
            "#,
        )?;

        let commands = stmt
            .query_map([], |row| {
                Ok(Command {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    confirm: row.get::<_, i32>(7)? != 0,
                    passthrough: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    use_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// Delete a command by name
    pub fn delete_command(&self, name: &str) -> Result<bool> {
        let rows_affected = self.conn.execute("DELETE FROM commands WHERE name = ?1", params![name])?;
        Ok(rows_affected > 0)
    }

    /// Update a command
    pub fn update_command(&self, cmd: &Command) -> Result<()> {
        let now = chrono::Local::now().to_rfc3339();
        let rows_affected = self.conn.execute(
            r#"
            UPDATE commands SET
                command = ?2,
                description = ?3,
                category = ?4,
                tags = ?5,
                confirm = ?6,
                passthrough = ?7,
                updated_at = ?8
            WHERE name = ?1
            "#,
            params![
                cmd.name,
                cmd.command,
                cmd.description,
                cmd.category,
                cmd.tags,
                cmd.confirm as i32,
                cmd.passthrough as i32,
                now,
            ],
        )?;

        if rows_affected == 0 {
            anyhow::bail!("Command '{}' not found", cmd.name);
        }
        Ok(())
    }

    /// Search commands by keyword (searches name, command, description)
    pub fn search_commands(&self, keyword: &str) -> Result<Vec<Command>> {
        let pattern = format!("%{}%", keyword);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands
            WHERE name LIKE ?1 OR command LIKE ?1 OR description LIKE ?1 OR category LIKE ?1 OR tags LIKE ?1
            ORDER BY favorite DESC, name ASC
            "#,
        )?;

        let commands = stmt
            .query_map(params![pattern], |row| {
                Ok(Command {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    confirm: row.get::<_, i32>(7)? != 0,
                    passthrough: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    use_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// Toggle favorite status
    pub fn toggle_favorite(&self, name: &str, favorite: bool) -> Result<()> {
        let rows_affected = self.conn.execute(
            "UPDATE commands SET favorite = ?2 WHERE name = ?1",
            params![name, favorite as i32],
        )?;

        if rows_affected == 0 {
            anyhow::bail!("Command '{}' not found", name);
        }
        Ok(())
    }

    /// Get all favorite commands
    pub fn get_favorites(&self) -> Result<Vec<Command>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands WHERE favorite = 1 ORDER BY name ASC
            "#,
        )?;

        let commands = stmt
            .query_map([], |row| {
                Ok(Command {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    confirm: row.get::<_, i32>(7)? != 0,
                    passthrough: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    use_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// Get recent commands (by last_used_at)
    pub fn get_recent(&self, limit: usize) -> Result<Vec<Command>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands
            WHERE last_used_at IS NOT NULL
            ORDER BY last_used_at DESC
            LIMIT ?1
            "#,
        )?;

        let commands = stmt
            .query_map(params![limit as i32], |row| {
                Ok(Command {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    confirm: row.get::<_, i32>(7)? != 0,
                    passthrough: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    use_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// List commands filtered by category
    pub fn list_by_category(&self, category: &str) -> Result<Vec<Command>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands WHERE category = ?1 ORDER BY favorite DESC, name ASC
            "#,
        )?;

        let commands = stmt
            .query_map(params![category], |row| {
                Ok(Command {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    confirm: row.get::<_, i32>(7)? != 0,
                    passthrough: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    use_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// List commands filtered by tag
    pub fn list_by_tag(&self, tag: &str) -> Result<Vec<Command>> {
        let pattern = format!("%{}%", tag);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, name, command, description, category, tags, favorite, confirm, passthrough,
                   created_at, updated_at, last_used_at, use_count
            FROM commands WHERE tags LIKE ?1 ORDER BY favorite DESC, name ASC
            "#,
        )?;

        let commands = stmt
            .query_map(params![pattern], |row| {
                Ok(Command {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    command: row.get(2)?,
                    description: row.get(3)?,
                    category: row.get(4)?,
                    tags: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    confirm: row.get::<_, i32>(7)? != 0,
                    passthrough: row.get::<_, i32>(8)? != 0,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    last_used_at: row.get(11)?,
                    use_count: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commands)
    }

    /// Increment use count and update last_used_at
    pub fn increment_use_count(&self, name: &str) -> Result<()> {
        let now = chrono::Local::now().to_rfc3339();
        self.conn.execute(
            "UPDATE commands SET use_count = use_count + 1, last_used_at = ?2 WHERE name = ?1",
            params![name, now],
        )?;
        Ok(())
    }

    /// Log an execution to history
    pub fn log_execution(&self, entry: &History) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO history (command_name, args, exit_code, duration_ms)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![entry.command_name, entry.args, entry.exit_code, entry.duration_ms],
        )?;
        Ok(())
    }

    /// Get execution history
    pub fn get_history(&self, limit: usize) -> Result<Vec<History>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, command_name, args, exit_code, duration_ms, created_at
            FROM history ORDER BY created_at DESC LIMIT ?1
            "#,
        )?;

        let entries = stmt
            .query_map(params![limit as i32], |row| {
                Ok(History {
                    id: Some(row.get(0)?),
                    command_name: row.get(1)?,
                    args: row.get(2)?,
                    exit_code: row.get(3)?,
                    duration_ms: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_command() {
        let db = Database::new_in_memory().unwrap();
        let cmd = Command::new("test".to_string(), "echo hello".to_string());
        db.add_command(&cmd).unwrap();

        let retrieved = db.get_command("test").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "test");
        assert_eq!(retrieved.command, "echo hello");
    }

    #[test]
    fn test_duplicate_name_rejection() {
        let db = Database::new_in_memory().unwrap();
        let cmd = Command::new("test".to_string(), "echo hello".to_string());
        db.add_command(&cmd).unwrap();

        let cmd2 = Command::new("test".to_string(), "echo world".to_string());
        let result = db.add_command(&cmd2);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_commands() {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("cmd1".to_string(), "echo 1".to_string())).unwrap();
        db.add_command(&Command::new("cmd2".to_string(), "echo 2".to_string())).unwrap();

        let commands = db.list_commands().unwrap();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_delete_command() {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("test".to_string(), "echo hello".to_string())).unwrap();

        let deleted = db.delete_command("test").unwrap();
        assert!(deleted);

        let retrieved = db.get_command("test").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_delete_nonexistent() {
        let db = Database::new_in_memory().unwrap();
        let deleted = db.delete_command("nonexistent").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_search_commands() {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("test_cmd".to_string(), "echo hello".to_string())).unwrap();
        db.add_command(&Command::new("other".to_string(), "echo world".to_string())).unwrap();

        let results = db.search_commands("test").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test_cmd");
    }

    #[test]
    fn test_toggle_favorite() {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("test".to_string(), "echo hello".to_string())).unwrap();

        db.toggle_favorite("test", true).unwrap();
        let cmd = db.get_command("test").unwrap().unwrap();
        assert!(cmd.favorite);

        db.toggle_favorite("test", false).unwrap();
        let cmd = db.get_command("test").unwrap().unwrap();
        assert!(!cmd.favorite);
    }

    #[test]
    fn test_get_favorites() {
        let db = Database::new_in_memory().unwrap();
        let mut cmd1 = Command::new("fav1".to_string(), "echo 1".to_string());
        cmd1.favorite = true;
        db.add_command(&cmd1).unwrap();
        db.add_command(&Command::new("normal".to_string(), "echo 2".to_string())).unwrap();

        db.toggle_favorite("fav1", true).unwrap();
        let favorites = db.get_favorites().unwrap();
        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].name, "fav1");
    }

    #[test]
    fn test_increment_use_count() {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("test".to_string(), "echo hello".to_string())).unwrap();

        db.increment_use_count("test").unwrap();
        let cmd = db.get_command("test").unwrap().unwrap();
        assert_eq!(cmd.use_count, 1);
        assert!(cmd.last_used_at.is_some());
    }

    #[test]
    fn test_log_execution() {
        let db = Database::new_in_memory().unwrap();
        let history = History::new("test".to_string(), Some("arg1".to_string()), 0, 100);
        db.log_execution(&history).unwrap();

        let entries = db.get_history(10).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command_name, "test");
        assert_eq!(entries[0].exit_code, 0);
    }

    #[test]
    fn test_update_command() {
        let db = Database::new_in_memory().unwrap();
        db.add_command(&Command::new("test".to_string(), "echo hello".to_string())).unwrap();

        let mut updated = Command::new("test".to_string(), "echo world".to_string());
        updated.description = Some("Updated desc".to_string());
        db.update_command(&updated).unwrap();

        let cmd = db.get_command("test").unwrap().unwrap();
        assert_eq!(cmd.command, "echo world");
        assert_eq!(cmd.description, Some("Updated desc".to_string()));
    }

    #[test]
    fn test_list_by_category() {
        let db = Database::new_in_memory().unwrap();
        let mut cmd1 = Command::new("git1".to_string(), "git status".to_string());
        cmd1.category = Some("git".to_string());
        db.add_command(&cmd1).unwrap();

        let mut cmd2 = Command::new("docker1".to_string(), "docker ps".to_string());
        cmd2.category = Some("docker".to_string());
        db.add_command(&cmd2).unwrap();

        let git_commands = db.list_by_category("git").unwrap();
        assert_eq!(git_commands.len(), 1);
        assert_eq!(git_commands[0].name, "git1");
    }

    #[test]
    fn test_list_by_tag() {
        let db = Database::new_in_memory().unwrap();
        let mut cmd1 = Command::new("test1".to_string(), "echo 1".to_string());
        cmd1.tags = Some("important,daily".to_string());
        db.add_command(&cmd1).unwrap();

        let mut cmd2 = Command::new("test2".to_string(), "echo 2".to_string());
        cmd2.tags = Some("rare".to_string());
        db.add_command(&cmd2).unwrap();

        let daily_commands = db.list_by_tag("daily").unwrap();
        assert_eq!(daily_commands.len(), 1);
        assert_eq!(daily_commands[0].name, "test1");
    }
}
