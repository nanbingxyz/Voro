use serde::{Deserialize, Serialize};

/// A saved command with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: Option<i64>,
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub tags: Option<String>,
    pub favorite: bool,
    pub confirm: bool,
    pub passthrough: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_used_at: Option<String>,
    pub use_count: i32,
}

impl Command {
    /// Create a new command with minimal required fields
    pub fn new(name: String, command: String) -> Self {
        Self {
            id: None,
            name,
            command,
            description: None,
            category: None,
            tags: None,
            favorite: false,
            confirm: false,
            passthrough: false,
            created_at: None,
            updated_at: None,
            last_used_at: None,
            use_count: 0,
        }
    }

    /// Parse tags into a vector
    #[allow(dead_code)]
    pub fn get_tags(&self) -> Vec<String> {
        parse_tags(self.tags.as_deref())
    }
}

/// Execution history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub id: Option<i64>,
    pub command_name: String,
    pub args: Option<String>,
    pub exit_code: i32,
    pub duration_ms: i64,
    pub created_at: Option<String>,
}

impl History {
    /// Create a new history entry
    pub fn new(command_name: String, args: Option<String>, exit_code: i32, duration_ms: i64) -> Self {
        Self {
            id: None,
            command_name,
            args,
            exit_code,
            duration_ms,
            created_at: None,
        }
    }
}

/// Parse comma-separated tags into a vector
#[allow(dead_code)]
pub fn parse_tags(tags: Option<&str>) -> Vec<String> {
    match tags {
        Some(s) if !s.is_empty() => s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
        _ => Vec::new(),
    }
}

/// Join tags into a comma-separated string
#[allow(dead_code)]
pub fn join_tags(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        None
    } else {
        Some(tags.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tags_basic() {
        assert_eq!(parse_tags(Some("oco,zh")), vec!["oco", "zh"]);
    }

    #[test]
    fn test_parse_tags_with_spaces() {
        assert_eq!(parse_tags(Some("oco, zh, test")), vec!["oco", "zh", "test"]);
    }

    #[test]
    fn test_parse_tags_empty() {
        assert_eq!(parse_tags(Some("")), Vec::<String>::new());
        assert_eq!(parse_tags(None), Vec::<String>::new());
    }

    #[test]
    fn test_parse_tags_single() {
        assert_eq!(parse_tags(Some("single")), vec!["single"]);
    }

    #[test]
    fn test_join_tags_basic() {
        let tags = vec!["oco".to_string(), "zh".to_string()];
        assert_eq!(join_tags(&tags), Some("oco,zh".to_string()));
    }

    #[test]
    fn test_join_tags_empty() {
        let tags: Vec<String> = vec![];
        assert_eq!(join_tags(&tags), None);
    }

    #[test]
    fn test_command_get_tags() {
        let mut cmd = Command::new("test".to_string(), "echo hello".to_string());
        cmd.tags = Some("tag1,tag2".to_string());
        assert_eq!(cmd.get_tags(), vec!["tag1", "tag2"]);
    }

    #[test]
    fn test_command_new() {
        let cmd = Command::new("test".to_string(), "echo hello".to_string());
        assert_eq!(cmd.name, "test");
        assert_eq!(cmd.command, "echo hello");
        assert!(!cmd.favorite);
        assert!(!cmd.confirm);
        assert!(!cmd.passthrough);
        assert_eq!(cmd.use_count, 0);
    }

    #[test]
    fn test_history_new() {
        let history = History::new(
            "test".to_string(),
            Some("arg1".to_string()),
            0,
            100,
        );
        assert_eq!(history.command_name, "test");
        assert_eq!(history.args, Some("arg1".to_string()));
        assert_eq!(history.exit_code, 0);
        assert_eq!(history.duration_ms, 100);
    }
}
