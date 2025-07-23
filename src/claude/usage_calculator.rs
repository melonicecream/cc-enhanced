#![allow(dead_code)] // Allow unused code during migration

use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::pricing::TokenUsage as PricingTokenUsage;
use super::session_parser::SessionMessage;

/// Usage statistics for a time period
#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
    pub total_cost: f64,
    pub message_count: u32,
    pub reset_time: Option<SystemTime>,
    pub is_subscription_user: bool, // true if using fallback pricing (subscription)
}

/// Session block statistics (5-hour periods)
#[derive(Debug, Clone)]
pub struct SessionBlock {
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub usage_stats: UsageStats,
    pub is_active: bool,
}

/// Project analytics data
#[derive(Debug, Clone)]
pub struct ProjectAnalytics {
    pub total_sessions: usize,
    pub total_messages: usize,
    pub total_tokens: u32,
    pub estimated_cost: f64,
    pub first_session: Option<SystemTime>,
    pub last_session: Option<SystemTime>,
    pub cache_efficiency: f64, // Percentage of cache usage
    pub session_blocks: Vec<SessionBlock>,
}

/// Daily usage breakdown
#[derive(Debug, Clone)]
pub struct DailyUsage {
    pub date: NaiveDate,
    pub usage_stats: UsageStats,
}

/// Model usage breakdown
#[derive(Debug, Clone)]
pub struct ModelUsage {
    pub model_name: String,
    pub usage_stats: UsageStats,
}

/// Daily usage detail with enhanced analytics
#[derive(Debug, Clone)]
pub struct DailyUsageDetail {
    pub date: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
    pub total_cost: f64,
    pub message_count: u32,
    pub session_count: u32,
    pub models_used: Vec<String>,
    pub peak_hour: Option<String>,
    pub efficiency_score: f64, // Cache usage efficiency
}

/// Project usage statistics
#[derive(Debug, Clone)]
pub struct ProjectUsageStats {
    pub project_name: String,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub session_count: u32,
    pub message_count: u32,
    pub models_used: HashSet<String>,
    pub first_activity: Option<DateTime<Local>>,
    pub last_activity: Option<DateTime<Local>>,
    pub cache_efficiency: f64,
    pub most_used_model: String,
    pub avg_session_length: f64, // Average session duration in minutes
}

/// Usage calculator for Claude session data
pub struct UsageCalculator {
    claude_dir: PathBuf,
}

impl UsageCalculator {
    /// Create new usage calculator
    pub fn new(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// Calculate today's usage statistics
    pub fn calculate_today_usage(&self) -> Result<UsageStats> {
        let mut stats = UsageStats::default();
        let today = Local::now().date_naive();

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(stats);
        }

        // Scan all JSONL files for today's data
        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        self.process_jsonl_for_usage(&file_path, &mut stats, today)?;
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Process a JSONL file for usage statistics
    fn process_jsonl_for_usage(
        &self,
        file_path: &Path,
        stats: &mut UsageStats,
        target_date: NaiveDate,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                // Check if this entry is from today
                if let Some(timestamp_str) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(datetime) = DateTime::parse_from_rfc3339(timestamp_str) {
                        let entry_date = datetime.with_timezone(&Local).date_naive();

                        if entry_date == target_date {
                            self.extract_usage_from_entry(&entry, stats)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract usage data from a single JSONL entry
    fn extract_usage_from_entry(&self, entry: &Value, stats: &mut UsageStats) -> Result<()> {
        let mut current_input_tokens = 0u32;
        let mut current_output_tokens = 0u32;
        let mut current_cache_creation_tokens = 0u32;
        let mut current_cache_read_tokens = 0u32;

        // Check for usage data in message.usage
        if let Some(message) = entry.get("message") {
            if let Some(usage) = message.get("usage") {
                if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                    current_input_tokens = input as u32;
                }
                if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                    current_output_tokens = output as u32;
                }
                if let Some(cache_creation) = usage
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_u64())
                {
                    current_cache_creation_tokens = cache_creation as u32;
                }
                if let Some(cache_read) = usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_u64())
                {
                    current_cache_read_tokens = cache_read as u32;
                }
            }
        }

        // Only add if there's actual usage
        if current_input_tokens > 0
            || current_output_tokens > 0
            || current_cache_creation_tokens > 0
            || current_cache_read_tokens > 0
        {
            stats.input_tokens += current_input_tokens;
            stats.output_tokens += current_output_tokens;
            stats.cache_creation_tokens += current_cache_creation_tokens;
            stats.cache_read_tokens += current_cache_read_tokens;
            stats.message_count += 1;

            // Calculate cost using model information if available
            if let Some(message) = entry.get("message") {
                if let Some(model) = message.get("model").and_then(|v| v.as_str()) {
                    let usage = PricingTokenUsage {
                        input_tokens: current_input_tokens,
                        output_tokens: current_output_tokens,
                        cache_creation_tokens: current_cache_creation_tokens,
                        cache_read_tokens: current_cache_read_tokens,
                    };

                    // Use basic calculation with fallback pricing
                    // This would be replaced with actual pricing manager in integration
                    let cost = self.calculate_basic_cost(&usage, model);
                    stats.total_cost += cost;
                }
            }
        }

        Ok(())
    }

    /// Calculate usage statistics for a single session
    pub fn calculate_session_usage(&self, session_path: &Path) -> Result<UsageStats> {
        let mut stats = UsageStats::default();
        let content = fs::read_to_string(session_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                self.extract_usage_from_entry(&entry, &mut stats)?;
            }
        }

        Ok(stats)
    }

    /// Calculate daily usage breakdown for the last N days (optimized single-pass)
    pub fn calculate_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>> {
        let today = Local::now().date_naive();
        let mut daily_stats: HashMap<NaiveDate, UsageStats> = HashMap::new();

        // Initialize empty stats for each day
        for i in 0..days {
            let date = today - chrono::Duration::days(i as i64);
            daily_stats.insert(date, UsageStats::default());
        }

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(daily_stats
                .into_iter()
                .map(|(date, usage_stats)| DailyUsage { date, usage_stats })
                .collect());
        }

        // Single pass through all JSONL files
        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        self.process_jsonl_for_daily_usage(&file_path, &mut daily_stats, days)?;
                    }
                }
            }
        }

        Ok(daily_stats
            .into_iter()
            .map(|(date, usage_stats)| DailyUsage { date, usage_stats })
            .collect())
    }

    /// Process JSONL file for daily usage statistics
    fn process_jsonl_for_daily_usage(
        &self,
        file_path: &Path,
        daily_stats: &mut HashMap<NaiveDate, UsageStats>,
        days: u32,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path)?;
        let today = Local::now().date_naive();
        let earliest_date = today - chrono::Duration::days(days as i64);

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                if let Some(timestamp_str) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(datetime) = DateTime::parse_from_rfc3339(timestamp_str) {
                        let entry_date = datetime.with_timezone(&Local).date_naive();

                        // Only process if within our date range
                        if entry_date >= earliest_date && entry_date <= today {
                            if let Some(stats) = daily_stats.get_mut(&entry_date) {
                                self.extract_usage_from_entry(&entry, stats)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Calculate model usage breakdown
    pub fn calculate_model_usage(&self) -> Result<Vec<ModelUsage>> {
        let mut model_stats: HashMap<String, UsageStats> = HashMap::new();

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        self.process_jsonl_for_model_usage(&file_path, &mut model_stats)?;
                    }
                }
            }
        }

        Ok(model_stats
            .into_iter()
            .map(|(model_name, usage_stats)| ModelUsage {
                model_name,
                usage_stats,
            })
            .collect())
    }

    /// Process JSONL file for model-specific usage statistics
    fn process_jsonl_for_model_usage(
        &self,
        file_path: &Path,
        model_stats: &mut HashMap<String, UsageStats>,
    ) -> Result<()> {
        let content = fs::read_to_string(file_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                if let Some(message) = entry.get("message") {
                    if let Some(model_name) = message.get("model").and_then(|v| v.as_str()) {
                        let stats = model_stats.entry(model_name.to_string()).or_default();
                        self.extract_usage_from_entry(&entry, stats)?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze daily usage with full detail
    pub fn analyze_daily_usage_detailed(
        &self,
        messages: &[SessionMessage],
    ) -> Result<Vec<DailyUsageDetail>> {
        let mut daily_stats: BTreeMap<String, DailyUsageDetail> = BTreeMap::new();
        let mut sessions_tracked: HashSet<String> = HashSet::new();

        for message in messages {
            if let Some(timestamp) = &message.message.timestamp {
                if let Ok(datetime) = DateTime::parse_from_rfc3339(timestamp) {
                    let local_dt = datetime.with_timezone(&Local);
                    let date_str = local_dt.format("%Y-%m-%d").to_string();

                    let detail =
                        daily_stats
                            .entry(date_str.clone())
                            .or_insert_with(|| DailyUsageDetail {
                                date: date_str.clone(),
                                input_tokens: 0,
                                output_tokens: 0,
                                cache_creation_tokens: 0,
                                cache_read_tokens: 0,
                                total_cost: 0.0,
                                message_count: 0,
                                session_count: 0,
                                models_used: Vec::new(),
                                peak_hour: None,
                                efficiency_score: 0.0,
                            });

                    if let Some(usage) = &message.message.usage {
                        detail.input_tokens += usage.input_tokens.unwrap_or(0);
                        detail.output_tokens += usage.output_tokens.unwrap_or(0);
                        detail.cache_creation_tokens +=
                            usage.cache_creation_input_tokens.unwrap_or(0);
                        detail.cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
                        detail.message_count += 1;

                        if let Some(model) = &message.message.model {
                            if !detail.models_used.contains(model) {
                                detail.models_used.push(model.clone());
                            }
                        }

                        // Track session count
                        let session_key = format!("{}_{}", date_str, message.session_id);
                        if !sessions_tracked.contains(&session_key) {
                            detail.session_count += 1;
                            sessions_tracked.insert(session_key);
                        }
                    }
                }
            }
        }

        // Calculate efficiency scores and costs
        for detail in daily_stats.values_mut() {
            let total_cache = detail.cache_creation_tokens + detail.cache_read_tokens;
            let total_tokens = detail.input_tokens + detail.output_tokens + total_cache;

            if total_tokens > 0 {
                detail.efficiency_score = (total_cache as f64 / total_tokens as f64) * 100.0;
            }

            // Basic cost calculation (would be replaced with actual pricing in integration)
            detail.total_cost = self.calculate_basic_cost_from_tokens(
                detail.input_tokens,
                detail.output_tokens,
                detail.cache_creation_tokens,
                detail.cache_read_tokens,
            );
        }

        Ok(daily_stats.into_values().collect())
    }

    /// Analyze project usage
    pub fn analyze_project_usage(
        &self,
        messages: &[SessionMessage],
    ) -> Result<HashMap<String, ProjectUsageStats>> {
        let mut project_stats: HashMap<String, ProjectUsageStats> = HashMap::new();
        let mut session_projects: HashMap<String, String> = HashMap::new();

        // First pass: map sessions to projects based on cwd
        for message in messages {
            if let Some(cwd) = &message.cwd {
                // Extract project name from path
                let project_name = cwd.split('/').next_back().unwrap_or("Unknown").to_string();

                session_projects.insert(message.session_id.clone(), project_name);
            }
        }

        // Second pass: aggregate usage by project
        for message in messages {
            if let Some(project_name) = session_projects.get(&message.session_id) {
                let stats = project_stats
                    .entry(project_name.clone())
                    .or_insert_with(|| ProjectUsageStats {
                        project_name: project_name.clone(),
                        total_tokens: 0,
                        total_cost: 0.0,
                        session_count: 0,
                        message_count: 0,
                        models_used: HashSet::new(),
                        first_activity: None,
                        last_activity: None,
                        cache_efficiency: 0.0,
                        most_used_model: String::new(),
                        avg_session_length: 0.0,
                    });

                if let Some(usage) = &message.message.usage {
                    stats.total_tokens += usage.input_tokens.unwrap_or(0)
                        + usage.output_tokens.unwrap_or(0)
                        + usage.cache_creation_input_tokens.unwrap_or(0)
                        + usage.cache_read_input_tokens.unwrap_or(0);

                    stats.message_count += 1;

                    if let Some(model) = &message.message.model {
                        stats.models_used.insert(model.clone());
                    }

                    if let Some(timestamp) = &message.message.timestamp {
                        if let Ok(datetime) = DateTime::parse_from_rfc3339(timestamp) {
                            let local_dt = datetime.with_timezone(&Local);

                            if stats.first_activity.is_none()
                                || stats.first_activity.unwrap() > local_dt
                            {
                                stats.first_activity = Some(local_dt);
                            }
                            if stats.last_activity.is_none()
                                || stats.last_activity.unwrap() < local_dt
                            {
                                stats.last_activity = Some(local_dt);
                            }
                        }
                    }
                }
            }
        }

        Ok(project_stats)
    }

    /// Calculate basic cost with fallback pricing
    fn calculate_basic_cost(&self, usage: &PricingTokenUsage, model: &str) -> f64 {
        // Basic Claude 3.5 Sonnet pricing (fallback)
        let input_cost_per_token = if model.contains("claude-3-5-sonnet") {
            0.000003
        } else if model.contains("claude-3-opus") {
            0.000015
        } else if model.contains("claude-3-haiku") {
            0.00000025
        } else {
            0.000003 // default to sonnet
        };

        let output_cost_per_token = input_cost_per_token * 5.0; // typical output cost multiplier
        let cache_creation_cost = input_cost_per_token * 1.25;
        let cache_read_cost = input_cost_per_token * 0.1;

        (usage.input_tokens as f64 * input_cost_per_token)
            + (usage.output_tokens as f64 * output_cost_per_token)
            + (usage.cache_creation_tokens as f64 * cache_creation_cost)
            + (usage.cache_read_tokens as f64 * cache_read_cost)
    }

    /// Calculate basic cost from individual token counts
    fn calculate_basic_cost_from_tokens(
        &self,
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_tokens: u32,
        cache_read_tokens: u32,
    ) -> f64 {
        let usage = PricingTokenUsage {
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
        };
        self.calculate_basic_cost(&usage, "claude-3-5-sonnet") // default model
    }
}

impl UsageStats {
    /// Get total tokens
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens + self.cache_creation_tokens + self.cache_read_tokens
    }

    /// Get cache efficiency percentage
    pub fn cache_efficiency(&self) -> f64 {
        let total_cache = self.cache_creation_tokens + self.cache_read_tokens;
        let total = self.total_tokens();

        if total > 0 {
            (total_cache as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Check if has any usage
    pub fn has_usage(&self) -> bool {
        self.total_tokens() > 0 || self.message_count > 0
    }
}
