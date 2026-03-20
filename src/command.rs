use anyhow::Result;
use std::collections::HashSet;
use std::io::{self, Write};
use std::process::Command as ProcessCommand;
use std::time::Instant;

use crate::db::Database;
use crate::model::{Command, History};

/// Execute a saved command with optional arguments
pub fn execute_command(cmd: &Command, args: &[String], db: &Database) -> Result<i32> {
    // Extract template parameters and prompt for values
    let command_with_params = if has_template_params(&cmd.command) {
        let params = extract_template_params(&cmd.command);
        let values = prompt_for_params(&params)?;
        replace_template_params(&cmd.command, &values)
    } else {
        cmd.command.clone()
    };

    // Check if confirmation is required
    if cmd.confirm {
        if !confirm_dangerous()? {
            println!("Aborted.");
            return Ok(130); // Standard exit code for Ctrl+C
        }
    }

    // Build the full command string
    let full_command = build_full_command(&command_with_params, args, cmd.passthrough);

    // Record start time
    let start = Instant::now();

    // Execute via shell
    let exit_code = run_shell_command(&full_command)?;

    // Record duration
    let duration_ms = start.elapsed().as_millis() as i64;

    // Log to history and increment use count
    let history = History::new(
        cmd.name.clone(),
        if args.is_empty() { None } else { Some(args.join(" ")) },
        exit_code,
        duration_ms,
    );
    db.log_execution(&history)?;
    db.increment_use_count(&cmd.name)?;

    Ok(exit_code)
}

/// Build full command string with arguments
fn build_full_command(command: &str, args: &[String], passthrough: bool) -> String {
    if args.is_empty() || !passthrough {
        return command.to_string();
    }

    // Append arguments to the command
    let args_str = args.iter()
        .map(|arg| shell_escape::escape(arg))
        .collect::<Vec<_>>()
        .join(" ");

    format!("{} {}", command, args_str)
}

/// Shell-escape an argument (simple implementation)
mod shell_escape {
    pub fn escape(s: &str) -> String {
        // If the string is simple (no special characters), return as-is
        if s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/') {
            return s.to_string();
        }

        // Otherwise, quote it
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Run a command via the shell
fn run_shell_command(command: &str) -> Result<i32> {
    let status = ProcessCommand::new("sh")
        .arg("-c")
        .arg(command)
        .status()?;

    Ok(status.code().unwrap_or(1))
}

/// Prompt user to confirm a dangerous command
fn confirm_dangerous() -> Result<bool> {
    print!("⚠️  Dangerous command, continue? (y/n): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

/// Check if command contains template parameters
fn has_template_params(command: &str) -> bool {
    command.contains('{') && command.contains('}')
}

/// Extract template parameters from command string
/// Returns unique parameter names in order of first appearance
fn extract_template_params(command: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut seen = HashSet::new();
    let mut in_brace = false;
    let mut current_param = String::new();

    for ch in command.chars() {
        match ch {
            '{' => {
                in_brace = true;
                current_param.clear();
            }
            '}' if in_brace => {
                in_brace = false;
                let param = current_param.trim().to_string();
                if !param.is_empty() && !seen.contains(&param) {
                    seen.insert(param.clone());
                    params.push(param);
                }
            }
            _ if in_brace => {
                current_param.push(ch);
            }
            _ => {}
        }
    }

    params
}

/// Prompt user for each template parameter value
fn prompt_for_params(params: &[String]) -> Result<Vec<(String, String)>> {
    let mut values = Vec::new();

    for param in params {
        print!("Enter value for '{}': ", param);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let value = input.trim().to_string();
        values.push((param.clone(), value));
    }

    Ok(values)
}

/// Replace template parameters with provided values
fn replace_template_params(command: &str, values: &[(String, String)]) -> String {
    let mut result = command.to_string();

    for (param, value) in values {
        let placeholder = format!("{{{}}}", param);
        result = result.replace(&placeholder, value);
    }

    result
}

/// Execute a raw shell command (for testing)
#[allow(dead_code)]
pub fn execute_raw(command: &str) -> Result<i32> {
    run_shell_command(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_full_command_no_args() {
        let cmd = "echo hello";
        let args: Vec<String> = vec![];
        let result = build_full_command(cmd, &args, true);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_build_full_command_with_args() {
        let cmd = "echo";
        let args = vec!["hello".to_string(), "world".to_string()];
        let result = build_full_command(cmd, &args, true);
        assert_eq!(result, "echo hello world");
    }

    #[test]
    fn test_build_full_command_no_passthrough() {
        let cmd = "echo hello";
        let args = vec!["extra".to_string()];
        let result = build_full_command(cmd, &args, false);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_shell_escape_simple() {
        assert_eq!(shell_escape::escape("hello"), "hello");
        assert_eq!(shell_escape::escape("hello-world"), "hello-world");
        assert_eq!(shell_escape::escape("hello_world"), "hello_world");
    }

    #[test]
    fn test_shell_escape_special() {
        assert_eq!(shell_escape::escape("hello world"), "'hello world'");
        assert_eq!(shell_escape::escape("hello$world"), "'hello$world'");
    }

    #[test]
    fn test_shell_escape_with_quotes() {
        assert_eq!(shell_escape::escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_run_shell_command_success() {
        let exit_code = run_shell_command("exit 0").unwrap();
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_run_shell_command_failure() {
        let exit_code = run_shell_command("exit 42").unwrap();
        assert_eq!(exit_code, 42);
    }

    #[test]
    fn test_has_template_params() {
        assert!(has_template_params("weather {city}"));
        assert!(has_template_params("echo {name} and {value}"));
        assert!(!has_template_params("echo hello"));
        assert!(!has_template_params("echo { incomplete"));
    }

    #[test]
    fn test_extract_template_params_single() {
        let params = extract_template_params("weather {city}");
        assert_eq!(params, vec!["city"]);
    }

    #[test]
    fn test_extract_template_params_multiple() {
        let params = extract_template_params("curl {url} -d '{data}'");
        assert_eq!(params, vec!["url", "data"]);
    }

    #[test]
    fn test_extract_template_params_duplicate() {
        let params = extract_template_params("echo {name} and {name}");
        assert_eq!(params, vec!["name"]);
    }

    #[test]
    fn test_extract_template_params_empty() {
        let params = extract_template_params("echo {} and { }");
        assert!(params.is_empty());
    }

    #[test]
    fn test_replace_template_params_single() {
        let values = vec![("city".to_string(), "Beijing".to_string())];
        let result = replace_template_params("weather {city}", &values);
        assert_eq!(result, "weather Beijing");
    }

    #[test]
    fn test_replace_template_params_multiple() {
        let values = vec![
            ("url".to_string(), "http://example.com".to_string()),
            ("data".to_string(), "{\"key\": \"value\"}".to_string()),
        ];
        let result = replace_template_params("curl {url} -d '{data}'", &values);
        assert_eq!(result, "curl http://example.com -d '{\"key\": \"value\"}'");
    }

    #[test]
    fn test_replace_template_params_no_match() {
        let values: Vec<(String, String)> = vec![];
        let result = replace_template_params("echo hello", &values);
        assert_eq!(result, "echo hello");
    }
}
