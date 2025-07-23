use ratatui::style::{Color, Modifier, Style};

/// Modern color palette for the Claude Code Enhanced TUI
#[derive(Debug, Clone)]
pub struct ModernTheme {
    // Primary colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,

    // Status colors
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub info: Color,

    // Background and surface colors
    #[allow(dead_code)]
    pub background: Color,
    #[allow(dead_code)]
    pub surface: Color,
    #[allow(dead_code)]
    pub surface_variant: Color,

    // Text colors
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_disabled: Color,

    // Interactive colors
    pub hover: Color,
    pub selected: Color,
    pub border: Color,
    pub border_focused: Color,
}

impl Default for ModernTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl ModernTheme {
    /// Modern dark theme with vibrant accents
    pub fn dark() -> Self {
        Self {
            // Primary colors - Indigo/Purple gradient
            primary: Color::Rgb(99, 102, 241),   // Indigo-500
            secondary: Color::Rgb(139, 92, 246), // Violet-500
            accent: Color::Rgb(168, 85, 247),    // Purple-500

            // Status colors
            success: Color::Rgb(34, 197, 94),  // Green-500
            warning: Color::Rgb(251, 191, 36), // Amber-500
            danger: Color::Rgb(239, 68, 68),   // Red-500
            info: Color::Rgb(59, 130, 246),    // Blue-500

            // Background colors
            background: Color::Rgb(17, 24, 39),      // Gray-900
            surface: Color::Rgb(31, 41, 55),         // Gray-800
            surface_variant: Color::Rgb(55, 65, 81), // Gray-700

            // Text colors
            text_primary: Color::Rgb(243, 244, 246), // Gray-100
            text_secondary: Color::Rgb(156, 163, 175), // Gray-400
            text_disabled: Color::Rgb(107, 114, 128), // Gray-500

            // Interactive colors
            hover: Color::Rgb(75, 85, 99),            // Gray-600
            selected: Color::Rgb(99, 102, 241),       // Indigo-500
            border: Color::Rgb(75, 85, 99),           // Gray-600
            border_focused: Color::Rgb(99, 102, 241), // Indigo-500
        }
    }

    /// Light theme variant
    pub fn light() -> Self {
        Self {
            primary: Color::Rgb(99, 102, 241),
            secondary: Color::Rgb(139, 92, 246),
            accent: Color::Rgb(168, 85, 247),

            success: Color::Rgb(34, 197, 94),
            warning: Color::Rgb(251, 191, 36),
            danger: Color::Rgb(239, 68, 68),
            info: Color::Rgb(59, 130, 246),

            background: Color::Rgb(255, 255, 255),
            surface: Color::Rgb(249, 250, 251),
            surface_variant: Color::Rgb(243, 244, 246),

            text_primary: Color::Rgb(17, 24, 39),
            text_secondary: Color::Rgb(107, 114, 128),
            text_disabled: Color::Rgb(156, 163, 175),

            hover: Color::Rgb(229, 231, 235),
            selected: Color::Rgb(99, 102, 241),
            border: Color::Rgb(209, 213, 219),
            border_focused: Color::Rgb(99, 102, 241),
        }
    }

    /// Ocean blue theme with cool tones
    pub fn ocean() -> Self {
        Self {
            // Ocean blues and teals
            primary: Color::Rgb(14, 165, 233),  // Sky-500
            secondary: Color::Rgb(6, 182, 212), // Cyan-500
            accent: Color::Rgb(20, 184, 166),   // Teal-500

            success: Color::Rgb(16, 185, 129), // Emerald-500
            warning: Color::Rgb(245, 158, 11), // Amber-500
            danger: Color::Rgb(239, 68, 68),   // Red-500
            info: Color::Rgb(59, 130, 246),    // Blue-500

            background: Color::Rgb(15, 23, 42),      // Slate-900
            surface: Color::Rgb(30, 41, 59),         // Slate-800
            surface_variant: Color::Rgb(51, 65, 85), // Slate-700

            text_primary: Color::Rgb(248, 250, 252), // Slate-50
            text_secondary: Color::Rgb(148, 163, 184), // Slate-400
            text_disabled: Color::Rgb(100, 116, 139), // Slate-500

            hover: Color::Rgb(71, 85, 105),           // Slate-600
            selected: Color::Rgb(14, 165, 233),       // Sky-500
            border: Color::Rgb(71, 85, 105),          // Slate-600
            border_focused: Color::Rgb(14, 165, 233), // Sky-500
        }
    }

    /// Forest green theme with nature tones
    pub fn forest() -> Self {
        Self {
            // Forest greens and earth tones
            primary: Color::Rgb(34, 197, 94),   // Green-500
            secondary: Color::Rgb(22, 163, 74), // Green-600
            accent: Color::Rgb(132, 204, 22),   // Lime-500

            success: Color::Rgb(34, 197, 94), // Green-500
            warning: Color::Rgb(234, 179, 8), // Yellow-500
            danger: Color::Rgb(220, 38, 38),  // Red-600
            info: Color::Rgb(59, 130, 246),   // Blue-500

            background: Color::Rgb(20, 83, 45),       // Green-900
            surface: Color::Rgb(22, 101, 52),         // Green-800
            surface_variant: Color::Rgb(21, 128, 61), // Green-700

            text_primary: Color::Rgb(240, 253, 244), // Green-50
            text_secondary: Color::Rgb(134, 239, 172), // Green-300
            text_disabled: Color::Rgb(74, 222, 128), // Green-400

            hover: Color::Rgb(22, 163, 74),          // Green-600
            selected: Color::Rgb(34, 197, 94),       // Green-500
            border: Color::Rgb(22, 163, 74),         // Green-600
            border_focused: Color::Rgb(34, 197, 94), // Green-500
        }
    }

    /// Sunset orange theme with warm tones
    pub fn sunset() -> Self {
        Self {
            // Sunset oranges and reds
            primary: Color::Rgb(251, 146, 60),   // Orange-400
            secondary: Color::Rgb(249, 115, 22), // Orange-500
            accent: Color::Rgb(245, 101, 101),   // Red-400

            success: Color::Rgb(34, 197, 94),  // Green-500
            warning: Color::Rgb(251, 191, 36), // Amber-400
            danger: Color::Rgb(239, 68, 68),   // Red-500
            info: Color::Rgb(96, 165, 250),    // Blue-400

            background: Color::Rgb(69, 10, 10),       // Red-950
            surface: Color::Rgb(87, 13, 13),          // Red-900
            surface_variant: Color::Rgb(127, 29, 29), // Red-800

            text_primary: Color::Rgb(254, 242, 242), // Red-50
            text_secondary: Color::Rgb(252, 165, 165), // Red-300
            text_disabled: Color::Rgb(248, 113, 113), // Red-400

            hover: Color::Rgb(185, 28, 28),           // Red-700
            selected: Color::Rgb(251, 146, 60),       // Orange-400
            border: Color::Rgb(185, 28, 28),          // Red-700
            border_focused: Color::Rgb(251, 146, 60), // Orange-400
        }
    }

    /// Purple galaxy theme with cosmic tones
    pub fn galaxy() -> Self {
        Self {
            // Galaxy purples and magentas
            primary: Color::Rgb(147, 51, 234),   // Purple-600
            secondary: Color::Rgb(168, 85, 247), // Purple-500
            accent: Color::Rgb(217, 70, 239),    // Fuchsia-500

            success: Color::Rgb(34, 197, 94),  // Green-500
            warning: Color::Rgb(251, 191, 36), // Amber-400
            danger: Color::Rgb(244, 63, 94),   // Rose-500
            info: Color::Rgb(139, 92, 246),    // Violet-500

            background: Color::Rgb(35, 0, 81),        // Purple-950
            surface: Color::Rgb(59, 7, 100),          // Purple-900
            surface_variant: Color::Rgb(88, 28, 135), // Purple-800

            text_primary: Color::Rgb(250, 245, 255), // Purple-50
            text_secondary: Color::Rgb(196, 181, 253), // Purple-300
            text_disabled: Color::Rgb(147, 197, 253), // Blue-300

            hover: Color::Rgb(126, 34, 206),          // Purple-700
            selected: Color::Rgb(147, 51, 234),       // Purple-600
            border: Color::Rgb(126, 34, 206),         // Purple-700
            border_focused: Color::Rgb(147, 51, 234), // Purple-600
        }
    }
}

/// Modern iconography using Unicode symbols
pub struct ModernIcons;

impl ModernIcons {
    // Project status icons
    pub const ACTIVE: &'static str = "●"; // Solid circle
    pub const INACTIVE: &'static str = "○"; // Hollow circle
    #[allow(dead_code)]
    pub const PARTIAL: &'static str = "◐"; // Half circle
    #[allow(dead_code)]
    pub const STARRED: &'static str = "✦"; // Star

    // Todo status icons
    pub const COMPLETED: &'static str = "✓"; // Check mark
    pub const IN_PROGRESS: &'static str = "⟳"; // Rotating arrows
    pub const PENDING: &'static str = "○"; // Hollow circle

    // Tab icons
    pub const OVERVIEW: &'static str = "◉"; // Solid circle with ring
    pub const USAGE: &'static str = "▲"; // Triangle up
    pub const SESSIONS: &'static str = "⚡"; // Lightning bolt
    pub const TODOS: &'static str = "✓"; // Check mark
    pub const QUOTA: &'static str = "⬢"; // Hexagon

    // UI elements
    pub const REFRESH: &'static str = "⟳"; // Rotating arrows
    pub const TIME: &'static str = "◷"; // Clock
    pub const ARROW_RIGHT: &'static str = "▶"; // Play button
    #[allow(dead_code)]
    pub const ARROW_DOWN: &'static str = "▼"; // Down arrow
    pub const BULLET: &'static str = "•"; // Bullet point
    pub const HELP: &'static str = "❓"; // Help question mark

    // Priority indicators - modern flat design
    pub const HIGH_PRIORITY: &'static str = "▲"; // Triangle (high urgency)
    pub const MEDIUM_PRIORITY: &'static str = "■"; // Square (medium urgency)
    pub const LOW_PRIORITY: &'static str = "▼"; // Down triangle (low urgency)

    // Alternative priority indicators (text-based)
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn test_modern_theme_default() {
        let theme = ModernTheme::default();
        assert_eq!(theme.primary, Color::Rgb(99, 102, 241)); // Indigo-500
        assert_eq!(theme.success, Color::Rgb(34, 197, 94)); // Green-500
    }

    #[test]
    fn test_all_themes_creation() {
        // Test that all themes can be created without panicking
        let _dark = ModernTheme::dark();
        let _light = ModernTheme::light();
        let _ocean = ModernTheme::ocean();
        let _forest = ModernTheme::forest();
        let _sunset = ModernTheme::sunset();
        let _galaxy = ModernTheme::galaxy();
        // All themes created successfully
    }

    #[test]
    fn test_priority_style_high() {
        let theme = ModernTheme::dark();
        let style = theme.priority_style("high");
        assert_eq!(style.fg, Some(theme.danger));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_priority_style_medium() {
        let theme = ModernTheme::dark();
        let style = theme.priority_style("medium");
        assert_eq!(style.fg, Some(theme.warning));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_priority_style_low() {
        let theme = ModernTheme::dark();
        let style = theme.priority_style("low");
        assert_eq!(style.fg, Some(theme.success));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn test_priority_style_case_insensitive() {
        let theme = ModernTheme::dark();
        assert_eq!(
            theme.priority_style("HIGH").fg,
            theme.priority_style("high").fg
        );
        assert_eq!(
            theme.priority_style("MEDIUM").fg,
            theme.priority_style("medium").fg
        );
        assert_eq!(
            theme.priority_style("LOW").fg,
            theme.priority_style("low").fg
        );
    }

    #[test]
    fn test_priority_style_unknown() {
        let theme = ModernTheme::dark();
        let style = theme.priority_style("unknown");
        assert_eq!(style.fg, theme.secondary_text_style().fg);
    }

    #[test]
    fn test_progress_style_for_percentage() {
        let theme = ModernTheme::dark();

        // Low usage should be success (green)
        let low_style = theme.progress_style_for_percentage(50.0);
        assert_eq!(low_style.fg, Some(theme.success));

        // Medium usage should be warning (yellow/amber)
        let medium_style = theme.progress_style_for_percentage(80.0);
        assert_eq!(medium_style.fg, Some(theme.warning));

        // High usage should be danger (red)
        let high_style = theme.progress_style_for_percentage(95.0);
        assert_eq!(high_style.fg, Some(theme.danger));
    }

    #[test]
    fn test_predefined_styles() {
        let theme = ModernTheme::dark();

        // Test that all predefined styles can be created
        let _header = theme.header_style();
        let _secondary = theme.secondary_text_style();
        let _success = theme.success_style();
        let _warning = theme.warning_style();
        let _danger = theme.danger_style();
        let _info = theme.info_style();
        let _selected = theme.selected_style();
        let _hover = theme.hover_style();
        let _border = theme.border_style();
        let _border_focused = theme.border_focused_style();
        let _metric = theme.metric_style();
        let _dimmed = theme.dimmed_style();

        // All styles created successfully
    }

    #[test]
    fn test_theme_color_consistency() {
        // Test that each theme has consistent color assignments
        let themes = vec![
            ModernTheme::dark(),
            ModernTheme::light(),
            ModernTheme::ocean(),
            ModernTheme::forest(),
            ModernTheme::sunset(),
            ModernTheme::galaxy(),
        ];

        for theme in themes {
            // All themes should have distinct primary, secondary, and accent colors
            assert_ne!(theme.primary, theme.secondary);
            assert_ne!(theme.primary, theme.accent);
            assert_ne!(theme.secondary, theme.accent);

            // Status colors should be defined
            assert_ne!(theme.success, Color::Reset);
            assert_ne!(theme.warning, Color::Reset);
            assert_ne!(theme.danger, Color::Reset);
            assert_ne!(theme.info, Color::Reset);
        }
    }

    #[test]
    fn test_modern_icons_constants() {
        // Test that all icon constants are defined and accessible
        assert_eq!(ModernIcons::ACTIVE, "●");
        assert_eq!(ModernIcons::INACTIVE, "○");
        assert_eq!(ModernIcons::COMPLETED, "✓");
        assert_eq!(ModernIcons::IN_PROGRESS, "⟳");
        assert_eq!(ModernIcons::PENDING, "○"); // Same as inactive by design
        assert_eq!(ModernIcons::HIGH_PRIORITY, "▲");
        assert_eq!(ModernIcons::MEDIUM_PRIORITY, "■");
        assert_eq!(ModernIcons::LOW_PRIORITY, "▼");
        assert_eq!(ModernIcons::REFRESH, "⟳");
        assert_eq!(ModernIcons::TIME, "◷");
        assert_eq!(ModernIcons::BULLET, "•");
    }

    #[test]
    fn test_progress_chars_constants() {
        // Test that progress bar characters are valid Unicode
        assert_ne!(ProgressChars::FILLED, '\0');
        assert_ne!(ProgressChars::EMPTY, '\0');
        assert_ne!(ProgressChars::PARTIAL, '\0');
    }

    #[test]
    fn test_gradient_style_creation() {
        let start_color = Color::Red;
        let end_color = Color::Blue;

        // Test different positions
        let style_start = create_gradient_style(start_color, end_color, 0.0);
        assert_eq!(style_start.fg, Some(start_color));

        let style_end = create_gradient_style(start_color, end_color, 1.0);
        assert_eq!(style_end.fg, Some(end_color));

        let style_mid = create_gradient_style(start_color, end_color, 0.5);
        // Should be end color based on current implementation
        assert_eq!(style_mid.fg, Some(end_color));
    }

    #[test]
    fn test_theme_specific_colors() {
        // Test specific color values for different themes
        let dark = ModernTheme::dark();
        let light = ModernTheme::light();
        let ocean = ModernTheme::ocean();

        // Dark theme should have dark background
        assert_eq!(dark.background, Color::Rgb(17, 24, 39));

        // Light theme should have light background
        assert_eq!(light.background, Color::Rgb(255, 255, 255));

        // Ocean theme should have ocean-like colors
        assert_eq!(ocean.primary, Color::Rgb(14, 165, 233)); // Sky blue
    }
}

/// Progress bar characters for modern look
pub struct ProgressChars;

impl ProgressChars {
    pub const FILLED: char = '█'; // Full block
    pub const EMPTY: char = '░'; // Light shade
    pub const PARTIAL: char = '▒'; // Medium shade
    #[allow(dead_code)]
    pub const CORNER_LEFT: char = '▐'; // Left half block
    #[allow(dead_code)]
    pub const CORNER_RIGHT: char = '▌'; // Right half block
}

/// Pre-defined styles for common UI elements
impl ModernTheme {
    /// Style for headers and titles
    pub fn header_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for secondary text
    pub fn secondary_text_style(&self) -> Style {
        Style::default().fg(self.text_secondary)
    }

    /// Style for success messages
    pub fn success_style(&self) -> Style {
        Style::default()
            .fg(self.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for warnings
    pub fn warning_style(&self) -> Style {
        Style::default()
            .fg(self.warning)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for errors
    pub fn danger_style(&self) -> Style {
        Style::default()
            .fg(self.danger)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for info text
    pub fn info_style(&self) -> Style {
        Style::default().fg(self.info)
    }

    /// Style for selected items
    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.text_primary)
            .bg(self.selected)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for hovered items
    pub fn hover_style(&self) -> Style {
        Style::default().bg(self.hover).add_modifier(Modifier::BOLD)
    }

    /// Style for borders
    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    /// Style for focused borders
    pub fn border_focused_style(&self) -> Style {
        Style::default()
            .fg(self.border_focused)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for progress bars - success variant
    pub fn progress_success_style(&self) -> Style {
        Style::default()
            .fg(self.success)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for progress bars - warning variant
    pub fn progress_warning_style(&self) -> Style {
        Style::default()
            .fg(self.warning)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for progress bars - danger variant
    pub fn progress_danger_style(&self) -> Style {
        Style::default()
            .fg(self.danger)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for numbers and metrics
    pub fn metric_style(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for dimmed/disabled elements
    pub fn dimmed_style(&self) -> Style {
        Style::default().fg(self.text_disabled)
    }

    /// Get progress style based on percentage
    pub fn progress_style_for_percentage(&self, percentage: f64) -> Style {
        if percentage >= 90.0 {
            self.progress_danger_style()
        } else if percentage >= 70.0 {
            self.progress_warning_style()
        } else {
            self.progress_success_style()
        }
    }

    /// Get priority style based on priority level
    pub fn priority_style(&self, priority: &str) -> Style {
        match priority.to_lowercase().as_str() {
            "high" => Style::default()
                .fg(self.danger)
                .add_modifier(Modifier::BOLD),
            "medium" => Style::default()
                .fg(self.warning)
                .add_modifier(Modifier::BOLD),
            "low" => Style::default()
                .fg(self.success)
                .add_modifier(Modifier::BOLD),
            _ => self.secondary_text_style(),
        }
    }
}

/// Helper function to create gradient-like effects with available colors
#[allow(dead_code)]
pub fn create_gradient_style(start_color: Color, end_color: Color, position: f64) -> Style {
    // Simple implementation: just interpolate between two colors
    // For more complex gradients, we'd need to implement color interpolation
    if position < 0.5 {
        Style::default().fg(start_color)
    } else {
        Style::default().fg(end_color)
    }
}

/// Card-style border characters
#[allow(dead_code)]
pub struct CardBorders;

#[allow(dead_code)]
impl CardBorders {
    // Rounded corner style borders
    pub const TOP_LEFT: &'static str = "╭";
    pub const TOP_RIGHT: &'static str = "╮";
    pub const BOTTOM_LEFT: &'static str = "╰";
    pub const BOTTOM_RIGHT: &'static str = "╯";
    pub const HORIZONTAL: &'static str = "─";
    pub const VERTICAL: &'static str = "│";

    // Double line borders for emphasis
    pub const DOUBLE_HORIZONTAL: &'static str = "═";
    pub const DOUBLE_VERTICAL: &'static str = "║";
    pub const DOUBLE_TOP_LEFT: &'static str = "╔";
    pub const DOUBLE_TOP_RIGHT: &'static str = "╗";
    pub const DOUBLE_BOTTOM_LEFT: &'static str = "╚";
    pub const DOUBLE_BOTTOM_RIGHT: &'static str = "╝";
}

/// Animation-related constants for pseudo-animations
#[allow(dead_code)]
pub struct AnimationFrames;

#[allow(dead_code)]
impl AnimationFrames {
    // Rotating/loading indicators
    pub const SPINNER: [&'static str; 4] = ["⠋", "⠙", "⠹", "⠸"];
    pub const DOTS: [&'static str; 3] = ["⠇", "⠏", "⠟"];
    pub const ROTATE: [&'static str; 8] = ["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"];
}
