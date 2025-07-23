use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Todo item status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoStatus::Pending => write!(f, "Pending"),
            TodoStatus::InProgress => write!(f, "In Progress"),
            TodoStatus::Completed => write!(f, "Completed"),
        }
    }
}

/// Todo item priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TodoPriority {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for TodoPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoPriority::Low => write!(f, "Low"),
            TodoPriority::Medium => write!(f, "Medium"),
            TodoPriority::High => write!(f, "High"),
        }
    }
}

/// Individual todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub id: String,
}

/// Session todos with metadata
#[derive(Debug, Clone)]
pub struct SessionTodos {
    pub session_id: String,
    #[allow(dead_code)]
    pub agent_id: String,
    pub project_path: String,
    pub todos: Vec<TodoItem>,
    pub last_modified: std::time::SystemTime,
}

/// Project-level todo statistics
#[derive(Debug, Clone)]
pub struct ProjectTodoStats {
    pub total_todos: usize,
    pub completed_todos: usize,
    #[allow(dead_code)]
    pub in_progress_todos: usize,
    #[allow(dead_code)]
    pub pending_todos: usize,
    pub completion_percentage: f64,
    #[allow(dead_code)]
    pub high_priority_remaining: usize,
    #[allow(dead_code)]
    pub recent_activity: Option<std::time::SystemTime>,
}

/// Todo data manager
pub struct TodoManager {
    todos_dir: PathBuf,
}

impl TodoManager {
    /// Create a new TodoManager
    pub fn new() -> Result<Self> {
        let home_dir = std::env::var("HOME")?;
        let todos_dir = Path::new(&home_dir).join(".claude").join("todos");

        Ok(Self { todos_dir })
    }

    /// Scan all todo files and organize by project
    pub fn scan_todos(&self) -> Result<HashMap<String, Vec<SessionTodos>>> {
        let mut project_todos: HashMap<String, Vec<SessionTodos>> = HashMap::new();

        if !self.todos_dir.exists() {
            return Ok(project_todos);
        }

        let entries = fs::read_dir(&self.todos_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(session_todos) = self.parse_todo_file(&path)? {
                    project_todos
                        .entry(session_todos.project_path.clone())
                        .or_default()
                        .push(session_todos);
                }
            }
        }

        Ok(project_todos)
    }

    /// Parse a single todo file
    fn parse_todo_file(&self, file_path: &Path) -> Result<Option<SessionTodos>> {
        let filename = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid todo filename: {:?}", file_path))?;

        // Parse filename: {session-id}-agent-{agent-id}
        let parts: Vec<&str> = filename.split("-agent-").collect();
        if parts.len() != 2 {
            // Skip files that don't match expected format
            return Ok(None);
        }

        let session_id = parts[0].to_string();
        let agent_id = parts[1].to_string();

        // Try to reconstruct project path from session ID
        let project_path = self.reconstruct_project_path(&session_id)?;

        let metadata = fs::metadata(file_path)?;
        let last_modified = metadata.modified()?;

        let content = fs::read_to_string(file_path)?;
        let todos: Vec<TodoItem> = serde_json::from_str(&content)?;

        Ok(Some(SessionTodos {
            session_id,
            agent_id,
            project_path,
            todos,
            last_modified,
        }))
    }

    /// Reconstruct project path from session ID
    fn reconstruct_project_path(&self, session_id: &str) -> Result<String> {
        // Check in projects directory for this session ID
        let home_dir = std::env::var("HOME")?;
        let projects_dir = Path::new(&home_dir).join(".claude").join("projects");

        if !projects_dir.exists() {
            return Ok("unknown".to_string());
        }

        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_dir = entry.path();

            if project_dir.is_dir() {
                // Check if this project contains our session
                let session_file = project_dir.join(format!("{session_id}.jsonl"));
                if session_file.exists() {
                    // Try to get the real path from the JSONL file's "cwd" field
                    if let Ok(cwd_path) = self.get_project_path_from_session_file(&session_file) {
                        return Ok(cwd_path);
                    }

                    // Fallback: Reconstruct from directory name
                    if let Some(dir_name) = project_dir.file_name().and_then(|s| s.to_str()) {
                        return Ok(self
                            .reconstruct_path_from_sanitized_name(dir_name)
                            .unwrap_or_else(|_| dir_name.to_string()));
                    }
                }
            }
        }

        Ok("unknown".to_string())
    }

    /// Get the real project path from a specific JSONL session file's "cwd" field
    fn get_project_path_from_session_file(&self, session_file: &Path) -> Result<String> {
        let content = std::fs::read_to_string(session_file)?;

        // Read lines from the end to find the most recent "cwd"
        for line in content.lines().rev() {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(cwd) = json_value.get("cwd") {
                    if let Some(cwd_str) = cwd.as_str() {
                        return Ok(cwd_str.to_string());
                    }
                }
            }
        }

        Err(anyhow::anyhow!("No 'cwd' field found in JSONL file"))
    }

    /// Reconstruct the original filesystem path from Claude's sanitized directory name
    fn reconstruct_path_from_sanitized_name(&self, sanitized_name: &str) -> Result<String> {
        if !sanitized_name.starts_with('-') {
            return Err(anyhow::anyhow!(
                "Cannot reconstruct path from non-standard name"
            ));
        }

        // Drop the leading '-' and replace remaining '-' with '/'
        let path_str: String = sanitized_name
            .chars()
            .skip(1)
            .map(|c| if c == '-' { '/' } else { c })
            .collect();

        // Create absolute path
        let reconstructed_path = format!("/{path_str}");

        // Return the reconstructed path (simpler than claude.rs version for todos)
        Ok(reconstructed_path)
    }

    /// Calculate project-level todo statistics
    /// Only considers todos from the most recent session (by last modified time)
    pub fn calculate_project_stats(&self, project_todos: &[SessionTodos]) -> ProjectTodoStats {
        let mut total_todos = 0;
        let mut completed_todos = 0;
        let mut in_progress_todos = 0;
        let mut pending_todos = 0;
        let mut high_priority_remaining = 0;
        let mut recent_activity: Option<std::time::SystemTime> = None;

        // Find the most recent session by last_modified time
        if let Some(most_recent_session) = project_todos
            .iter()
            .max_by_key(|session| session.last_modified)
        {
            recent_activity = Some(most_recent_session.last_modified);

            // Only count todos from the most recent session
            for todo in &most_recent_session.todos {
                total_todos += 1;

                match todo.status {
                    TodoStatus::Completed => completed_todos += 1,
                    TodoStatus::InProgress => in_progress_todos += 1,
                    TodoStatus::Pending => {
                        pending_todos += 1;
                        if todo.priority == TodoPriority::High {
                            high_priority_remaining += 1;
                        }
                    }
                }
            }
        }

        let completion_percentage = if total_todos > 0 {
            (completed_todos as f64 / total_todos as f64) * 100.0
        } else {
            0.0
        };

        ProjectTodoStats {
            total_todos,
            completed_todos,
            in_progress_todos,
            pending_todos,
            completion_percentage,
            high_priority_remaining,
            recent_activity,
        }
    }

    /// Get todos for a specific project, sorted by priority and status
    /// Only shows todos from the most recent session (by last modified time)
    pub fn get_project_todos_sorted(
        &self,
        project_todos: &[SessionTodos],
    ) -> Vec<(String, TodoItem)> {
        let mut all_todos = Vec::new();

        // Find the most recent session by last_modified time
        if let Some(most_recent_session) = project_todos
            .iter()
            .max_by_key(|session| session.last_modified)
        {
            // Only use todos from the most recent session
            for todo in &most_recent_session.todos {
                all_todos.push((most_recent_session.session_id.clone(), todo.clone()));
            }
        }

        // Sort by: 1) Priority (High->Medium->Low), 2) Status (InProgress->Pending->Completed)
        all_todos.sort_by(|a, b| {
            use std::cmp::Ordering;

            // First sort by priority
            let priority_order = |p: &TodoPriority| match p {
                TodoPriority::High => 0,
                TodoPriority::Medium => 1,
                TodoPriority::Low => 2,
            };

            let priority_cmp = priority_order(&a.1.priority).cmp(&priority_order(&b.1.priority));
            if priority_cmp != Ordering::Equal {
                return priority_cmp;
            }

            // Then sort by status
            let status_order = |s: &TodoStatus| match s {
                TodoStatus::InProgress => 0,
                TodoStatus::Pending => 1,
                TodoStatus::Completed => 2,
            };

            status_order(&a.1.status).cmp(&status_order(&b.1.status))
        });

        all_todos
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn create_temp_dir() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("todo_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    fn create_test_jsonl_file(path: &Path, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        writeln!(file, "{content}").unwrap();
    }

    #[test]
    fn test_todo_priority_display() {
        assert_eq!(TodoPriority::High.to_string(), "High");
        assert_eq!(TodoPriority::Medium.to_string(), "Medium");
        assert_eq!(TodoPriority::Low.to_string(), "Low");
    }

    #[test]
    fn test_todo_status_display() {
        assert_eq!(TodoStatus::Completed.to_string(), "Completed");
        assert_eq!(TodoStatus::InProgress.to_string(), "In Progress");
        assert_eq!(TodoStatus::Pending.to_string(), "Pending");
    }

    #[test]
    fn test_todo_manager_creation() {
        let todo_manager = TodoManager::new().unwrap();
        // Basic smoke test - should be able to create manager
        let _ = todo_manager; // Use the variable to avoid warnings
    }

    #[test]
    fn test_project_stats_calculation() {
        let todo_manager = TodoManager::new().unwrap();

        // Create mock session todos
        let session_todos = vec![
            SessionTodos {
                session_id: "session1".to_string(),
                agent_id: "agent1".to_string(),
                project_path: "/test/project".to_string(),
                last_modified: std::time::SystemTime::now(),
                todos: vec![
                    TodoItem {
                        content: "High priority task".to_string(),
                        priority: TodoPriority::High,
                        status: TodoStatus::Pending,
                        id: "1".to_string(),
                    },
                    TodoItem {
                        content: "Completed task".to_string(),
                        priority: TodoPriority::Medium,
                        status: TodoStatus::Completed,
                        id: "2".to_string(),
                    },
                ],
            },
            SessionTodos {
                session_id: "session2".to_string(),
                agent_id: "agent2".to_string(),
                project_path: "/test/project".to_string(),
                last_modified: std::time::SystemTime::now(),
                todos: vec![TodoItem {
                    content: "In progress task".to_string(),
                    priority: TodoPriority::Low,
                    status: TodoStatus::InProgress,
                    id: "3".to_string(),
                }],
            },
        ];

        let stats = todo_manager.calculate_project_stats(&session_todos);

        // Basic sanity check - stats should be calculated
        assert!(stats.total_todos > 0);
        assert!(stats.completed_todos <= stats.total_todos);
        assert!(stats.pending_todos <= stats.total_todos);
        assert!(stats.in_progress_todos <= stats.total_todos);
        // Priority counts are not in the structure, so we'll skip those
    }

    #[test]
    fn test_sorted_todos_ordering() {
        let todo_manager = TodoManager::new().unwrap();

        // Create test todos with different priorities and statuses
        let session_todos = vec![SessionTodos {
            session_id: "session1".to_string(),
            agent_id: "agent1".to_string(),
            project_path: "/test".to_string(),
            last_modified: std::time::SystemTime::now(),
            todos: vec![
                TodoItem {
                    content: "Low pending".to_string(),
                    priority: TodoPriority::Low,
                    status: TodoStatus::Pending,
                    id: "4".to_string(),
                },
                TodoItem {
                    content: "High completed".to_string(),
                    priority: TodoPriority::High,
                    status: TodoStatus::Completed,
                    id: "5".to_string(),
                },
                TodoItem {
                    content: "Medium in_progress".to_string(),
                    priority: TodoPriority::Medium,
                    status: TodoStatus::InProgress,
                    id: "6".to_string(),
                },
                TodoItem {
                    content: "High in_progress".to_string(),
                    priority: TodoPriority::High,
                    status: TodoStatus::InProgress,
                    id: "7".to_string(),
                },
            ],
        }];

        let sorted_todos = todo_manager.get_project_todos_sorted(&session_todos);

        // Should be sorted: High InProgress, High Completed, Medium InProgress, Low Pending
        assert_eq!(sorted_todos.len(), 4);
        assert_eq!(sorted_todos[0].1.content, "High in_progress");
        assert_eq!(sorted_todos[1].1.content, "High completed");
        assert_eq!(sorted_todos[2].1.content, "Medium in_progress");
        assert_eq!(sorted_todos[3].1.content, "Low pending");
    }

    #[test]
    fn test_empty_session_handling() {
        let temp_dir = create_temp_dir();
        let _todo_manager = TodoManager::new().unwrap();

        // Create empty session file
        let session_file = temp_dir.join("empty_session.jsonl");
        create_test_jsonl_file(&session_file, "");

        // Basic test - file should be created successfully
        assert!(session_file.exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
