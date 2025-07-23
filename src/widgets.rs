#![allow(dead_code)]

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::shared::theme::{ModernIcons, ModernTheme, ProgressChars};

/// Helper function to render text with proper Unicode support
/// Returns the number of columns (visual width) consumed
fn render_text_unicode_aware(
    text: &str,
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_x: u16,
    style: Style,
) -> u16 {
    let mut current_x = x;

    for grapheme in text.graphemes(true) {
        let width = grapheme.width();

        // Check if we have enough space for this grapheme
        if current_x + width as u16 > max_x {
            break;
        }

        // Render the grapheme
        let cell = buf.get_mut(current_x, y);
        cell.set_symbol(grapheme);
        cell.set_style(style);

        // Move to next position based on grapheme width
        current_x += width as u16;

        // For zero-width graphemes, ensure we advance at least one position
        if width == 0 && current_x == x {
            current_x += 1;
        }
    }

    current_x - x
}

/// Modern card widget with rounded corners and shadow effect
pub struct ModernCard<'a> {
    title: Option<&'a str>,
    content: Text<'a>,
    theme: &'a ModernTheme,
    focused: bool,
    hover: bool,
}

impl<'a> ModernCard<'a> {
    pub fn new(content: Text<'a>, theme: &'a ModernTheme) -> Self {
        Self {
            title: None,
            content,
            theme,
            focused: false,
            hover: false,
        }
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn hover(mut self, hover: bool) -> Self {
        self.hover = hover;
        self
    }
}

impl<'a> Widget for ModernCard<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Choose border style based on state
        let border_style = if self.focused {
            self.theme.border_focused_style()
        } else if self.hover {
            self.theme.hover_style()
        } else {
            self.theme.border_style()
        };

        // Create block with rounded corners
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        if let Some(title) = self.title {
            block = block.title(title);
        }

        // Render the card background
        let inner = block.inner(area);
        block.render(area, buf);

        // Render content
        Paragraph::new(self.content)
            .wrap(Wrap { trim: true })
            .render(inner, buf);
    }
}

/// Modern progress bar with gradient colors and percentage display
pub struct ModernProgressBar<'a> {
    percentage: f64,
    label: Option<&'a str>,
    theme: &'a ModernTheme,
    show_percentage: bool,
    variant: ProgressVariant,
}

#[derive(Clone, Copy)]
pub enum ProgressVariant {
    Success,
    Warning,
    Danger,
    Info,
    Auto, // Chooses color based on percentage
}

impl<'a> ModernProgressBar<'a> {
    pub fn new(percentage: f64, theme: &'a ModernTheme) -> Self {
        Self {
            percentage: percentage.clamp(0.0, 100.0),
            label: None,
            theme,
            show_percentage: true,
            variant: ProgressVariant::Auto,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn show_percentage(mut self, show: bool) -> Self {
        self.show_percentage = show;
        self
    }

    pub fn variant(mut self, variant: ProgressVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl<'a> Widget for ModernProgressBar<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 3 || area.height < 1 {
            return;
        }

        // Choose style based on variant
        let progress_style = match self.variant {
            ProgressVariant::Success => self.theme.progress_success_style(),
            ProgressVariant::Warning => self.theme.progress_warning_style(),
            ProgressVariant::Danger => self.theme.progress_danger_style(),
            ProgressVariant::Info => self.theme.info_style(),
            ProgressVariant::Auto => self.theme.progress_style_for_percentage(self.percentage),
        };

        // Calculate filled width
        let progress_width = area.width as f64 * (self.percentage / 100.0);
        let filled_chars = progress_width.floor() as u16;
        let remaining_width = progress_width - filled_chars as f64;

        // Render progress bar
        for x in 0..area.width {
            let cell = buf.get_mut(area.x + x, area.y);

            if x < filled_chars {
                cell.set_char(ProgressChars::FILLED);
                cell.set_style(progress_style);
            } else if x == filled_chars && remaining_width > 0.5 {
                cell.set_char(ProgressChars::PARTIAL);
                cell.set_style(progress_style);
            } else {
                cell.set_char(ProgressChars::EMPTY);
                cell.set_style(self.theme.dimmed_style());
            }
        }

        // Render percentage or label overlay
        if area.height > 1 || area.width > 10 {
            let overlay_text = if let Some(label) = self.label {
                if self.show_percentage {
                    format!("{} {:.1}%", label, self.percentage)
                } else {
                    label.to_string()
                }
            } else if self.show_percentage {
                format!("{:.1}%", self.percentage)
            } else {
                String::new()
            };

            if !overlay_text.is_empty() {
                let text_width = overlay_text.width();
                if text_width <= area.width as usize {
                    let x_offset = (area.width as usize - text_width) / 2;
                    render_text_unicode_aware(
                        &overlay_text,
                        buf,
                        area.x + x_offset as u16,
                        area.y,
                        area.x + area.width,
                        Style::default().fg(self.theme.text_primary),
                    );
                }
            }
        }
    }
}

/// Modern list widget with icons and hover effects
pub struct ModernList<'a> {
    items: Vec<ModernListItem<'a>>,
    selected: Option<usize>,
    hovered: Option<usize>,
    theme: &'a ModernTheme,
    title: Option<&'a str>,
}

pub struct ModernListItem<'a> {
    text: Line<'a>,
    icon: Option<&'a str>,
    style: Option<Style>,
}

impl<'a> ModernListItem<'a> {
    pub fn new<T: Into<Line<'a>>>(text: T) -> Self {
        Self {
            text: text.into(),
            icon: None,
            style: None,
        }
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }
}

impl<'a> ModernList<'a> {
    pub fn new(items: Vec<ModernListItem<'a>>, theme: &'a ModernTheme) -> Self {
        Self {
            items,
            selected: None,
            hovered: None,
            theme,
            title: None,
        }
    }

    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    pub fn hovered(mut self, index: Option<usize>) -> Self {
        self.hovered = index;
        self
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
}

impl<'a> Widget for ModernList<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style());

        if let Some(title) = self.title {
            block = block.title(title);
        }

        let inner = block.inner(area);
        block.render(area, buf);

        // Render list items
        for (i, item) in self.items.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let y = inner.y + i as u16;
            let mut x = inner.x;

            // Determine item style
            let item_style = if Some(i) == self.selected {
                self.theme.selected_style()
            } else if Some(i) == self.hovered {
                self.theme.hover_style()
            } else {
                item.style
                    .unwrap_or_else(|| self.theme.secondary_text_style())
            };

            // Render background for selected/hovered items
            if Some(i) == self.selected || Some(i) == self.hovered {
                for bg_x in inner.x..inner.x + inner.width {
                    let cell = buf.get_mut(bg_x, y);
                    cell.set_style(item_style);
                }
            }

            // Render icon if present
            if let Some(icon) = item.icon {
                let consumed =
                    render_text_unicode_aware(icon, buf, x, y, inner.x + inner.width, item_style);
                x += consumed;
                if x < inner.x + inner.width {
                    let cell = buf.get_mut(x, y);
                    cell.set_char(' ');
                    x += 1;
                }
            }

            // Render text
            for span in &item.text.spans {
                let consumed = render_text_unicode_aware(
                    &span.content,
                    buf,
                    x,
                    y,
                    inner.x + inner.width,
                    span.style.patch(item_style),
                );
                x += consumed;
            }
        }
    }
}

/// Modern button widget with hover and press states
pub struct ModernButton<'a> {
    text: &'a str,
    theme: &'a ModernTheme,
    pressed: bool,
    hovered: bool,
    variant: ButtonVariant,
}

#[derive(Clone, Copy)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
}

impl<'a> ModernButton<'a> {
    pub fn new(text: &'a str, theme: &'a ModernTheme) -> Self {
        Self {
            text,
            theme,
            pressed: false,
            hovered: false,
            variant: ButtonVariant::Primary,
        }
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl<'a> Widget for ModernButton<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 3 || area.height < 1 {
            return;
        }

        // Choose style based on variant and state
        let (bg_color, fg_color) = match self.variant {
            ButtonVariant::Primary => (self.theme.primary, self.theme.text_primary),
            ButtonVariant::Secondary => (self.theme.secondary, self.theme.text_primary),
            ButtonVariant::Success => (self.theme.success, self.theme.text_primary),
            ButtonVariant::Warning => (self.theme.warning, self.theme.text_primary),
            ButtonVariant::Danger => (self.theme.danger, self.theme.text_primary),
        };

        let button_style = if self.pressed {
            Style::default()
                .fg(fg_color)
                .bg(bg_color)
                .add_modifier(Modifier::DIM)
        } else if self.hovered {
            Style::default()
                .fg(fg_color)
                .bg(bg_color)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(fg_color).bg(bg_color)
        };

        // Fill background
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = buf.get_mut(x, y);
                cell.set_char(' ');
                cell.set_style(button_style);
            }
        }

        // Render text centered
        let text_width = self.text.width();
        if text_width <= area.width as usize {
            let x_offset = (area.width as usize - text_width) / 2;
            let y_offset = area.height / 2;

            render_text_unicode_aware(
                self.text,
                buf,
                area.x + x_offset as u16,
                area.y + y_offset,
                area.x + area.width,
                button_style,
            );
        }
    }
}

/// Modern gauge with custom styling
pub struct ModernGauge<'a> {
    ratio: f64,
    label: Option<&'a str>,
    theme: &'a ModernTheme,
    variant: ProgressVariant,
    show_ratio: bool,
}

impl<'a> ModernGauge<'a> {
    pub fn new(ratio: f64, theme: &'a ModernTheme) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            label: None,
            theme,
            variant: ProgressVariant::Auto,
            show_ratio: true,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn variant(mut self, variant: ProgressVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn show_ratio(mut self, show: bool) -> Self {
        self.show_ratio = show;
        self
    }
}

impl<'a> Widget for ModernGauge<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Convert to percentage for styling
        let percentage = self.ratio * 100.0;

        let progress_bar = ModernProgressBar::new(percentage, self.theme)
            .variant(self.variant)
            .show_percentage(self.show_ratio);

        if let Some(label) = self.label {
            progress_bar.label(label).render(area, buf);
        } else {
            progress_bar.render(area, buf);
        }
    }
}

/// Helper function to create a modern styled block
pub fn modern_block<'a>(
    title: Option<&'a str>,
    theme: &'a ModernTheme,
    focused: bool,
) -> Block<'a> {
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(if focused {
            theme.border_focused_style()
        } else {
            theme.border_style()
        });

    if let Some(title) = title {
        block = block.title(title);
    }

    block
}

/// Create a styled icon span
pub fn icon_span<'a>(icon: &'a str, style: Style) -> Span<'a> {
    Span::styled(icon, style)
}

/// Create a metric display span (for numbers)
pub fn metric_span<'a>(text: String, theme: &'a ModernTheme) -> Span<'a> {
    Span::styled(text, theme.metric_style())
}

/// Create a status icon based on boolean state
pub fn status_icon(active: bool) -> &'static str {
    if active {
        ModernIcons::ACTIVE
    } else {
        ModernIcons::INACTIVE
    }
}

/// Create priority icon based on priority level
pub fn priority_icon(priority: &str) -> &'static str {
    match priority.to_lowercase().as_str() {
        "high" => ModernIcons::HIGH_PRIORITY,
        "medium" => ModernIcons::MEDIUM_PRIORITY,
        "low" => ModernIcons::LOW_PRIORITY,
        _ => ModernIcons::BULLET,
    }
}

/// Format project name with consistent truncation across the UI
/// Handles Unicode characters properly and provides consistent display
pub fn format_project_name(project_name: &str, max_width: usize) -> String {
    if project_name.width() <= max_width {
        project_name.to_string()
    } else {
        // Use grapheme clusters to handle Unicode properly
        let mut result = String::new();
        let mut current_width = 0;
        let ellipsis = "...";
        let ellipsis_width = ellipsis.width();
        let target_width = max_width.saturating_sub(ellipsis_width);

        for grapheme in project_name.graphemes(true) {
            let grapheme_width = grapheme.width();
            if current_width + grapheme_width > target_width {
                break;
            }
            result.push_str(grapheme);
            current_width += grapheme_width;
        }

        result.push_str(ellipsis);
        result
    }
}
