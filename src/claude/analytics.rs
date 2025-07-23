#![allow(dead_code)] // Allow unused code during migration

use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, Timelike};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::pricing::OpenRouterPricing;
use super::session_parser::{SessionMessage, MessageContent, TokenUsage};
use super::usage_calculator::{ProjectAnalytics, SessionBlock, UsageStats};

/// Comprehensive usage analytics
#[derive(Debug, Clone)]
pub struct UsageAnalytics {
    pub daily_usage: Vec<DailyUsageDetail>,
    pub model_distribution: HashMap<String, ModelUsageStats>,
    pub hourly_patterns: Vec<HourlyUsage>,
    pub cache_efficiency: CacheEfficiencyStats,
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
    pub total_cost: f64,
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_creation_cost: f64,
    pub cache_read_cost: f64,
    pub daily_average: f64,
    pub projected_monthly: f64,
}

#[derive(Debug, Clone)]
pub struct ProjectUsageStats {
    pub project_name: String,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub session_count: usize,
    pub most_used_model: String,
    pub avg_session_length: f64,
}

#[derive(Debug, Clone)]
pub struct SessionAnalytics {
    pub session_id: String,
    pub project_name: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_minutes: f64,
    pub total_tokens: u32,
    pub total_cost: f64,
    pub message_count: usize,
    pub models_used: Vec<String>,
}

/// Analytics calculator for Claude session data
pub struct AnalyticsCalculator {
    claude_dir: PathBuf,
}

impl AnalyticsCalculator {
    /// Create new analytics calculator
    pub fn new(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// Generate comprehensive usage analytics
    pub fn generate_comprehensive_analytics(&self, openrouter_pricing: &OpenRouterPricing) -> Result<UsageAnalytics> {
        let messages = self.parse_all_session_messages()?;
        
        // Daily usage patterns
        let daily_usage = self.analyze_daily_usage_details(&messages, openrouter_pricing)?;
        
        // Model distribution analysis
        let model_distribution = self.analyze_model_distribution(&messages, openrouter_pricing)?;
        
        // Hourly patterns
        let hourly_patterns = self.analyze_hourly_patterns(&messages)?;
        
        // Cache efficiency analysis
        let cache_efficiency = self.analyze_cache_efficiency(&messages)?;
        
        // Session analytics
        let session_analytics = self.analyze_session_details(&messages, openrouter_pricing)?;

        // Cost breakdown
        let cost_breakdown = self.analyze_cost_breakdown(&messages, openrouter_pricing)?;
        
        // Project usage patterns  
        let project_usage = self.analyze_project_usage_patterns(&messages, openrouter_pricing)?;

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

    /// Parse all session messages from all projects
    fn parse_all_session_messages(&self) -> Result<Vec<SessionMessage>> {
        let mut all_messages = Vec::new();
        let projects_dir = self.claude_dir.join("projects");

        if !projects_dir.exists() {
            return Ok(all_messages);
        }

        let entries = fs::read_dir(&projects_dir)?;
        for entry in entries {
            let entry = entry?;
            let project_dir = entry.path();

            if project_dir.is_dir() {
                let session_entries = fs::read_dir(&project_dir)?;
                for session_entry in session_entries {
                    let session_entry = session_entry?;
                    let file_path = session_entry.path();

                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        if let Ok(messages) = self.parse_session_messages(&file_path) {
                            all_messages.extend(messages);
                        }
                    }
                }
            }
        }

        Ok(all_messages)
    }

    /// Parse messages from a single session file
    fn parse_session_messages(&self, file_path: &Path) -> Result<Vec<SessionMessage>> {
        let content = fs::read_to_string(file_path)?;
        let mut messages = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if let Some(message) = self.parse_message_from_json(&json) {
                    messages.push(message);
                }
            }
        }

        Ok(messages)
    }

    /// Parse a single message from JSON
    fn parse_message_from_json(&self, json: &Value) -> Option<SessionMessage> {
        // Implementation would match the existing message parsing logic
        // This is a simplified version for the module structure
        if let Some(message_type) = json.get("type").and_then(|t| t.as_str()) {
            if message_type == "user" || message_type == "assistant" {
                let session_id = json.get("sessionId")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let message_content = MessageContent {
                    usage: None, // Would be parsed from actual usage data
                    model: json.get("model").and_then(|m| m.as_str()).map(String::from),
                    timestamp: json.get("timestamp").and_then(|t| t.as_str()).map(String::from),
                    content: json.get("content").cloned(),
                };

                return Some(SessionMessage {
                    session_id,
                    message_type: message_type.to_string(),
                    message: message_content,
                    cwd: json.get("cwd").and_then(|c| c.as_str()).map(String::from),
                });
            }
        }
        None
    }

    /// Analyze daily usage details
    fn analyze_daily_usage_details(&self, messages: &[SessionMessage], pricing: &OpenRouterPricing) -> Result<Vec<DailyUsageDetail>> {
        let mut daily_stats: HashMap<String, DailyUsageDetail> = HashMap::new();

        for message in messages {
            if let Some(timestamp) = &message.message.timestamp {
                if let Ok(date_time) = DateTime::parse_from_rfc3339(timestamp) {
                    let date_str = date_time.format("%Y-%m-%d").to_string();
                    
                    let entry = daily_stats.entry(date_str.clone()).or_insert(DailyUsageDetail {
                        date: date_str,
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
                    
                    if let Some(model) = &message.message.model {
                        if !entry.models_used.contains(model) {
                            entry.models_used.push(model.clone());
                        }
                    }

                    // Add token and cost calculation logic here
                }
            }
        }

        let mut result: Vec<DailyUsageDetail> = daily_stats.into_values().collect();
        result.sort_by(|a, b| a.date.cmp(&b.date));
        Ok(result)
    }

    /// Analyze hourly usage patterns
    fn analyze_hourly_patterns(&self, messages: &[SessionMessage]) -> Result<Vec<HourlyUsage>> {
        let mut hourly_stats: HashMap<u8, HourlyUsage> = HashMap::new();

        for message in messages {
            if let Some(timestamp) = &message.message.timestamp {
                if let Ok(date_time) = DateTime::parse_from_rfc3339(timestamp) {
                    let hour = date_time.hour() as u8;
                    
                    let entry = hourly_stats.entry(hour).or_insert(HourlyUsage {
                        hour,
                        total_tokens: 0,
                        total_cost: 0.0,
                        message_count: 0,
                    });

                    entry.message_count += 1;
                    // Add token and cost calculation logic here
                }
            }
        }

        let mut result: Vec<HourlyUsage> = hourly_stats.into_values().collect();
        result.sort_by_key(|h| h.hour);
        Ok(result)
    }

    /// Analyze cache efficiency
    fn analyze_cache_efficiency(&self, messages: &[SessionMessage]) -> Result<CacheEfficiencyStats> {
        let mut total_cache_creation = 0u32;
        let mut total_cache_read = 0u32;

        for message in messages {
            if let Some(usage) = &message.message.usage {
                if let Some(creation_tokens) = usage.cache_creation_input_tokens {
                    total_cache_creation += creation_tokens;
                }
                if let Some(read_tokens) = usage.cache_read_input_tokens {
                    total_cache_read += read_tokens;
                }
            }
        }

        let total_cache_tokens = total_cache_creation + total_cache_read;
        let cache_hit_rate = if total_cache_tokens > 0 {
            total_cache_read as f64 / total_cache_tokens as f64 * 100.0
        } else {
            0.0
        };

        // Estimate cost savings (cache reads are typically 90% cheaper)
        let cache_cost_savings = total_cache_read as f64 * 0.9 * 0.000003; // Approximate savings

        Ok(CacheEfficiencyStats {
            total_cache_creation_tokens: total_cache_creation,
            total_cache_read_tokens: total_cache_read,
            cache_hit_rate,
            cache_cost_savings,
        })
    }

    /// Analyze model distribution
    fn analyze_model_distribution(&self, messages: &[SessionMessage], pricing: &OpenRouterPricing) -> Result<HashMap<String, ModelUsageStats>> {
        let mut model_stats: HashMap<String, ModelUsageStats> = HashMap::new();

        for message in messages {
            if let Some(model) = &message.message.model {
                let entry = model_stats.entry(model.clone()).or_insert(ModelUsageStats {
                    model_name: model.clone(),
                    total_input_tokens: 0,
                    total_output_tokens: 0,
                    total_cost: 0.0,
                    usage_count: 0,
                    first_used: message.message.timestamp.clone().unwrap_or_default(),
                    last_used: message.message.timestamp.clone().unwrap_or_default(),
                    avg_cost_per_message: 0.0,
                });

                entry.usage_count += 1;
                
                if let Some(timestamp) = &message.message.timestamp {
                    entry.last_used = timestamp.clone();
                }

                // Add token and cost calculation logic here
            }
        }

        // Calculate averages
        for stats in model_stats.values_mut() {
            if stats.usage_count > 0 {
                stats.avg_cost_per_message = stats.total_cost / stats.usage_count as f64;
            }
        }

        Ok(model_stats)
    }

    /// Analyze session details
    fn analyze_session_details(&self, messages: &[SessionMessage], pricing: &OpenRouterPricing) -> Result<Vec<SessionAnalytics>> {
        // Group messages by session (simplified logic)
        let mut sessions: HashMap<String, Vec<&SessionMessage>> = HashMap::new();
        
        for message in messages {
            let session_key = format!("session_{}", message.message.timestamp.as_deref().unwrap_or("unknown"));
            sessions.entry(session_key).or_default().push(message);
        }

        let mut session_analytics = Vec::new();
        for (session_id, session_messages) in sessions {
            if session_messages.is_empty() {
                continue;
            }

            let start_time = session_messages.first()
                .and_then(|m| m.message.timestamp.as_ref())
                .unwrap_or(&"".to_string())
                .clone();
            
            let end_time = session_messages.last()
                .and_then(|m| m.message.timestamp.as_ref())
                .unwrap_or(&"".to_string())
                .clone();

            let models_used: HashSet<String> = session_messages
                .iter()
                .filter_map(|m| m.message.model.as_ref())
                .cloned()
                .collect();

            session_analytics.push(SessionAnalytics {
                session_id,
                project_name: "Unknown".to_string(), // Would be extracted from path
                start_time,
                end_time,
                duration_minutes: 0.0, // Would be calculated from timestamps
                total_tokens: 0, // Would be calculated from usage
                total_cost: 0.0, // Would be calculated from usage
                message_count: session_messages.len(),
                models_used: models_used.into_iter().collect(),
            });
        }

        Ok(session_analytics)
    }

    /// Analyze cost breakdown
    fn analyze_cost_breakdown(&self, messages: &[SessionMessage], pricing: &OpenRouterPricing) -> Result<CostBreakdown> {
        let mut total_cost = 0.0;
        let mut input_cost = 0.0;
        let mut output_cost = 0.0;
        let mut cache_creation_cost = 0.0;
        let mut cache_read_cost = 0.0;

        // Calculate costs from messages (simplified logic)
        for message in messages {
            if let Some(usage) = &message.message.usage {
                // Add cost calculation logic here based on usage and pricing
            }
        }

        let daily_average = total_cost / 30.0; // Simplified calculation
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

    /// Analyze project usage patterns
    fn analyze_project_usage_patterns(&self, messages: &[SessionMessage], pricing: &OpenRouterPricing) -> Result<HashMap<String, ProjectUsageStats>> {
        let mut project_stats: HashMap<String, ProjectUsageStats> = HashMap::new();

        // Group messages by project (simplified logic based on timestamp patterns)
        for message in messages {
            let project_name = "Default Project".to_string(); // Would be extracted from session path
            
            let entry = project_stats.entry(project_name.clone()).or_insert(ProjectUsageStats {
                project_name,
                total_tokens: 0,
                total_cost: 0.0,
                session_count: 0,
                most_used_model: "claude-3-sonnet-20240229".to_string(),
                avg_session_length: 0.0,
            });

            // Add aggregation logic here
        }

        Ok(project_stats)
    }
}