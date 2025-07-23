use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::io;

use crate::{claude, features, ide, shared, ui};
use features::todos::{ProjectTodoStats, SessionTodos, TodoItem, TodoManager};
use shared::{Config, I18n, ModernTheme, ThemeMode};
use tokio::sync::mpsc;

/// Available tabs in the application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tab {
    Overview = 0,
    Usage = 1,
    Sessions = 2,
    Todos = 3,
    Quota = 4,
}

/// Application view modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    ProjectView,     // Normal project-focused view
    GlobalDashboard, // Full-screen global analytics
}

/// IDE selection popup state
#[derive(Debug, Clone)]
pub struct IdeSelectionState {
    pub available_ides: Vec<(ide::IdeType, String)>,
    pub selected_index: usize,
    pub project_path: std::path::PathBuf,
}

/// Background data loading message
#[derive(Debug)]
#[allow(dead_code)] // Allow unused variants during migration
pub enum DataLoadingMessage {
    ProjectAnalytics(String, claude::ProjectAnalytics),
    ProjectTodoStats(String, ProjectTodoStats),
    RefreshComplete(RefreshResult),
}

/// Result of a background refresh operation
#[derive(Debug)]
#[allow(dead_code)] // Allow unused fields during migration
pub struct RefreshResult {
    pub projects: Vec<claude::Project>,
    pub project_todos: HashMap<String, Vec<SessionTodos>>,
    pub usage_stats: claude::UsageStats,
    pub reset_time_str: String,
    pub selected_project_name: Option<String>,
}

/// Background data loading request
#[derive(Debug, Clone)]
#[allow(dead_code)] // Allow unused fields during migration
pub struct DataLoadingRequest {
    pub project_name: String,
    pub project_path: std::path::PathBuf,
}

/// Loading states for different operations
#[derive(Debug)]
pub struct LoadingStates {
    pub project_switching: bool,
    pub data_refresh: bool,
    pub analytics_loading: HashMap<String, bool>,
    pub spinner_frame: usize,
    pub last_spinner_update: std::time::Instant,
}

/// Status message for user feedback
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub timestamp: std::time::Instant,
    pub message_type: StatusType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusType {
    Info,
    Success,
    Warning,
    Error,
}

impl LoadingStates {
    pub fn new() -> Self {
        Self {
            project_switching: false,
            data_refresh: false,
            analytics_loading: HashMap::new(),
            spinner_frame: 0,
            last_spinner_update: std::time::Instant::now(),
        }
    }

    pub fn get_spinner_char(&mut self) -> char {
        const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

        // Update spinner every 100ms
        if self.last_spinner_update.elapsed().as_millis() > 100 {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_CHARS.len();
            self.last_spinner_update = std::time::Instant::now();
        }

        SPINNER_CHARS[self.spinner_frame]
    }

    pub fn is_loading(&self) -> bool {
        self.project_switching || self.data_refresh || !self.analytics_loading.is_empty()
    }
}

impl Tab {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Tab::Overview,
            1 => Tab::Usage,
            2 => Tab::Sessions,
            3 => Tab::Todos,
            4 => Tab::Quota,
            _ => Tab::Overview,
        }
    }

    #[allow(dead_code)] // Keep for potential future use
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::Usage => "Usage",
            Tab::Sessions => "Sessions",
            Tab::Todos => "Todos",
            Tab::Quota => "Quota",
        }
    }

    pub fn count() -> usize {
        5
    }
}

/// Main application state
pub struct App {
    /// Flag to indicate if the app should quit
    pub should_quit: bool,
    /// Claude data manager
    pub claude_manager: claude::ClaudeDataManager,
    /// Todo data manager
    pub todo_manager: TodoManager,
    /// Application configuration
    pub config: Config,
    /// Application theme
    pub theme: ModernTheme,
    /// Internationalization
    pub i18n: I18n,
    /// List of projects
    pub projects: Vec<claude::Project>,
    /// Currently selected project index
    pub selected_project: usize,
    /// Today's usage statistics
    pub usage_stats: claude::UsageStats,
    /// Time until next quota reset
    pub reset_time_str: String,
    /// Last refresh time for background updates
    last_refresh: std::time::Instant,
    /// Current active tab
    pub current_tab: Tab,
    /// Todo data organized by project with caching
    pub project_todos: HashMap<String, Vec<SessionTodos>>,
    /// Cached todo stats per project
    cached_todo_stats: HashMap<String, (ProjectTodoStats, std::time::Instant)>,
    /// Next refresh countdown (for display)
    pub next_refresh_in: std::time::Duration,
    /// Cached project analytics per project to avoid expensive recalculation
    cached_analytics: HashMap<String, (claude::ProjectAnalytics, std::time::Instant)>,
    /// Cached daily usage data  
    cached_daily_usage: Option<Vec<claude::DailyUsage>>,
    /// Cache timestamp for invalidation
    last_cache_update: std::time::Instant,
    /// Tab-specific rendering cache to avoid expensive recalculations
    tab_render_cache: HashMap<(Tab, String), (String, std::time::Instant)>,
    /// Current view mode (project view or global dashboard)
    pub view_mode: ViewMode,
    /// Cached global analytics to avoid expensive recalculation
    cached_global_analytics: Option<(claude::UsageAnalytics, std::time::Instant)>,
    /// IDE selection state
    pub ide_selection_state: Option<IdeSelectionState>,
    /// Last selected project index to detect changes
    last_selected_project: usize,
    /// Flag to indicate if UI needs redraw
    needs_redraw: bool,
    /// Loading states for various operations
    pub loading_states: LoadingStates,
    /// Background data loading channels
    #[allow(dead_code)] // Unused during migration
    data_loader_tx: Option<mpsc::UnboundedSender<DataLoadingRequest>>,
    data_loader_rx: mpsc::UnboundedReceiver<DataLoadingMessage>,
    /// Current status message
    pub status_message: Option<StatusMessage>,
    /// Background refresh task handle
    refresh_tx: Option<mpsc::UnboundedSender<()>>,
    /// Flag to track if background refresh is in progress
    pub background_refresh_in_progress: bool,
}

impl App {
    /// Create a new App instance
    pub async fn new() -> Result<Self> {
        let mut claude_manager = claude::ClaudeDataManager::new()?;

        // Update OpenRouter pricing cache in background if needed
        if let Err(e) = claude_manager.update_pricing_cache_if_needed().await {
            eprintln!("Warning: Failed to update pricing cache: {e}");
            // Continue with existing cache or fallback pricing
        }
        let todo_manager = TodoManager::new()?;
        let config = Config::load()?;
        let theme = match config.theme_mode {
            ThemeMode::Dark => ModernTheme::dark(),
            ThemeMode::Light => ModernTheme::light(),
            ThemeMode::Ocean => ModernTheme::ocean(),
            ThemeMode::Forest => ModernTheme::forest(),
            ThemeMode::Sunset => ModernTheme::sunset(),
            ThemeMode::Galaxy => ModernTheme::galaxy(),
            ThemeMode::Auto => {
                // For now, default to dark theme for Auto mode
                // In the future, this could detect system theme
                ModernTheme::dark()
            }
        };
        let i18n = I18n::new(config.language.clone());

        let projects = claude_manager.scan_projects()?;
        let usage_stats = claude_manager.calculate_today_usage()?;
        let reset_time_str = claude_manager.time_until_reset();
        let project_todos = todo_manager.scan_todos()?;

        let current_tab = Tab::from_index(config.current_tab);

        // Setup background data loading channels
        let (data_tx, data_rx) = mpsc::unbounded_channel::<DataLoadingMessage>();
        let (req_tx, _req_rx) = mpsc::unbounded_channel::<DataLoadingRequest>();

        // Setup background refresh system
        let (refresh_tx, refresh_rx) = mpsc::unbounded_channel::<()>();

        let mut app = Self {
            should_quit: false,
            claude_manager,
            todo_manager,
            config,
            theme,
            i18n,
            projects,
            selected_project: 0,
            usage_stats,
            reset_time_str,
            last_refresh: std::time::Instant::now(),
            current_tab,
            project_todos,
            next_refresh_in: std::time::Duration::from_secs(0),
            cached_analytics: HashMap::new(),
            cached_todo_stats: HashMap::new(),
            cached_daily_usage: None,
            last_cache_update: std::time::Instant::now(),
            tab_render_cache: HashMap::new(),
            view_mode: ViewMode::ProjectView,
            cached_global_analytics: None,
            ide_selection_state: None,
            last_selected_project: 0,
            needs_redraw: true,
            loading_states: LoadingStates::new(),
            data_loader_tx: Some(req_tx),
            data_loader_rx: data_rx,
            status_message: None,
            refresh_tx: Some(refresh_tx),
            background_refresh_in_progress: false,
        };

        // Spawn background refresh task
        app.spawn_background_refresh_task(refresh_rx, data_tx).await;

        // Preload data for initial project and nearby ones
        app.request_background_loading();

        Ok(app)
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        if !IsTty::is_tty(&io::stdout()) {
            eprintln!("This application requires a TTY terminal to run.");
            return Ok(());
        }

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        while !self.should_quit {
            // Update spinner animation if loading
            if self.loading_states.is_loading() {
                self.loading_states.get_spinner_char(); // This updates internal state
                self.needs_redraw = true;
            }

            // Process background data loading messages
            while let Ok(message) = self.data_loader_rx.try_recv() {
                match message {
                    DataLoadingMessage::ProjectAnalytics(project_name, analytics) => {
                        self.cached_analytics
                            .insert(project_name.clone(), (analytics, std::time::Instant::now()));
                        self.loading_states.analytics_loading.remove(&project_name);
                        self.needs_redraw = true;
                    }
                    DataLoadingMessage::ProjectTodoStats(project_name, stats) => {
                        self.cached_todo_stats
                            .insert(project_name.clone(), (stats, std::time::Instant::now()));
                        self.loading_states.analytics_loading.remove(&project_name);
                        self.needs_redraw = true;
                    }
                    DataLoadingMessage::RefreshComplete(result) => {
                        self.apply_refresh_result(result);
                        self.background_refresh_in_progress = false;
                        self.loading_states.data_refresh = false;
                        self.needs_redraw = true;
                    }
                }
            }

            // Check if it's time to refresh (non-blocking)
            let refresh_interval = self.config.refresh_interval();
            if self.last_refresh.elapsed() >= refresh_interval {
                self.trigger_background_refresh();
            }

            // Calculate time until next refresh
            self.next_refresh_in = refresh_interval.saturating_sub(self.last_refresh.elapsed());

            // Update status message (auto-clear after 2 seconds)
            self.update_status_message(std::time::Duration::from_secs(2));

            // Only redraw if something changed
            if self.needs_redraw {
                terminal.draw(|f| ui::draw(f, self))?;
                self.needs_redraw = false;
            }

            if event::poll(std::time::Duration::from_millis(16))? {
                // ~60fps instead of 10fps
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key.code, key.modifiers)?;
                    self.needs_redraw = true; // Redraw after user input
                }
            }
        }

        self.cleanup().await?;

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;

        Ok(())
    }

    /// Handle keyboard input
    fn handle_key_event(&mut self, key: KeyCode, _modifiers: KeyModifiers) -> Result<()> {
        // Handle IDE selection popup first
        if self.ide_selection_state.is_some() {
            match key {
                KeyCode::Enter => {
                    self.launch_selected_ide()?;
                }
                KeyCode::Esc => {
                    self.close_ide_selection();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.move_ide_selection_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.move_ide_selection_down();
                }
                _ => {}
            }
            return Ok(());
        }

        // Handle help overlay second
        if self.config.show_help {
            match key {
                KeyCode::Char('?') | KeyCode::Char('/') | KeyCode::Esc => {
                    self.config.toggle_help();
                    let _ = self.config.save(); // Save config after change
                }
                _ => {}
            }
            return Ok(());
        }

        // Normal key handling when help is not shown
        match key {
            KeyCode::Esc => {
                // ESC behavior depends on current mode
                match self.view_mode {
                    ViewMode::GlobalDashboard => {
                        // In global view, ESC returns to project view
                        self.view_mode = ViewMode::ProjectView;
                    }
                    ViewMode::ProjectView => {
                        // In project view, ESC quits the application
                        self.should_quit = true;
                    }
                }
            }
            KeyCode::Char('q') | KeyCode::Char('ㅂ') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Char('ㅓ') | KeyCode::Down => self.move_selection_down(),
            KeyCode::Char('k') | KeyCode::Char('ㅏ') | KeyCode::Up => self.move_selection_up(),
            KeyCode::Tab => self.next_tab()?,
            KeyCode::BackTab => self.prev_tab()?,
            KeyCode::Char('r') | KeyCode::Char('ㄱ') => {
                self.trigger_background_refresh();
            }
            KeyCode::Char('1')
            | KeyCode::Char('2')
            | KeyCode::Char('3')
            | KeyCode::Char('4')
            | KeyCode::Char('5') => {
                self.handle_refresh_interval_key(key)?;
            }
            KeyCode::Char('t') | KeyCode::Char('ㅅ') => self.toggle_theme()?,
            KeyCode::Char('g') | KeyCode::Char('ㅎ') => self.toggle_global_dashboard()?,
            KeyCode::Char('o') | KeyCode::Char('ㅗ') => self.show_ide_selection()?,
            KeyCode::Char('?') | KeyCode::Char('/') => self.toggle_help()?,
            _ => {}
        }
        Ok(())
    }

    /// Handle refresh interval key presses
    fn handle_refresh_interval_key(&mut self, key: KeyCode) -> Result<()> {
        if let KeyCode::Char(c) = key {
            if let Some(interval) = shared::config::get_refresh_interval_from_key(c) {
                self.config.set_refresh_interval(interval);
                let _ = self.config.save(); // Save config after change
                                            // Reset refresh timer
                self.last_refresh = std::time::Instant::now();
            }
        }
        Ok(())
    }

    /// Move to next tab
    fn next_tab(&mut self) -> Result<()> {
        let next_index = (self.current_tab as usize + 1) % Tab::count();
        self.current_tab = Tab::from_index(next_index);
        self.config.set_current_tab(next_index);
        let _ = self.config.save(); // Save config after change

        // Show status feedback
        let tab_name = match self.current_tab {
            Tab::Overview => "Overview",
            Tab::Usage => "Usage",
            Tab::Sessions => "Sessions",
            Tab::Todos => "Todos",
            Tab::Quota => "Quota",
        };
        self.show_status(&format!("Switched to {tab_name} tab"), StatusType::Info);

        Ok(())
    }

    /// Move to previous tab
    fn prev_tab(&mut self) -> Result<()> {
        let prev_index = if self.current_tab as usize == 0 {
            Tab::count() - 1
        } else {
            self.current_tab as usize - 1
        };
        self.current_tab = Tab::from_index(prev_index);
        self.config.set_current_tab(prev_index);
        let _ = self.config.save(); // Save config after change

        // Show status feedback
        let tab_name = match self.current_tab {
            Tab::Overview => "Overview",
            Tab::Usage => "Usage",
            Tab::Sessions => "Sessions",
            Tab::Todos => "Todos",
            Tab::Quota => "Quota",
        };
        self.show_status(&format!("Switched to {tab_name} tab"), StatusType::Info);

        Ok(())
    }

    /// Move selection down in the project list
    fn move_selection_down(&mut self) {
        if !self.projects.is_empty() {
            let old_selection = self.selected_project;
            self.selected_project = (self.selected_project + 1) % self.projects.len();

            // Clear tab render cache only if selection actually changed
            if old_selection != self.selected_project {
                self.loading_states.project_switching = true;
                self.needs_redraw = true; // Immediately show loading state

                self.clear_tab_render_cache();
                self.last_selected_project = self.selected_project;
                self.request_background_loading();

                self.loading_states.project_switching = false;
                self.needs_redraw = true; // Update after loading
            }
        }
    }

    /// Move selection up in the project list
    fn move_selection_up(&mut self) {
        if !self.projects.is_empty() {
            let old_selection = self.selected_project;
            self.selected_project = if self.selected_project == 0 {
                self.projects.len() - 1
            } else {
                self.selected_project - 1
            };

            // Clear tab render cache only if selection actually changed
            if old_selection != self.selected_project {
                self.loading_states.project_switching = true;
                self.needs_redraw = true; // Immediately show loading state

                self.clear_tab_render_cache();
                self.last_selected_project = self.selected_project;
                self.request_background_loading();

                self.loading_states.project_switching = false;
                self.needs_redraw = true; // Update after loading
            }
        }
    }

    /// Clear tab render cache to force refresh for new project
    fn clear_tab_render_cache(&mut self) {
        // Only keep cache entries that are still recent (within 1 minute)
        let cutoff = std::time::Instant::now() - std::time::Duration::from_secs(60);
        self.tab_render_cache
            .retain(|_, (_, timestamp)| *timestamp > cutoff);
    }

    /// Pre-cache data for frequently accessed projects
    pub fn preload_project_data(&mut self, project_indices: &[usize]) {
        for &index in project_indices {
            if let Some(project) = self.projects.get(index) {
                // Pre-load analytics if not cached
                if !self.cached_analytics.contains_key(&project.name) {
                    if let Ok(analytics) = self.claude_manager.calculate_project_analytics(project)
                    {
                        self.cached_analytics
                            .insert(project.name.clone(), (analytics, std::time::Instant::now()));
                    }
                }
            }
        }
    }

    /// Request background loading for current project
    fn request_background_loading(&mut self) {
        // Preload current and adjacent projects for faster switching
        let current = self.selected_project;
        let mut indices_to_load = vec![current];

        // Add adjacent projects (up to 2 before and 2 after)
        for offset in 1..=2 {
            if current >= offset {
                indices_to_load.push(current - offset);
            }
            if current + offset < self.projects.len() {
                indices_to_load.push(current + offset);
            }
        }

        self.preload_project_data(&indices_to_load);
    }

    /// Get the currently selected project
    pub fn selected_project(&self) -> Option<&claude::Project> {
        self.projects.get(self.selected_project)
    }

    /// Get todo statistics for the currently selected project
    pub fn selected_project_todo_stats(&self) -> Option<ProjectTodoStats> {
        if let Some(project) = self.selected_project() {
            let project_path = project.path.to_string_lossy().to_string();

            // Try exact match first
            if let Some(project_todos) = self.project_todos.get(&project_path) {
                return Some(self.todo_manager.calculate_project_stats(project_todos));
            }

            // Try to find a match by checking all keys
            for (key, project_todos) in &self.project_todos {
                if key == &project_path || key.ends_with(&project.name) {
                    return Some(self.todo_manager.calculate_project_stats(project_todos));
                }
            }
        }
        None
    }

    /// Get todos for the currently selected project
    pub fn selected_project_todos(&self) -> Vec<(String, TodoItem)> {
        if let Some(project) = self.selected_project() {
            let project_path = project.path.to_string_lossy().to_string();

            // Try exact match first
            if let Some(project_todos) = self.project_todos.get(&project_path) {
                return self.todo_manager.get_project_todos_sorted(project_todos);
            }

            // Try to find a match by checking all keys
            for (key, project_todos) in &self.project_todos {
                if key == &project_path || key.ends_with(&project.name) {
                    return self.todo_manager.get_project_todos_sorted(project_todos);
                }
            }
        }
        Vec::new()
    }

    /// Toggle theme mode
    fn toggle_theme(&mut self) -> Result<()> {
        let new_theme_mode = match self.config.theme_mode {
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Ocean,
            ThemeMode::Ocean => ThemeMode::Forest,
            ThemeMode::Forest => ThemeMode::Sunset,
            ThemeMode::Sunset => ThemeMode::Galaxy,
            ThemeMode::Galaxy => ThemeMode::Auto,
            ThemeMode::Auto => ThemeMode::Dark,
        };

        // Show theme change status first (before moving the value)
        let theme_name = match new_theme_mode {
            ThemeMode::Dark => "Dark",
            ThemeMode::Light => "Light",
            ThemeMode::Ocean => "Ocean",
            ThemeMode::Forest => "Forest",
            ThemeMode::Sunset => "Sunset",
            ThemeMode::Galaxy => "Galaxy",
            ThemeMode::Auto => "Auto",
        };

        self.config.set_theme_mode(new_theme_mode);
        let _ = self.config.save(); // Save config after change

        // Update the theme instance
        self.theme = match self.config.theme_mode {
            ThemeMode::Dark => ModernTheme::dark(),
            ThemeMode::Light => ModernTheme::light(),
            ThemeMode::Ocean => ModernTheme::ocean(),
            ThemeMode::Forest => ModernTheme::forest(),
            ThemeMode::Sunset => ModernTheme::sunset(),
            ThemeMode::Galaxy => ModernTheme::galaxy(),
            ThemeMode::Auto => ModernTheme::dark(),
        };

        self.show_status(
            &format!("Changed theme to {theme_name}"),
            StatusType::Success,
        );

        Ok(())
    }

    /// Toggle help overlay
    fn toggle_help(&mut self) -> Result<()> {
        self.config.toggle_help();
        let _ = self.config.save(); // Save config after change
        Ok(())
    }

    fn show_ide_selection(&mut self) -> Result<()> {
        if let Some(project) = self.projects.get(self.selected_project) {
            // Check if the project path exists
            if !project.path.exists() {
                self.show_status("Project path not found", StatusType::Warning);
                return Ok(());
            }

            // Clone the path before borrowing self mutably
            let project_path = project.path.clone();

            // Show detecting IDEs status
            self.show_status("Detecting available IDEs...", StatusType::Info);
            let available_ides = ide::get_available_ides_for_project(&project_path);

            if available_ides.is_empty() {
                self.show_status("No compatible IDEs found", StatusType::Warning);
                return Ok(());
            }

            self.ide_selection_state = Some(IdeSelectionState {
                available_ides,
                selected_index: 0,
                project_path,
            });
        }
        Ok(())
    }

    fn launch_selected_ide(&mut self) -> Result<()> {
        if let Some(ref state) = self.ide_selection_state {
            if let Some((ide_type, command)) = state.available_ides.get(state.selected_index) {
                // Clone the necessary data before borrowing self mutably
                let ide_name = ide_type.display_name();
                let project_path = state.project_path.clone();
                let command = command.clone();

                self.show_status(&format!("Launching {ide_name}..."), StatusType::Info);

                match ide::launch_ide_with_command(&project_path, &command) {
                    Ok(()) => {
                        // IDE launched successfully, close the selection menu
                        self.show_status(
                            &format!("{ide_name} launched successfully"),
                            StatusType::Success,
                        );
                        self.ide_selection_state = None;
                    }
                    Err(e) => {
                        self.show_status(
                            &format!("Failed to launch {ide_name}: {e}"),
                            StatusType::Error,
                        );
                        self.ide_selection_state = None;
                    }
                }
            }
        }
        Ok(())
    }

    fn close_ide_selection(&mut self) {
        self.ide_selection_state = None;
    }

    fn move_ide_selection_up(&mut self) {
        if let Some(ref mut state) = self.ide_selection_state {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            } else {
                state.selected_index = state.available_ides.len().saturating_sub(1);
            }
        }
    }

    fn move_ide_selection_down(&mut self) {
        if let Some(ref mut state) = self.ide_selection_state {
            state.selected_index = (state.selected_index + 1) % state.available_ides.len();
        }
    }

    /// Get current theme instance
    pub fn current_theme(&self) -> &ModernTheme {
        &self.theme
    }

    /// Get cached project analytics or calculate if needed
    pub fn get_project_analytics(
        &mut self,
        project: &claude::Project,
    ) -> Option<claude::ProjectAnalytics> {
        let project_name = &project.name;
        let cache_ttl_seconds = 300; // 5 minutes cache per project

        // Check if we have valid cached data for this project
        if let Some((analytics, timestamp)) = self.cached_analytics.get(project_name) {
            if timestamp.elapsed().as_secs() < cache_ttl_seconds {
                return Some(analytics.clone());
            }
        }

        // Calculate new analytics and cache it
        if let Ok(analytics) = self.claude_manager.calculate_project_analytics(project) {
            self.cached_analytics.insert(
                project_name.clone(),
                (analytics.clone(), std::time::Instant::now()),
            );
            Some(analytics)
        } else {
            None
        }
    }

    /// Get cached daily usage or calculate if needed
    #[allow(dead_code)] // Keep for potential future use
    pub fn get_daily_usage(&mut self, days: usize) -> Vec<claude::DailyUsage> {
        // Check if we have valid cached data (longer TTL for tab switching)
        if let Some(ref usage) = self.cached_daily_usage {
            if self.last_cache_update.elapsed().as_secs() < 180 {
                // Cache for 3 minutes
                return usage.clone();
            }
        }

        // Calculate new usage and cache it
        if let Ok(usage) = self.claude_manager.calculate_daily_usage(days as u32) {
            self.cached_daily_usage = Some(usage.clone());
            usage
        } else {
            Vec::new()
        }
    }

    /// Toggle between project view and global dashboard
    fn toggle_global_dashboard(&mut self) -> Result<()> {
        self.view_mode = match self.view_mode {
            ViewMode::ProjectView => ViewMode::GlobalDashboard,
            ViewMode::GlobalDashboard => ViewMode::ProjectView,
        };
        Ok(())
    }

    /// Get cached global analytics or calculate if needed
    pub fn get_global_analytics(&mut self) -> Result<&claude::UsageAnalytics> {
        const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(30); // 30 seconds cache

        let should_refresh = self
            .cached_global_analytics
            .as_ref()
            .map(|(_, timestamp)| timestamp.elapsed() > CACHE_TTL)
            .unwrap_or(true);

        if should_refresh {
            // Calculate fresh analytics
            let analytics = self.claude_manager.generate_comprehensive_analytics()?;
            self.cached_global_analytics = Some((analytics, std::time::Instant::now()));
        }

        Ok(&self.cached_global_analytics.as_ref().unwrap().0)
    }

    /// Show a status message to the user
    pub fn show_status(&mut self, text: &str, status_type: StatusType) {
        self.status_message = Some(StatusMessage {
            text: text.to_string(),
            timestamp: std::time::Instant::now(),
            message_type: status_type,
        });
        self.needs_redraw = true;
    }

    /// Clear status message if it's older than the specified duration
    pub fn update_status_message(&mut self, max_age: std::time::Duration) {
        if let Some(ref msg) = self.status_message {
            if msg.timestamp.elapsed() > max_age {
                self.status_message = None;
                self.needs_redraw = true;
            }
        }
    }

    /// Spawn background refresh task
    async fn spawn_background_refresh_task(
        &self,
        mut refresh_rx: mpsc::UnboundedReceiver<()>,
        data_tx: mpsc::UnboundedSender<DataLoadingMessage>,
    ) {
        // Clone the paths we need for the background task
        let claude_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".claude");

        tokio::spawn(async move {
            while (refresh_rx.recv().await).is_some() {
                // Perform refresh in background
                if let Ok(result) = Self::perform_background_refresh(&claude_dir).await {
                    let _ = data_tx.send(DataLoadingMessage::RefreshComplete(result));
                }
            }
        });
    }

    /// Perform background refresh without blocking UI
    async fn perform_background_refresh(_claude_dir: &std::path::Path) -> Result<RefreshResult> {
        // Create new managers for background task
        let mut claude_manager = claude::ClaudeDataManager::new()?;
        let todo_manager = features::todos::TodoManager::new()?;

        // Update pricing cache if needed
        let _ = claude_manager.update_pricing_cache_if_needed().await;

        // Perform refresh operations
        let usage_stats = claude_manager.calculate_today_usage()?;
        let reset_time_str = claude_manager.time_until_reset();
        let projects = claude_manager.scan_projects()?;
        let project_todos = todo_manager.scan_todos()?;

        Ok(RefreshResult {
            projects,
            project_todos,
            usage_stats,
            reset_time_str,
            selected_project_name: None,
        })
    }

    /// Trigger background refresh
    pub fn trigger_background_refresh(&mut self) {
        if !self.background_refresh_in_progress {
            if let Some(ref tx) = self.refresh_tx {
                if tx.send(()).is_ok() {
                    self.background_refresh_in_progress = true;
                    self.loading_states.data_refresh = true;
                    // Background refresh started silently
                }
            }
        }
    }

    /// Apply background refresh result to the application state
    fn apply_refresh_result(&mut self, result: RefreshResult) {
        // Store current selected project name for preservation
        let old_selected_name =
            if !self.projects.is_empty() && self.selected_project < self.projects.len() {
                Some(self.projects[self.selected_project].name.clone())
            } else {
                None
            };

        // Apply the refresh result
        self.projects = result.projects;
        self.project_todos = result.project_todos;
        self.usage_stats = result.usage_stats;
        self.reset_time_str = result.reset_time_str;

        // Restore selected project
        self.selected_project = if let Some(old_name) = old_selected_name {
            self.projects
                .iter()
                .position(|p| p.name == old_name)
                .unwrap_or(0)
        } else {
            0
        };

        if self.selected_project >= self.projects.len() {
            self.selected_project = if self.projects.is_empty() {
                0
            } else {
                self.projects.len() - 1
            };
        }

        // Update refresh time and clear caches
        self.last_refresh = std::time::Instant::now();
        self.cached_analytics.clear();
        self.cached_todo_stats.clear();
        self.cached_daily_usage = None;
        self.tab_render_cache.clear();
        self.last_cache_update = std::time::Instant::now();

        // Refresh completed silently without status message
    }

    /// Clean up resources before exiting
    async fn cleanup(&mut self) -> Result<()> {
        // Save current configuration before exiting
        self.config.save()?;
        Ok(())
    }
}
