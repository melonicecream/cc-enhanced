use super::config::Language;

/// Internationalization support for UI text
pub struct I18n {
    #[allow(dead_code)]
    language: Language,
}

impl I18n {
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    pub fn t<'a>(&self, key: &'a str) -> &'a str {
        match key {
            // Tab names
            "tab.overview" => "Overview",
            "tab.usage" => "Usage",
            "tab.sessions" => "Sessions",
            "tab.todos" => "Todos",
            "tab.quota" => "Quota",

            // Overview tab
            "overview.project" => "Project:",
            "overview.path" => "Path:",
            "overview.sessions" => "Sessions:",
            "overview.messages" => "Messages:",
            "overview.tokens" => "Tokens:",
            "overview.cost" => "Cost:",
            "overview.cache_efficiency" => "Cache Efficiency:",
            "overview.todo_progress" => "Todo Progress:",

            // Usage tab
            "usage.analytics" => "Usage Analytics",
            "usage.last_7_days" => "Last 7 Days:",
            "usage.session_blocks" => "Session Blocks:",

            // Sessions tab
            "sessions.recent" => "Recent Sessions",
            "sessions.no_sessions" => "No sessions found for this project",

            // Todos tab
            "todos.progress" => "Todo Progress",
            "todos.recent" => "Recent Todos",
            "todos.completed" => "completed",
            "todos.no_todos" => "No todos found for this project",

            // Quota tab
            "quota.daily_usage" => "Daily Quota Usage",
            "quota.tokens" => "tokens",
            "quota.breakdown" => "Today's Breakdown:",
            "quota.input" => "Input:",
            "quota.output" => "Output:",
            "quota.cache_create" => "Cache Create:",
            "quota.cache_read" => "Cache Read:",
            "quota.total_cost" => "Total Cost:",
            "quota.messages" => "Messages:",
            "quota.next_reset" => "Next Reset:",

            // Controls
            "controls.navigation" => "Navigation:",
            "controls.theme" => "Theme:",
            "controls.tabs" => "Tabs:",
            "controls.refresh" => "Refresh:",
            "controls.manual" => "manual",
            "controls.quit" => "quit",
            "controls.language" => "Language:",
            "controls.help" => "Help:",

            // Help explanations - Overview tab
            "help.overview.title" => "Overview Tab",
            "help.overview.line1" => "Shows key project information:",
            "help.overview.line2" => "• Project path and working directory",
            "help.overview.line3" => "• Session count and total messages",
            "help.overview.line4" => "• Token usage (input/output/cache)",
            "help.overview.line5" => "• Estimated costs in USD",
            "help.overview.line6" => "• Todo completion progress",
            "help.ui_layout.title" => "Application Layout",
            "help.ui_layout.line1" => "Interface organized into 4 areas:",
            "help.ui_layout.line2" => "",
            "help.ui_layout.line3" => "TOP: Dashboard with tokens, costs, reset time",
            "help.ui_layout.line4" => "LEFT: Project list (j/k or ↑↓ to navigate)",
            "help.ui_layout.line5" => "RIGHT: Tabs (Tab/Shift+Tab to switch)",
            "help.ui_layout.line6" => "BOTTOM: Controls and refresh info",
            "help.project_stats.title" => "Project Statistics",
            "help.project_stats.line1" => "Key metrics explained:",
            "help.project_stats.line2" => "",
            "help.project_stats.line3" => "SESSIONS: Conversation files with Claude",
            "help.project_stats.line4" => "MESSAGES: Total exchanges in conversations",
            "help.project_stats.line5" => "TOKENS: Text units for billing",
            "help.project_stats.line6" => "COST: Estimated charges",
            "help.project_stats.line7" => "CACHE: Reused tokens (higher = cheaper)",
            // Usage tab help - Revolutionary Analytics Dashboard
            "help.usage_analytics.title" => "Revolutionary Usage Analytics Dashboard",
            "help.usage_analytics.line1" => {
                "Real-time comprehensive usage analysis with 6-section dashboard:"
            }
            "help.usage_analytics.line2" => "",
            "help.usage_analytics.line3" => "TOP ROW (Daily Analytics):",
            "help.usage_analytics.line4" => {
                "• Daily Spending Trends: 7-day cost history with visual bars"
            }
            "help.usage_analytics.line5" => {
                "• Model Distribution: Usage breakdown by Claude model types"
            }
            "help.usage_analytics.line6" => "",
            "help.usage_analytics.line7" => "MIDDLE ROW (Activity & Performance):",
            "help.usage_analytics.line8" => "• Activity Heatmap: 24-hour usage intensity patterns",
            "help.usage_analytics.line9" => {
                "• Cache Performance: Efficiency metrics & cost savings"
            }
            "help.usage_analytics.line10" => "",
            "help.usage_analytics.line11" => "BOTTOM ROW (Project & Session Analysis):",
            "help.usage_analytics.line12" => {
                "• Project Rankings: Top projects ranked by total cost"
            }
            "help.usage_analytics.line13" => {
                "• Session Insights: Most expensive individual sessions"
            }
            "help.real_time_costs.title" => "Advanced Real-Time Cost Tracking",
            "help.real_time_costs.line1" => "Sophisticated live cost calculation system:",
            "help.real_time_costs.line2" => "",
            "help.real_time_costs.line3" => {
                "• OpenRouter API integration for precise model-specific pricing"
            }
            "help.real_time_costs.line4" => {
                "• Token-level cost calculations (input/output/cache tokens)"
            }
            "help.real_time_costs.line5" => {
                "• Smart cache optimization tracking (up to 90% savings)"
            }
            "help.real_time_costs.line6" => {
                "• Time-series analysis with daily/weekly/monthly trends"
            }
            "help.real_time_costs.line7" => "• Session-level and project-level cost breakdowns",
            "help.real_time_costs.line8" => "• Automatic data synchronization every refresh cycle",
            "help.data_parsing.title" => "Smart Session Data Parsing",
            "help.data_parsing.line1" => "Advanced JSONL session file analysis:",
            "help.data_parsing.line2" => "",
            "help.data_parsing.line3" => "• Parses all .jsonl files from ~/.claude/projects/",
            "help.data_parsing.line4" => {
                "• Extracts token usage, timestamps, and model information"
            }
            "help.data_parsing.line5" => "• Filters out zero-usage entries for accurate analytics",
            "help.data_parsing.line6" => "• Converts UTC timestamps to local timezone",
            "help.data_parsing.line7" => "• Groups sessions by date and project for analysis",
            "help.dashboard_features.title" => "Dashboard Features Overview",
            "help.dashboard_features.line1" => {
                "Six-card analytics layout with color-coded insights:"
            }
            "help.dashboard_features.line2" => "",
            "help.dashboard_features.line3" => {
                "• Visual cost trends with ascending/descending bars"
            }
            "help.dashboard_features.line4" => "• Model usage percentages with top 5 models shown",
            "help.dashboard_features.line5" => "• Hourly heatmap showing peak usage times",
            "help.dashboard_features.line6" => "• Cache efficiency with savings calculations",
            "help.dashboard_features.line7" => "• Project ranking with cost per project",
            "help.dashboard_features.line8" => {
                "• Session analysis with highest cost sessions identified"
            }
            // Quota tab help
            "help.quota.title" => "Usage Quota",
            "help.quota.line1" => "Claude usage restrictions:",
            "help.quota.line2" => "",
            "help.quota.line3" => "• Daily limits: Max tokens per day",
            "help.quota.line4" => "• Hourly limits: Short-term restrictions",
            "help.quota.line5" => "• 5-hour blocks: Primary mechanism",
            "help.quota.line6" => "• Reset times: When limits refresh",
            "help.quota.line7" => "",
            "help.quota.line8" => "Status indicators:",
            "help.quota.line9" => "Green: Well within limits",
            "help.quota.line10" => "Yellow: Approaching limits",
            "help.quota.line11" => "Red: At or near limit",
            // Todos tab help
            "help.todos.title" => "Todo System",
            "help.todos.line1" => "Task tracking from Claude sessions:",
            "help.todos.line2" => "",
            "help.todos.line3" => "• Auto-generated from conversations",
            "help.todos.line4" => "• Priorities: High (!), Medium (▪), Low (·)",
            "help.todos.line5" => "• Status: Pending, In Progress, Completed",
            "help.todos.line6" => "• Shows most recent session only",
            "help.todos.line7" => "• Visual completion percentage",
            // Sessions tab help
            "help.sessions.title" => "Session History",
            "help.sessions.line1" => "Individual conversation files:",
            "help.sessions.line2" => "",
            "help.sessions.line3" => "• JSONL format with message data",
            "help.sessions.line4" => "• 8-character unique identifiers",
            "help.sessions.line5" => "• Message count and token usage",
            "help.sessions.line6" => "• Last modified timestamps",
            "help.sessions.line7" => "• Sorted by most recent first",

            // General
            "no_projects" => "No Claude projects found.",
            "make_sure_used" => "Make sure you have used Claude Code before.",
            "no_project_selected" => "No project selected",
            "select_project" => "Select a project from the list",
            "auto_refresh" => "Auto Refresh",
            "next" => "Next:",

            _ => key, // Fallback to key if not found
        }
    }
}
