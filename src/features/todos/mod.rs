//! Todos module - handles todo extraction and management from Claude sessions
//!
//! This module provides:
//! - Todo extraction from Claude session files using regex patterns
//! - Priority and status management (High, Medium, Low)
//! - Todo item persistence and state tracking

pub mod data;

// Re-export commonly used types
pub use data::{ProjectTodoStats, SessionTodos, TodoItem, TodoManager, TodoPriority, TodoStatus};
