use anyhow::Result;
use chrono::Timelike;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// Import types from the new modular system
use crate::claude::usage_calculator::SessionBlock;
use crate::claude::{
    MessageContent, Project, ProjectAnalytics, Session, SessionMessage, TokenUsage, UsageStats,
};

/// Enhanced todo item from ~/.claude/todos/ directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedTodoItem {
    pub content: String,
    pub status: String,
    pub priority: String,
    pub id: String,
    pub session_id: Option<String>,   // Session this todo came from
    pub project_name: Option<String>, // Inferred project name
}

/// Cost warning states from ~/.claude/config/notification_states.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationStates {
    pub switch_to_custom: NotificationState,
    pub exceed_max_limit: NotificationState,
    pub tokens_will_run_out: NotificationState,
    pub cost_will_exceed: NotificationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationState {
    pub triggered: bool,
    pub timestamp: Option<String>,
}

/// Session intelligence metrics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionMetrics {
    #[allow(dead_code)]
    pub session_id: String,
    pub line_count: usize,
    pub project_name: String,
    pub is_most_active: bool,
    pub estimated_duration_hours: f64,
}

// Session-related types moved to session_parser.rs module

/// Comprehensive usage analytics
#[derive(Debug, Clone)]
pub struct UsageAnalytics {
    pub daily_usage: Vec<DailyUsageDetail>,
    pub model_distribution: HashMap<String, ModelUsageStats>,
    pub hourly_patterns: Vec<HourlyUsage>,
    pub cache_efficiency: CacheEfficiencyStats,
    #[allow(dead_code)]
    pub cost_breakdown: CostBreakdown,
    pub project_usage: HashMap<String, ProjectUsageStats>,
    pub session_analytics: Vec<SessionAnalytics>,
}

#[derive(Debug, Clone)]
pub struct DailyUsageDetail {
    pub date: String,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub total_cache_creation_tokens: u32,
    pub total_cache_read_tokens: u32,
    pub total_cost: f64,
    #[allow(dead_code)]
    pub session_count: usize,
    pub message_count: usize,
    pub models_used: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModelUsageStats {
    pub model_name: String,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
    pub total_cost: f64,
    pub usage_count: usize,
    pub first_used: String,
    pub last_used: String,
    pub avg_cost_per_message: f64,
}

#[derive(Debug, Clone)]
pub struct HourlyUsage {
    #[allow(dead_code)]
    pub hour: u8,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub message_count: usize,
}

#[derive(Debug, Clone)]
pub struct CacheEfficiencyStats {
    pub total_cache_creation_tokens: u32,
    pub total_cache_read_tokens: u32,
    pub cache_hit_rate: f64,
    pub cache_cost_savings: f64,
}

#[derive(Debug, Clone)]
pub struct CostBreakdown {
    #[allow(dead_code)]
    pub total_cost: f64,
    #[allow(dead_code)]
    pub input_cost: f64,
    #[allow(dead_code)]
    pub output_cost: f64,
    #[allow(dead_code)]
    pub cache_creation_cost: f64,
    #[allow(dead_code)]
    pub cache_read_cost: f64,
    #[allow(dead_code)]
    pub daily_average: f64,
    #[allow(dead_code)]
    pub projected_monthly: f64,
}

#[derive(Debug, Clone)]
pub struct ProjectUsageStats {
    pub project_name: String,
    pub total_tokens: u32,
    pub total_cost: f64,
    #[allow(dead_code)]
    pub session_count: usize,
    pub most_used_model: String,
    #[allow(dead_code)]
    pub avg_session_length: f64,
}

#[derive(Debug, Clone)]
pub struct SessionAnalytics {
    pub session_id: String,
    #[allow(dead_code)]
    pub project_name: String,
    #[allow(dead_code)]
    pub start_time: String,
    #[allow(dead_code)]
    pub end_time: String,
    pub duration_minutes: f64,
    #[allow(dead_code)]
    pub total_tokens: u32,
    pub total_cost: f64,
    #[allow(dead_code)]
    pub message_count: usize,
    #[allow(dead_code)]
    pub models_used: Vec<String>,
    #[allow(dead_code)]
    pub efficiency_score: f64,
}

/// OpenRouter model pricing data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub pricing: ModelPricing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    #[serde(deserialize_with = "string_to_f64")]
    pub prompt: f64, // Price per token for input
    #[serde(deserialize_with = "string_to_f64")]
    pub completion: f64, // Price per token for output
    #[serde(default, deserialize_with = "optional_string_to_f64")]
    pub cache_creation_input_token_cost: Option<f64>,
    #[serde(default, deserialize_with = "optional_string_to_f64")]
    pub cache_read_input_token_cost: Option<f64>,
}

/// Custom deserializer for string to f64 conversion
fn string_to_f64<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde::Deserialize;

    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::String(s) => s.parse().map_err(D::Error::custom),
        serde_json::Value::Number(num) => num
            .as_f64()
            .ok_or_else(|| D::Error::custom("invalid number")),
        _ => Err(D::Error::custom("expected string or number")),
    }
}

/// Custom deserializer for optional string to f64 conversion
fn optional_string_to_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    use serde::Deserialize;

    match Option::<serde_json::Value>::deserialize(deserializer)? {
        None => Ok(None),
        Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse().map(Some).map_err(D::Error::custom)
            }
        }
        Some(serde_json::Value::Number(num)) => Ok(num.as_f64()),
        Some(_) => Err(D::Error::custom("expected string, number, or null")),
    }
}

/// OpenRouter API response structure
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    data: Vec<OpenRouterModel>,
}

/// Cached pricing data with expiration
#[derive(Debug, Serialize, Deserialize)]
struct PricingCache {
    last_updated: u64, // Unix timestamp
    models: HashMap<String, OpenRouterModel>,
}

impl PricingCache {
    const CACHE_DURATION_HOURS: u64 = 24;

    /// Check if cache is expired (older than 24 hours)
    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        now - self.last_updated > Self::CACHE_DURATION_HOURS * 3600
    }

    /// Load cache from file
    fn load_from_file(cache_path: &Path) -> Result<Option<Self>> {
        if !cache_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(cache_path)?;
        let cache: PricingCache = serde_json::from_str(&content)?;

        if cache.is_expired() {
            Ok(None)
        } else {
            Ok(Some(cache))
        }
    }

    /// Save cache to file
    fn save_to_file(&self, cache_path: &Path) -> Result<()> {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, content)?;
        Ok(())
    }

    /// Fetch fresh data from OpenRouter API
    async fn fetch_from_openrouter() -> Result<Self> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://openrouter.ai/api/v1/models")
            .send()
            .await?;

        let api_response: OpenRouterResponse = response.json().await?;

        let mut models = HashMap::new();
        for model in api_response.data {
            // Only cache Claude models
            if model.id.contains("claude") {
                models.insert(model.id.clone(), model);
            }
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(PricingCache {
            last_updated: now,
            models,
        })
    }
}

/// OpenRouter pricing manager with daily caching
pub struct OpenRouterPricing {
    cache_path: PathBuf,
    cache: Option<PricingCache>,
}

impl Default for OpenRouterPricing {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
        let cache_path = home_dir.join(".claude").join("pricing_cache.json");
        Self {
            cache_path,
            cache: None,
        }
    }
}

impl OpenRouterPricing {
    pub fn new(claude_dir: &Path) -> Self {
        let cache_path = claude_dir.join("pricing_cache.json");
        Self {
            cache_path,
            cache: None,
        }
    }

    /// Get pricing for a model, loading from cache or fetching if needed
    #[allow(dead_code)]
    pub async fn get_model_pricing(&mut self, model_name: &str) -> Result<Option<ModelPricing>> {
        // Load cache if not already loaded
        if self.cache.is_none() {
            self.cache = PricingCache::load_from_file(&self.cache_path)?;
        }

        // If cache is empty or expired, fetch fresh data
        if self.cache.is_none() {
            match PricingCache::fetch_from_openrouter().await {
                Ok(fresh_cache) => {
                    fresh_cache.save_to_file(&self.cache_path)?;
                    self.cache = Some(fresh_cache);
                }
                Err(_) => {
                    // Fallback to hardcoded values if API fails
                    return Ok(None);
                }
            }
        }

        if let Some(cache) = &self.cache {
            // Try exact match first
            if let Some(model) = cache.models.get(model_name) {
                return Ok(Some(model.pricing.clone()));
            }

            // Try pattern matching for Claude models
            for (key, model) in &cache.models {
                if self.matches_claude_model(model_name, key) {
                    return Ok(Some(model.pricing.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Initialize cache by loading from file (synchronous)
    pub fn init_cache(&mut self) -> Result<()> {
        self.cache = PricingCache::load_from_file(&self.cache_path)?;
        Ok(())
    }

    /// Get pricing for a model (synchronous, uses existing cache)
    pub fn get_model_pricing_sync(&self, model_name: &str) -> Option<ModelPricing> {
        if let Some(cache) = &self.cache {
            // Try exact match first
            if let Some(model) = cache.models.get(model_name) {
                return Some(model.pricing.clone());
            }

            // Try pattern matching for Claude models
            for (key, model) in &cache.models {
                if self.matches_claude_model(model_name, key) {
                    return Some(model.pricing.clone());
                }
            }
        }

        None
    }

    /// Calculate cost using OpenRouter pricing or fallback to hardcoded (synchronous)
    pub fn calculate_cost_sync(
        &self,
        model_name: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_tokens: u32,
        cache_read_tokens: u32,
    ) -> f64 {
        if let Some(model) = model_name {
            if let Some(pricing) = self.get_model_pricing_sync(model) {
                // OpenRouter provides price per token, not per million tokens
                let input_cost = (input_tokens as f64) * pricing.prompt;
                let output_cost = (output_tokens as f64) * pricing.completion;

                let cache_creation_cost =
                    if let Some(cache_cost) = pricing.cache_creation_input_token_cost {
                        (cache_creation_tokens as f64) * cache_cost
                    } else {
                        // Default: 25% markup over input cost
                        (cache_creation_tokens as f64) * pricing.prompt * 1.25
                    };

                let cache_read_cost = if let Some(read_cost) = pricing.cache_read_input_token_cost {
                    (cache_read_tokens as f64) * read_cost
                } else {
                    // Default: 90% discount from input cost
                    (cache_read_tokens as f64) * pricing.prompt * 0.1
                };

                return input_cost + output_cost + cache_creation_cost + cache_read_cost;
            }
        }

        // Fallback to hardcoded pricing
        PricingConstants::calculate_cost(
            model_name,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
        )
    }

    /// Calculate cost using OpenRouter pricing or fallback to hardcoded
    #[allow(dead_code)]
    pub async fn calculate_cost(
        &mut self,
        model_name: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_tokens: u32,
        cache_read_tokens: u32,
    ) -> f64 {
        if let Some(model) = model_name {
            if let Ok(Some(pricing)) = self.get_model_pricing(model).await {
                let input_cost = (input_tokens as f64) * pricing.prompt;
                let output_cost = (output_tokens as f64) * pricing.completion;

                let cache_creation_cost =
                    if let Some(cache_cost) = pricing.cache_creation_input_token_cost {
                        (cache_creation_tokens as f64) * cache_cost
                    } else {
                        // Default: 25% markup over input cost
                        (cache_creation_tokens as f64) * pricing.prompt * 1.25
                    };

                let cache_read_cost = if let Some(read_cost) = pricing.cache_read_input_token_cost {
                    (cache_read_tokens as f64) * read_cost
                } else {
                    // Default: 90% discount from input cost
                    (cache_read_tokens as f64) * pricing.prompt * 0.1
                };

                return input_cost + output_cost + cache_creation_cost + cache_read_cost;
            }
        }

        // Fallback to hardcoded pricing
        PricingConstants::calculate_cost(
            model_name,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
        )
    }

    /// Check if a model name matches a cached Claude model
    fn matches_claude_model(&self, requested: &str, cached: &str) -> bool {
        let requested_lower = requested.to_lowercase();
        let cached_lower = cached.to_lowercase();

        // Direct match
        if requested_lower == cached_lower {
            return true;
        }

        // Check for Claude model variants
        if requested_lower.contains("claude") && cached_lower.contains("claude") {
            // Extract model type (sonnet, opus, haiku)
            let requested_type = if requested_lower.contains("sonnet") {
                "sonnet"
            } else if requested_lower.contains("opus") {
                "opus"
            } else if requested_lower.contains("haiku") {
                "haiku"
            } else {
                return false;
            };

            cached_lower.contains(requested_type)
        } else {
            false
        }
    }
}

/// Claude API pricing constants (per individual token)
/// Used as fallback when OpenRouter API is unavailable
/// Pricing based on official API documentation and market analysis
/// and OpenRouter API data as of January 2025
pub struct PricingConstants;

impl PricingConstants {
    // Claude 3.5 Sonnet pricing - per individual token
    // Based on $3.00 input / $15.00 output per 1M tokens (from API documentation)
    pub const SONNET_INPUT_COST: f64 = 0.000003; // $3.00 / 1M tokens
    pub const SONNET_OUTPUT_COST: f64 = 0.000015; // $15.00 / 1M tokens
    pub const SONNET_CACHE_CREATION_COST: f64 = 0.00000375; // 25% markup on input
    pub const SONNET_CACHE_READ_COST: f64 = 0.0000003; // 90% discount on input

    // Claude 3 Opus pricing - per individual token
    // Based on $15.00 input / $75.00 output per 1M tokens (from API documentation)
    pub const OPUS_INPUT_COST: f64 = 0.000015; // $15.00 / 1M tokens
    pub const OPUS_OUTPUT_COST: f64 = 0.000075; // $75.00 / 1M tokens
    pub const OPUS_CACHE_CREATION_COST: f64 = 0.00001875; // 25% markup on input
    pub const OPUS_CACHE_READ_COST: f64 = 0.0000015; // 90% discount on input

    // Claude 3 Haiku pricing - per individual token
    // Based on $0.25 input / $1.25 output per 1M tokens (from API documentation)
    pub const HAIKU_INPUT_COST: f64 = 0.00000025; // $0.25 / 1M tokens
    pub const HAIKU_OUTPUT_COST: f64 = 0.00000125; // $1.25 / 1M tokens
    pub const HAIKU_CACHE_CREATION_COST: f64 = 0.0000003125; // 25% markup on input
    pub const HAIKU_CACHE_READ_COST: f64 = 0.000000025; // 90% discount on input

    /// Calculate cost based on model name and token usage
    /// Returns cost in USD with improved accuracy based on API specifications
    pub fn calculate_cost(
        model_name: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
        cache_creation_tokens: u32,
        cache_read_tokens: u32,
    ) -> f64 {
        let (input_cost, output_cost, cache_creation_cost, cache_read_cost) =
            Self::get_pricing_for_model(model_name);

        let input_cost_usd = (input_tokens as f64) * input_cost;
        let output_cost_usd = (output_tokens as f64) * output_cost;
        let cache_creation_cost_usd = (cache_creation_tokens as f64) * cache_creation_cost;
        let cache_read_cost_usd = (cache_read_tokens as f64) * cache_read_cost;

        let total =
            input_cost_usd + output_cost_usd + cache_creation_cost_usd + cache_read_cost_usd;

        // Round to reasonable precision (6 decimal places) to avoid floating-point artifacts
        // Use 6 decimal places precision to avoid floating-point artifacts
        (total * 1_000_000.0).round() / 1_000_000.0
    }

    /// Get pricing constants for a specific model
    /// Returns (input_cost, output_cost, cache_creation_cost, cache_read_cost) per million tokens
    fn get_pricing_for_model(model_name: Option<&str>) -> (f64, f64, f64, f64) {
        let model = model_name.unwrap_or("").to_lowercase();

        if model.contains("opus") || model.contains("claude-4-opus") {
            (
                Self::OPUS_INPUT_COST,
                Self::OPUS_OUTPUT_COST,
                Self::OPUS_CACHE_CREATION_COST,
                Self::OPUS_CACHE_READ_COST,
            )
        } else if model.contains("haiku") || model.contains("claude-3-haiku") {
            (
                Self::HAIKU_INPUT_COST,
                Self::HAIKU_OUTPUT_COST,
                Self::HAIKU_CACHE_CREATION_COST,
                Self::HAIKU_CACHE_READ_COST,
            )
        } else {
            // Default to Sonnet (most common model)
            (
                Self::SONNET_INPUT_COST,
                Self::SONNET_OUTPUT_COST,
                Self::SONNET_CACHE_CREATION_COST,
                Self::SONNET_CACHE_READ_COST,
            )
        }
    }
}

// Project, Session, and UsageStats types moved to respective modules

// SessionBlock type moved to usage_calculator.rs module

// ProjectAnalytics moved to usage_calculator.rs module

/// Daily usage breakdown
#[derive(Debug, Clone)]
pub struct DailyUsage {
    #[allow(dead_code)]
    pub date: chrono::NaiveDate,
    #[allow(dead_code)]
    pub usage_stats: UsageStats,
}

/// Model usage statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModelUsage {
    pub model_name: String,
    pub usage_stats: UsageStats,
}

/// Claude data manager
pub struct ClaudeDataManager {
    claude_dir: PathBuf,
    openrouter_pricing: OpenRouterPricing,
}

impl ClaudeDataManager {
    /// Create a new Claude data manager
    pub fn new() -> Result<Self> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let claude_dir = home_dir.join(".claude");
        let mut openrouter_pricing = OpenRouterPricing::new(&claude_dir);

        // Initialize cache from file (ignore errors, fallback will be used)
        let _ = openrouter_pricing.init_cache();

        Ok(Self {
            claude_dir,
            openrouter_pricing,
        })
    }

    /// Update OpenRouter pricing cache in background (if needed)
    pub async fn update_pricing_cache_if_needed(&mut self) -> Result<()> {
        // Check if cache needs update
        if let Ok(Some(cache)) = PricingCache::load_from_file(&self.openrouter_pricing.cache_path) {
            if !cache.is_expired() {
                // Cache is still valid
                return Ok(());
            }
        }

        // Cache is expired or missing, fetch new data
        match PricingCache::fetch_from_openrouter().await {
            Ok(fresh_cache) => {
                fresh_cache.save_to_file(&self.openrouter_pricing.cache_path)?;
                self.openrouter_pricing.cache = Some(fresh_cache);
                println!("Updated OpenRouter pricing cache");
            }
            Err(e) => {
                eprintln!("Failed to update OpenRouter pricing cache: {e}");
                // Continue with existing cache or fallback pricing
            }
        }

        Ok(())
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
                .unwrap_or(std::time::UNIX_EPOCH);
            let b_last = b
                .sessions
                .iter()
                .map(|s| s.last_modified)
                .max()
                .unwrap_or(std::time::UNIX_EPOCH);
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
        let (original_path, is_orphaned) =
            match self.get_project_path_from_recent_session(project_dir_path) {
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
                    sanitized_name.replace('-', "/")
                } else {
                    last_part.to_string()
                }
            } else {
                sanitized_name.to_string()
            }
        } else {
            original_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        };

        let mut sessions = Vec::new();
        let mut session_files = Vec::new();

        // Collect all session files first
        let entries = fs::read_dir(project_dir_path)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        session_files.push((path, modified));
                    }
                }
            }
        }

        // Sort by modification time (newest first) and take only the most recent ones
        session_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Only process the 3 most recent sessions for better performance
        for (path, _) in session_files.into_iter().take(3) {
            if let Some(session) = self.parse_session(&path)? {
                sessions.push(session);
            }
        }

        // Show projects even if they have no sessions (as inactive)
        if sessions.is_empty() && !is_orphaned {
            // For real projects with no sessions, still show them
        }

        let is_active = if is_orphaned {
            false // Orphaned projects are always inactive
        } else {
            sessions.iter().any(|s| {
                s.last_modified
                    .elapsed()
                    .map(|d| d.as_secs() < 3600) // Active if modified within last hour
                    .unwrap_or(false)
            })
        };

        Ok(Some(Project {
            name: project_name,
            path: original_path,
            sessions,
            is_active,
        }))
    }

    /// Parse a session JSONL file
    fn parse_session(&self, session_path: &Path) -> Result<Option<Session>> {
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

    /// Get the real project path from the most recent JSONL session file's "cwd" field
    fn get_project_path_from_recent_session(&self, project_dir_path: &Path) -> Result<String> {
        // Find the most recent JSONL file in the project directory
        let entries = std::fs::read_dir(project_dir_path)?;
        let mut jsonl_files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();

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

        if jsonl_files.is_empty() {
            return Err(anyhow::anyhow!("No JSONL files found in project directory"));
        }

        // Sort by modification time (newest first)
        jsonl_files.sort_by(|a, b| b.1.cmp(&a.1));

        // Read the most recent JSONL file and find the last line with "cwd"
        let most_recent_file = &jsonl_files[0].0;
        let content = std::fs::read_to_string(most_recent_file)?;

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

    /// Reconstruct the original filesystem path from Claude's sanitized directory name.
    /// e.g., "-Users-user-dev-project" -> "/Users/user/dev/project"
    fn reconstruct_path_from_sanitized_name(&self, sanitized_name: &str) -> Result<PathBuf> {
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
        let reconstructed_path = PathBuf::from(format!("/{path_str}"));

        // Debug: Check if path exists with proper error handling
        if reconstructed_path.exists() {
            Ok(reconstructed_path)
        } else {
            // Try alternative reconstruction - sometimes Claude uses different encodings
            // Handle special case where the path might have different casing or symlinks
            let canonical_path = if let Ok(canonical) = reconstructed_path.canonicalize() {
                canonical
            } else if reconstructed_path
                .parent()
                .map(|p| p.exists())
                .unwrap_or(false)
            {
                // Parent exists, but this specific directory doesn't - could be deleted
                reconstructed_path
            } else {
                // Try resolving symlinks in the path
                let resolved = std::fs::read_link(&reconstructed_path).or_else(|_| {
                    // If not a symlink, check if it's a case sensitivity issue on macOS
                    if cfg!(target_os = "macos") {
                        // On macOS, try lowercase version
                        let lowercase_path = PathBuf::from(format!("/{}", path_str.to_lowercase()));
                        if lowercase_path.exists() {
                            Ok(lowercase_path)
                        } else {
                            Err(anyhow::anyhow!("Path does not exist"))
                        }
                    } else {
                        Err(anyhow::anyhow!("Path does not exist"))
                    }
                })?;
                resolved
            };

            Ok(canonical_path)
        }
    }

    /// Calculate today's usage statistics
    pub fn calculate_today_usage(&self) -> Result<UsageStats> {
        let mut stats = UsageStats::default();
        let today = chrono::Local::now().date_naive();

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(stats);
        }

        // Scan all JSONL files for today's data
        for entry in std::fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in std::fs::read_dir(&project_path)? {
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
        target_date: chrono::NaiveDate,
    ) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                // Check if this entry is from today
                if let Some(timestamp_str) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                        let entry_date = datetime.with_timezone(&chrono::Local).date_naive();

                        if entry_date == target_date {
                            self.extract_usage_from_entry(&entry, stats)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Process a JSONL file for usage statistics for multiple dates in single pass (optimized)
    #[allow(dead_code)]
    fn process_jsonl_for_all_dates(
        &self,
        file_path: &Path,
        daily_stats: &mut std::collections::HashMap<chrono::NaiveDate, UsageStats>,
        target_dates: &[chrono::NaiveDate],
    ) -> Result<()> {
        // Early return if no target dates
        if target_dates.is_empty() {
            return Ok(());
        }

        // Convert target dates to HashSet for O(1) lookup
        let target_set: std::collections::HashSet<chrono::NaiveDate> =
            target_dates.iter().cloned().collect();

        let content = std::fs::read_to_string(file_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                // Check if this entry is from any of our target dates
                if let Some(timestamp_str) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    if let Ok(datetime) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
                        let entry_date = datetime.with_timezone(&chrono::Local).date_naive();

                        // Only process if this date is in our target set
                        if target_set.contains(&entry_date) {
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

    /// Extract usage data from a single JSONL entry
    fn extract_usage_from_entry(&self, entry: &Value, stats: &mut UsageStats) -> Result<()> {
        let mut current_input_tokens = 0u32;
        let mut current_output_tokens = 0u32;
        let mut current_cache_creation_tokens = 0u32;
        let mut current_cache_read_tokens = 0u32;

        // Extract usage data from message field
        if let Some(message) = entry.get("message") {
            if let Some(usage) = message.get("usage") {
                if let Some(input_tokens) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                    current_input_tokens = input_tokens as u32;
                    stats.input_tokens += current_input_tokens;
                }
                if let Some(output_tokens) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                    current_output_tokens = output_tokens as u32;
                    stats.output_tokens += current_output_tokens;
                }
                if let Some(cache_creation) = usage
                    .get("cache_creation_input_tokens")
                    .and_then(|v| v.as_u64())
                {
                    current_cache_creation_tokens = cache_creation as u32;
                    stats.cache_creation_tokens += current_cache_creation_tokens;
                }
                if let Some(cache_read) = usage
                    .get("cache_read_input_tokens")
                    .and_then(|v| v.as_u64())
                {
                    current_cache_read_tokens = cache_read as u32;
                    stats.cache_read_tokens += current_cache_read_tokens;
                }

                stats.message_count += 1;
            }
        }

        // Extract cost data if available, otherwise calculate from tokens
        if let Some(cost_usd) = entry.get("costUSD").and_then(|v| v.as_f64()) {
            if cost_usd > 0.0 {
                // Use actual cost from API users
                stats.total_cost += cost_usd;
                stats.is_subscription_user = false;
            } else {
                // Fallback calculation for subscription users
                let model_name = entry
                    .get("message")
                    .and_then(|m| m.get("model"))
                    .and_then(|v| v.as_str());

                let calculated_cost = self.openrouter_pricing.calculate_cost_sync(
                    model_name,
                    current_input_tokens,
                    current_output_tokens,
                    current_cache_creation_tokens,
                    current_cache_read_tokens,
                );
                stats.total_cost += calculated_cost;
                stats.is_subscription_user = true;
            }
        } else {
            // No costUSD field - calculate from tokens (subscription users)
            let model_name = entry
                .get("message")
                .and_then(|m| m.get("model"))
                .and_then(|v| v.as_str());

            let calculated_cost = self.openrouter_pricing.calculate_cost_sync(
                model_name,
                current_input_tokens,
                current_output_tokens,
                current_cache_creation_tokens,
                current_cache_read_tokens,
            );
            stats.total_cost += calculated_cost;
            stats.is_subscription_user = true;
        }

        Ok(())
    }

    /// Round timestamp to the nearest hour for block calculations
    pub fn round_to_hour(&self, time: std::time::SystemTime) -> std::time::SystemTime {
        let duration_since_epoch = time
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let total_seconds = duration_since_epoch.as_secs();
        let minute_seconds = total_seconds % 3600;
        let rounded_seconds = total_seconds - minute_seconds;
        std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(rounded_seconds)
    }

    /// Calculate next 5-hour block reset time using block-based algorithm
    pub fn calculate_next_reset_time(&self) -> std::time::SystemTime {
        let now = std::time::SystemTime::now();

        // Find all possible active blocks from recent entries
        if let Some(active_block_end) = self.find_active_block_end_time() {
            return active_block_end;
        }

        // Fallback: create a block starting from current hour
        let now_rounded = self.round_to_hour(now);
        now_rounded + std::time::Duration::from_secs(5 * 3600)
    }

    /// Find the end time of the currently active 5-hour block
    /// Implementation approach: create time blocks from session entries, then find active ones
    pub fn find_active_block_end_time(&self) -> Option<std::time::SystemTime> {
        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return None;
        }

        let now = std::time::SystemTime::now();
        let mut all_entries = Vec::new();

        // Collect all entries with timestamps
        for entry in std::fs::read_dir(&projects_dir).ok()? {
            let entry = entry.ok()?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in std::fs::read_dir(&project_path).ok()? {
                    let file_entry = file_entry.ok()?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            for line in content.lines() {
                                if let Ok(json_value) =
                                    serde_json::from_str::<serde_json::Value>(line)
                                {
                                    if let Some(timestamp_str) =
                                        json_value.get("timestamp").and_then(|v| v.as_str())
                                    {
                                        if let Ok(dt) =
                                            chrono::DateTime::parse_from_rfc3339(timestamp_str)
                                        {
                                            let entry_time = std::time::SystemTime::UNIX_EPOCH
                                                + std::time::Duration::from_secs(
                                                    dt.timestamp() as u64
                                                );
                                            all_entries.push(entry_time);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if all_entries.is_empty() {
            return None;
        }

        // Sort entries by time for chronological processing
        all_entries.sort();

        // Transform entries into 5-hour time blocks
        let mut blocks: Vec<(std::time::SystemTime, std::time::SystemTime)> = Vec::new(); // (start, end)
        let mut current_block_start: Option<std::time::SystemTime> = None;
        let mut current_block_end: Option<std::time::SystemTime> = None;

        for entry_time in all_entries.iter() {
            let entry_block_start = self.round_to_hour(*entry_time);
            let entry_block_end = entry_block_start + std::time::Duration::from_secs(5 * 3600);

            // Check if we need a new block based on time boundaries
            let need_new_block = match (current_block_start, current_block_end) {
                (None, None) => true, // First entry
                (Some(_start), Some(end)) => {
                    // New block needed if entry would be outside current block's end time
                    *entry_time >= end
                }
                _ => true, // Safety fallback
            };

            if need_new_block {
                // Save current block if it exists
                if let (Some(start), Some(end)) = (current_block_start, current_block_end) {
                    blocks.push((start, end));
                }
                // Start new block
                current_block_start = Some(entry_block_start);
                current_block_end = Some(entry_block_end);
            }
        }

        // Add the last block
        if let (Some(start), Some(end)) = (current_block_start, current_block_end) {
            blocks.push((start, end));
        }

        // Find active block by checking current time against block boundaries
        // A block is active if block.end_time > current_time
        for (_block_start, block_end) in blocks.iter() {
            // Active condition: block end time is in the future
            if *block_end > now {
                return Some(*block_end);
            }
        }

        None
    }

    /// Calculate time until next reset in a human-readable format
    pub fn time_until_reset(&self) -> String {
        let next_reset = self.calculate_next_reset_time();
        let now = std::time::SystemTime::now();

        if let Ok(duration) = next_reset.duration_since(now) {
            let hours = duration.as_secs() / 3600;
            let minutes = (duration.as_secs() % 3600) / 60;

            if hours > 0 {
                format!("{hours}h {minutes}m")
            } else if minutes > 0 {
                format!("{minutes}m")
            } else {
                "Soon".to_string()
            }
        } else {
            "Soon".to_string()
        }
    }

    /// Calculate comprehensive analytics for a specific project
    pub fn calculate_project_analytics(&self, project: &Project) -> Result<ProjectAnalytics> {
        let mut total_messages = 0;
        let mut total_tokens = 0;
        let mut estimated_cost = 0.0;
        let mut first_session: Option<std::time::SystemTime> = None;
        let mut last_session: Option<std::time::SystemTime> = None;
        let mut cache_read_tokens = 0;
        let mut cache_creation_tokens = 0;
        let mut total_input_tokens = 0;

        for session in &project.sessions {
            total_messages += session.message_count;

            // Parse session file for detailed statistics
            if let Ok(session_stats) = self.calculate_session_usage(&session.path) {
                total_tokens += session_stats.input_tokens + session_stats.output_tokens;
                estimated_cost += session_stats.total_cost;
                cache_read_tokens += session_stats.cache_read_tokens;
                cache_creation_tokens += session_stats.cache_creation_tokens;
                total_input_tokens += session_stats.input_tokens;
            }

            // Track first and last session times
            if first_session.is_none() || session.last_modified < first_session.unwrap() {
                first_session = Some(session.last_modified);
            }
            if last_session.is_none() || session.last_modified > last_session.unwrap() {
                last_session = Some(session.last_modified);
            }
        }

        // Calculate cache efficiency (cache read tokens / total tokens including cache creation)
        // Cache efficiency = how much we're reusing vs creating new cache
        let total_cacheable_tokens = total_input_tokens + cache_creation_tokens;
        let cache_efficiency = if total_cacheable_tokens > 0 {
            (cache_read_tokens as f64 / total_cacheable_tokens as f64) * 100.0
        } else {
            0.0
        };

        // Generate session blocks (placeholder for now)
        let session_blocks = self.calculate_session_blocks(project)?;

        Ok(ProjectAnalytics {
            total_sessions: project.sessions.len(),
            total_messages,
            total_tokens,
            estimated_cost,
            first_session,
            last_session,
            cache_efficiency,
            session_blocks,
        })
    }

    /// Calculate usage statistics for a single session
    fn calculate_session_usage(&self, session_path: &Path) -> Result<UsageStats> {
        let mut stats = UsageStats::default();
        let content = std::fs::read_to_string(session_path)?;

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

    /// Calculate session blocks for a project (5-hour periods)
    fn calculate_session_blocks(&self, project: &Project) -> Result<Vec<SessionBlock>> {
        let mut blocks = Vec::new();

        if project.sessions.is_empty() {
            return Ok(blocks);
        }

        // Group sessions into 5-hour blocks
        let mut current_block_start: Option<std::time::SystemTime> = None;
        let mut current_block_stats = UsageStats::default();

        for session in &project.sessions {
            // If this is the first session or more than 5 hours from current block
            if current_block_start.is_none()
                || session
                    .last_modified
                    .duration_since(current_block_start.unwrap())
                    .unwrap_or_default()
                    .as_secs()
                    > 5 * 3600
            {
                // Finalize previous block if exists
                if let Some(start) = current_block_start {
                    let end = start + std::time::Duration::from_secs(5 * 3600);
                    blocks.push(SessionBlock {
                        start_time: start,
                        end_time: end,
                        usage_stats: current_block_stats.clone(),
                        is_active: false,
                    });
                }

                // Start new block
                current_block_start = Some(session.last_modified);
                current_block_stats = UsageStats::default();
            }

            // Add session stats to current block
            if let Ok(session_stats) = self.calculate_session_usage(&session.path) {
                current_block_stats.input_tokens += session_stats.input_tokens;
                current_block_stats.output_tokens += session_stats.output_tokens;
                current_block_stats.cache_creation_tokens += session_stats.cache_creation_tokens;
                current_block_stats.cache_read_tokens += session_stats.cache_read_tokens;
                current_block_stats.total_cost += session_stats.total_cost;
                current_block_stats.message_count += session_stats.message_count;
            }
        }

        // Finalize last block
        if let Some(start) = current_block_start {
            let end = start + std::time::Duration::from_secs(5 * 3600);
            let is_active = std::time::SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs()
                < 5 * 3600;

            blocks.push(SessionBlock {
                start_time: start,
                end_time: end,
                usage_stats: current_block_stats,
                is_active,
            });
        }

        Ok(blocks)
    }

    /// Calculate daily usage breakdown for the last N days (optimized single-pass)
    #[allow(dead_code)]
    pub fn calculate_daily_usage(&self, days: u32) -> Result<Vec<DailyUsage>> {
        let today = chrono::Local::now().date_naive();
        let mut daily_stats: std::collections::HashMap<chrono::NaiveDate, UsageStats> =
            std::collections::HashMap::new();

        // Initialize all target dates
        let target_dates: Vec<chrono::NaiveDate> = (0..days)
            .map(|i| today - chrono::Duration::days(i as i64))
            .collect();

        for date in &target_dates {
            daily_stats.insert(*date, UsageStats::default());
        }

        let projects_dir = self.claude_dir.join("projects");
        if projects_dir.exists() {
            for entry in std::fs::read_dir(&projects_dir)? {
                let entry = entry?;
                let project_path = entry.path();

                if project_path.is_dir() {
                    for file_entry in std::fs::read_dir(&project_path)? {
                        let file_entry = file_entry?;
                        let file_path = file_entry.path();

                        if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                            // Skip files that are too old to contain relevant data
                            if let Ok(metadata) = std::fs::metadata(&file_path) {
                                if let Ok(modified) = metadata.modified() {
                                    let file_age = modified.elapsed().unwrap_or_default();
                                    // Skip files older than 8 days (1 day buffer)
                                    if file_age.as_secs() > (days + 1) as u64 * 24 * 3600 {
                                        continue;
                                    }
                                }
                            }

                            // Single-pass processing: read file once and extract data for all dates
                            self.process_jsonl_for_all_dates(
                                &file_path,
                                &mut daily_stats,
                                &target_dates,
                            )?;
                        }
                    }
                }
            }
        }

        // Convert to Vec and sort by date (most recent first)
        let mut daily_usage: Vec<DailyUsage> = target_dates
            .into_iter()
            .map(|date| DailyUsage {
                date,
                usage_stats: daily_stats.remove(&date).unwrap_or_default(),
            })
            .collect();

        daily_usage.sort_by(|a, b| b.date.cmp(&a.date));
        Ok(daily_usage)
    }

    /// Calculate model usage breakdown
    #[allow(dead_code)]
    pub fn calculate_model_usage(&self) -> Result<Vec<ModelUsage>> {
        let mut model_stats: std::collections::HashMap<String, UsageStats> =
            std::collections::HashMap::new();

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        for entry in std::fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in std::fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        self.process_jsonl_for_model_usage(&file_path, &mut model_stats)?;
                    }
                }
            }
        }

        let mut model_usage: Vec<ModelUsage> = model_stats
            .into_iter()
            .map(|(model_name, usage_stats)| ModelUsage {
                model_name,
                usage_stats,
            })
            .collect();

        // Sort by total tokens (descending)
        model_usage.sort_by(|a, b| {
            let a_total = a.usage_stats.input_tokens + a.usage_stats.output_tokens;
            let b_total = b.usage_stats.input_tokens + b.usage_stats.output_tokens;
            b_total.cmp(&a_total)
        });

        Ok(model_usage)
    }

    /// Process JSONL file for model-specific usage statistics
    #[allow(dead_code)]
    fn process_jsonl_for_model_usage(
        &self,
        file_path: &Path,
        model_stats: &mut std::collections::HashMap<String, UsageStats>,
    ) -> Result<()> {
        let content = std::fs::read_to_string(file_path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<Value>(line) {
                // Extract model name
                let model_name = entry
                    .get("message")
                    .and_then(|m| m.get("model"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                // Get or create stats for this model
                let stats = model_stats.entry(model_name).or_default();

                // Extract usage data
                self.extract_usage_from_entry(&entry, stats)?;
            }
        }

        Ok(())
    }

    /// Calculate burn rate (tokens per minute) for active sessions
    #[allow(dead_code)]
    pub fn calculate_burn_rate(&self, project: &Project) -> f64 {
        // Find the most recent active session
        let recent_session = project
            .sessions
            .iter()
            .filter(|s| {
                s.last_modified
                    .elapsed()
                    .map(|d| d.as_secs() < 3600) // Active within last hour
                    .unwrap_or(false)
            })
            .max_by_key(|s| s.last_modified);

        if let Some(session) = recent_session {
            if let Ok(session_stats) = self.calculate_session_usage(&session.path) {
                let total_tokens = session_stats.input_tokens + session_stats.output_tokens;
                let session_duration = session
                    .last_modified
                    .elapsed()
                    .unwrap_or_default()
                    .as_secs() as f64
                    / 60.0; // Convert to minutes

                if session_duration > 0.0 {
                    return total_tokens as f64 / session_duration;
                }
            }
        }

        0.0
    }

    // 🎯 INNOVATIVE FEATURES BASED ON ~/.claude DEEP ANALYSIS

    /// Load all todos from ~/.claude/todos/ directory - Cross-session todo intelligence
    #[allow(dead_code)]
    pub fn load_enhanced_todos(&self) -> Result<Vec<EnhancedTodoItem>> {
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
                let file_name = file_path
                    .file_name()
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
                        todo.project_name =
                            self.infer_project_from_session(session_id.as_deref().unwrap_or(""));
                        enhanced_todos.push(todo);
                    }
                }
            }
        }

        Ok(enhanced_todos)
    }

    /// Load cost notification states - Smart cost warning system
    #[allow(dead_code)]
    pub fn load_notification_states(&self) -> Result<NotificationStates> {
        let config_path = self
            .claude_dir
            .join("config")
            .join("notification_states.json");

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

    /// Analyze session intelligence - Most active sessions, productivity patterns
    #[allow(dead_code)]
    pub fn analyze_session_intelligence(&self) -> Result<Vec<SessionMetrics>> {
        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            return Ok(Vec::new());
        }

        let mut session_metrics = Vec::new();
        let mut max_lines = 0usize;

        // First pass: find sessions and count lines
        for entry in fs::read_dir(&projects_dir)? {
            let entry = entry?;
            let project_path = entry.path();

            if project_path.is_dir() {
                let sanitized_name = project_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let project_name = self
                    .reconstruct_path_from_sanitized_name(sanitized_name)
                    .unwrap_or_else(|_| PathBuf::from(sanitized_name))
                    .to_string_lossy()
                    .to_string();

                for file_entry in fs::read_dir(&project_path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        let session_id = file_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();

                        // Count lines in session file
                        let line_count = fs::read_to_string(&file_path)
                            .map(|content| content.lines().count())
                            .unwrap_or(0);

                        max_lines = max_lines.max(line_count);

                        session_metrics.push(SessionMetrics {
                            session_id,
                            line_count,
                            project_name: project_name.clone(),
                            is_most_active: false, // Will be set in second pass
                            estimated_duration_hours: (line_count as f64 / 100.0), // Rough estimate
                        });
                    }
                }
            }
        }

        // Second pass: mark most active sessions (top 10% by line count)
        let threshold = (max_lines as f64 * 0.9) as usize;
        for metric in &mut session_metrics {
            metric.is_most_active = metric.line_count >= threshold;
        }

        // Sort by activity level (most active first)
        session_metrics.sort_by(|a, b| b.line_count.cmp(&a.line_count));

        Ok(session_metrics)
    }

    /// Get comprehensive todo statistics - Project-wide todo insights
    #[allow(dead_code)]
    pub fn get_todo_statistics(&self) -> Result<TodoStatistics> {
        let enhanced_todos = self.load_enhanced_todos()?;

        let mut stats = TodoStatistics {
            total_todos: enhanced_todos.len(),
            completed_todos: 0,
            pending_todos: 0,
            in_progress_todos: 0,
            high_priority_todos: 0,
            projects_with_todos: std::collections::HashSet::new(),
        };

        for todo in &enhanced_todos {
            match todo.status.to_lowercase().as_str() {
                "completed" => stats.completed_todos += 1,
                "pending" => stats.pending_todos += 1,
                "in_progress" | "in progress" => stats.in_progress_todos += 1,
                _ => {}
            }

            if todo.priority.to_lowercase() == "high" {
                stats.high_priority_todos += 1;
            }

            if let Some(project) = &todo.project_name {
                stats.projects_with_todos.insert(project.clone());
            }
        }

        Ok(stats)
    }

    /// Infer project name from session ID (helper method)
    #[allow(dead_code)]
    fn infer_project_from_session(&self, session_id: &str) -> Option<String> {
        let projects_dir = self.claude_dir.join("projects");

        for entry in fs::read_dir(&projects_dir).ok()? {
            let entry = entry.ok()?;
            let project_path = entry.path();

            if project_path.is_dir() {
                for file_entry in fs::read_dir(&project_path).ok()? {
                    let file_entry = file_entry.ok()?;
                    let file_path = file_entry.path();

                    if let Some(file_stem) = file_path.file_stem().and_then(|s| s.to_str()) {
                        if file_stem == session_id {
                            let sanitized_name = project_path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("");
                            return Some(
                                self.reconstruct_path_from_sanitized_name(sanitized_name)
                                    .unwrap_or_else(|_| PathBuf::from(sanitized_name))
                                    .to_string_lossy()
                                    .to_string(),
                            );
                        }
                    }
                }
            }
        }

        None
    }

    // 🚀 COMPREHENSIVE USAGE ANALYTICS - 최고 수준의 사용량 분석

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
                                    if let Some(timestamp) = json_value.get("timestamp") {
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
                                                            usage: serde_json::from_value(
                                                                usage.clone(),
                                                            )
                                                            .ok(),
                                                            model: message
                                                                .get("model")
                                                                .and_then(|v| v.as_str())
                                                                .map(|s| s.to_string()),
                                                            timestamp: timestamp
                                                                .as_str()
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

        // Sort by timestamp for chronological analysis
        all_messages.sort_by(|a, b| {
            let ts_a = a.message.timestamp.as_deref().unwrap_or("");
            let ts_b = b.message.timestamp.as_deref().unwrap_or("");
            ts_a.cmp(ts_b)
        });

        Ok(all_messages)
    }

    /// Generate comprehensive usage analytics - 매력적인 모든 통계
    pub fn generate_comprehensive_analytics(&self) -> Result<UsageAnalytics> {
        let messages = self.parse_all_session_messages()?;
        let pricing = &self.openrouter_pricing;

        // Daily usage analysis
        let daily_usage = self.analyze_daily_usage_detailed(&messages, pricing)?;

        // Model distribution analysis
        let model_distribution = self.analyze_model_distribution(&messages, pricing)?;

        // Hourly patterns
        let hourly_patterns = self.analyze_hourly_patterns(&messages)?;

        // Cache efficiency
        let cache_efficiency = self.analyze_cache_efficiency(&messages)?;

        // Cost breakdown
        let cost_breakdown = self.analyze_cost_breakdown(&messages, pricing)?;

        // Project usage
        let project_usage = self.analyze_project_usage(&messages, pricing)?;

        // Session analytics
        let session_analytics = self.analyze_session_details(&messages, pricing)?;

        Ok(UsageAnalytics {
            daily_usage,
            model_distribution,
            hourly_patterns,
            cache_efficiency,
            cost_breakdown,
            project_usage,
            session_analytics,
        })
    }

    /// Analyze daily usage with full detail - 일별 상세 사용량
    fn analyze_daily_usage_detailed(
        &self,
        messages: &[SessionMessage],
        pricing: &OpenRouterPricing,
    ) -> Result<Vec<DailyUsageDetail>> {
        use std::collections::BTreeMap;
        let mut daily_stats: BTreeMap<String, DailyUsageDetail> = BTreeMap::new();
        let mut sessions_tracked: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for message in messages {
            if let Some(timestamp) = &message.message.timestamp {
                // Parse timestamp more robustly
                let date = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                    dt.with_timezone(&chrono::Local).date_naive().to_string()
                } else if timestamp.len() >= 10 {
                    // Fallback: extract date part if ISO format fails
                    timestamp[0..10].to_string()
                } else {
                    continue; // Skip invalid timestamps
                };

                let entry = daily_stats.entry(date.clone()).or_insert(DailyUsageDetail {
                    date: date.clone(),
                    total_input_tokens: 0,
                    total_output_tokens: 0,
                    total_cache_creation_tokens: 0,
                    total_cache_read_tokens: 0,
                    total_cost: 0.0,
                    session_count: 0,
                    message_count: 0,
                    models_used: Vec::new(),
                });

                entry.message_count += 1;

                // Track unique sessions
                let session_key = format!("{}-{}", date, message.session_id);
                if !sessions_tracked.contains(&session_key) {
                    sessions_tracked.insert(session_key);
                    entry.session_count += 1;
                }

                if let Some(usage) = &message.message.usage {
                    entry.total_input_tokens += usage.input_tokens.unwrap_or(0);
                    entry.total_output_tokens += usage.output_tokens.unwrap_or(0);
                    entry.total_cache_creation_tokens +=
                        usage.cache_creation_input_tokens.unwrap_or(0);
                    entry.total_cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);

                    // Calculate cost for this message
                    if let Some(model) = &message.message.model {
                        if !entry.models_used.contains(model) {
                            entry.models_used.push(model.clone());
                        }

                        let cost = self.calculate_message_cost(usage, model, pricing);
                        entry.total_cost += cost;
                    }
                }
            }
        }

        Ok(daily_stats.into_values().collect())
    }

    /// Analyze model distribution - 모델별 사용 패턴
    fn analyze_model_distribution(
        &self,
        messages: &[SessionMessage],
        pricing: &OpenRouterPricing,
    ) -> Result<HashMap<String, ModelUsageStats>> {
        let mut model_stats: HashMap<String, ModelUsageStats> = HashMap::new();

        for message in messages {
            if let (Some(model), Some(usage)) = (&message.message.model, &message.message.usage) {
                let entry = model_stats.entry(model.clone()).or_insert(ModelUsageStats {
                    model_name: model.clone(),
                    total_input_tokens: 0,
                    total_output_tokens: 0,
                    total_cost: 0.0,
                    usage_count: 0,
                    first_used: message
                        .message
                        .timestamp
                        .as_deref()
                        .unwrap_or("")
                        .to_string(),
                    last_used: message
                        .message
                        .timestamp
                        .as_deref()
                        .unwrap_or("")
                        .to_string(),
                    avg_cost_per_message: 0.0,
                });

                entry.total_input_tokens += usage.input_tokens.unwrap_or(0);
                entry.total_output_tokens += usage.output_tokens.unwrap_or(0);
                entry.usage_count += 1;

                let cost = self.calculate_message_cost(usage, model, pricing);
                entry.total_cost += cost;

                if let Some(timestamp) = &message.message.timestamp {
                    if timestamp > &entry.last_used {
                        entry.last_used = timestamp.clone();
                    }
                    if entry.first_used.is_empty() || timestamp < &entry.first_used {
                        entry.first_used = timestamp.clone();
                    }
                }
            }
        }

        // Calculate average cost per message
        for stats in model_stats.values_mut() {
            if stats.usage_count > 0 {
                stats.avg_cost_per_message = stats.total_cost / stats.usage_count as f64;
            }
        }

        Ok(model_stats)
    }

    /// Analyze hourly usage patterns - 시간대별 사용 패턴
    fn analyze_hourly_patterns(&self, messages: &[SessionMessage]) -> Result<Vec<HourlyUsage>> {
        let mut hourly_stats: [HourlyUsage; 24] = std::array::from_fn(|hour| HourlyUsage {
            hour: hour as u8,
            total_tokens: 0,
            total_cost: 0.0,
            message_count: 0,
        });

        for message in messages {
            if let Some(timestamp) = &message.message.timestamp {
                // Parse hour from timestamp, converting to local timezone
                let hour = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(timestamp) {
                    dt.with_timezone(&chrono::Local).hour() as usize
                } else if let Some(time_part) = timestamp.split('T').nth(1) {
                    // Fallback: parse hour directly from timestamp string
                    if let Some(hour_str) = time_part.split(':').next() {
                        hour_str.parse::<usize>().unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    0
                };

                if hour < 24 {
                    hourly_stats[hour].message_count += 1;

                    if let Some(usage) = &message.message.usage {
                        let total_tokens =
                            usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0);
                        hourly_stats[hour].total_tokens += total_tokens;

                        if let Some(model) = &message.message.model {
                            let pricing = &self.openrouter_pricing;
                            let cost = self.calculate_message_cost(usage, model, pricing);
                            hourly_stats[hour].total_cost += cost;
                        }
                    }
                }
            }
        }

        Ok(hourly_stats.to_vec())
    }

    /// Analyze cache efficiency - 캐시 효율성 분석
    fn analyze_cache_efficiency(
        &self,
        messages: &[SessionMessage],
    ) -> Result<CacheEfficiencyStats> {
        let mut total_cache_creation = 0u32;
        let mut total_cache_read = 0u32;
        let mut total_cache_cost_saved = 0.0;

        for message in messages {
            if let Some(usage) = &message.message.usage {
                total_cache_creation += usage.cache_creation_input_tokens.unwrap_or(0);
                total_cache_read += usage.cache_read_input_tokens.unwrap_or(0);

                // Calculate actual cache savings using model-specific pricing
                if let Some(model) = &message.message.model {
                    let input_cost = self
                        .openrouter_pricing
                        .get_model_pricing_sync(model)
                        .map(|p| p.prompt)
                        .unwrap_or(PricingConstants::SONNET_INPUT_COST);

                    let cache_read_cost = self
                        .openrouter_pricing
                        .get_model_pricing_sync(model)
                        .and_then(|p| p.cache_read_input_token_cost)
                        .unwrap_or(PricingConstants::SONNET_CACHE_READ_COST);

                    // Savings = (regular_cost - cache_cost) * cache_read_tokens
                    let savings_per_token = input_cost - cache_read_cost;
                    total_cache_cost_saved +=
                        (usage.cache_read_input_tokens.unwrap_or(0) as f64) * savings_per_token;
                }
            }
        }

        let cache_hit_rate = if total_cache_creation + total_cache_read > 0 {
            (total_cache_read as f64) / ((total_cache_creation + total_cache_read) as f64) * 100.0
        } else {
            0.0
        };

        Ok(CacheEfficiencyStats {
            total_cache_creation_tokens: total_cache_creation,
            total_cache_read_tokens: total_cache_read,
            cache_hit_rate,
            cache_cost_savings: total_cache_cost_saved,
        })
    }

    /// Calculate message cost - 메시지별 정확한 비용 계산
    fn calculate_message_cost(
        &self,
        usage: &TokenUsage,
        model: &str,
        pricing: &OpenRouterPricing,
    ) -> f64 {
        // Use the existing pricing calculation logic
        pricing.calculate_cost_sync(
            Some(model),
            usage.input_tokens.unwrap_or(0),
            usage.output_tokens.unwrap_or(0),
            usage.cache_creation_input_tokens.unwrap_or(0),
            usage.cache_read_input_tokens.unwrap_or(0),
        )
    }

    /// Analyze cost breakdown - 비용 세부 분석
    fn analyze_cost_breakdown(
        &self,
        messages: &[SessionMessage],
        pricing: &OpenRouterPricing,
    ) -> Result<CostBreakdown> {
        let mut total_cost = 0.0;
        let mut input_cost = 0.0;
        let mut output_cost = 0.0;
        let mut cache_creation_cost = 0.0;
        let mut cache_read_cost = 0.0;

        for message in messages {
            if let (Some(usage), Some(model)) = (&message.message.usage, &message.message.model) {
                // Calculate individual costs using the OpenRouter pricing
                let msg_input_cost = (usage.input_tokens.unwrap_or(0) as f64)
                    * pricing
                        .get_model_pricing_sync(model)
                        .map(|p| p.prompt)
                        .unwrap_or(PricingConstants::SONNET_INPUT_COST);

                let msg_output_cost = (usage.output_tokens.unwrap_or(0) as f64)
                    * pricing
                        .get_model_pricing_sync(model)
                        .map(|p| p.completion)
                        .unwrap_or(PricingConstants::SONNET_OUTPUT_COST);

                let msg_cache_creation_cost = (usage.cache_creation_input_tokens.unwrap_or(0)
                    as f64)
                    * pricing
                        .get_model_pricing_sync(model)
                        .and_then(|p| p.cache_creation_input_token_cost)
                        .unwrap_or(PricingConstants::SONNET_CACHE_CREATION_COST);

                let msg_cache_read_cost = (usage.cache_read_input_tokens.unwrap_or(0) as f64)
                    * pricing
                        .get_model_pricing_sync(model)
                        .and_then(|p| p.cache_read_input_token_cost)
                        .unwrap_or(PricingConstants::SONNET_CACHE_READ_COST);

                input_cost += msg_input_cost;
                output_cost += msg_output_cost;
                cache_creation_cost += msg_cache_creation_cost;
                cache_read_cost += msg_cache_read_cost;
                total_cost += msg_input_cost
                    + msg_output_cost
                    + msg_cache_creation_cost
                    + msg_cache_read_cost;
            }
        }

        let days_analyzed = self.count_unique_days(messages);
        let daily_average = if days_analyzed > 0 {
            total_cost / days_analyzed as f64
        } else {
            0.0
        };
        let projected_monthly = daily_average * 30.0;

        Ok(CostBreakdown {
            total_cost,
            input_cost,
            output_cost,
            cache_creation_cost,
            cache_read_cost,
            daily_average,
            projected_monthly,
        })
    }

    /// Count unique days in messages - 분석 기간 계산
    fn count_unique_days(&self, messages: &[SessionMessage]) -> usize {
        use std::collections::HashSet;
        let mut unique_days = HashSet::new();

        for message in messages {
            if let Some(timestamp) = &message.message.timestamp {
                if let Some(date) = timestamp.split('T').next() {
                    unique_days.insert(date);
                }
            }
        }

        unique_days.len()
    }

    /// Analyze project usage - 프로젝트별 사용량
    fn analyze_project_usage(
        &self,
        messages: &[SessionMessage],
        pricing: &OpenRouterPricing,
    ) -> Result<HashMap<String, ProjectUsageStats>> {
        let mut project_stats: HashMap<String, ProjectUsageStats> = HashMap::new();
        let mut session_projects: HashMap<String, String> = HashMap::new();

        // First pass: map sessions to projects
        let projects_dir = self.claude_dir.join("projects");
        if projects_dir.exists() {
            for entry in fs::read_dir(&projects_dir)? {
                let entry = entry?;
                let project_path = entry.path();

                if project_path.is_dir() {
                    let sanitized_name = project_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    // Use the same project name logic as project_scanner.rs
                    let project_name =
                        match self.reconstruct_path_from_sanitized_name(sanitized_name) {
                            Ok(original_path) => {
                                if original_path.exists() {
                                    // Extract just the directory name (e.g., "cc-enhanced" from full path)
                                    original_path
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or(sanitized_name)
                                        .to_string()
                                } else {
                                    // For orphaned projects, use cleaner name logic
                                    if let Some(last_part) = sanitized_name.split('-').next_back() {
                                        if last_part.is_empty() {
                                            sanitized_name.to_string()
                                        } else {
                                            last_part.to_string()
                                        }
                                    } else {
                                        sanitized_name.to_string()
                                    }
                                }
                            }
                            Err(_) => {
                                // Fallback: use cleaner name from sanitized_name
                                if let Some(last_part) = sanitized_name.split('-').next_back() {
                                    if last_part.is_empty() {
                                        sanitized_name.to_string()
                                    } else {
                                        last_part.to_string()
                                    }
                                } else {
                                    sanitized_name.to_string()
                                }
                            }
                        };

                    for file_entry in fs::read_dir(&project_path)? {
                        let file_entry = file_entry?;
                        let file_path = file_entry.path();

                        if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                            let session_id = file_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_string();
                            session_projects.insert(session_id, project_name.clone());
                        }
                    }
                }
            }
        }

        // Second pass: analyze messages
        for message in messages {
            let project_name = session_projects
                .get(&message.session_id)
                .cloned()
                .unwrap_or_else(|| "Unknown Project".to_string());

            let entry = project_stats
                .entry(project_name.clone())
                .or_insert(ProjectUsageStats {
                    project_name: project_name.clone(),
                    total_tokens: 0,
                    total_cost: 0.0,
                    session_count: 0,
                    most_used_model: String::new(),
                    avg_session_length: 0.0,
                });

            if let Some(usage) = &message.message.usage {
                entry.total_tokens +=
                    usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0);

                if let Some(model) = &message.message.model {
                    let cost = self.calculate_message_cost(usage, model, pricing);
                    entry.total_cost += cost;
                    entry.most_used_model = model.clone(); // Simplified - could track frequency
                }
            }
        }

        Ok(project_stats)
    }

    /// Analyze session details - 세션별 상세 분석
    fn analyze_session_details(
        &self,
        messages: &[SessionMessage],
        pricing: &OpenRouterPricing,
    ) -> Result<Vec<SessionAnalytics>> {
        use std::collections::HashMap;
        let mut session_map: HashMap<String, Vec<&SessionMessage>> = HashMap::new();

        // Group messages by session
        for message in messages {
            session_map
                .entry(message.session_id.clone())
                .or_default()
                .push(message);
        }

        let mut session_analytics = Vec::new();

        for (session_id, session_messages) in session_map {
            if session_messages.is_empty() {
                continue;
            }

            let start_time = session_messages
                .first()
                .and_then(|m| m.message.timestamp.as_ref())
                .cloned()
                .unwrap_or_default();
            let end_time = session_messages
                .last()
                .and_then(|m| m.message.timestamp.as_ref())
                .cloned()
                .unwrap_or_default();

            let duration_minutes = self.calculate_session_duration(&start_time, &end_time);

            let mut total_tokens = 0u32;
            let mut total_cost = 0.0;
            let mut models_used = Vec::new();

            for message in &session_messages {
                if let Some(usage) = &message.message.usage {
                    total_tokens +=
                        usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0);

                    if let Some(model) = &message.message.model {
                        if !models_used.contains(model) {
                            models_used.push(model.clone());
                        }
                        total_cost += self.calculate_message_cost(usage, model, pricing);
                    }
                }
            }

            let efficiency_score = if total_tokens > 0 {
                (session_messages.len() as f64 / total_tokens as f64) * 1000.0
            } else {
                0.0
            };

            session_analytics.push(SessionAnalytics {
                session_id: session_id.clone(),
                project_name: "Unknown".to_string(), // Could be enhanced with project mapping
                start_time,
                end_time,
                duration_minutes,
                total_tokens,
                total_cost,
                message_count: session_messages.len(),
                models_used,
                efficiency_score,
            });
        }

        // Sort by total cost (highest first)
        session_analytics.sort_by(|a, b| {
            b.total_cost
                .partial_cmp(&a.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(session_analytics)
    }

    /// Calculate session duration in minutes
    fn calculate_session_duration(&self, start_time: &str, end_time: &str) -> f64 {
        use chrono::DateTime;

        if let (Ok(start), Ok(end)) = (
            DateTime::parse_from_rfc3339(start_time),
            DateTime::parse_from_rfc3339(end_time),
        ) {
            let duration = end.signed_duration_since(start);
            duration.num_minutes() as f64
        } else {
            0.0
        }
    }
}

/// Enhanced todo statistics for project-wide insights
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TodoStatistics {
    pub total_todos: usize,
    pub completed_todos: usize,
    pub pending_todos: usize,
    pub in_progress_todos: usize,
    pub high_priority_todos: usize,
    pub projects_with_todos: std::collections::HashSet<String>,
}
