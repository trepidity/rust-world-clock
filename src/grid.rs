use crate::config::{AppConfig, WidgetKind};
use crate::edit_mode::EditState;
use crate::theme::Theme;
use crate::widget::ClockWidget;
use crate::Clock;
use chrono_tz::Tz;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

#[derive(Debug, Clone)]
pub struct Grid {
    pub rows: u16,
    pub cols: u16,
    pub cells: Vec<Vec<Option<ClockWidget>>>,
}

impl Grid {
    pub fn from_config(config: &AppConfig, clocks: &[Clock]) -> Self {
        if !config.cells.is_empty() && config.grid.rows > 0 && config.grid.cols > 0 {
            Self::from_explicit_config(config)
        } else {
            Self::from_clocks(clocks)
        }
    }

    fn from_clocks(clocks: &[Clock]) -> Self {
        if clocks.is_empty() {
            return Grid {
                rows: 1,
                cols: 1,
                cells: vec![vec![None]],
            };
        }

        let count = clocks.len();
        let cols = (count as f64).sqrt().ceil() as u16;
        let rows = (count as f64 / cols as f64).ceil() as u16;

        let mut cells = vec![vec![None; cols as usize]; rows as usize];

        for (i, clock) in clocks.iter().enumerate() {
            let r = i / cols as usize;
            let c = i % cols as usize;
            cells[r][c] = Some(ClockWidget::new(
                &clock.name,
                clock.timezone,
                WidgetKind::TimeAndDate,
                None,
            ));
        }

        Grid { rows, cols, cells }
    }

    fn from_explicit_config(config: &AppConfig) -> Self {
        let rows = config.grid.rows;
        let cols = config.grid.cols;
        let mut cells = vec![vec![None; cols as usize]; rows as usize];

        for cell_config in &config.cells {
            if (cell_config.row as usize) < rows as usize
                && (cell_config.col as usize) < cols as usize
            {
                if let Ok(tz) = cell_config.timezone.parse::<Tz>() {
                    cells[cell_config.row as usize][cell_config.col as usize] =
                        Some(ClockWidget::new(
                            &cell_config.timezone,
                            tz,
                            cell_config.widget.clone(),
                            cell_config.label.clone(),
                        ));
                }
            }
        }

        Grid { rows, cols, cells }
    }

    pub fn render(
        &self,
        f: &mut Frame,
        area: Rect,
        theme: &Theme,
        is_alarm_active: bool,
        edit_state: Option<&EditState>,
    ) {
        if self.rows == 0 || self.cols == 0 {
            return;
        }

        // Reserve space for status bar in edit mode
        let (grid_area, status_area) = if edit_state.is_some() {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

        let row_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                std::iter::repeat(Constraint::Ratio(1, self.rows as u32))
                    .take(self.rows as usize)
                    .collect::<Vec<_>>(),
            )
            .split(grid_area);

        for r in 0..self.rows as usize {
            if r >= row_chunks.len() {
                break;
            }

            let col_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    std::iter::repeat(Constraint::Ratio(1, self.cols as u32))
                        .take(self.cols as usize)
                        .collect::<Vec<_>>(),
                )
                .split(row_chunks[r]);

            for c in 0..self.cols as usize {
                if c >= col_chunks.len() {
                    break;
                }

                let cell_area = col_chunks[c];
                let is_selected = edit_state
                    .is_some_and(|es| es.selected == (r as u16, c as u16));

                match &self.cells[r][c] {
                    Some(widget) => {
                        widget.render(f, cell_area, theme, is_alarm_active, is_selected);
                    }
                    None => {
                        if edit_state.is_some() {
                            render_empty_cell(f, cell_area, theme, is_selected);
                        } else {
                            let block = Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(theme.border));
                            f.render_widget(block, cell_area);
                        }
                    }
                }
            }
        }

        // Render status bar in edit mode
        if let (Some(edit_state), Some(status_area)) = (edit_state, status_area) {
            render_status_bar(f, status_area, edit_state, self, theme);
        }
    }

    pub fn cell_rects(&self, area: Rect) -> Vec<Vec<Rect>> {
        // Reserve status bar space
        let grid_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(area)[0];

        let row_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                std::iter::repeat(Constraint::Ratio(1, self.rows as u32))
                    .take(self.rows as usize)
                    .collect::<Vec<_>>(),
            )
            .split(grid_area);

        let mut rects = Vec::new();
        for r in 0..self.rows as usize {
            let col_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    std::iter::repeat(Constraint::Ratio(1, self.cols as u32))
                        .take(self.cols as usize)
                        .collect::<Vec<_>>(),
                )
                .split(row_chunks[r]);
            rects.push(col_chunks.to_vec());
        }
        rects
    }

    pub fn swap_cells(&mut self, from: (u16, u16), to: (u16, u16)) {
        if from == to {
            return;
        }
        let (fr, fc) = (from.0 as usize, from.1 as usize);
        let (tr, tc) = (to.0 as usize, to.1 as usize);
        if fr < self.rows as usize
            && fc < self.cols as usize
            && tr < self.rows as usize
            && tc < self.cols as usize
        {
            let temp = self.cells[fr][fc].take();
            self.cells[fr][fc] = self.cells[tr][tc].take();
            self.cells[tr][tc] = temp;
        }
    }

    pub fn set_cell(&mut self, pos: (u16, u16), widget: ClockWidget) {
        let (r, c) = (pos.0 as usize, pos.1 as usize);
        if r < self.rows as usize && c < self.cols as usize {
            self.cells[r][c] = Some(widget);
        }
    }

    pub fn remove_cell(&mut self, pos: (u16, u16)) {
        let (r, c) = (pos.0 as usize, pos.1 as usize);
        if r < self.rows as usize && c < self.cols as usize {
            self.cells[r][c] = None;
        }
    }

    pub fn add_column(&mut self) {
        self.cols += 1;
        for row in &mut self.cells {
            row.push(None);
        }
    }

    pub fn remove_column(&mut self) {
        if self.cols <= 1 {
            return;
        }
        self.cols -= 1;
        for row in &mut self.cells {
            row.pop();
        }
    }

    pub fn add_row(&mut self) {
        self.rows += 1;
        self.cells.push(vec![None; self.cols as usize]);
    }

    pub fn remove_row(&mut self) {
        if self.rows <= 1 {
            return;
        }
        self.rows -= 1;
        self.cells.pop();
    }

    pub fn cycle_widget_kind(&mut self, pos: (u16, u16)) {
        let (r, c) = (pos.0 as usize, pos.1 as usize);
        if r < self.rows as usize && c < self.cols as usize {
            if let Some(widget) = &mut self.cells[r][c] {
                widget.kind = widget.kind.cycle();
            }
        }
    }
}

fn render_empty_cell(f: &mut Frame, area: Rect, theme: &Theme, is_selected: bool) {
    let border_color = if is_selected {
        theme.edit_selected
    } else {
        theme.edit_empty
    };

    let border_type = if is_selected {
        ratatui::widgets::BorderType::Double
    } else {
        ratatui::widgets::BorderType::Plain
    };

    let text_color = theme.edit_empty;

    let lines = vec![
        Line::from(Span::styled("[+]", Style::default().fg(text_color))),
        Line::from(Span::styled(
            "click or 'a'",
            Style::default().fg(text_color),
        )),
        Line::from(Span::styled(
            "to add clock",
            Style::default().fg(text_color),
        )),
    ];

    let content_height = lines.len() as u16;
    let block_height = area.height.saturating_sub(2);
    let v_padding = block_height.saturating_sub(content_height) / 2;

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true });

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(v_padding),
            Constraint::Length(content_height),
            Constraint::Min(0),
        ])
        .split(area)[1];

    f.render_widget(paragraph, inner);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(border_type)
        .border_style(Style::default().fg(border_color));

    f.render_widget(block, area);
}

fn render_status_bar(f: &mut Frame, area: Rect, edit_state: &EditState, grid: &Grid, theme: &Theme) {
    let dirty_flag = if edit_state.dirty { " [modified]" } else { "" };

    let left = format!(" EDIT{}", dirty_flag);
    let center = "a:add  x:del  t:type  h:theme  ]/[:col  }}/{{:row  s:save  Esc:exit";
    let right = format!("{}x{} ", grid.cols, grid.rows);

    let total_width = area.width as usize;
    let left_len = left.len();
    let right_len = right.len();
    let center_len = center.len();

    let available_for_center = total_width.saturating_sub(left_len + right_len);
    let center_padding = available_for_center.saturating_sub(center_len) / 2;

    let mut line_spans = vec![
        Span::styled(left, Style::default().fg(Color::Black).bg(theme.edit_selected).add_modifier(Modifier::BOLD)),
    ];

    let pad = " ".repeat(center_padding);
    line_spans.push(Span::styled(
        format!("{}{}{}", pad, center, pad),
        Style::default().fg(theme.edit_empty).bg(Color::Reset),
    ));

    // Right-align grid dimensions
    let remaining = total_width.saturating_sub(left_len + center_len + center_padding * 2 + right_len);
    if remaining > 0 {
        line_spans.push(Span::raw(" ".repeat(remaining)));
    }
    line_spans.push(Span::styled(
        right,
        Style::default().fg(theme.label_text).add_modifier(Modifier::BOLD),
    ));

    let status_line = Line::from(line_spans);
    let paragraph = Paragraph::new(status_line);
    f.render_widget(paragraph, area);
}
