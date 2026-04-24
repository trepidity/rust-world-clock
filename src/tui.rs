use crate::Clock;
use crate::config::AppConfig;
use crate::edit_mode::{self, AppMode, EditAction, EditState};
use crate::grid::Grid;
use crate::theme::Theme;
use chrono::{Local, NaiveTime, Timelike};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::{io, time::Duration};

pub fn run(
    clocks: &[Clock],
    alarms: &[NaiveTime],
    config: AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Resolve theme
    let theme = if config.theme == "custom" {
        if let Some(ref custom) = config.custom_theme {
            Theme::from_custom(custom)
        } else {
            Theme::from_name(&config.theme)
        }
    } else {
        Theme::from_name(&config.theme)
    };

    // Build grid
    let grid = Grid::from_config(&config, clocks);

    // Run app
    let res = run_app_loop(&mut terminal, grid, theme, alarms, config);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut grid: Grid,
    mut theme: Theme,
    alarms: &[NaiveTime],
    mut config: AppConfig,
) -> io::Result<()>
where
    std::io::Error: From<B::Error>,
{
    let mut dismissed_time: Option<NaiveTime> = None;
    let mut mode = AppMode::Normal;

    loop {
        let local_now = Local::now().time();

        // Reset dismissal if minute changed
        if let Some(dismissed) = dismissed_time
            && (local_now.hour() != dismissed.hour() || local_now.minute() != dismissed.minute())
        {
            dismissed_time = None;
        }

        let is_alarm_active = alarms
            .iter()
            .any(|&alarm| local_now.hour() == alarm.hour() && local_now.minute() == alarm.minute())
            && dismissed_time.is_none();

        // Draw
        match &mut mode {
            AppMode::Normal => {
                terminal.draw(|f| {
                    let area = f.area();
                    if theme.background != Color::Reset {
                        let bg = ratatui::widgets::Block::default()
                            .style(Style::default().bg(theme.background));
                        f.render_widget(bg, area);
                    }
                    grid.render(f, area, &theme, is_alarm_active, None);
                })?;
            }
            AppMode::Edit(edit_state) => {
                terminal.draw(|f| {
                    let area = f.area();
                    if theme.background != Color::Reset {
                        let bg = ratatui::widgets::Block::default()
                            .style(Style::default().bg(theme.background));
                        f.render_widget(bg, area);
                    }
                    grid.render(f, area, &theme, is_alarm_active, Some(&*edit_state));

                    if let Some(popup) = &mut edit_state.popup {
                        edit_mode::render_popup(f, popup, &theme);
                    }
                })?;
            }
        }

        // Handle events
        if event::poll(Duration::from_millis(100))? {
            let ev = event::read()?;

            match &mut mode {
                AppMode::Normal => {
                    if let Event::Key(key) = ev
                        && key.kind == KeyEventKind::Press
                    {
                        match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('c')
                                if key.modifiers.contains(event::KeyModifiers::CONTROL) =>
                            {
                                return Ok(());
                            }
                            KeyCode::Char(' ') | KeyCode::Char('d') if is_alarm_active => {
                                dismissed_time = Some(
                                    NaiveTime::from_hms_opt(
                                        local_now.hour(),
                                        local_now.minute(),
                                        0,
                                    )
                                    .unwrap(),
                                );
                            }
                            KeyCode::Char('e') => {
                                mode = AppMode::Edit(EditState::new());
                            }
                            _ => {}
                        }
                    }
                }
                AppMode::Edit(edit_state) => match ev {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match edit_mode::handle_edit_key(
                            key,
                            edit_state,
                            &mut grid,
                            &mut theme,
                            &mut config,
                        ) {
                            EditAction::ExitEditMode => {
                                mode = AppMode::Normal;
                            }
                            EditAction::Quit => return Ok(()),
                            EditAction::None => {}
                        }
                    }
                    Event::Mouse(mouse) if edit_state.popup.is_none() => {
                        let area = terminal.get_frame().area();
                        let cell_rects = grid.cell_rects(area);
                        edit_mode::handle_edit_mouse(mouse, edit_state, &mut grid, &cell_rects);
                    }
                    _ => {}
                },
            }
        }
    }
}
