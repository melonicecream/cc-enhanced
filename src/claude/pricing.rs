#![allow(dead_code)] // Allow unused code during migration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Model pricing information with input/output costs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_cost_per_token: f64,
    pub output_cost_per_token: f64,
    pub cache_creation_cost_per_token: f64,
    pub cache_read_cost_per_token: f64,
}

/// OpenRouter model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: String,
    pub pricing: ModelPricing,
}

/// OpenRouter API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenRouterResponse {
    data: Vec<OpenRouterModel>,
}

/// Cached pricing data with timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PricingCache {
    pub models: HashMap<String, ModelPricing>,
    pub timestamp: u64,
}

impl PricingCache {
    /// Load pricing cache from file
    fn load_from_file(cache_path: &Path) -> Result<Option<PricingCache>> {
        if !cache_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(cache_path)?;
        if content.trim().is_empty() {
            return Ok(None);
        }

        match serde_json::from_str::<PricingCache>(&content) {
            Ok(cache) => {
                // Check if cache is still valid (24 hours)
                let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

                if now - cache.timestamp < 24 * 3600 {
                    Ok(Some(cache))
                } else {
                    Ok(None) // Cache expired
                }
            }
            Err(_) => Ok(None), // Invalid cache format
        }
    }

    /// Save pricing cache to file
    fn save_to_file(&self, cache_path: &Path) -> Result<()> {
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(cache_path, content)?;
        Ok(())
    }

    /// Create new cache from OpenRouter response
    fn from_openrouter_response(response: OpenRouterResponse) -> PricingCache {
        let mut models = HashMap::new();

        for model in response.data {
            models.insert(model.id, model.pricing);
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        PricingCache { models, timestamp }
    }
}

/// OpenRouter pricing manager
pub struct OpenRouterPricing {
    cache_path: PathBuf,
    cache: Option<PricingCache>,
}

impl OpenRouterPricing {
    /// Create new OpenRouter pricing manager
    pub fn new(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            cache: None,
        }
    }

    /// Get model pricing (async version)
    pub async fn get_model_pricing(&mut self, model_name: &str) -> Result<Option<ModelPricing>> {
        // Load cache if not already loaded
        if self.cache.is_none() {
            self.cache = PricingCache::load_from_file(&self.cache_path)?;
        }

        // Try to get from cache first
        if let Some(ref cache) = self.cache {
            if let Some(pricing) = cache.models.get(model_name) {
                return Ok(Some(pricing.clone()));
            }
        }

        // If not in cache or cache is empty, try to update
        self.update_cache_if_needed().await?;

        // Try again after update
        if let Some(ref cache) = self.cache {
            Ok(cache.models.get(model_name).cloned())
        } else {
            Ok(None)
        }
    }

    /// Get model pricing (synchronous version with fallback)
    pub fn get_model_pricing_sync(&self, model_name: &str) -> Option<ModelPricing> {
        if let Some(ref cache) = self.cache {
            cache.models.get(model_name).cloned()
        } else {
            None
        }
    }

    /// Update cache if needed
    pub async fn update_cache_if_needed(&mut self) -> Result<()> {
        // Check if cache is still valid
        if let Some(ref cache) = self.cache {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

            if now - cache.timestamp < 24 * 3600 {
                return Ok(()); // Cache is still valid
            }
        }

        // Update cache from OpenRouter API
        self.update_cache_from_api().await
    }

    /// Force update cache from OpenRouter API
    async fn update_cache_from_api(&mut self) -> Result<()> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://openrouter.ai/api/v1/models")
            .header("User-Agent", "cc-enhanced/1.0")
            .send()
            .await?;

        let openrouter_response: OpenRouterResponse = response.json().await?;
        let new_cache = PricingCache::from_openrouter_response(openrouter_response);

        // Save to file
        new_cache.save_to_file(&self.cache_path)?;

        // Update in-memory cache
        self.cache = Some(new_cache);

        Ok(())
    }
}

/// Fallback pricing constants when OpenRouter API is unavailable
pub struct PricingConstants;

impl PricingConstants {
    /// Get fallback pricing for Claude models
    pub fn get_claude_fallback_pricing(model: &str) -> ModelPricing {
        match model {
            m if m.contains("claude-3-5-sonnet") => ModelPricing {
                input_cost_per_token: 0.000003,
                output_cost_per_token: 0.000015,
                cache_creation_cost_per_token: 0.00000375,
                cache_read_cost_per_token: 0.0000003,
            },
            m if m.contains("claude-3-opus") => ModelPricing {
                input_cost_per_token: 0.000015,
                output_cost_per_token: 0.000075,
                cache_creation_cost_per_token: 0.00001875,
                cache_read_cost_per_token: 0.0000015,
            },
            m if m.contains("claude-3-haiku") => ModelPricing {
                input_cost_per_token: 0.00000025,
                output_cost_per_token: 0.00000125,
                cache_creation_cost_per_token: 0.0000003125,
                cache_read_cost_per_token: 0.000000025,
            },
            _ => ModelPricing {
                // Default to Claude 3.5 Sonnet pricing
                input_cost_per_token: 0.000003,
                output_cost_per_token: 0.000015,
                cache_creation_cost_per_token: 0.00000375,
                cache_read_cost_per_token: 0.0000003,
            },
        }
    }
}

/// Token usage information for cost calculation
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_tokens: u32,
    pub cache_read_tokens: u32,
}

/// Pricing manager that combines OpenRouter and fallback pricing
pub struct PricingManager {
    openrouter: OpenRouterPricing,
}

impl PricingManager {
    /// Create new pricing manager
    pub fn new(claude_dir: &Path) -> Self {
        let cache_path = claude_dir.join("openrouter_pricing_cache.json");
        Self {
            openrouter: OpenRouterPricing::new(cache_path),
        }
    }

    /// Calculate cost for given token usage and model
    pub async fn calculate_cost(&mut self, usage: &TokenUsage, model: &str) -> f64 {
        let pricing = if let Ok(Some(pricing)) = self.openrouter.get_model_pricing(model).await {
            pricing
        } else {
            PricingConstants::get_claude_fallback_pricing(model)
        };

        (usage.input_tokens as f64 * pricing.input_cost_per_token)
            + (usage.output_tokens as f64 * pricing.output_cost_per_token)
            + (usage.cache_creation_tokens as f64 * pricing.cache_creation_cost_per_token)
            + (usage.cache_read_tokens as f64 * pricing.cache_read_cost_per_token)
    }

    /// Calculate cost synchronously with fallback
    pub fn calculate_cost_sync(&self, usage: &TokenUsage, model: &str) -> f64 {
        let pricing = if let Some(pricing) = self.openrouter.get_model_pricing_sync(model) {
            pricing
        } else {
            PricingConstants::get_claude_fallback_pricing(model)
        };

        (usage.input_tokens as f64 * pricing.input_cost_per_token)
            + (usage.output_tokens as f64 * pricing.output_cost_per_token)
            + (usage.cache_creation_tokens as f64 * pricing.cache_creation_cost_per_token)
            + (usage.cache_read_tokens as f64 * pricing.cache_read_cost_per_token)
    }

    /// Get pricing for a specific model
    pub async fn get_pricing_for_model(&mut self, model: &str) -> ModelPricing {
        if let Ok(Some(pricing)) = self.openrouter.get_model_pricing(model).await {
            pricing
        } else {
            PricingConstants::get_claude_fallback_pricing(model)
        }
    }

    /// Update pricing cache
    pub async fn update_pricing_cache_if_needed(&mut self) -> Result<()> {
        self.openrouter.update_cache_if_needed().await
    }

    /// Check if model matches Claude patterns
    pub fn matches_claude_model(&self, model: &str) -> bool {
        model.contains("claude") || model.contains("anthropic") || model.starts_with("claude-")
    }
}
