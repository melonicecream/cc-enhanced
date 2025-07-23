#![allow(dead_code)] // Allow unused code during migration

use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::pricing::{OpenRouterPricing, PricingManager};
use super::project_scanner::{Project, ProjectScanner};
use super::session_parser::{Session, SessionParser};
use super::usage_calculator::{UsageCalculator, UsageStats, ProjectAnalytics, DailyUsage, ModelUsage};

/// Enhanced todo item from ~/.claude/todos/ directory
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedTodoItem {
    pub content: String,
    pub status: String,
    pub priority: String,
    pub id: String,
    pub session_id: Option<String>,    // Session this todo came from
    pub project_name: Option<String>,  // Inferred project name
}

/// Cost warning states from ~/.claude/config/notification_states.json
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationStates {
    pub switch_to_custom: NotificationState,
    pub exceed_max_limit: NotificationState,
    pub tokens_will_run_out: NotificationState,
    pub cost_will_exceed: NotificationState,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NotificationState {
    pub triggered: bool,
    pub timestamp: Option<String>,
}

/// Session intelligence metrics
#[derive(Debug, Clone)]
pub struct SessionMetrics {
    pub session_id: String,
    pub project_name: String,
    pub total_messages: usize,
    pub total_tokens: u32,
    pub average_response_time: f64,
    pub model_switches: u32,
    pub efficiency_score: f64,
}

/// Main Claude data management orchestration layer
pub struct ClaudeDataManager {
    claude_dir: PathBuf,
    project_scanner: ProjectScanner,
    session_parser: SessionParser,
    usage_calculator: UsageCalculator,
    pricing_manager: PricingManager,
}

impl ClaudeDataManager {
    /// Create a new Claude data manager
    pub fn new() -> Result<Self> {
        let home_dir = std::env::var("HOME")?;
        let claude_dir = Path::new(&home_dir).join(".claude");
        
        let project_scanner = ProjectScanner::new(claude_dir.clone());
        let session_parser = SessionParser::new(claude_dir.clone());
        let usage_calculator = UsageCalculator::new(claude_dir.clone());
        
        let mut pricing_manager = PricingManager::new(&claude_dir);
        // Initialize cache from file (ignore errors, fallback will be used)
        let _ = pricing_manager.init_cache();

        Ok(Self { 
            claude_dir,
            project_scanner,
            session_parser,
            usage_calculator,
            pricing_manager,
        })
    }

    /// Get the Claude directory path
    pub fn claude_dir(&self) -> &Path {
        &self.claude_dir
    }

    /// Scan for all Claude projects
    pub fn scan_projects(&self) -> Result<Vec<Project>> {
        self.project_scanner.scan_projects()
    }

    /// Get a project by name
    pub fn get_project_by_name(&self, name: &str) -> Result<Option<Project>> {
        self.project_scanner.get_project_by_name(name)
    }

    /// Get active projects (with recent activity)
    pub fn get_active_projects(&self) -> Result<Vec<Project>> {
        self.project_scanner.get_active_projects()
    }

    /// Calculate today's usage statistics
    pub fn calculate_today_usage(&self) -> Result<UsageStats> {
        self.usage_calculator.calculate_today_usage(&self.pricing_manager.openrouter_pricing)
    }

    /// Calculate daily usage breakdown for the last N days
    pub fn calculate_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>> {
        self.usage_calculator.calculate_daily_usage(days, &self.pricing_manager.openrouter_pricing)
    }

    /// Calculate model usage distribution
    pub fn calculate_model_usage(&self) -> Result<Vec<ModelUsage>> {
        self.usage_calculator.calculate_model_usage(&self.pricing_manager.openrouter_pricing)
    }

    /// Calculate comprehensive analytics for a specific project
    pub fn calculate_project_analytics(&self, project: &Project) -> Result<ProjectAnalytics> {
        self.usage_calculator.calculate_project_analytics(project)
    }

    /// Calculate burn rate for a specific project (cost per day)
    pub fn calculate_burn_rate(&self, project: &Project) -> f64 {
        // Implementation would calculate the daily cost average for the project
        // This is a simplified placeholder
        0.0
    }

    /// Round time to the nearest hour (for block calculations)
    pub fn round_to_hour(&self, time: SystemTime) -> SystemTime {
        use std::time::{Duration, UNIX_EPOCH};
        
        if let Ok(duration_since_epoch) = time.duration_since(UNIX_EPOCH) {
            let seconds = duration_since_epoch.as_secs();
            let rounded_seconds = (seconds / 3600) * 3600; // Round down to nearest hour
            UNIX_EPOCH + Duration::from_secs(rounded_seconds)
        } else {
            time // Return original if calculation fails
        }
    }

    /// Calculate next reset time (based on Claude's 5-hour blocks)
    pub fn calculate_next_reset_time(&self) -> SystemTime {
        let now = SystemTime::now();
        let rounded_now = self.round_to_hour(now);
        
        // Add 5 hours for the next reset
        rounded_now + std::time::Duration::from_secs(5 * 3600)
    }

    /// Find when the current active block ends
    pub fn find_active_block_end_time(&self) -> Option<SystemTime> {
        // This would analyze recent usage patterns to determine block boundaries
        // For now, return the next reset time
        Some(self.calculate_next_reset_time())
    }

    /// Get human-readable time until next reset
    pub fn time_until_reset(&self) -> String {
        let next_reset = self.calculate_next_reset_time();
        let now = SystemTime::now();
        
        if let Ok(duration) = next_reset.duration_since(now) {
            let total_seconds = duration.as_secs();
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            
            if hours > 0 {
                format!("{}h {}m {}s", hours, minutes, seconds)
            } else if minutes > 0 {
                format!("{}m {}s", minutes, seconds)
            } else {
                format!("{}s", seconds)
            }
        } else {
            "Reset time passed".to_string()
        }
    }

    /// Load all todos from ~/.claude/todos/ directory - Cross-session todo intelligence
    pub fn load_enhanced_todos(&self) -> Result<Vec<EnhancedTodoItem>> {
        use std::fs;
        
        let todos_dir = self.claude_dir.join("todos");
        if !todos_dir.exists() {
            return Ok(Vec::new());
        }

        let mut enhanced_todos = Vec::new();

        for entry in fs::read_dir(&todos_dir)? {
            let entry = entry?;
            let file_path = entry.path();

            if file_path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&file_path)?;
                let file_name = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Extract session ID from filename (format: {session-id}-agent-{agent-id}.json)
                let session_id = file_name.split('-').next().map(|s| s.to_string());

                // Try to parse as todo array
                if let Ok(todos) = serde_json::from_str::<Vec<EnhancedTodoItem>>(&content) {
                    for mut todo in todos {
                        todo.session_id = session_id.clone();
                        // Infer project name from session if possible
                        todo.project_name = self.infer_project_from_session(&session_id.as_deref().unwrap_or(""));
                        enhanced_todos.push(todo);
                    }
                }
            }
        }

        Ok(enhanced_todos)
    }

    /// Load cost notification states - Smart cost warning system
    pub fn load_notification_states(&self) -> Result<NotificationStates> {
        use std::fs;
        
        let config_path = self.claude_dir.join("config").join("notification_states.json");
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let states: NotificationStates = serde_json::from_str(&content)?;
            Ok(states)
        } else {
            // Return default states if file doesn't exist
            Ok(NotificationStates {
                switch_to_custom: NotificationState {
                    triggered: false,
                    timestamp: None,
                },
                exceed_max_limit: NotificationState {
                    triggered: false,
                    timestamp: None,
                },
                tokens_will_run_out: NotificationState {
                    triggered: false,
                    timestamp: None,
                },
                cost_will_exceed: NotificationState {
                    triggered: false,
                    timestamp: None,
                },
            })
        }
    }

    /// Analyze session intelligence - Advanced session metrics
    pub fn analyze_session_intelligence(&self) -> Result<Vec<SessionMetrics>> {
        let projects = self.scan_projects()?;
        let mut session_metrics = Vec::new();

        for project in &projects {
            for session in &project.sessions {
                // This would perform deep session analysis
                // For now, return basic metrics
                let metrics = SessionMetrics {
                    session_id: session.id.clone(),
                    project_name: project.name.clone(),
                    total_messages: session.message_count,
                    total_tokens: 0, // Would be calculated from session data
                    average_response_time: 0.0, // Would be calculated from timestamps
                    model_switches: 0, // Would be calculated from model changes
                    efficiency_score: 0.0, // Would be calculated based on various metrics
                };
                session_metrics.push(metrics);
            }
        }

        Ok(session_metrics)
    }

    /// Infer project name from session ID (helper method)
    fn infer_project_from_session(&self, session_id: &str) -> Option<String> {
        // This would analyze session data to infer the project name
        // For now, return None as placeholder
        None
    }

    /// Get pricing manager reference
    pub fn pricing_manager(&self) -> &PricingManager {
        &self.pricing_manager
    }

    /// Get usage calculator reference
    pub fn usage_calculator(&self) -> &UsageCalculator {
        &self.usage_calculator
    }

    /// Get project scanner reference
    pub fn project_scanner(&self) -> &ProjectScanner {
        &self.project_scanner
    }

    /// Get session parser reference
    pub fn session_parser(&self) -> &SessionParser {
        &self.session_parser
    }
}