use chrono::Utc;
use chrono_tz::Tz;
use clap::Parser;
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of time zones to display (e.g., "America/New_York" "Europe/London")
    #[arg(num_args = 1.., required = true)]
    zones: Vec<String>,
}

struct Clock {
    name: String,
    timezone: Tz,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    let mut clocks = Vec::new();
    for zone_str in args.zones {
        match zone_str.parse::<Tz>() {
            Ok(tz) => {
                clocks.push(Clock {
                    name: zone_str,
                    timezone: tz,
                });
            }
            Err(_) => {
                eprintln!("Invalid time zone: {}", zone_str);
                return Ok(());
            }
        }
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let res = run_app(&mut terminal, &clocks);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, clocks: &[Clock]) -> io::Result<()> 
where
    std::io::Error: From<B::Error>,
{
    loop {
        terminal.draw(|f| ui(f, clocks))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if key.code == KeyCode::Char('q') {
                        return Ok(());
                    }
                    if key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, clocks: &[Clock]) {
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
        
        // Draw the border around the full area
        f.render_widget(Block::default().borders(Borders::ALL).title(clock.name.clone()), area);
    }
}
