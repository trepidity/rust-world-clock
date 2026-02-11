use crate::config::{self, AppConfig, WidgetKind};
use crate::grid::Grid;
use crate::theme::Theme;
use crate::widget::ClockWidget;
use chrono_tz::Tz;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

pub enum AppMode {
    Normal,
    Edit(EditState),
}

#[derive(Debug)]
pub struct EditState {
    pub selected: (u16, u16),
    pub drag: DragState,
    pub popup: Option<Popup>,
    pub dirty: bool,
    pub status_message: Option<String>,
}

impl EditState {
    pub fn new() -> Self {
        EditState {
            selected: (0, 0),
            drag: DragState::Idle,
            popup: None,
            dirty: false,
            status_message: None,
        }
    }
}

#[derive(Debug)]
pub enum DragState {
    Idle,
    Dragging { source: (u16, u16) },
}

#[derive(Debug)]
pub enum Popup {
    AddWidget(AddWidgetPopup),
    ThemePicker(ThemePickerState),
}

#[derive(Debug)]
pub struct AddWidgetPopup {
    pub step: AddWidgetStep,
    pub search_query: String,
    pub filtered_zones: Vec<String>,
    pub list_state: ListState,
    pub selected_timezone: Option<String>,
    pub selected_kind: Option<WidgetKind>,
    pub label_input: String,
    all_zones: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub enum AddWidgetStep {
    SearchTimezone,
    PickWidgetType,
    EditLabel,
}

impl AddWidgetPopup {
    pub fn new() -> Self {
        let all_zones: Vec<String> = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|tz| tz.name().to_string())
            .collect();
        let filtered_zones = all_zones.clone();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        AddWidgetPopup {
            step: AddWidgetStep::SearchTimezone,
            search_query: String::new(),
            filtered_zones,
            list_state,
            selected_timezone: None,
            selected_kind: None,
            label_input: String::new(),
            all_zones,
        }
    }

    pub fn update_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_zones = self
            .all_zones
            .iter()
            .filter(|z| z.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if self.filtered_zones.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }
}

#[derive(Debug)]
pub struct ThemePickerState {
    pub list_state: ListState,
    pub themes: Vec<String>,
}

impl ThemePickerState {
    pub fn new() -> Self {
        let themes: Vec<String> = Theme::available_themes()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        ThemePickerState { list_state, themes }
    }
}

pub enum EditAction {
    None,
    ExitEditMode,
    Quit,
}

pub fn handle_edit_key(
    key: KeyEvent,
    edit_state: &mut EditState,
    grid: &mut Grid,
    theme: &mut Theme,
    config: &mut AppConfig,
) -> EditAction {
    // If a popup is active, delegate to popup handler
    if edit_state.popup.is_some() {
        return handle_popup_key(key, edit_state, grid, theme, config);
    }

    match key.code {
        KeyCode::Esc => return EditAction::ExitEditMode,
        KeyCode::Char('q') => return EditAction::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return EditAction::Quit
        }

        // Arrow key navigation
        KeyCode::Up => {
            if edit_state.selected.0 > 0 {
                edit_state.selected.0 -= 1;
            }
        }
        KeyCode::Down => {
            if edit_state.selected.0 < grid.rows - 1 {
                edit_state.selected.0 += 1;
            }
        }
        KeyCode::Left => {
            if edit_state.selected.1 > 0 {
                edit_state.selected.1 -= 1;
            }
        }
        KeyCode::Right => {
            if edit_state.selected.1 < grid.cols - 1 {
                edit_state.selected.1 += 1;
            }
        }

        // Add widget
        KeyCode::Char('a') => {
            edit_state.popup = Some(Popup::AddWidget(AddWidgetPopup::new()));
        }

        // Delete widget
        KeyCode::Char('x') | KeyCode::Delete => {
            grid.remove_cell(edit_state.selected);
            edit_state.dirty = true;
        }

        // Cycle widget type
        KeyCode::Char('t') => {
            grid.cycle_widget_kind(edit_state.selected);
            edit_state.dirty = true;
        }

        // Theme picker
        KeyCode::Char('h') => {
            edit_state.popup = Some(Popup::ThemePicker(ThemePickerState::new()));
        }

        // Save
        KeyCode::Char('s') => {
            config.grid.rows = grid.rows;
            config.grid.cols = grid.cols;
            config.cells = config::config_to_cells(grid);
            config::save_config(config);
            edit_state.dirty = false;
            edit_state.status_message = Some("Saved!".to_string());
        }

        // Grid resize
        KeyCode::Char(']') => {
            grid.add_column();
            edit_state.dirty = true;
        }
        KeyCode::Char('[') => {
            grid.remove_column();
            // Clamp selection
            if edit_state.selected.1 >= grid.cols {
                edit_state.selected.1 = grid.cols - 1;
            }
            edit_state.dirty = true;
        }
        KeyCode::Char('}') => {
            grid.add_row();
            edit_state.dirty = true;
        }
        KeyCode::Char('{') => {
            grid.remove_row();
            if edit_state.selected.0 >= grid.rows {
                edit_state.selected.0 = grid.rows - 1;
            }
            edit_state.dirty = true;
        }

        _ => {}
    }

    EditAction::None
}

fn handle_popup_key(
    key: KeyEvent,
    edit_state: &mut EditState,
    grid: &mut Grid,
    theme: &mut Theme,
    config: &mut AppConfig,
) -> EditAction {
    let popup = edit_state.popup.as_mut().unwrap();

    match popup {
        Popup::AddWidget(add_popup) => {
            match add_popup.step {
                AddWidgetStep::SearchTimezone => match key.code {
                    KeyCode::Esc => {
                        edit_state.popup = None;
                    }
                    KeyCode::Enter => {
                        if let Some(selected_idx) = add_popup.list_state.selected() {
                            if selected_idx < add_popup.filtered_zones.len() {
                                add_popup.selected_timezone =
                                    Some(add_popup.filtered_zones[selected_idx].clone());
                                add_popup.step = AddWidgetStep::PickWidgetType;
                                add_popup.list_state.select(Some(0));
                            }
                        }
                    }
                    KeyCode::Up => {
                        if let Some(sel) = add_popup.list_state.selected() {
                            if sel > 0 {
                                add_popup.list_state.select(Some(sel - 1));
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(sel) = add_popup.list_state.selected() {
                            if sel + 1 < add_popup.filtered_zones.len() {
                                add_popup.list_state.select(Some(sel + 1));
                            }
                        }
                    }
                    KeyCode::Char(c) => {
                        add_popup.search_query.push(c);
                        add_popup.update_filter();
                    }
                    KeyCode::Backspace => {
                        add_popup.search_query.pop();
                        add_popup.update_filter();
                    }
                    _ => {}
                },
                AddWidgetStep::PickWidgetType => match key.code {
                    KeyCode::Esc => {
                        add_popup.step = AddWidgetStep::SearchTimezone;
                    }
                    KeyCode::Enter => {
                        if let Some(selected_idx) = add_popup.list_state.selected() {
                            let kinds = WidgetKind::all();
                            if selected_idx < kinds.len() {
                                add_popup.selected_kind = Some(kinds[selected_idx].clone());
                                add_popup.step = AddWidgetStep::EditLabel;
                                add_popup.list_state.select(Some(0));
                                if let Some(ref tz) = add_popup.selected_timezone {
                                    add_popup.label_input = tz.clone();
                                }
                            }
                        }
                    }
                    KeyCode::Up => {
                        if let Some(sel) = add_popup.list_state.selected() {
                            if sel > 0 {
                                add_popup.list_state.select(Some(sel - 1));
                            }
                        }
                    }
                    KeyCode::Down => {
                        if let Some(sel) = add_popup.list_state.selected() {
                            let kinds = WidgetKind::all();
                            if sel + 1 < kinds.len() {
                                add_popup.list_state.select(Some(sel + 1));
                            }
                        }
                    }
                    _ => {}
                },
                AddWidgetStep::EditLabel => match key.code {
                    KeyCode::Esc => {
                        add_popup.step = AddWidgetStep::PickWidgetType;
                    }
                    KeyCode::Enter => {
                        if let Some(ref tz_name) = add_popup.selected_timezone {
                            if let Ok(tz) = tz_name.parse::<Tz>() {
                                let kind = add_popup
                                    .selected_kind
                                    .clone()
                                    .unwrap_or(WidgetKind::TimeAndDate);
                                let label = if add_popup.label_input.is_empty() {
                                    None
                                } else {
                                    Some(add_popup.label_input.clone())
                                };
                                let widget = ClockWidget::new(tz_name, tz, kind, label);
                                grid.set_cell(edit_state.selected, widget);
                                edit_state.dirty = true;
                            }
                        }
                        edit_state.popup = None;
                    }
                    KeyCode::Char(c) => {
                        add_popup.label_input.push(c);
                    }
                    KeyCode::Backspace => {
                        add_popup.label_input.pop();
                    }
                    _ => {}
                },
            }
        }
        Popup::ThemePicker(picker) => match key.code {
            KeyCode::Esc => {
                edit_state.popup = None;
            }
            KeyCode::Enter => {
                if let Some(sel) = picker.list_state.selected() {
                    if sel < picker.themes.len() {
                        let theme_name = &picker.themes[sel];
                        *theme = Theme::from_name(theme_name);
                        config.theme = theme_name.clone();
                        edit_state.dirty = true;
                    }
                }
                edit_state.popup = None;
            }
            KeyCode::Up => {
                if let Some(sel) = picker.list_state.selected() {
                    if sel > 0 {
                        picker.list_state.select(Some(sel - 1));
                    }
                }
            }
            KeyCode::Down => {
                if let Some(sel) = picker.list_state.selected() {
                    if sel + 1 < picker.themes.len() {
                        picker.list_state.select(Some(sel + 1));
                    }
                }
            }
            _ => {}
        },
    }

    EditAction::None
}

pub fn handle_edit_mouse(
    mouse: MouseEvent,
    edit_state: &mut EditState,
    grid: &mut Grid,
    cell_rects: &[Vec<Rect>],
) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(pos) = hit_test(mouse.column, mouse.row, cell_rects) {
                edit_state.selected = pos;
                edit_state.drag = DragState::Dragging { source: pos };
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let DragState::Dragging { source } = edit_state.drag {
                if let Some(target) = hit_test(mouse.column, mouse.row, cell_rects) {
                    if source != target {
                        grid.swap_cells(source, target);
                        edit_state.selected = target;
                        edit_state.dirty = true;
                    }
                }
            }
            edit_state.drag = DragState::Idle;
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // Visual feedback could be added here
        }
        _ => {}
    }
}

fn hit_test(col: u16, row: u16, cell_rects: &[Vec<Rect>]) -> Option<(u16, u16)> {
    for (r, row_rects) in cell_rects.iter().enumerate() {
        for (c, rect) in row_rects.iter().enumerate() {
            if col >= rect.x
                && col < rect.x + rect.width
                && row >= rect.y
                && row < rect.y + rect.height
            {
                return Some((r as u16, c as u16));
            }
        }
    }
    None
}

pub fn render_popup(f: &mut Frame, popup: &mut Popup, theme: &Theme) {
    let area = f.area();
    let popup_width = 44.min(area.width.saturating_sub(4));
    let popup_height = 16.min(area.height.saturating_sub(4));

    let popup_area = Rect::new(
        (area.width.saturating_sub(popup_width)) / 2,
        (area.height.saturating_sub(popup_height)) / 2,
        popup_width,
        popup_height,
    );

    f.render_widget(Clear, popup_area);

    match popup {
        Popup::AddWidget(add_popup) => {
            render_add_widget_popup(f, popup_area, add_popup, theme);
        }
        Popup::ThemePicker(picker) => {
            render_theme_picker_popup(f, popup_area, picker, theme);
        }
    }
}

fn render_add_widget_popup(
    f: &mut Frame,
    area: Rect,
    popup: &mut AddWidgetPopup,
    theme: &Theme,
) {
    match popup.step {
        AddWidgetStep::SearchTimezone => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Add Clock ")
                .title_style(Style::default().fg(theme.title).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(theme.border));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // padding
                    Constraint::Length(1), // search label
                    Constraint::Length(1), // padding
                    Constraint::Min(1),    // list
                    Constraint::Length(1), // hint
                ])
                .split(inner);

            let search_line = Line::from(vec![
                Span::styled("  Search: ", Style::default().fg(theme.label_text)),
                Span::styled(
                    format!("{}_", popup.search_query),
                    Style::default().fg(theme.time_text),
                ),
            ]);
            f.render_widget(Paragraph::new(search_line), chunks[1]);

            let items: Vec<ListItem> = popup
                .filtered_zones
                .iter()
                .take(chunks[3].height as usize)
                .map(|z| {
                    ListItem::new(format!("  {}", z))
                        .style(Style::default().fg(theme.date_text))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .fg(theme.time_text)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[3], &mut popup.list_state);

            let hint = Line::from(Span::styled(
                "  Up/Down: navigate  Enter: select  Esc: cancel",
                Style::default().fg(theme.edit_empty),
            ));
            f.render_widget(Paragraph::new(hint), chunks[4]);
        }
        AddWidgetStep::PickWidgetType => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Widget Type ")
                .title_style(Style::default().fg(theme.title).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(theme.border));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // padding
                    Constraint::Length(1), // timezone info
                    Constraint::Length(1), // padding
                    Constraint::Min(1),    // list
                    Constraint::Length(1), // hint
                ])
                .split(inner);

            let tz_info = Line::from(vec![
                Span::styled("  Timezone: ", Style::default().fg(theme.label_text)),
                Span::styled(
                    popup.selected_timezone.as_deref().unwrap_or(""),
                    Style::default().fg(theme.time_text),
                ),
            ]);
            f.render_widget(Paragraph::new(tz_info), chunks[1]);

            let kinds = WidgetKind::all();
            let items: Vec<ListItem> = kinds
                .iter()
                .map(|k| {
                    ListItem::new(format!("  {}", k.label()))
                        .style(Style::default().fg(theme.date_text))
                })
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .fg(theme.time_text)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[3], &mut popup.list_state);

            let hint = Line::from(Span::styled(
                "  Enter: select  Esc: back",
                Style::default().fg(theme.edit_empty),
            ));
            f.render_widget(Paragraph::new(hint), chunks[4]);
        }
        AddWidgetStep::EditLabel => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Label ")
                .title_style(Style::default().fg(theme.title).add_modifier(Modifier::BOLD))
                .border_style(Style::default().fg(theme.border));

            let inner = block.inner(area);
            f.render_widget(block, area);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // padding
                    Constraint::Length(1), // tz info
                    Constraint::Length(1), // padding
                    Constraint::Length(1), // label prompt
                    Constraint::Length(1), // label input
                    Constraint::Min(1),    // spacer
                    Constraint::Length(1), // hint
                ])
                .split(inner);

            let tz_info = Line::from(vec![
                Span::styled("  Timezone: ", Style::default().fg(theme.label_text)),
                Span::styled(
                    popup.selected_timezone.as_deref().unwrap_or(""),
                    Style::default().fg(theme.time_text),
                ),
            ]);
            f.render_widget(Paragraph::new(tz_info), chunks[1]);

            let label_prompt = Line::from(Span::styled(
                "  Label (Enter to confirm):",
                Style::default().fg(theme.label_text),
            ));
            f.render_widget(Paragraph::new(label_prompt), chunks[3]);

            let label_line = Line::from(Span::styled(
                format!("  {}_", popup.label_input),
                Style::default().fg(theme.time_text),
            ));
            f.render_widget(Paragraph::new(label_line), chunks[4]);

            let hint = Line::from(Span::styled(
                "  Enter: confirm  Esc: back",
                Style::default().fg(theme.edit_empty),
            ));
            f.render_widget(Paragraph::new(hint), chunks[6]);
        }
    }
}

fn render_theme_picker_popup(
    f: &mut Frame,
    area: Rect,
    picker: &mut ThemePickerState,
    theme: &Theme,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Theme ")
        .title_style(Style::default().fg(theme.title).add_modifier(Modifier::BOLD))
        .border_style(Style::default().fg(theme.border));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Min(1),    // list
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let items: Vec<ListItem> = picker
        .themes
        .iter()
        .map(|t| {
            ListItem::new(format!("  {}", t)).style(Style::default().fg(theme.date_text))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(theme.time_text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[1], &mut picker.list_state);

    let hint = Line::from(Span::styled(
        "  Up/Down: navigate  Enter: select  Esc: cancel",
        Style::default().fg(theme.edit_empty),
    ));
    f.render_widget(Paragraph::new(hint), chunks[2]);
}
