use chrono::{Local, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use directories::ProjectDirs;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of time zones to display (e.g., "America/New_York" "Europe/London")
    #[arg(num_args = 0..)]
    zones: Vec<String>,

    /// Alarms in HH:MM format (local time)
    #[arg(long, num_args = 1..)]
    alarms: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredClocks(Vec<String>);

#[derive(Serialize, Deserialize)]
struct StoredAlarms(Vec<String>);

struct Clock {
    name: String,
    timezone: Tz,
}

fn get_config_dir() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("com", "rust_world_clock", "rust_world_clock") {
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = fs::create_dir_all(config_dir);
        }
        Some(config_dir.to_path_buf())
    } else {
        None
    }
}

fn save_clocks(zones: &[String]) {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join("clocks.json");
        let stored = StoredClocks(zones.to_vec());
        if let Ok(json) = serde_json::to_string(&stored) {
            let _ = fs::write(path, json);
        }
    }
}

fn load_clocks() -> Vec<String> {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join("clocks.json");
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(stored) = serde_json::from_str::<StoredClocks>(&content) {
                return stored.0;
            }
        }
    }
    Vec::new()
}

fn save_alarms(alarms: &[NaiveTime]) {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join("alarms.json");
        let alarm_strings: Vec<String> = alarms.iter().map(|t| t.format("%H:%M").to_string()).collect();
        let stored = StoredAlarms(alarm_strings);
        if let Ok(json) = serde_json::to_string(&stored) {
            let _ = fs::write(path, json);
        }
    }
}

fn load_alarms() -> Vec<NaiveTime> {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join("alarms.json");
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(stored) = serde_json::from_str::<StoredAlarms>(&content) {
                return stored.0
                    .iter()
                    .filter_map(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
                    .collect();
            }
        }
    }
    Vec::new()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Handle Alarms
    let mut alarms = Vec::new();
    if !args.alarms.is_empty() {
        // Alarms provided via CLI: Parse, use, and save them.
        for alarm_str in &args.alarms {
             match NaiveTime::parse_from_str(alarm_str, "%H:%M") {
                Ok(time) => alarms.push(time),
                Err(_) => {
                    eprintln!("Invalid alarm format: {}", alarm_str);
                    return Ok(());
                }
            }
        }
        save_alarms(&alarms);
    } else {
        // No alarms via CLI: Try to load from config.
        alarms = load_alarms();
        if alarms.is_empty() && args.alarms.is_empty() { 
            // Optional: You could choose to do nothing or set defaults.
            // For now, empty is fine.
        }
    }

    // Handle Clocks
    let mut clocks = Vec::new();
    let zone_strs = if !args.zones.is_empty() {
        save_clocks(&args.zones);
        args.zones
    } else {
        // This case might be tricky because `zones` is required=true in Clap args.
        // We'll address this by relaxing the requirement or handling it logic-wise?
        // Wait, if it's required=true, clap errors before we get here if it's empty.
        // We'll need to make it optional in Args struct first.
        load_clocks()
    };
    
    // If after loading we still have nothing, we should probably default or error.
    // Since we are changing `zones` to be optional in next step, we handle empty here.
    // If after loading we still have nothing, we should probably default or error.
    // We let the user know, then default to London.
    let zone_strs = if zone_strs.is_empty() {
        println!("No timezones specified and no configuration found.");
        println!("To customize, run: cargo run -- <TimeZones...>");
        println!("Example: cargo run -- America/New_York Europe/London");
        println!("Defaulting to Europe/London in 3 seconds...");
        std::thread::sleep(Duration::from_secs(3));
        vec!["Europe/London".to_string()]
    } else {
        zone_strs
    };

    for zone_str in zone_strs {
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
    let res = run_app(&mut terminal, &clocks, &alarms);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, clocks: &[Clock], alarms: &[NaiveTime]) -> io::Result<()> 
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
