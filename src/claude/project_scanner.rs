#![allow(dead_code)] // Allow unused code during migration

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::session_parser::{Session, SessionParser};

/// Represents a Claude project
#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub sessions: Vec<Session>,
    pub is_active: bool,
}

/// Project scanner for Claude directories
pub struct ProjectScanner {
    claude_dir: PathBuf,
    session_parser: SessionParser,
}

impl ProjectScanner {
    /// Create new project scanner
    pub fn new(claude_dir: PathBuf) -> Self {
        let session_parser = SessionParser::new(claude_dir.clone());
        Self {
            claude_dir,
            session_parser,
        }
    }

    /// Scan for all projects in the Claude directory
    pub fn scan_projects(&self) -> Result<Vec<Project>> {
        let projects_dir = self.claude_dir.join("projects");

        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut projects = Vec::new();
        let entries = fs::read_dir(&projects_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(project) = self.parse_project(&path)? {
                    projects.push(project);
                }
            }
        }

        // Sort by most recently active
        projects.sort_by(|a, b| {
            let a_last = a
                .sessions
                .iter()
                .map(|s| s.last_modified)
                .max()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let b_last = b
                .sessions
                .iter()
                .map(|s| s.last_modified)
                .max()
                .unwrap_or(SystemTime::UNIX_EPOCH);
            b_last.cmp(&a_last)
        });

        Ok(projects)
    }

    /// Parse a project directory
    fn parse_project(&self, project_dir_path: &Path) -> Result<Option<Project>> {
        let sanitized_name = project_dir_path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("Invalid project directory path: {:?}", project_dir_path)
            })?;

        // Try to get the real path from the most recent JSONL file's "cwd" field
        let (original_path, is_orphaned) = match self
            .session_parser
            .get_project_path_from_recent_session(project_dir_path)
        {
            Ok(cwd_path) => {
                if PathBuf::from(&cwd_path).exists() {
                    (PathBuf::from(cwd_path), false)
                } else {
                    // CWD exists in JSONL but directory doesn't exist anymore
                    (PathBuf::from(format!("Orphaned: {cwd_path}")), true)
                }
            }
            Err(_) => {
                // Fallback to old reconstruction method
                match self.reconstruct_path_from_sanitized_name(sanitized_name) {
                    Ok(p) => {
                        if p.exists() {
                            (p, false)
                        } else {
                            (PathBuf::from(format!("Orphaned: {sanitized_name}")), true)
                        }
                    }
                    Err(_) => (PathBuf::from(format!("Unknown: {sanitized_name}")), true),
                }
            }
        };

        let project_name = if is_orphaned {
            // For orphaned/unknown projects, use a cleaner name
            if let Some(last_part) = sanitized_name.split('-').next_back() {
                if last_part.is_empty() {
                    sanitized_name.to_string()
                } else {
                    last_part.to_string()
                }
            } else {
                sanitized_name.to_string()
            }
        } else {
            original_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(sanitized_name)
                .to_string()
        };

        // Scan for sessions in the project directory
        let mut sessions = Vec::new();
        let entries = fs::read_dir(project_dir_path)?;

        for entry in entries {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Some(session) = self.session_parser.parse_session(&file_path)? {
                    sessions.push(session);
                }
            }
        }

        if sessions.is_empty() {
            return Ok(None);
        }

        // Check if project is active (has recent activity)
        let is_active = sessions.iter().any(|session| {
            if let Ok(elapsed) = session.last_modified.elapsed() {
                elapsed.as_secs() < 24 * 3600 // Active if modified in last 24 hours
            } else {
                false
            }
        });

        Ok(Some(Project {
            name: project_name,
            path: original_path,
            sessions,
            is_active,
        }))
    }

    /// Reconstruct original project path from sanitized directory name
    fn reconstruct_path_from_sanitized_name(&self, sanitized_name: &str) -> Result<PathBuf> {
        // Claude sanitizes directory names by replacing special characters
        // This is a reverse engineering attempt

        // Common patterns:
        // 1. Spaces become "-"
        // 2. "/" becomes "--"
        // 3. Special characters get removed or replaced

        let reconstructed = sanitized_name.replace("--", "/").replace("-", " ");

        // Try various common project locations
        let possible_paths = vec![
            PathBuf::from(&reconstructed),
            dirs::home_dir().unwrap_or_default().join(&reconstructed),
            dirs::home_dir()
                .unwrap_or_default()
                .join("Desktop")
                .join(&reconstructed),
            dirs::home_dir()
                .unwrap_or_default()
                .join("Documents")
                .join(&reconstructed),
            dirs::home_dir()
                .unwrap_or_default()
                .join("workspace")
                .join(&reconstructed),
            dirs::home_dir()
                .unwrap_or_default()
                .join("dev")
                .join(&reconstructed),
            dirs::home_dir()
                .unwrap_or_default()
                .join("projects")
                .join(&reconstructed),
        ];

        for path in possible_paths {
            if path.exists() {
                return Ok(path);
            }
        }

        // If nothing found, return the best guess
        Ok(PathBuf::from(reconstructed))
    }

    /// Get project by name
    pub fn get_project_by_name(&self, name: &str) -> Result<Option<Project>> {
        let projects = self.scan_projects()?;
        Ok(projects.into_iter().find(|p| p.name == name))
    }

    /// Get active projects (with recent activity)
    pub fn get_active_projects(&self) -> Result<Vec<Project>> {
        let projects = self.scan_projects()?;
        Ok(projects.into_iter().filter(|p| p.is_active).collect())
    }

    /// Get project statistics
    pub fn get_project_stats(&self) -> Result<ProjectScanStats> {
        let projects = self.scan_projects()?;

        let total_projects = projects.len();
        let active_projects = projects.iter().filter(|p| p.is_active).count();
        let orphaned_projects = projects
            .iter()
            .filter(|p| p.path.to_string_lossy().contains("Orphaned:"))
            .count();
        let total_sessions: usize = projects.iter().map(|p| p.sessions.len()).sum();

        let most_recent_activity = projects
            .iter()
            .filter_map(|p| p.sessions.iter().map(|s| s.last_modified).max())
            .max();

        Ok(ProjectScanStats {
            total_projects,
            active_projects,
            orphaned_projects,
            total_sessions,
            most_recent_activity,
        })
    }

    /// Find projects by path pattern
    pub fn find_projects_by_pattern(&self, pattern: &str) -> Result<Vec<Project>> {
        let projects = self.scan_projects()?;
        let pattern = pattern.to_lowercase();

        Ok(projects
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&pattern)
                    || p.path.to_string_lossy().to_lowercase().contains(&pattern)
            })
            .collect())
    }

    /// Check if a project directory has valid sessions
    pub fn has_valid_sessions(&self, project_dir: &Path) -> Result<bool> {
        if !project_dir.is_dir() {
            return Ok(false);
        }

        let entries = fs::read_dir(project_dir)?;
        for entry in entries {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl")
                && self.session_parser.parse_session(&file_path)?.is_some()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Get project directory size
    pub fn get_project_size(&self, project_dir: &Path) -> Result<u64> {
        let mut total_size = 0;

        if !project_dir.is_dir() {
            return Ok(0);
        }

        let entries = fs::read_dir(project_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(metadata) = fs::metadata(&path) {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }

    /// Clean up empty or invalid project directories
    pub fn cleanup_invalid_projects(&self) -> Result<Vec<String>> {
        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut cleaned_projects = Vec::new();
        let entries = fs::read_dir(&projects_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let has_sessions = self.has_valid_sessions(&path)?;
                if !has_sessions {
                    let project_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    // Only clean up if the directory is truly empty or has no valid sessions
                    if self.is_safe_to_cleanup(&path)? {
                        fs::remove_dir_all(&path)?;
                        cleaned_projects.push(project_name);
                    }
                }
            }
        }

        Ok(cleaned_projects)
    }

    /// Check if a project directory is safe to cleanup
    fn is_safe_to_cleanup(&self, project_dir: &Path) -> Result<bool> {
        let entries = fs::read_dir(project_dir)?;
        let mut file_count = 0;
        let mut has_jsonl = false;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                file_count += 1;
                if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    has_jsonl = true;
                    // Check if the JSONL file has any meaningful content
                    if let Ok(content) = fs::read_to_string(&path) {
                        if content
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .count()
                            > 0
                        {
                            return Ok(false); // Has content, not safe to cleanup
                        }
                    }
                }
            }
        }

        // Safe to cleanup if: no files, or only empty JSONL files
        Ok(file_count == 0 || (has_jsonl && file_count <= 2)) // Allow for 1-2 empty files
    }
}

/// Statistics about project scanning
#[derive(Debug, Clone)]
pub struct ProjectScanStats {
    pub total_projects: usize,
    pub active_projects: usize,
    pub orphaned_projects: usize,
    pub total_sessions: usize,
    pub most_recent_activity: Option<SystemTime>,
}

impl Project {
    /// Get the most recent session
    pub fn most_recent_session(&self) -> Option<&Session> {
        self.sessions.iter().max_by_key(|s| s.last_modified)
    }

    /// Get total message count across all sessions
    pub fn total_messages(&self) -> usize {
        self.sessions.iter().map(|s| s.message_count).sum()
    }

    /// Check if project has been active in the last N days
    pub fn active_within_days(&self, days: u64) -> bool {
        let threshold = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            - (days * 24 * 3600);

        self.sessions.iter().any(|s| {
            s.last_modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                > threshold
        })
    }

    /// Get project age in days
    pub fn age_days(&self) -> f64 {
        if let Some(oldest_session) = self.sessions.iter().min_by_key(|s| s.last_modified) {
            if let Ok(elapsed) = oldest_session.last_modified.elapsed() {
                elapsed.as_secs() as f64 / (24.0 * 3600.0)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Check if project path exists on filesystem
    pub fn path_exists(&self) -> bool {
        !self.path.to_string_lossy().contains("Orphaned:")
            && !self.path.to_string_lossy().contains("Unknown:")
            && self.path.exists()
    }

    /// Get project display name (cleaned up)
    pub fn display_name(&self) -> &str {
        &self.name
    }
}
