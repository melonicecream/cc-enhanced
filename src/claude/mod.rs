//! Claude data management modules
//!
//! This module contains the refactored components for managing Claude project data,
//! usage analytics, pricing, and related functionality. The original large claude.rs
//! file has been split into focused, maintainable modules.

pub mod pricing;
pub mod project_scanner;
pub mod session_parser;
pub mod usage_calculator;
// pub mod data_manager;  // Temporarily disabled - needs API alignment
// pub mod analytics;  // Temporarily disabled for gradual migration

// Re-export only used types
pub use project_scanner::Project;
pub use session_parser::{MessageContent, Session, SessionMessage, TokenUsage};
pub use usage_calculator::{ProjectAnalytics, UsageStats};
// pub use data_manager::{ClaudeDataManager, EnhancedTodoItem, NotificationStates, SessionMetrics};
// pub use analytics::{UsageAnalytics, AnalyticsCalculator};  // Temporarily disabled

// Temporary re-exports from the original claude_legacy.rs until full migration
pub use crate::claude_legacy::*;
