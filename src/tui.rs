use crate::Clock;
use chrono::{Local, NaiveTime, Timelike, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use std::{io, time::Duration};

pub fn run(clocks: &[Clock], alarms: &[NaiveTime]) -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let res = run_app_loop(&mut terminal, clocks, alarms);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app_loop<B: Backend>(terminal: &mut Terminal<B>, clocks: &[Clock], alarms: &[NaiveTime]) -> io::Result<()> 
where
    std::io::Error: From<B::Error>,
{
    let mut dismissed_time: Option<NaiveTime> = None;

    loop {
        let local_now = Local::now().time();
        
        // Reset dismissal if minute changed
        if let Some(dismissed) = dismissed_time {
            if local_now.hour() != dismissed.hour() || local_now.minute() != dismissed.minute() {
                dismissed_time = None;
            }
        }

        let is_alarm_active = alarms.iter().any(|&alarm| {
            local_now.hour() == alarm.hour() && local_now.minute() == alarm.minute()
        }) && dismissed_time.is_none();

        terminal.draw(|f| ui(f, clocks, is_alarm_active))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => return Ok(()),
                        KeyCode::Char(' ') | KeyCode::Char('d') => {
                            if is_alarm_active {
                                dismissed_time = Some(NaiveTime::from_hms_opt(local_now.hour(), local_now.minute(), 0).unwrap());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, clocks: &[Clock], is_alarm_active: bool) {
    let size = f.area();
    let clock_count = clocks.len();
    
    if clock_count == 0 {
        return;
    }

    // Simple grid layout logic
    // Calculate columns and rows based on count to try and keep it square-ish
    let cols = (clock_count as f64).sqrt().ceil() as usize;
    let rows = (clock_count as f64 / cols as f64).ceil() as usize;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            std::iter::repeat(Constraint::Ratio(1, rows as u32))
                .take(rows)
                .collect::<Vec<_>>(),
        )
        .split(size);

    for (i, clock) in clocks.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;

        if row >= chunks.len() {
            break;
        }

        let row_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                std::iter::repeat(Constraint::Ratio(1, cols as u32))
                    .take(cols)
                    .collect::<Vec<_>>(),
            )
            .split(chunks[row]);
        
        if col >= row_chunks.len() {
             break;
        }

        let area = row_chunks[col];
        
        let time = Utc::now().with_timezone(&clock.timezone);
        let time_str = time.format("%H:%M:%S").to_string();
        let date_str = time.format("%Y-%m-%d").to_string();

        let text = vec![
            Line::from(Span::styled(
                &clock.name,
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                time_str,
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD), // font_size isn't real in TUI
            )),
            Line::from(Span::styled(
                date_str,
                Style::default().fg(Color::Gray),
            )),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        // Centering vertically is a bit manual in basic TUI without Flex, 
        // but let's just render the paragraph in the block.
        // To center vertically effectively, we can use a layout inside the block or padding
        // simplified here to just fill the block.
        
        // Let's try to center it vertically by calculating padding
        let content_height = 4; // 4 lines of text
        let block_height = area.height.saturating_sub(2); // minus borders
        let v_padding = block_height.saturating_sub(content_height) / 2;
        
        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(v_padding),
                Constraint::Length(content_height),
                Constraint::Min(0),
            ])
            .split(area)[1];

        f.render_widget(paragraph, inner_area);
        
        let border_color = if is_alarm_active {
            Color::Red
        } else {
            Color::White
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(clock.name.clone())
            .border_style(Style::default().fg(border_color));

        f.render_widget(block, area);
    }
}
