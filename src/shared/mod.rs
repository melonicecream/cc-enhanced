/// Shared modules used across the application
pub mod config;
pub mod i18n;
pub mod theme;

// Re-export commonly used items
pub use config::{Config, ThemeMode};
pub use i18n::I18n;
pub use theme::ModernTheme;
