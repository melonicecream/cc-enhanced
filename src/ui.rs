use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Tabs, Widget},
    Frame,
};

use crate::{
    app::{App, Tab, ViewMode},
    claude,
    features::todos::{TodoPriority, TodoStatus},
    shared::theme::{ModernIcons, ModernTheme},
    widgets::{
        icon_span, metric_span, modern_block, priority_icon, status_icon, ModernCard, ModernGauge,
        ModernList, ModernListItem, ModernProgressBar, ProgressVariant,
    },
};

/// Format currency with adaptive precision based on amount size
/// This addresses the issue of over-representation of micro-costs in AI model pricing
/// Uses a custom adaptive formatting algorithm for better user experience
fn format_cost(amount: f64) -> String {
    if amount == 0.0 {
        "$0.00".to_string()
    } else if amount < 0.001 {
        // For micro-costs under $0.001, use scientific notation
        // This prevents misleading $0.0000 display that loses all cost information
        format!("${amount:.2e}")
    } else if amount < 0.01 {
        // For fractional cents, show up to 4 decimal places but trim trailing zeros
        let formatted = format!("{amount:.4}");
        format!("${}", formatted.trim_end_matches('0').trim_end_matches('.'))
    } else if amount < 1.0 {
        // For sub-dollar amounts, use 3 decimal places but trim trailing zeros
        let formatted = format!("{amount:.3}");
        format!("${}", formatted.trim_end_matches('0').trim_end_matches('.'))
    } else {
        // For dollar amounts and above, use standard 2 decimal places
        format!("${amount:.2}")
    }
}

/// Format cost range for context (e.g., "$0.001 - $0.015")
#[allow(dead_code)]
fn format_cost_range(min_cost: f64, max_cost: f64) -> String {
    format!("{} - {}", format_cost(min_cost), format_cost(max_cost))
}

/// Draw the main UI
pub fn draw(f: &mut Frame, app: &mut App) {
    let theme = app.current_theme().clone();

    match app.view_mode {
        ViewMode::ProjectView => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Main content
                    Constraint::Length(4), // Footer (increased for modern style)
                ])
                .split(f.size());

            draw_modern_header(f, chunks[0], app, &theme);
            draw_modern_main_content(f, chunks[1], app, &theme);
            draw_modern_footer(f, chunks[2], app, &theme);
        }
        ViewMode::GlobalDashboard => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Global dashboard content
                    Constraint::Length(2), // Minimal footer
                ])
                .split(f.size());

            draw_global_header(f, chunks[0], app, &theme);
            draw_global_dashboard(f, chunks[1], app, &theme);
            draw_global_footer(f, chunks[2], app, &theme);
        }
    }

    // Draw help overlay if enabled
    if app.config.show_help {
        draw_help_overlay(f, f.size(), app, &theme);
    }

    // Draw IDE selection overlay if enabled
    if app.ide_selection_state.is_some() {
        draw_ide_selection_overlay(f, f.size(), app, &theme);
    }
}

/// Draw the modern header with enhanced styling
fn draw_modern_header(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    let stats = &app.usage_stats;
    let total_tokens = stats.input_tokens
        + stats.output_tokens
        + stats.cache_creation_tokens
        + stats.cache_read_tokens;

    // Create header content with modern icons
    let mut header_spans = vec![
        icon_span(ModernIcons::REFRESH, Style::default().fg(theme.accent)),
        Span::styled(" Claude Code Enhanced", theme.header_style()),
    ];

    header_spans.extend_from_slice(&[
        Span::styled(" │ ", theme.border_style()),
        icon_span(ModernIcons::BULLET, Style::default().fg(theme.success)),
        metric_span(format_number(total_tokens), theme),
        Span::styled(" tokens", theme.secondary_text_style()),
        Span::styled(" │ ", theme.border_style()),
        Span::styled("$", Style::default().fg(theme.success)),
        metric_span(format!("{:.4}", stats.total_cost), theme),
        if stats.is_subscription_user {
            Span::styled("*", theme.warning_style())
        } else {
            Span::styled("", Style::default())
        },
        Span::styled(" │ ", theme.border_style()),
        icon_span(ModernIcons::TIME, Style::default().fg(theme.info)),
        Span::styled(format!(" {}", app.reset_time_str), theme.warning_style()),
        Span::styled(" │ ", theme.border_style()),
        Span::styled(
            format!("⟳ {}", app.config.refresh_interval_display()),
            theme.info_style(),
        ),
    ]);

    // Add loading indicator after refresh time if needed
    if app.loading_states.is_loading() {
        let spinner_char = app.loading_states.get_spinner_char();

        header_spans.push(Span::styled(" ", theme.border_style()));
        header_spans.push(Span::styled(
            format!("{spinner_char}"),
            theme.warning_style(),
        ));
    }

    // Add status message to header if present
    if let Some(ref status) = app.status_message {
        let style = match status.message_type {
            crate::app::StatusType::Info => theme.info_style(),
            crate::app::StatusType::Success => theme.success_style(),
            crate::app::StatusType::Warning => theme.warning_style(),
            crate::app::StatusType::Error => theme.danger_style(),
        };

        let icon = match status.message_type {
            crate::app::StatusType::Info => "ℹ",
            crate::app::StatusType::Success => "✓",
            crate::app::StatusType::Warning => "⚠",
            crate::app::StatusType::Error => "✗",
        };

        header_spans.push(Span::styled(" │ ", theme.border_style()));
        header_spans.push(Span::styled(format!(" {} {}", icon, status.text), style));
    }

    let header_content = Text::from(vec![Line::from(header_spans)]);

    let header_card = ModernCard::new(header_content, theme)
        .title("Dashboard")
        .focused(false);

    header_card.render(area, f.buffer_mut());
}

/// Draw the modern main content area with card-based layout
fn draw_modern_main_content(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Left panel (projects)
            Constraint::Length(1),      // Separator
            Constraint::Percentage(69), // Right panel (tabs)
        ])
        .split(area);

    draw_modern_project_list(f, main_chunks[0], app, theme);
    draw_separator(f, main_chunks[1], theme);
    draw_modern_right_panel(f, main_chunks[2], app, theme);
}

/// Draw a modern separator between panels
fn draw_separator(f: &mut Frame, area: Rect, theme: &ModernTheme) {
    for y in area.y..area.y + area.height {
        let cell = f.buffer_mut().get_mut(area.x, y);
        cell.set_char('│');
        cell.set_style(theme.border_style());
    }
}

/// Draw the modern project list panel with cards
fn draw_modern_project_list(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    if app.projects.is_empty() {
        let empty_content = Text::from(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                app.i18n.t("no_projects"),
                theme.secondary_text_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                app.i18n.t("make_sure_used"),
                theme.dimmed_style(),
            )]),
        ]);

        let empty_card = ModernCard::new(empty_content, theme).title("Projects");

        empty_card.render(area, f.buffer_mut());
        return;
    }

    // Create modern list items
    let mut list_items = Vec::new();

    for (i, project) in app.projects.iter().enumerate() {
        let is_selected = i == app.selected_project;
        let status_ico = status_icon(project.is_active);

        // Get todo completion info for selected project
        let todo_info = if is_selected {
            if let Some(stats) = app.selected_project_todo_stats() {
                format!(" [{}%]", stats.completion_percentage as u32)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let line = Line::from(vec![
            Span::styled(
                status_ico,
                if project.is_active {
                    theme.success_style()
                } else {
                    theme.dimmed_style()
                },
            ),
            Span::styled(" ", Style::default()),
            Span::styled(
                crate::widgets::format_project_name(&project.name, 25),
                if is_selected {
                    theme.selected_style()
                } else {
                    Style::default().fg(theme.text_primary)
                },
            ),
            Span::styled(
                format!(" ({})", project.sessions.len()),
                theme.secondary_text_style(),
            ),
            Span::styled(todo_info, Style::default().fg(theme.accent)),
        ]);

        let mut item = ModernListItem::new(line);
        if is_selected {
            item = item.icon(ModernIcons::ARROW_RIGHT);
        }

        list_items.push(item);
    }

    let projects_list = ModernList::new(list_items, theme)
        .title("Projects ↑↓")
        .selected(Some(app.selected_project));

    projects_list.render(area, f.buffer_mut());
}

/// Draw the modern right panel with enhanced tabs
fn draw_modern_right_panel(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab bar
            Constraint::Min(0),    // Tab content
        ])
        .split(area);

    draw_modern_tab_bar(f, right_chunks[0], app, theme);
    draw_modern_tab_content(f, right_chunks[1], app, theme);
}

/// Draw the modern tab bar with icons
fn draw_modern_tab_bar(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    let tab_titles = vec![
        format!("{} {}", ModernIcons::OVERVIEW, app.i18n.t("tab.overview")),
        format!("{} {}", ModernIcons::USAGE, app.i18n.t("tab.usage")),
        format!("{} {}", ModernIcons::SESSIONS, app.i18n.t("tab.sessions")),
        format!("{} {}", ModernIcons::TODOS, app.i18n.t("tab.todos")),
        format!("{} {}", ModernIcons::QUOTA, app.i18n.t("tab.quota")),
    ];

    let tabs = Tabs::new(tab_titles)
        .block(modern_block(Some("Information"), theme, false))
        .style(theme.secondary_text_style())
        .highlight_style(theme.selected_style())
        .select(app.current_tab as usize);

    f.render_widget(tabs, area);
}

/// Draw the content for the selected tab with modern styling
fn draw_modern_tab_content(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    // Only render the currently selected tab to avoid expensive calculations
    match app.current_tab {
        Tab::Overview => draw_modern_overview_tab(f, area, app, theme),
        Tab::Usage => draw_modern_usage_tab(f, area, app, theme),
        Tab::Sessions => draw_modern_sessions_tab(f, area, app, theme),
        Tab::Todos => draw_modern_todos_tab(f, area, app, theme),
        Tab::Quota => draw_modern_quota_tab(f, area, app, theme),
    }
}

/// Draw the modern Overview tab with cards
fn draw_modern_overview_tab(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    let project_clone = app.selected_project().cloned();
    if let Some(project) = project_clone {
        // Use a default analytics for quick rendering if cache miss
        let analytics =
            app.get_project_analytics(&project)
                .unwrap_or_else(|| claude::ProjectAnalytics {
                    total_sessions: project.sessions.len(),
                    total_messages: project.sessions.iter().map(|s| s.message_count).sum(),
                    total_tokens: 0,
                    estimated_cost: 0.0,
                    first_session: None,
                    last_session: None,
                    cache_efficiency: 0.0,
                    session_blocks: Vec::new(),
                });

        let todo_stats = app.selected_project_todo_stats();

        let mut content_lines = vec![
            Line::from(vec![
                icon_span(ModernIcons::OVERVIEW, Style::default().fg(theme.accent)),
                Span::styled(" Project: ", theme.secondary_text_style()),
                Span::styled(&project.name, theme.header_style()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("📁 Path: ", theme.secondary_text_style()),
                Span::styled(project.path.display().to_string(), theme.info_style()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("📊 Sessions: ", theme.secondary_text_style()),
                metric_span(analytics.total_sessions.to_string(), theme),
                Span::styled("  Messages: ", theme.secondary_text_style()),
                metric_span(analytics.total_messages.to_string(), theme),
            ]),
            Line::from(vec![
                Span::styled("🔢 Tokens: ", theme.secondary_text_style()),
                metric_span(format_number(analytics.total_tokens), theme),
                Span::styled("  Cost: ", theme.secondary_text_style()),
                Span::styled(format_cost(analytics.estimated_cost), theme.success_style()),
            ]),
            Line::from(vec![
                Span::styled("⚡ Cache Efficiency: ", theme.secondary_text_style()),
                metric_span(format!("{:.1}%", analytics.cache_efficiency), theme),
            ]),
        ];

        if let Some(stats) = todo_stats {
            content_lines.extend(vec![
                Line::from(""),
                Line::from(vec![
                    icon_span(ModernIcons::TODOS, Style::default().fg(theme.success)),
                    Span::styled(" Todo Progress: ", theme.secondary_text_style()),
                    metric_span(
                        format!("{}/{}", stats.completed_todos, stats.total_todos),
                        theme,
                    ),
                    Span::styled(
                        format!(" ({:.1}%)", stats.completion_percentage),
                        Style::default().fg(theme.accent),
                    ),
                ]),
            ]);
        }

        let content = Text::from(content_lines);
        let overview_card = ModernCard::new(content, theme).title("Project Overview");

        overview_card.render(area, f.buffer_mut());
    } else {
        let empty_content = Text::from(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "No project selected",
                theme.secondary_text_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Select a project from the list",
                theme.dimmed_style(),
            )]),
        ]);

        let empty_card = ModernCard::new(empty_content, theme).title("Project Overview");

        empty_card.render(area, f.buffer_mut());
    }
}

/// Draw the project-specific Usage tab - Lightweight local project statistics
fn draw_modern_usage_tab(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    if let Some(project) = app.selected_project().cloned() {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([
                Constraint::Percentage(50), // Left panel - Project stats
                Constraint::Percentage(50), // Right panel - Session details
            ])
            .split(area);

        // Left: Project Usage Statistics
        draw_project_usage_card(f, chunks[0], &project, app, theme);

        // Right: Recent Session Activity
        draw_project_sessions_card(f, chunks[1], &project, theme);
    } else {
        let no_project_content = Text::from(vec![
            Line::from(vec![Span::styled(
                "📁 No Project Selected",
                theme.header_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Select a project from the left panel to view",
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                "detailed usage statistics for that project.",
                theme.secondary_text_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "💡 Tip: Press G for global analytics",
                theme.info_style(),
            )]),
        ]);
        let card = ModernCard::new(no_project_content, theme).title("📊 Project Usage");
        card.render(area, f.buffer_mut());
    }
}

/// Draw the modern Sessions tab
fn draw_modern_sessions_tab(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    if let Some(project) = app.selected_project() {
        let mut list_items = Vec::new();

        for session in project.sessions.iter().take(20) {
            let time_str = format_time_ago(session.last_modified);
            let line = Line::from(vec![
                icon_span(ModernIcons::SESSIONS, Style::default().fg(theme.info)),
                Span::styled(
                    format!(" {}", &session.id[..8]),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(
                    format!(" • {} msgs", session.message_count),
                    theme.secondary_text_style(),
                ),
                Span::styled(format!(" • {time_str}"), theme.dimmed_style()),
            ]);

            list_items.push(ModernListItem::new(line));
        }

        if list_items.is_empty() {
            let empty_content = Text::from("No sessions found for this project");
            let empty_card = ModernCard::new(empty_content, theme).title("Recent Sessions");
            empty_card.render(area, f.buffer_mut());
        } else {
            let sessions_list = ModernList::new(list_items, theme).title("Recent Sessions");
            sessions_list.render(area, f.buffer_mut());
        }
    } else {
        let empty_content = Text::from("Select a project to view sessions");
        let empty_card = ModernCard::new(empty_content, theme).title("Recent Sessions");
        empty_card.render(area, f.buffer_mut());
    }
}

/// Draw the modern Todos tab with progress bars
fn draw_modern_todos_tab(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    let todos = app.selected_project_todos();
    let todo_stats = app.selected_project_todo_stats();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Stats section
            Constraint::Min(0),    // Todo list
        ])
        .split(area);

    // Draw todo statistics with modern progress bar
    if let Some(stats) = todo_stats {
        let progress_content = Text::from(vec![
            Line::from(vec![
                icon_span(ModernIcons::TODOS, Style::default().fg(theme.success)),
                Span::styled(" Todo Progress", theme.header_style()),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("{}/{} completed", stats.completed_todos, stats.total_todos),
                    theme.secondary_text_style(),
                ),
                Span::styled(" • ", theme.secondary_text_style()),
                icon_span(ModernIcons::HIGH_PRIORITY, theme.priority_style("high")),
                Span::styled(
                    format!(" {} high priority", stats.high_priority_remaining),
                    if stats.high_priority_remaining > 0 {
                        theme.danger_style()
                    } else {
                        theme.secondary_text_style()
                    },
                ),
            ]),
        ]);

        let progress_card = ModernCard::new(progress_content, theme).title("Progress");
        progress_card.render(chunks[0], f.buffer_mut());

        // Render progress bar separately
        let progress_area = Rect {
            x: chunks[0].x + 2,
            y: chunks[0].y + chunks[0].height - 2,
            width: chunks[0].width - 4,
            height: 1,
        };

        let progress_bar = ModernProgressBar::new(stats.completion_percentage, theme).variant(
            if stats.completion_percentage >= 80.0 {
                ProgressVariant::Success
            } else if stats.completion_percentage >= 50.0 {
                ProgressVariant::Warning
            } else {
                ProgressVariant::Danger
            },
        );

        progress_bar.render(progress_area, f.buffer_mut());
    } else {
        let no_todos_content = Text::from("No todos found for this project");
        let no_todos_card = ModernCard::new(no_todos_content, theme).title("Todo Progress");
        no_todos_card.render(chunks[0], f.buffer_mut());
    }

    // Draw todo list
    if !todos.is_empty() {
        let mut list_items = Vec::new();

        // Sort todos by priority (High -> Medium -> Low) then by status (Pending -> InProgress -> Completed)
        let mut sorted_todos: Vec<_> = todos.iter().collect();
        sorted_todos.sort_by(|a, b| {
            use std::cmp::Ordering;

            // First sort by priority
            let priority_order = match (&b.1.priority, &a.1.priority) {
                (TodoPriority::High, TodoPriority::High) => Ordering::Equal,
                (TodoPriority::High, _) => Ordering::Greater,
                (_, TodoPriority::High) => Ordering::Less,
                (TodoPriority::Medium, TodoPriority::Medium) => Ordering::Equal,
                (TodoPriority::Medium, TodoPriority::Low) => Ordering::Greater,
                (TodoPriority::Low, TodoPriority::Medium) => Ordering::Less,
                (TodoPriority::Low, TodoPriority::Low) => Ordering::Equal,
            };

            if priority_order != Ordering::Equal {
                return priority_order;
            }

            // Then sort by status (Pending first, then InProgress, then Completed)
            match (&a.1.status, &b.1.status) {
                (TodoStatus::Pending, TodoStatus::Pending) => Ordering::Equal,
                (TodoStatus::Pending, _) => Ordering::Less,
                (_, TodoStatus::Pending) => Ordering::Greater,
                (TodoStatus::InProgress, TodoStatus::InProgress) => Ordering::Equal,
                (TodoStatus::InProgress, TodoStatus::Completed) => Ordering::Less,
                (TodoStatus::Completed, TodoStatus::InProgress) => Ordering::Greater,
                (TodoStatus::Completed, TodoStatus::Completed) => Ordering::Equal,
            }
        });

        for (_session_id, todo) in sorted_todos.iter().take(15) {
            let status_icon = match todo.status {
                TodoStatus::Completed => ModernIcons::COMPLETED,
                TodoStatus::InProgress => ModernIcons::IN_PROGRESS,
                TodoStatus::Pending => ModernIcons::PENDING,
            };

            let priority_ico = priority_icon(&todo.priority.to_string());

            let content_style = match todo.status {
                TodoStatus::Completed => {
                    // Add strikethrough for completed todos
                    theme.dimmed_style().add_modifier(Modifier::CROSSED_OUT)
                }
                TodoStatus::InProgress => Style::default().fg(theme.text_primary),
                TodoStatus::Pending => Style::default().fg(theme.text_primary),
            };

            // Calculate content width for better alignment using Unicode-safe truncation
            let max_content_width = 60; // Adjust as needed
            let content_text = if todo.content.chars().count() > max_content_width {
                let truncated: String = todo.content.chars().take(max_content_width).collect();
                format!("{truncated}...")
            } else {
                todo.content.clone()
            };

            let line = Line::from(vec![
                // Priority indicator first - shows importance
                icon_span(
                    priority_ico,
                    theme.priority_style(&todo.priority.to_string()),
                ),
                Span::styled(" ", Style::default()),
                // Status icon second - shows current state
                icon_span(
                    status_icon,
                    match todo.status {
                        TodoStatus::Completed => Style::default().fg(theme.success),
                        TodoStatus::InProgress => Style::default().fg(theme.info),
                        TodoStatus::Pending => Style::default().fg(theme.secondary),
                    },
                ),
                Span::styled(" ", Style::default()),
                // Content with priority-based styling
                Span::styled(
                    content_text,
                    match todo.priority {
                        TodoPriority::High => {
                            if matches!(todo.status, TodoStatus::Completed) {
                                content_style
                            } else {
                                content_style.add_modifier(Modifier::BOLD)
                            }
                        }
                        TodoPriority::Medium => content_style,
                        TodoPriority::Low => content_style.add_modifier(Modifier::DIM),
                    },
                ),
            ]);

            list_items.push(ModernListItem::new(line));
        }

        let todos_list = ModernList::new(list_items, theme).title("Recent Todos");
        todos_list.render(chunks[1], f.buffer_mut());
    } else {
        let empty_content = Text::from("No todos found for this project");
        let empty_card = ModernCard::new(empty_content, theme).title("Recent Todos");
        empty_card.render(chunks[1], f.buffer_mut());
    }
}

/// Draw the modern Quota tab with gauges
fn draw_modern_quota_tab(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    let stats = &app.usage_stats;
    let total_tokens = stats.input_tokens
        + stats.output_tokens
        + stats.cache_creation_tokens
        + stats.cache_read_tokens;

    let estimated_daily_limit = 20_000_000;
    let _quota_percentage = (total_tokens as f64 / estimated_daily_limit as f64) * 100.0;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Quota gauge
            Constraint::Min(0),    // Details
        ])
        .split(area);

    // Draw quota gauge
    let gauge_content = Text::from(vec![
        Line::from(vec![
            icon_span(ModernIcons::QUOTA, Style::default().fg(theme.accent)),
            Span::styled(" Daily Quota Usage", theme.header_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            metric_span(format_number(total_tokens), theme),
            Span::styled(" / ", theme.secondary_text_style()),
            metric_span(format_number(estimated_daily_limit), theme),
            Span::styled(" tokens", theme.secondary_text_style()),
        ]),
    ]);

    let gauge_card = ModernCard::new(gauge_content, theme).title("Quota");
    gauge_card.render(chunks[0], f.buffer_mut());

    // Render quota gauge
    let gauge_area = Rect {
        x: chunks[0].x + 2,
        y: chunks[0].y + chunks[0].height - 2,
        width: chunks[0].width - 4,
        height: 1,
    };

    let quota_ratio = (total_tokens as f64 / estimated_daily_limit as f64).min(1.0);
    let quota_gauge = ModernGauge::new(quota_ratio, theme).variant(if quota_ratio > 0.9 {
        ProgressVariant::Danger
    } else if quota_ratio > 0.7 {
        ProgressVariant::Warning
    } else {
        ProgressVariant::Success
    });

    quota_gauge.render(gauge_area, f.buffer_mut());

    // Draw details
    let details_content = Text::from(vec![
        Line::from(vec![
            icon_span(ModernIcons::USAGE, Style::default().fg(theme.info)),
            Span::styled(
                " Today's Breakdown:",
                theme.warning_style().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Input: ", theme.secondary_text_style()),
            metric_span(format_number(stats.input_tokens), theme),
            Span::styled("  Output: ", theme.secondary_text_style()),
            metric_span(format_number(stats.output_tokens), theme),
        ]),
        Line::from(vec![
            Span::styled("Cache Create: ", theme.secondary_text_style()),
            metric_span(format_number(stats.cache_creation_tokens), theme),
            Span::styled("  Cache Read: ", theme.secondary_text_style()),
            metric_span(format_number(stats.cache_read_tokens), theme),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("💰 Total Cost: ", theme.secondary_text_style()),
            Span::styled(format_cost(stats.total_cost), theme.success_style()),
            if stats.is_subscription_user {
                Span::styled(" (estimated*)", theme.warning_style())
            } else {
                Span::styled(" (calculated)", theme.success_style())
            },
        ]),
        Line::from(vec![
            Span::styled("📨 Messages: ", theme.secondary_text_style()),
            metric_span(stats.message_count.to_string(), theme),
        ]),
        Line::from(""),
        Line::from(vec![
            icon_span(ModernIcons::TIME, Style::default().fg(theme.warning)),
            Span::styled(" Next Reset: ", theme.secondary_text_style()),
            Span::styled(&app.reset_time_str, theme.warning_style()),
        ]),
        Line::from(""),
        if stats.is_subscription_user {
            Line::from(vec![
                Span::styled("ℹ️  ", theme.info_style()),
                Span::styled(
                    "*Estimated cost for subscription users",
                    theme.warning_style(),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled("✅ ", theme.success_style()),
                Span::styled("Actual API costs tracked", theme.success_style()),
            ])
        },
    ]);

    let details_card = ModernCard::new(details_content, theme).title("Usage Details");
    details_card.render(chunks[1], f.buffer_mut());
}

/// Draw the modern footer with enhanced styling
fn draw_modern_footer(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),     // Help text
            Constraint::Length(25), // Refresh info
        ])
        .split(area);

    // Help text with modern styling
    let help_content = Text::from(vec![
        Line::from(vec![
            Span::styled(
                app.i18n.t("controls.navigation"),
                theme.secondary_text_style(),
            ),
            Span::styled(
                " j/k ↑↓",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", theme.secondary_text_style()),
            Span::styled(app.i18n.t("controls.theme"), theme.secondary_text_style()),
            Span::styled(
                " t",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({})", app.config.theme_display()),
                theme.info_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled(app.i18n.t("controls.tabs"), theme.secondary_text_style()),
            Span::styled(
                " Tab/Shift+Tab",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", theme.secondary_text_style()),
            Span::styled(app.i18n.t("controls.refresh"), theme.secondary_text_style()),
            Span::styled(
                " 1-5",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", theme.secondary_text_style()),
            Span::styled(
                "r",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} | ", app.i18n.t("controls.manual")),
                theme.secondary_text_style(),
            ),
            Span::styled(app.i18n.t("controls.help"), theme.secondary_text_style()),
            Span::styled(
                " ?",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", theme.secondary_text_style()),
            Span::styled("q", theme.danger_style().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" {} | ", app.i18n.t("controls.quit")),
                theme.secondary_text_style(),
            ),
            Span::styled("G", theme.success_style().add_modifier(Modifier::BOLD)),
            Span::styled(" Global Analytics | ", theme.success_style()),
            Span::styled("o", theme.info_style().add_modifier(Modifier::BOLD)),
            Span::styled(" Open IDE", theme.info_style()),
        ]),
    ]);

    let help_card = ModernCard::new(help_content, theme).title("Controls");
    help_card.render(chunks[0], f.buffer_mut());

    // Refresh info with countdown
    let refresh_content = Text::from(vec![
        Line::from(vec![
            icon_span(ModernIcons::REFRESH, Style::default().fg(theme.info)),
            Span::styled(
                format!(" {}", app.config.refresh_interval_display()),
                Style::default().fg(theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::styled("Next: ", theme.secondary_text_style()),
            metric_span(format!("{:.0}s", app.next_refresh_in.as_secs()), theme),
        ]),
    ]);

    let refresh_card = ModernCard::new(refresh_content, theme).title("Auto Refresh");
    refresh_card.render(chunks[1], f.buffer_mut());
}

/// Draw help overlay with explanations
fn draw_help_overlay(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    // Create centered popup area
    let popup_area = Rect {
        x: area.width / 6,
        y: area.height / 6,
        width: area.width * 2 / 3,
        height: area.height * 2 / 3,
    };

    // Clear the background
    f.render_widget(Clear, popup_area);

    let help_content = match app.current_tab {
        crate::app::Tab::Overview => Text::from(vec![
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.overview.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.ui_layout.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.project_stats.line7"),
                theme.secondary_text_style(),
            )]),
        ]),
        crate::app::Tab::Usage => Text::from(vec![
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line7"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line8"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line9"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line10"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line11"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line12"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.usage_analytics.line13"),
                theme.secondary_text_style(),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line7"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.real_time_costs.line8"),
                theme.secondary_text_style(),
            )]),
        ]),
        crate::app::Tab::Sessions => Text::from(vec![
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.sessions.line7"),
                theme.secondary_text_style(),
            )]),
        ]),
        crate::app::Tab::Quota => Text::from(vec![
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line7"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line8"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line9"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line10"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.quota.line11"),
                theme.secondary_text_style(),
            )]),
        ]),
        crate::app::Tab::Todos => Text::from(vec![
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.title"),
                theme.header_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line1"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line2"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line3"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line4"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line5"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line6"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                app.i18n.t("help.todos.line7"),
                theme.secondary_text_style(),
            )]),
        ]),
    };

    let help_title = format!("{} Help", ModernIcons::HELP);
    let help_card = ModernCard::new(help_content, theme).title(&help_title);

    help_card.render(popup_area, f.buffer_mut());
}

/// Format numbers with K/M suffixes for readability
fn format_number(num: u32) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f32 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f32 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// Format time ago in human readable format
fn format_time_ago(time: std::time::SystemTime) -> String {
    if let Ok(elapsed) = time.elapsed() {
        let secs = elapsed.as_secs();
        if secs < 60 {
            format!("{secs}s ago")
        } else if secs < 3600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86400 {
            format!("{}h ago", secs / 3600)
        } else {
            format!("{}d ago", secs / 86400)
        }
    } else {
        "just now".to_string()
    }
}

/// Draw project-specific usage statistics card
fn draw_project_usage_card(
    f: &mut Frame,
    area: Rect,
    project: &crate::claude::Project,
    app: &mut App,
    theme: &ModernTheme,
) {
    let analytics =
        app.get_project_analytics(project)
            .unwrap_or_else(|| crate::claude::ProjectAnalytics {
                total_sessions: project.sessions.len(),
                total_messages: project.sessions.iter().map(|s| s.message_count).sum(),
                total_tokens: 0,
                estimated_cost: 0.0,
                first_session: None,
                last_session: None,
                cache_efficiency: 0.0,
                session_blocks: Vec::new(),
            });

    // Calculate recent activity (last 7 days)
    let now = std::time::SystemTime::now();
    let week_ago = now - std::time::Duration::from_secs(7 * 24 * 60 * 60);
    let recent_sessions = project
        .sessions
        .iter()
        .filter(|s| s.last_modified >= week_ago)
        .count();

    // Average messages per session
    let avg_messages = if project.sessions.is_empty() {
        0.0
    } else {
        analytics.total_messages as f64 / project.sessions.len() as f64
    };

    let content = Text::from(vec![
        Line::from(vec![Span::styled(
            "📊 Project Usage Statistics",
            theme.header_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Sessions: ", theme.secondary_text_style()),
            metric_span(project.sessions.len().to_string(), theme),
        ]),
        Line::from(vec![
            Span::styled("Total Messages: ", theme.secondary_text_style()),
            metric_span(analytics.total_messages.to_string(), theme),
        ]),
        Line::from(vec![
            Span::styled("Avg Msgs/Session: ", theme.secondary_text_style()),
            metric_span(format!("{avg_messages:.1}"), theme),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Recent Activity (7d): ", theme.secondary_text_style()),
            if recent_sessions > 0 {
                Span::styled(format!("{recent_sessions} sessions"), theme.success_style())
            } else {
                Span::styled("No recent activity", theme.warning_style())
            },
        ]),
        Line::from(""),
        if analytics.estimated_cost > 0.0 {
            Line::from(vec![
                Span::styled("Estimated Cost: ", theme.secondary_text_style()),
                Span::styled(format_cost(analytics.estimated_cost), theme.warning_style()),
            ])
        } else {
            Line::from(vec![Span::styled(
                "💡 For detailed cost analysis,",
                theme.info_style(),
            )])
        },
        Line::from(vec![Span::styled(
            "   press G for global analytics",
            theme.info_style(),
        )]),
    ]);

    let card = ModernCard::new(content, theme).title("📈 Usage Overview");
    card.render(area, f.buffer_mut());
}

/// Draw project-specific sessions activity card  
fn draw_project_sessions_card(
    f: &mut Frame,
    area: Rect,
    project: &crate::claude::Project,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![Span::styled(
            "📝 Recent Sessions",
            theme.header_style(),
        )]),
        Line::from(""),
    ];

    if project.sessions.is_empty() {
        content_lines.push(Line::from(vec![Span::styled(
            "No sessions found for this project",
            theme.secondary_text_style(),
        )]));
    } else {
        // Show last 5 sessions
        let recent_sessions: Vec<_> = project.sessions.iter().rev().take(5).collect();

        for session in recent_sessions {
            let time_str = format_time_ago(session.last_modified);

            let status_icon = if session.message_count > 20 {
                "🔥" // Active session
            } else if session.message_count > 10 {
                "💬" // Normal session
            } else {
                "📝" // Light session
            };

            content_lines.push(Line::from(vec![
                Span::styled(status_icon, theme.success_style()),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{} messages", session.message_count),
                    if session.message_count > 20 {
                        theme.success_style()
                    } else if session.message_count > 10 {
                        theme.warning_style()
                    } else {
                        theme.secondary_text_style()
                    },
                ),
            ]));
            content_lines.push(Line::from(vec![Span::styled(
                format!("   {time_str}"),
                theme.secondary_text_style(),
            )]));
        }

        // Show total if there are more sessions
        if project.sessions.len() > 5 {
            content_lines.extend(vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("... and {} more sessions", project.sessions.len() - 5),
                    theme.info_style(),
                )]),
            ]);
        }
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("⚡ Session Activity");
    card.render(area, f.buffer_mut());
}

/// Draw daily costs and model distribution (top section)
#[allow(dead_code)]
fn draw_usage_top_section(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Daily Cost Analysis
    draw_daily_cost_card(f, chunks[0], analytics, theme);

    // Right: Model Distribution
    draw_model_distribution_card(f, chunks[1], analytics, theme);
}

/// Draw hourly patterns and cache efficiency (middle section)
#[allow(dead_code)]
fn draw_usage_middle_section(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Hourly Activity Pattern
    draw_hourly_pattern_card(f, chunks[0], analytics, theme);

    // Right: Cache Efficiency
    draw_cache_efficiency_card(f, chunks[1], analytics, theme);
}

/// Draw project rankings and session insights (bottom section)
#[allow(dead_code)]
fn draw_usage_bottom_section(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left: Project Usage Rankings
    draw_project_rankings_card(f, chunks[0], analytics, theme);

    // Right: Top Sessions
    draw_top_sessions_card(f, chunks[1], analytics, theme);
}

/// Daily Cost Analysis Card - 일별 실제 비용 데이터
#[allow(dead_code)]
fn draw_daily_cost_card(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![Span::styled(
            "💰 Daily Spending Trends",
            theme.header_style(),
        )]),
        Line::from(""),
    ];

    // Get last 7 days of usage
    let recent_days: Vec<_> = analytics.daily_usage.iter().rev().take(7).rev().collect();

    if recent_days.is_empty() {
        content_lines.push(Line::from(vec![Span::styled(
            "No usage data available",
            theme.secondary_text_style(),
        )]));
    } else {
        // Find max cost for scaling bars
        let max_cost = recent_days
            .iter()
            .map(|day| day.total_cost)
            .fold(0.0f64, f64::max)
            .max(0.01); // Avoid division by zero

        for day in &recent_days {
            let cost_ratio = (day.total_cost / max_cost).min(1.0);
            let bar_length = (cost_ratio * 15.0) as usize;

            let bar = format!(
                "{}{}",
                "█".repeat(bar_length.min(15)),
                "░".repeat(15 - bar_length.min(15))
            );

            let date_short = day.date.split('-').skip(1).collect::<Vec<_>>().join("/");

            content_lines.push(Line::from(vec![
                Span::styled(format!("{date_short}: "), theme.secondary_text_style()),
                Span::styled(
                    bar,
                    if day.total_cost > max_cost * 0.8 {
                        theme.danger_style()
                    } else if day.total_cost > max_cost * 0.5 {
                        theme.warning_style()
                    } else {
                        theme.success_style()
                    },
                ),
                Span::styled(
                    format!(" {}", format_cost(day.total_cost)),
                    theme.info_style(),
                ),
            ]));
        }

        // Summary stats
        let total_week_cost: f64 = recent_days.iter().map(|d| d.total_cost).sum();
        let avg_daily_cost = total_week_cost / recent_days.len() as f64;

        content_lines.extend(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("Week Total: ", theme.secondary_text_style()),
                Span::styled(format_cost(total_week_cost), theme.warning_style()),
            ]),
            Line::from(vec![
                Span::styled("Daily Avg: ", theme.secondary_text_style()),
                Span::styled(format_cost(avg_daily_cost), theme.info_style()),
            ]),
            Line::from(vec![
                Span::styled("Projected Monthly: ", theme.secondary_text_style()),
                Span::styled(
                    format_cost(analytics.cost_breakdown.projected_monthly),
                    theme.danger_style(),
                ),
            ]),
        ]);
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("📈 Real-Time Costs");
    card.render(area, f.buffer_mut());
}

/// Model Distribution Card - 모델별 사용량 분석
#[allow(dead_code)]
fn draw_model_distribution_card(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![Span::styled(
            "🤖 Model Usage Analysis",
            theme.header_style(),
        )]),
        Line::from(""),
    ];

    // Sort models by total cost (descending)
    let mut models: Vec<_> = analytics.model_distribution.values().collect();
    models.sort_by(|a, b| {
        b.total_cost
            .partial_cmp(&a.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if models.is_empty() {
        content_lines.push(Line::from(vec![Span::styled(
            "No model usage data",
            theme.secondary_text_style(),
        )]));
    } else {
        let total_cost: f64 = models.iter().map(|m| m.total_cost).sum();

        for (i, model) in models.iter().take(4).enumerate() {
            let percentage = if total_cost > 0.0 {
                (model.total_cost / total_cost) * 100.0
            } else {
                0.0
            };

            let model_display = model
                .model_name
                .replace("claude-sonnet-4-20250514", "Sonnet 4")
                .replace("claude-3-opus", "Opus 3")
                .replace("claude-3-sonnet", "Sonnet 3")
                .replace("claude-3-haiku", "Haiku 3");

            let rank_icon = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "🔸",
            };

            content_lines.push(Line::from(vec![
                Span::styled(format!("{rank_icon} "), theme.success_style()),
                Span::styled(model_display, theme.header_style()),
            ]));

            content_lines.push(Line::from(vec![
                Span::styled("   Cost: ", theme.secondary_text_style()),
                Span::styled(format_cost(model.total_cost), theme.warning_style()),
                Span::styled(format!(" ({percentage:.1}%)"), theme.info_style()),
            ]));

            content_lines.push(Line::from(vec![
                Span::styled("   Tokens: ", theme.secondary_text_style()),
                Span::styled(
                    format!(
                        "{}K",
                        (model.total_input_tokens + model.total_output_tokens) / 1000
                    ),
                    theme.info_style(),
                ),
            ]));
        }

        if models.len() > 4 {
            content_lines.push(Line::from(vec![Span::styled(
                format!("   ... and {} more models", models.len() - 4),
                theme.dimmed_style(),
            )]));
        }
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("🎯 Model Insights");
    card.render(area, f.buffer_mut());
}

/// Hourly Activity Pattern Card - 시간대별 활동 패턴
#[allow(dead_code)]
fn draw_hourly_pattern_card(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![Span::styled(
            "⏰ Activity Heatmap",
            theme.header_style(),
        )]),
        Line::from(""),
    ];

    // Find peak hours
    let max_tokens = analytics
        .hourly_patterns
        .iter()
        .map(|h| h.total_tokens)
        .max()
        .unwrap_or(1);

    if max_tokens == 0 {
        content_lines.push(Line::from(vec![Span::styled(
            "No activity data available",
            theme.secondary_text_style(),
        )]));
    } else {
        // Group hours for better visualization (every 4 hours)
        for chunk_start in (0..24).step_by(4) {
            let chunk_end = (chunk_start + 4).min(24);
            let chunk_tokens: u32 = analytics
                .hourly_patterns
                .iter()
                .skip(chunk_start)
                .take(chunk_end - chunk_start)
                .map(|h| h.total_tokens)
                .sum();

            let intensity = (chunk_tokens as f64 / max_tokens as f64).min(1.0);
            let heat_level = (intensity * 5.0) as usize;

            let heat_char = match heat_level {
                0 => "░",
                1 => "▒",
                2 => "▓",
                3 => "█",
                _ => "█",
            };

            let time_range = format!("{chunk_start:02}h-{chunk_end:02}h");

            content_lines.push(Line::from(vec![
                Span::styled(format!("{time_range}: "), theme.secondary_text_style()),
                Span::styled(
                    heat_char.repeat(8),
                    if intensity > 0.8 {
                        theme.danger_style()
                    } else if intensity > 0.5 {
                        theme.warning_style()
                    } else if intensity > 0.2 {
                        theme.info_style()
                    } else {
                        theme.dimmed_style()
                    },
                ),
                Span::styled(
                    format!(" {}K", chunk_tokens / 1000),
                    theme.secondary_text_style(),
                ),
            ]));
        }

        // Peak activity insight
        let peak_hour = analytics
            .hourly_patterns
            .iter()
            .enumerate()
            .max_by_key(|(_, h)| h.total_tokens)
            .map(|(i, _)| i)
            .unwrap_or(0);

        content_lines.extend(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("🔥 Peak Activity: ", theme.secondary_text_style()),
                Span::styled(
                    format!("{}:00-{}:00", peak_hour, peak_hour + 1),
                    theme.success_style(),
                ),
            ]),
        ]);
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("📊 Activity Patterns");
    card.render(area, f.buffer_mut());
}

/// Cache Efficiency Card - 캐시 효율성 분석
#[allow(dead_code)]
fn draw_cache_efficiency_card(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let cache = &analytics.cache_efficiency;

    let content_lines = vec![
        Line::from(vec![Span::styled(
            "🚀 Cache Performance",
            theme.header_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Hit Rate: ", theme.secondary_text_style()),
            Span::styled(
                format!("{:.1}%", cache.cache_hit_rate),
                if cache.cache_hit_rate > 70.0 {
                    theme.success_style()
                } else if cache.cache_hit_rate > 40.0 {
                    theme.warning_style()
                } else {
                    theme.danger_style()
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Cache Reads: ", theme.secondary_text_style()),
            Span::styled(
                format!("{}K", cache.total_cache_read_tokens / 1000),
                theme.info_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cache Creates: ", theme.secondary_text_style()),
            Span::styled(
                format!("{}K", cache.total_cache_creation_tokens / 1000),
                theme.info_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cost Saved: ", theme.secondary_text_style()),
            Span::styled(format_cost(cache.cache_cost_savings), theme.success_style()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            if cache.cache_hit_rate > 70.0 {
                "✨ Excellent cache usage!"
            } else if cache.cache_hit_rate > 40.0 {
                "👍 Good cache efficiency"
            } else if cache.cache_hit_rate > 0.0 {
                "📈 Room for improvement"
            } else {
                "💡 Enable caching for savings"
            },
            if cache.cache_hit_rate > 70.0 {
                theme.success_style()
            } else {
                theme.info_style()
            },
        )]),
    ];

    let card = ModernCard::new(Text::from(content_lines), theme).title("⚡ Cache Intelligence");
    card.render(area, f.buffer_mut());
}

/// Project Rankings Card - 프로젝트별 사용량 순위
#[allow(dead_code)]
fn draw_project_rankings_card(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![Span::styled(
            "🏆 Top Projects by Cost",
            theme.header_style(),
        )]),
        Line::from(""),
    ];

    // Sort projects by total cost
    let mut projects: Vec<_> = analytics.project_usage.values().collect();
    projects.sort_by(|a, b| {
        b.total_cost
            .partial_cmp(&a.total_cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if projects.is_empty() {
        content_lines.push(Line::from(vec![Span::styled(
            "No project data available",
            theme.secondary_text_style(),
        )]));
    } else {
        for (i, project) in projects.iter().take(5).enumerate() {
            let rank_icon = match i {
                0 => "👑",
                1 => "🥈",
                2 => "🥉",
                _ => "📁",
            };

            let project_display = crate::widgets::format_project_name(&project.project_name, 25);

            content_lines.push(Line::from(vec![
                Span::styled(format!("{rank_icon} "), theme.success_style()),
                Span::styled(project_display, theme.header_style()),
            ]));

            content_lines.push(Line::from(vec![
                Span::styled("   Cost: ", theme.secondary_text_style()),
                Span::styled(format_cost(project.total_cost), theme.warning_style()),
                Span::styled(
                    format!(" • {}K tokens", project.total_tokens / 1000),
                    theme.info_style(),
                ),
            ]));
        }
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("💼 Project Leaderboard");
    card.render(area, f.buffer_mut());
}

/// Top Sessions Card - 최고 비용 세션들
#[allow(dead_code)]
fn draw_top_sessions_card(
    f: &mut Frame,
    area: Rect,
    analytics: &crate::claude::UsageAnalytics,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![Span::styled(
            "💎 Most Expensive Sessions",
            theme.header_style(),
        )]),
        Line::from(""),
    ];

    if analytics.session_analytics.is_empty() {
        content_lines.push(Line::from(vec![Span::styled(
            "No session data available",
            theme.secondary_text_style(),
        )]));
    } else {
        for (i, session) in analytics.session_analytics.iter().take(4).enumerate() {
            let session_display = format!("{}...", &session.session_id[..8]);
            let duration_str = if session.duration_minutes > 60.0 {
                format!("{:.1}h", session.duration_minutes / 60.0)
            } else {
                format!("{:.0}m", session.duration_minutes)
            };

            content_lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), theme.secondary_text_style()),
                Span::styled(session_display, theme.info_style()),
                Span::styled(format!(" ({duration_str})"), theme.dimmed_style()),
            ]));

            content_lines.push(Line::from(vec![
                Span::styled("   ", Style::default()),
                Span::styled(format_cost(session.total_cost), theme.warning_style()),
                Span::styled(
                    format!(
                        " • {} msgs • {}K tokens",
                        session.message_count,
                        session.total_tokens / 1000
                    ),
                    theme.secondary_text_style(),
                ),
            ]));
        }

        // Total sessions summary
        let total_sessions = analytics.session_analytics.len();
        let total_cost: f64 = analytics
            .session_analytics
            .iter()
            .map(|s| s.total_cost)
            .sum();

        content_lines.extend(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("📊 Total: {total_sessions} sessions"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![
                Span::styled("💰 Combined cost: ", theme.secondary_text_style()),
                Span::styled(format_cost(total_cost), theme.warning_style()),
            ]),
        ]);
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("🎯 Session Insights");
    card.render(area, f.buffer_mut());
}

// ===== GLOBAL DASHBOARD UI FUNCTIONS =====

/// Draw the global dashboard header
fn draw_global_header(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    let stats = &app.usage_stats;
    let _total_tokens = stats.input_tokens
        + stats.output_tokens
        + stats.cache_creation_tokens
        + stats.cache_read_tokens;

    let header_text = vec![
        Span::styled("🌍 ", theme.success_style()),
        Span::styled("Global Analytics Dashboard", theme.header_style()),
        Span::styled(" - ", theme.secondary_text_style()),
        Span::styled(
            "Press 'ESC' or 'g' to return to project view",
            theme.warning_style(),
        ),
    ];

    let header_block = modern_block(Some("Global Analytics Dashboard"), theme, true);

    f.render_widget(header_block, area);
    let content_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let paragraph = ratatui::widgets::Paragraph::new(Line::from(header_text));
    f.render_widget(paragraph, content_area);
}

/// Draw the global dashboard main content (full-screen analytics)
fn draw_global_dashboard(f: &mut Frame, area: Rect, app: &mut App, theme: &ModernTheme) {
    // Get global analytics (this may trigger computation if cache is stale)
    if let Ok(analytics) = app.get_global_analytics() {
        // Create a comprehensive full-screen layout for global analytics
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Top section for trends and patterns
                Constraint::Percentage(40), // Bottom section for breakdowns
            ])
            .split(area);

        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Left: Daily trends
                Constraint::Percentage(50), // Right: Model distribution & activity
            ])
            .split(main_chunks[0]);

        let bottom_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(33), // Project rankings
                Constraint::Percentage(33), // Cache performance
                Constraint::Percentage(34), // Session insights
            ])
            .split(main_chunks[1]);

        let right_top_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(50), // Model distribution
                Constraint::Percentage(50), // Activity heatmap
            ])
            .split(top_chunks[1]);

        // Render all sections
        draw_global_daily_trends(f, top_chunks[0], &analytics.daily_usage, theme);
        draw_global_model_distribution(
            f,
            right_top_chunks[0],
            &analytics.model_distribution,
            theme,
        );
        draw_global_activity_heatmap(f, right_top_chunks[1], &analytics.hourly_patterns, theme);
        draw_global_project_rankings(f, bottom_chunks[0], &analytics.project_usage, theme);
        draw_global_cache_performance(f, bottom_chunks[1], &analytics.cache_efficiency, theme);
        draw_global_session_insights(f, bottom_chunks[2], &analytics.session_analytics, theme);
    } else {
        // Show loading or error state
        let error_card = ModernCard::new(
            Text::from(vec![
                Line::from("🔄 Loading global analytics..."),
                Line::from("This may take a few seconds for large datasets."),
            ]),
            theme,
        )
        .title("Global Analytics");
        error_card.render(area, f.buffer_mut());
    }
}

/// Draw the global dashboard footer (minimal)
fn draw_global_footer(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    let controls_text = format!(
        "Press: ESC/g=Project View | r=Refresh | t=Theme | q=Quit | Auto refresh: {}s",
        app.config.refresh_interval().as_secs()
    );

    let footer_paragraph = ratatui::widgets::Paragraph::new(Line::from(vec![Span::styled(
        controls_text,
        theme.secondary_text_style(),
    )]));

    f.render_widget(footer_paragraph, area);
}

/// Draw global daily spending trends (enhanced for full screen)
fn draw_global_daily_trends(
    f: &mut Frame,
    area: Rect,
    daily_usage: &[claude::DailyUsageDetail],
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![
            Span::styled("📊 ", theme.success_style()),
            Span::styled(
                "Daily Spending Analysis (Full Account)",
                theme.header_style(),
            ),
        ]),
        Line::from(""),
    ];

    if daily_usage.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "No usage data available",
            theme.secondary_text_style(),
        )));
    } else {
        // Show last 14 days for global view (more than project view)
        let recent_usage: Vec<_> = daily_usage.iter().rev().take(14).rev().collect();

        // Find max cost for bar scaling
        let max_cost = recent_usage
            .iter()
            .map(|u| u.total_cost)
            .fold(0.0_f64, |a, b| a.max(b));

        for usage in &recent_usage {
            let bar_length = if max_cost > 0.0 {
                ((usage.total_cost / max_cost) * 25.0) as usize
            } else {
                0
            };
            let bar = "█".repeat(bar_length.min(25));

            // Extract MM-DD from date string (YYYY-MM-DD format)
            let date_display = if usage.date.len() >= 10 {
                &usage.date[5..] // Skip YYYY- to show MM-DD
            } else {
                &usage.date
            };

            content_lines.push(Line::from(vec![
                Span::styled(date_display, theme.secondary_text_style()),
                Span::styled(format!(" {bar} "), theme.success_style()),
                Span::styled(format_cost(usage.total_cost), theme.warning_style()),
                Span::styled(format!(" ({}msg)", usage.message_count), theme.info_style()),
            ]));
        }

        // Summary statistics
        let total_cost: f64 = recent_usage.iter().map(|u| u.total_cost).sum();
        let total_tokens: u32 = recent_usage
            .iter()
            .map(|u| {
                u.total_input_tokens
                    + u.total_output_tokens
                    + u.total_cache_creation_tokens
                    + u.total_cache_read_tokens
            })
            .sum();
        let avg_cost = total_cost / recent_usage.len() as f64;

        content_lines.extend(vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("📈 ", theme.success_style()),
                Span::styled(
                    format!("14-day total: {}", format_cost(total_cost)),
                    theme.warning_style(),
                ),
            ]),
            Line::from(vec![
                Span::styled("🔤 ", theme.success_style()),
                Span::styled(format!("Total tokens: {total_tokens}"), theme.info_style()),
            ]),
            Line::from(vec![
                Span::styled("📊 ", theme.success_style()),
                Span::styled(
                    format!("Daily average: {}", format_cost(avg_cost)),
                    theme.info_style(),
                ),
            ]),
        ]);
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("📈 Global Daily Trends");
    card.render(area, f.buffer_mut());
}

/// Draw global model distribution
fn draw_global_model_distribution(
    f: &mut Frame,
    area: Rect,
    model_stats: &std::collections::HashMap<String, claude::ModelUsageStats>,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![
            Span::styled("🤖 ", theme.success_style()),
            Span::styled("Model Usage Distribution", theme.header_style()),
        ]),
        Line::from(""),
    ];

    if model_stats.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "No model data available",
            theme.secondary_text_style(),
        )));
    } else {
        // Sort models by total cost
        let mut models: Vec<_> = model_stats.values().collect();
        models.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());

        let total_cost: f64 = models.iter().map(|m| m.total_cost).sum();

        for (i, model) in models.iter().take(5).enumerate() {
            let percentage = if total_cost > 0.0 {
                (model.total_cost / total_cost) * 100.0
            } else {
                0.0
            };

            let bar_length = (percentage / 5.0) as usize; // Scale to fit
            let bar = "█".repeat(bar_length.min(20));

            let short_name = if model.model_name.len() > 20 {
                format!("{}…", &model.model_name[..19])
            } else {
                model.model_name.clone()
            };

            content_lines.push(Line::from(vec![
                Span::styled(format!("{}", i + 1), theme.success_style()),
                Span::styled(format!(". {short_name} "), theme.secondary_text_style()),
            ]));
            content_lines.push(Line::from(vec![
                Span::styled(format!("   {bar} "), theme.success_style()),
                Span::styled(format!("{percentage:.1}% "), theme.warning_style()),
                Span::styled(format_cost(model.total_cost), theme.info_style()),
            ]));
        }
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("🤖 Model Analytics");
    card.render(area, f.buffer_mut());
}

/// Draw global activity heatmap
fn draw_global_activity_heatmap(
    f: &mut Frame,
    area: Rect,
    hourly_patterns: &[claude::HourlyUsage],
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![
            Span::styled("🕐 ", theme.success_style()),
            Span::styled("24-Hour Activity Pattern", theme.header_style()),
        ]),
        Line::from(""),
    ];

    if hourly_patterns.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "No activity data available",
            theme.secondary_text_style(),
        )));
    } else {
        // Find the peak hour for normalization
        let max_activity = hourly_patterns
            .iter()
            .map(|h| h.message_count)
            .max()
            .unwrap_or(1) as f64;

        // Group hours in 4-hour blocks for compact display
        for block in 0..6 {
            let start_hour = block * 4;
            let end_hour = start_hour + 4;

            let mut block_line = vec![Span::styled(
                format!("{:02}-{:02}: ", start_hour, end_hour - 1),
                theme.secondary_text_style(),
            )];

            for hour in start_hour..end_hour {
                if hour < hourly_patterns.len() {
                    let activity = hourly_patterns[hour].message_count as f64;
                    let intensity = activity / max_activity;

                    let icon = if intensity > 0.8 {
                        "🔥"
                    } else if intensity > 0.6 {
                        "🟠"
                    } else if intensity > 0.4 {
                        "🟡"
                    } else if intensity > 0.2 {
                        "🔵"
                    } else {
                        "⚫"
                    };

                    block_line.push(Span::styled(icon, theme.success_style()));
                    block_line.push(Span::styled(" ", theme.secondary_text_style()));
                }
            }

            content_lines.push(Line::from(block_line));
        }

        content_lines.extend(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "🔥 High  🟠 Med  🟡 Low  🔵 Min  ⚫ None",
                theme.secondary_text_style(),
            )]),
        ]);
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("🕐 Activity Heatmap");
    card.render(area, f.buffer_mut());
}

/// Draw global project rankings
fn draw_global_project_rankings(
    f: &mut Frame,
    area: Rect,
    project_stats: &std::collections::HashMap<String, claude::ProjectUsageStats>,
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![
            Span::styled("🏆 ", theme.success_style()),
            Span::styled("Top Projects by Cost", theme.header_style()),
        ]),
        Line::from(""),
    ];

    if project_stats.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "No project data available",
            theme.secondary_text_style(),
        )));
    } else {
        // Sort projects by total cost
        let mut projects: Vec<_> = project_stats.values().collect();
        projects.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());

        for (i, project) in projects.iter().take(8).enumerate() {
            let rank_icon = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "📁",
            };

            let project_name = crate::widgets::format_project_name(&project.project_name, 25);

            content_lines.push(Line::from(vec![
                Span::styled(format!("{rank_icon} "), theme.success_style()),
                Span::styled(project_name, theme.secondary_text_style()),
            ]));
            content_lines.push(Line::from(vec![
                Span::styled("   ", theme.secondary_text_style()),
                Span::styled(format_cost(project.total_cost), theme.warning_style()),
                Span::styled(
                    format!(" ({} sessions)", project.session_count),
                    theme.secondary_text_style(),
                ),
            ]));
        }
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("🏆 Project Rankings");
    card.render(area, f.buffer_mut());
}

/// Draw global cache performance
fn draw_global_cache_performance(
    f: &mut Frame,
    area: Rect,
    cache_stats: &claude::CacheEfficiencyStats,
    theme: &ModernTheme,
) {
    let content_lines = vec![
        Line::from(vec![
            Span::styled("💾 ", theme.success_style()),
            Span::styled("Cache Performance", theme.header_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Hit Rate: ", theme.secondary_text_style()),
            Span::styled(
                format!("{:.1}%", cache_stats.cache_hit_rate),
                theme.success_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cache Created: ", theme.secondary_text_style()),
            Span::styled(
                format!("{}🔤", cache_stats.total_cache_creation_tokens),
                theme.secondary_text_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cache Read: ", theme.secondary_text_style()),
            Span::styled(
                format!("{}🔤", cache_stats.total_cache_read_tokens),
                theme.secondary_text_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cost Savings: ", theme.secondary_text_style()),
            Span::styled(
                format_cost(cache_stats.cache_cost_savings),
                theme.success_style(),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("💡 ", theme.success_style()),
            Span::styled(
                "Caching reduces costs by up to 90%",
                theme.secondary_text_style(),
            ),
        ]),
    ];

    let card = ModernCard::new(Text::from(content_lines), theme).title("💾 Cache Analytics");
    card.render(area, f.buffer_mut());
}

/// Draw global session insights
fn draw_global_session_insights(
    f: &mut Frame,
    area: Rect,
    session_analytics: &[claude::SessionAnalytics],
    theme: &ModernTheme,
) {
    let mut content_lines = vec![
        Line::from(vec![
            Span::styled("🎯 ", theme.success_style()),
            Span::styled("Session Insights", theme.header_style()),
        ]),
        Line::from(""),
    ];

    if session_analytics.is_empty() {
        content_lines.push(Line::from(Span::styled(
            "No session data available",
            theme.secondary_text_style(),
        )));
    } else {
        // Sort sessions by cost and show top 5
        let mut sessions: Vec<_> = session_analytics.iter().collect();
        sessions.sort_by(|a, b| b.total_cost.partial_cmp(&a.total_cost).unwrap());

        for (i, session) in sessions.iter().take(5).enumerate() {
            let session_id_short = if session.session_id.len() > 8 {
                &session.session_id[..8]
            } else {
                &session.session_id
            };

            content_lines.push(Line::from(vec![
                Span::styled(format!("{}. ", i + 1), theme.success_style()),
                Span::styled(session_id_short, theme.secondary_text_style()),
                Span::styled("…", theme.secondary_text_style()),
            ]));
            content_lines.push(Line::from(vec![
                Span::styled("   ", theme.secondary_text_style()),
                Span::styled(format_cost(session.total_cost), theme.warning_style()),
                Span::styled(
                    format!(" ({}m)", session.duration_minutes as u32),
                    theme.secondary_text_style(),
                ),
            ]));
        }

        // Summary stats
        let total_sessions = session_analytics.len();
        let total_cost: f64 = session_analytics.iter().map(|s| s.total_cost).sum();
        let avg_cost = if total_sessions > 0 {
            total_cost / total_sessions as f64
        } else {
            0.0
        };

        content_lines.extend(vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                format!("📊 {total_sessions} sessions total"),
                theme.secondary_text_style(),
            )]),
            Line::from(vec![Span::styled(
                format!("💰 {} avg cost", format_cost(avg_cost)),
                theme.info_style(),
            )]),
        ]);
    }

    let card = ModernCard::new(Text::from(content_lines), theme).title("🎯 Top Sessions");
    card.render(area, f.buffer_mut());
}

/// Draw IDE selection overlay
fn draw_ide_selection_overlay(f: &mut Frame, area: Rect, app: &App, theme: &ModernTheme) {
    if let Some(ref state) = app.ide_selection_state {
        // Create centered popup area
        let popup_width = 50.min(area.width - 4);
        let popup_height = (state.available_ides.len() as u16 + 4).min(area.height - 4);

        let popup_area = Rect {
            x: (area.width - popup_width) / 2,
            y: (area.height - popup_height) / 2,
            width: popup_width,
            height: popup_height,
        };

        // Clear the area first
        f.render_widget(Clear, popup_area);

        // Create content with available IDEs
        let mut content_lines = vec![
            Line::from(vec![Span::styled(
                "Select IDE to open project:",
                theme.header_style(),
            )]),
            Line::from(""),
        ];

        for (i, (ide_type, command)) in state.available_ides.iter().enumerate() {
            let (icon, style) = if i == state.selected_index {
                ("▶ ", theme.info_style().add_modifier(Modifier::BOLD))
            } else {
                ("  ", theme.secondary_text_style())
            };

            let display_text = format!("{}{} ({})", icon, ide_type.display_name(), command);
            content_lines.push(Line::from(vec![Span::styled(display_text, style)]));
        }

        content_lines.push(Line::from(""));
        content_lines.push(Line::from(vec![
            Span::styled("Press ", theme.secondary_text_style()),
            Span::styled("Enter", theme.info_style().add_modifier(Modifier::BOLD)),
            Span::styled(" to launch, ", theme.secondary_text_style()),
            Span::styled("Esc", theme.danger_style().add_modifier(Modifier::BOLD)),
            Span::styled(" to cancel", theme.secondary_text_style()),
        ]));

        let popup_card = ModernCard::new(Text::from(content_lines), theme).title("🚀 Open in IDE");
        popup_card.render(popup_area, f.buffer_mut());
    }
}
