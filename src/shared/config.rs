use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Theme options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    Ocean,
    Forest,
    Sunset,
    Galaxy,
    Auto,
}

/// Language options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum Language {
    Korean,
    #[default]
    English,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Refresh interval in seconds (minimum 2 seconds, default 5)
    pub refresh_interval_secs: u64,
    /// Current selected tab index
    pub current_tab: usize,
    /// Theme mode selection
    pub theme_mode: ThemeMode,
    /// Language selection
    pub language: Language,
    /// Show help overlay
    pub show_help: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 5, // Default to 5 seconds
            current_tab: 0,
            theme_mode: ThemeMode::default(),
            language: Language::default(),
            show_help: false,
        }
    }
}

impl Config {
    /// Load configuration from file, creating default if not found
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        // Try migration from old location first
        if !config_path.exists() {
            if let Ok(old_config) = Self::try_migrate_old_config() {
                return Ok(old_config);
            }
        }

        // Load existing config or create default
        let config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| {
                // If parsing fails, use default and save it
                let default_config = Config::default();
                let _ = default_config.save();
                default_config
            })
        } else {
            // Create and save default config
            let default_config = Config::default();
            let _ = default_config.save();
            default_config
        };

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the configuration file path
    fn config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

        // Use XDG config directory standard or fallback to ~/.config
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config)
        } else {
            home_dir.join(".config")
        };

        let app_config_dir = config_dir.join("cc-enhanced");

        // Create config directory if it doesn't exist
        fs::create_dir_all(&app_config_dir)?;

        Ok(app_config_dir.join("config.json"))
    }

    /// Try to migrate configuration from old location
    fn try_migrate_old_config() -> Result<Self> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let old_config_path = home_dir
            .join(".claude")
            .join("cc-enhanced-config.json");

        if old_config_path.exists() {
            let content = fs::read_to_string(&old_config_path)?;
            let old_config: Config = serde_json::from_str(&content)?;

            // Save the migrated config to the new location
            old_config.save()?;

            println!(
                "Migrated configuration from {} to {}",
                old_config_path.display(),
                Self::config_path()?.display()
            );

            Ok(old_config)
        } else {
            Err(anyhow::anyhow!("No old config to migrate"))
        }
    }

    /// Set refresh interval with validation (minimum 2 seconds)
    pub fn set_refresh_interval(&mut self, seconds: u64) {
        if seconds >= 2 {
            self.refresh_interval_secs = seconds;
        }
    }

    /// Get refresh interval as Duration
    pub fn refresh_interval(&self) -> Duration {
        Duration::from_secs(self.refresh_interval_secs)
    }

    /// Set current tab
    pub fn set_current_tab(&mut self, tab_index: usize) {
        self.current_tab = tab_index;
    }

    /// Set theme mode
    pub fn set_theme_mode(&mut self, theme_mode: ThemeMode) {
        self.theme_mode = theme_mode;
    }

    /// Toggle help overlay
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    /// Get refresh interval display string
    pub fn refresh_interval_display(&self) -> String {
        match self.refresh_interval_secs {
            2 => "2s".to_string(),
            5 => "5s".to_string(),
            10 => "10s".to_string(),
            30 => "30s".to_string(),
            60 => "1m".to_string(),
            n => format!("{n}s"),
        }
    }

    /// Get theme display string
    pub fn theme_display(&self) -> &str {
        match self.theme_mode {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
            ThemeMode::Ocean => "Ocean",
            ThemeMode::Forest => "Forest",
            ThemeMode::Sunset => "Sunset",
            ThemeMode::Galaxy => "Galaxy",
            ThemeMode::Auto => "Auto",
        }
    }
}

/// Get refresh interval from key press
pub fn get_refresh_interval_from_key(key: char) -> Option<u64> {
    match key {
        '1' => Some(2),
        '2' => Some(5),
        '3' => Some(10),
        '4' => Some(30),
        '5' => Some(60),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[allow(dead_code)] // Helper function for potential future tests
    fn create_temp_config_dir() -> PathBuf {
        let temp_dir = std::env::temp_dir().join(format!("config_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();
        temp_dir
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.refresh_interval_secs, 5);
        assert_eq!(config.current_tab, 0);
        assert_eq!(config.theme_mode, ThemeMode::Dark);
        assert_eq!(config.language, Language::English);
        assert!(!config.show_help);
    }

    #[test]
    fn test_theme_mode_serialization() {
        let themes = vec![
            ThemeMode::Dark,
            ThemeMode::Light,
            ThemeMode::Ocean,
            ThemeMode::Forest,
            ThemeMode::Sunset,
            ThemeMode::Galaxy,
            ThemeMode::Auto,
        ];

        for theme in themes {
            let serialized = serde_json::to_string(&theme).unwrap();
            let deserialized: ThemeMode = serde_json::from_str(&serialized).unwrap();
            assert_eq!(theme, deserialized);
        }
    }

    #[test]
    fn test_language_serialization() {
        let languages = vec![Language::Korean, Language::English];

        for lang in languages {
            let serialized = serde_json::to_string(&lang).unwrap();
            let deserialized: Language = serde_json::from_str(&serialized).unwrap();
            assert_eq!(lang, deserialized);
        }
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            refresh_interval_secs: 10,
            current_tab: 2,
            theme_mode: ThemeMode::Ocean,
            language: Language::Korean,
            show_help: true,
        };

        let serialized = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            config.refresh_interval_secs,
            deserialized.refresh_interval_secs
        );
        assert_eq!(config.current_tab, deserialized.current_tab);
        assert_eq!(config.theme_mode, deserialized.theme_mode);
        assert_eq!(config.language, deserialized.language);
        assert_eq!(config.show_help, deserialized.show_help);
    }

    #[test]
    fn test_refresh_interval_validation() {
        let mut config = Config::default();

        // Valid intervals
        config.set_refresh_interval(2);
        assert_eq!(config.refresh_interval_secs, 2);

        config.set_refresh_interval(60);
        assert_eq!(config.refresh_interval_secs, 60);

        // Invalid interval (too small) should be rejected
        config.set_refresh_interval(1);
        assert_eq!(config.refresh_interval_secs, 60); // Should remain unchanged

        config.set_refresh_interval(0);
        assert_eq!(config.refresh_interval_secs, 60); // Should remain unchanged
    }

    #[test]
    fn test_refresh_interval_display() {
        let mut config = Config::default();

        config.set_refresh_interval(2);
        assert_eq!(config.refresh_interval_display(), "2s");

        config.set_refresh_interval(5);
        assert_eq!(config.refresh_interval_display(), "5s");

        config.set_refresh_interval(10);
        assert_eq!(config.refresh_interval_display(), "10s");

        config.set_refresh_interval(30);
        assert_eq!(config.refresh_interval_display(), "30s");

        config.set_refresh_interval(60);
        assert_eq!(config.refresh_interval_display(), "1m");

        config.set_refresh_interval(90);
        assert_eq!(config.refresh_interval_display(), "90s");
    }

    #[test]
    fn test_theme_display() {
        let themes_and_displays = vec![
            (ThemeMode::Dark, "Dark"),
            (ThemeMode::Light, "Light"),
            (ThemeMode::Ocean, "Ocean"),
            (ThemeMode::Forest, "Forest"),
            (ThemeMode::Sunset, "Sunset"),
            (ThemeMode::Galaxy, "Galaxy"),
            (ThemeMode::Auto, "Auto"),
        ];

        for (theme, expected_display) in themes_and_displays {
            let config = Config {
                theme_mode: theme,
                ..Default::default()
            };
            assert_eq!(config.theme_display(), expected_display);
        }
    }

    #[test]
    fn test_help_toggle() {
        let mut config = Config::default();
        assert!(!config.show_help);

        config.toggle_help();
        assert!(config.show_help);

        config.toggle_help();
        assert!(!config.show_help);
    }

    #[test]
    fn test_get_refresh_interval_from_key() {
        assert_eq!(get_refresh_interval_from_key('1'), Some(2));
        assert_eq!(get_refresh_interval_from_key('2'), Some(5));
        assert_eq!(get_refresh_interval_from_key('3'), Some(10));
        assert_eq!(get_refresh_interval_from_key('4'), Some(30));
        assert_eq!(get_refresh_interval_from_key('5'), Some(60));
        assert_eq!(get_refresh_interval_from_key('6'), None);
        assert_eq!(get_refresh_interval_from_key('a'), None);
    }

    #[test]
    fn test_tab_and_theme_setters() {
        let mut config = Config::default();

        // Test tab setting
        config.set_current_tab(3);
        assert_eq!(config.current_tab, 3);

        // Test theme setting
        config.set_theme_mode(ThemeMode::Galaxy);
        assert_eq!(config.theme_mode, ThemeMode::Galaxy);
    }
}
