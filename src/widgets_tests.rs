//! Simple integration tests for widgets module
//! This file contains basic smoke tests to ensure core functionality works

#[cfg(test)]
mod tests {
    use crate::{shared::theme::*, widgets::*};

    #[test]
    fn test_format_project_name_basic() {
        let name = "test";
        let result = format_project_name(name, 10);
        assert_eq!(result, "test");
    }

    #[test]
    fn test_priority_icon_basic() {
        assert_eq!(priority_icon("high"), ModernIcons::HIGH_PRIORITY);
        assert_eq!(priority_icon("medium"), ModernIcons::MEDIUM_PRIORITY);
        assert_eq!(priority_icon("low"), ModernIcons::LOW_PRIORITY);
    }

    #[test]
    fn test_status_icon_basic() {
        assert_eq!(status_icon(true), ModernIcons::ACTIVE);
        assert_eq!(status_icon(false), ModernIcons::INACTIVE);
    }

    #[test]
    fn test_modern_theme_creation() {
        let theme = ModernTheme::dark();
        // Theme created successfully
        let _ = theme; // Use variable to avoid warnings
    }
}
