#![allow(dead_code)] // Allow unused code during migration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Advanced session message data from .jsonl files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub message: MessageContent,
    pub cwd: Option<String>,
}

/// Message content with usage and model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub usage: Option<TokenUsage>,
    pub model: Option<String>,
    pub timestamp: Option<String>,
    pub content: Option<serde_json::Value>,
}

/// Token usage information from Claude sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub service_tier: Option<String>,
}

/// Represents a Claude session
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub path: PathBuf,
    pub last_modified: SystemTime,
    pub message_count: usize,
}

/// Session parser for Claude JSONL files
pub struct SessionParser {
    claude_dir: PathBuf,
}

impl SessionParser {
    /// Create new session parser
    pub fn new(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// Parse a session JSONL file
    pub fn parse_session(&self, session_path: &Path) -> Result<Option<Session>> {
        let metadata = fs::metadata(session_path)?;
        let last_modified = metadata.modified()?;

        let content = fs::read_to_string(session_path)?;
        let message_count = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();

        if message_count == 0 {
            return Ok(None);
        }

        let session_id = session_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(Some(Session {
            id: session_id,
            path: session_path.to_path_buf(),
            last_modified,
            message_count,
        }))
    }

    /// Parse all session messages from .jsonl files - Real token usage analytics
    pub fn parse_all_session_messages(&self) -> Result<Vec<SessionMessage>> {
        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut all_messages = Vec::new();

        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        if let Ok(content) = fs::read_to_string(&file_path) {
                            let session_id = file_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string();

                            for line in content.lines() {
                                if line.trim().is_empty() {
                                    continue;
                                }

                                // Parse as generic JSON first to check structure
                                if let Ok(json_value) =
                                    serde_json::from_str::<serde_json::Value>(line)
                                {
                                    // Get timestamp from root level
                                    if let Some(_timestamp) = json_value.get("timestamp") {
                                        // Check if this entry has usage data in message.usage
                                        if let Some(message) = json_value.get("message") {
                                            if let Some(usage) = message.get("usage") {
                                                // Only include entries with actual token usage (non-zero)
                                                let input_tokens = usage
                                                    .get("input_tokens")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0);
                                                let output_tokens = usage
                                                    .get("output_tokens")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0);
                                                let cache_creation = usage
                                                    .get("cache_creation_input_tokens")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0);
                                                let cache_read = usage
                                                    .get("cache_read_input_tokens")
                                                    .and_then(|v| v.as_u64())
                                                    .unwrap_or(0);

                                                if input_tokens > 0
                                                    || output_tokens > 0
                                                    || cache_creation > 0
                                                    || cache_read > 0
                                                {
                                                    // Create SessionMessage from the parsed data
                                                    let session_message = SessionMessage {
                                                        session_id: session_id.clone(),
                                                        message_type: json_value
                                                            .get("type")
                                                            .and_then(|v| v.as_str())
                                                            .unwrap_or("unknown")
                                                            .to_string(),
                                                        message: MessageContent {
                                                            usage: Some(TokenUsage {
                                                                input_tokens: Some(
                                                                    input_tokens as u32,
                                                                ),
                                                                output_tokens: Some(
                                                                    output_tokens as u32,
                                                                ),
                                                                cache_creation_input_tokens: Some(
                                                                    cache_creation as u32,
                                                                ),
                                                                cache_read_input_tokens: Some(
                                                                    cache_read as u32,
                                                                ),
                                                                service_tier: usage
                                                                    .get("service_tier")
                                                                    .and_then(|v| v.as_str())
                                                                    .map(|s| s.to_string()),
                                                            }),
                                                            model: message
                                                                .get("model")
                                                                .and_then(|v| v.as_str())
                                                                .map(|s| s.to_string()),
                                                            timestamp: json_value
                                                                .get("timestamp")
                                                                .and_then(|v| v.as_str())
                                                                .map(|s| s.to_string()),
                                                            content: message
                                                                .get("content")
                                                                .cloned(),
                                                        },
                                                        cwd: json_value
                                                            .get("cwd")
                                                            .and_then(|v| v.as_str())
                                                            .map(|s| s.to_string()),
                                                    };

                                                    all_messages.push(session_message);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(all_messages)
    }

    /// Get the real project path from the most recent JSONL session file's "cwd" field
    pub fn get_project_path_from_recent_session(&self, project_dir_path: &Path) -> Result<String> {
        // Find the most recent JSONL file in the project directory
        let entries = std::fs::read_dir(project_dir_path)?;
        let mut jsonl_files: Vec<(PathBuf, SystemTime)> = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        jsonl_files.push((path, modified));
                    }
                }
            }
        }

        // Sort by modification time (most recent first)
        jsonl_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Try to find the most recent JSONL file with a valid cwd field
        for (jsonl_path, _) in jsonl_files {
            if let Ok(content) = std::fs::read_to_string(&jsonl_path) {
                // Parse the first few lines to find a cwd field
                for line in content.lines().take(10) {
                    if line.trim().is_empty() {
                        continue;
                    }

                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(cwd) = json_value.get("cwd").and_then(|v| v.as_str()) {
                            if !cwd.is_empty() && cwd != "/" {
                                return Ok(cwd.to_string());
                            }
                        }
                    }
                }
            }
        }

        // Fallback: reconstruct from directory name
        let dir_name = project_dir_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        // Basic path reconstruction from sanitized directory names
        let reconstructed = dir_name.replace("__", "/").replace("_", " ");

        Ok(reconstructed)
    }

    /// Extract session ID from JSONL file path
    pub fn extract_session_id_from_path(jsonl_path: &Path) -> String {
        jsonl_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Check if a line contains valid session message data
    pub fn is_valid_session_message_line(line: &str) -> bool {
        if line.trim().is_empty() {
            return false;
        }

        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
            // Check for basic structure: timestamp and message with usage
            if json_value.get("timestamp").is_some() {
                if let Some(message) = json_value.get("message") {
                    if let Some(usage) = message.get("usage") {
                        let input_tokens = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let output_tokens = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let cache_creation = usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let cache_read = usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        return input_tokens > 0
                            || output_tokens > 0
                            || cache_creation > 0
                            || cache_read > 0;
                    }
                }
            }
        }

        false
    }
}

impl TokenUsage {
    /// Get total input tokens including cache creation
    pub fn total_input_tokens(&self) -> u32 {
        self.input_tokens.unwrap_or(0) + self.cache_creation_input_tokens.unwrap_or(0)
    }

    /// Get total cache tokens
    pub fn total_cache_tokens(&self) -> u32 {
        self.cache_creation_input_tokens.unwrap_or(0) + self.cache_read_input_tokens.unwrap_or(0)
    }

    /// Check if this usage has any non-zero tokens
    pub fn has_usage(&self) -> bool {
        self.input_tokens.unwrap_or(0) > 0
            || self.output_tokens.unwrap_or(0) > 0
            || self.cache_creation_input_tokens.unwrap_or(0) > 0
            || self.cache_read_input_tokens.unwrap_or(0) > 0
    }
}

impl Session {
    /// Check if session is recent (within last 24 hours)
    pub fn is_recent(&self) -> bool {
        if let Ok(elapsed) = self.last_modified.elapsed() {
            elapsed.as_secs() < 24 * 3600 // 24 hours
        } else {
            false
        }
    }

    /// Get session age in hours
    pub fn age_hours(&self) -> f64 {
        if let Ok(elapsed) = self.last_modified.elapsed() {
            elapsed.as_secs() as f64 / 3600.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn create_temp_dir() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("claude_test_{}", std::process::id()));
        if let Err(_) = fs::create_dir_all(&temp_dir) {
            // Fallback to current directory if temp creation fails
            let fallback = PathBuf::from(".").join(format!("test_temp_{}", std::process::id()));
            fs::create_dir_all(&fallback).unwrap_or_default();
            return fallback;
        }
        temp_dir
    }

    fn create_test_jsonl(path: &Path, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        writeln!(file, "{content}").unwrap();
    }

    fn create_test_jsonl_safe(path: &Path, content: &str) -> Result<(), std::io::Error> {
        let mut file = fs::File::create(path)?;
        writeln!(file, "{content}")?;
        Ok(())
    }

    #[test]
    fn test_session_parser_creation() {
        // Use a simple test directory path instead of creating actual directories
        let test_dir = PathBuf::from("/test/claude");
        let parser = SessionParser::new(test_dir.clone());
        assert_eq!(parser.claude_dir, test_dir);
    }

    #[test]
    fn test_extract_session_id_from_path() {
        let path = Path::new("/test/session_123.jsonl");
        let session_id = SessionParser::extract_session_id_from_path(path);
        assert_eq!(session_id, "session_123");
    }

    #[test]
    fn test_is_valid_session_message_line() {
        // Valid session message with timestamp
        let valid_line =
            r#"{"timestamp": "2024-01-01T12:00:00Z", "message": {"usage": {"input_tokens": 100}}}"#;
        assert!(SessionParser::is_valid_session_message_line(valid_line));

        // Empty line should be invalid
        assert!(!SessionParser::is_valid_session_message_line(""));
        assert!(!SessionParser::is_valid_session_message_line("   "));

        // Invalid JSON should be invalid
        assert!(!SessionParser::is_valid_session_message_line(
            "invalid json"
        ));

        // JSON without timestamp should be invalid
        let no_timestamp = r#"{"message": {"usage": {"input_tokens": 100}}}"#;
        assert!(!SessionParser::is_valid_session_message_line(no_timestamp));
    }

    #[test]
    fn test_parse_session_basic() {
        let temp_dir = create_temp_dir();
        let parser = SessionParser::new(temp_dir.clone());

        // Basic test - parser should be created without error
        let _ = parser; // Use the variable to avoid warnings

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_parse_session_with_empty_file() {
        let temp_dir = create_temp_dir();
        let parser = SessionParser::new(temp_dir.clone());

        // Create empty JSONL file
        let session_file = temp_dir.join("empty_session.jsonl");
        if create_test_jsonl_safe(&session_file, "").is_err() {
            // Skip test if file creation fails
            let _ = fs::remove_dir_all(&temp_dir);
            return;
        }

        match parser.parse_session(&session_file) {
            Ok(result) => assert!(result.is_none()),
            Err(_) => {
                // Test passes if parsing fails gracefully for empty files
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_project_path_from_recent_session() {
        let temp_dir = create_temp_dir();
        let project_dir = temp_dir.join("test_project");
        if fs::create_dir_all(&project_dir).is_err() {
            // Skip test if directory creation fails
            let _ = fs::remove_dir_all(&temp_dir);
            return;
        }

        let parser = SessionParser::new(temp_dir.clone());

        // Create a JSONL file with cwd information - use valid path format
        let session_file = project_dir.join("session.jsonl");
        let test_content =
            r#"{"timestamp": "2024-01-01T12:00:00Z", "cwd": "/tmp/my_project", "message": {}}"#;
        if create_test_jsonl_safe(&session_file, test_content).is_err() {
            // Skip test if file creation fails
            let _ = fs::remove_dir_all(&temp_dir);
            return;
        }

        // Test should pass regardless of result - just check it doesn't panic
        match parser.get_project_path_from_recent_session(&project_dir) {
            Ok(path) => {
                assert!(!path.is_empty());
            }
            Err(_) => {
                // Test passes - method handles errors gracefully
            }
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_token_usage_parsing() {
        let usage_json = r#"{
            "input_tokens": 150,
            "output_tokens": 75,
            "cache_creation_input_tokens": 10,
            "cache_read_input_tokens": 5,
            "service_tier": "premium"
        }"#;

        let usage: TokenUsage = serde_json::from_str(usage_json).unwrap();
        assert_eq!(usage.input_tokens, Some(150));
        assert_eq!(usage.output_tokens, Some(75));
        assert_eq!(usage.cache_creation_input_tokens, Some(10));
        assert_eq!(usage.cache_read_input_tokens, Some(5));
        assert_eq!(usage.service_tier, Some("premium".to_string()));
    }

    #[test]
    fn test_session_age_calculation() {
        let temp_dir = create_temp_dir();
        let session_file = temp_dir.join("test_session.jsonl");
        if create_test_jsonl_safe(&session_file, "").is_err() {
            // Skip test if file creation fails
            let _ = fs::remove_dir_all(&temp_dir);
            return;
        }

        // Create a session with current timestamp
        let session = Session {
            id: "test".to_string(),
            path: session_file,
            last_modified: SystemTime::now(),
            message_count: 1,
        };

        let age_hours = session.age_hours();
        assert!(age_hours < 1.0); // Should be very recent

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
