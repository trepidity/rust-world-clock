use crate::config::WidgetKind;
use crate::theme::Theme;
use chrono::Utc;
use chrono_tz::Tz;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Paragraph},
};

#[derive(Debug, Clone)]
pub struct ClockWidget {
    pub label: String,
    pub timezone: Tz,
    pub timezone_name: String,
    pub kind: WidgetKind,
}

impl ClockWidget {
    pub fn new(timezone_name: &str, timezone: Tz, kind: WidgetKind, label: Option<String>) -> Self {
        let display_label = label.unwrap_or_else(|| timezone_name.to_string());
        ClockWidget {
            label: display_label,
            timezone,
            timezone_name: timezone_name.to_string(),
            kind,
        }
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        is_alarm_active: bool,
        is_selected: bool,
    ) {
        let time = Utc::now().with_timezone(&self.timezone);
        let time_str = time.format("%H:%M:%S %Z").to_string();
        let date_str = time.format("%Y-%m-%d").to_string();

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(Span::styled(
            &self.label,
            Style::default()
                .fg(theme.label_text)
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        match &self.kind {
            WidgetKind::TimeOnly => {
                lines.push(Line::from(Span::styled(
                    &time_str,
                    Style::default()
                        .fg(theme.time_text)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            WidgetKind::DateOnly => {
                lines.push(Line::from(Span::styled(
                    &date_str,
                    Style::default().fg(theme.date_text),
                )));
            }
            WidgetKind::TimeAndDate => {
                lines.push(Line::from(Span::styled(
                    &time_str,
                    Style::default()
                        .fg(theme.time_text)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(Span::styled(
                    &date_str,
                    Style::default().fg(theme.date_text),
                )));
            }
        }

        let content_height = lines.len() as u16;
        let block_height = area.height.saturating_sub(2);
        let v_padding = block_height.saturating_sub(content_height) / 2;

        let paragraph = Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_padding),
                Constraint::Length(content_height),
                Constraint::Min(0),
            ])
            .split(area)[1];

        f.render_widget(paragraph, inner_area);

        let border_color = if is_selected {
            theme.edit_selected
        } else if is_alarm_active {
            theme.border_alarm
        } else {
            theme.border
        };

        let (border_type, title_style) = if is_selected {
            (
                BorderType::Double,
                Style::default()
                    .fg(theme.edit_selected)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            (BorderType::Plain, Style::default().fg(theme.title))
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(self.label.clone())
            .title_style(title_style)
            .border_type(border_type)
            .border_style(Style::default().fg(border_color));

        f.render_widget(block, area);
    }
}
