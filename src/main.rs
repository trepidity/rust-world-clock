/*
 * Rust World Clock
 * Author: Jared L Jennings
 * Description: A terminal-based world clock application that displays multiple time zones
 *              in a tiled layout, supports local-time alarms, and persists user configuration.
 */

mod config;
mod edit_mode;
mod grid;
mod gui;
mod theme;
mod time_conversion;
mod tui;
mod tz_abbrev;
mod widget;

use chrono::NaiveTime;
use chrono_tz::Tz;
use clap::Parser;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, time::Duration};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// List of time zones to display (e.g., "America/New_York" "Europe/London")
    #[arg(num_args = 0..)]
    zones: Vec<String>,

    /// Alarms in HH:MM format (local time)
    #[arg(long, num_args = 1..)]
    alarms: Vec<String>,

    /// Run in GUI mode (default; retained for compatibility)
    #[arg(long)]
    gui: bool,

    /// Run in terminal UI mode
    #[arg(long)]
    tui: bool,

    /// Path to config file
    #[arg(long)]
    config: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct StoredClocks(Vec<String>);

#[derive(Serialize, Deserialize)]
struct StoredAlarms(Vec<String>);

#[derive(Clone, Debug)] // Added Clone/Debug for Iced
pub struct Clock {
    pub name: String,
    pub timezone: Tz,
}

pub fn get_config_dir() -> Option<PathBuf> {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "rust-world-clock") {
        let config_dir = proj_dirs.config_dir();
        if !config_dir.exists() {
            let _ = fs::create_dir_all(config_dir);
        }
        Some(config_dir.to_path_buf())
    } else {
        None
    }
}

pub fn save_clocks(zones: &[String]) {
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
        if let Ok(content) = fs::read_to_string(path)
            && let Ok(stored) = serde_json::from_str::<StoredClocks>(&content)
        {
            return stored.0;
        }
    }
    Vec::new()
}

fn save_alarms(alarms: &[NaiveTime]) {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join("alarms.json");
        let alarm_strings: Vec<String> = alarms
            .iter()
            .map(|t| t.format("%H:%M").to_string())
            .collect();
        let stored = StoredAlarms(alarm_strings);
        if let Ok(json) = serde_json::to_string(&stored) {
            let _ = fs::write(path, json);
        }
    }
}

fn load_alarms() -> Vec<NaiveTime> {
    if let Some(config_dir) = get_config_dir() {
        let path = config_dir.join("alarms.json");
        if let Ok(content) = fs::read_to_string(path)
            && let Ok(stored) = serde_json::from_str::<StoredAlarms>(&content)
        {
            return stored
                .0
                .iter()
                .filter_map(|s| NaiveTime::parse_from_str(s, "%H:%M").ok())
                .collect();
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
    }

    // Load app config (theme, grid, cells)
    let app_config = config::load_config(args.config.as_deref());

    // Handle Clocks
    let mut clocks = Vec::new();
    let zones_from_cli = !args.zones.is_empty();
    let zone_strs = if zones_from_cli {
        save_clocks(&args.zones);
        args.zones
    } else {
        load_clocks()
    };

    let zone_strs = if zone_strs.is_empty() {
        println!("No timezones specified and no configuration found.");
        println!("To customize, run: cargo run -- <TimeZones...>");
        println!("Example: cargo run -- America/New_York Europe/London");
        println!("Defaulting to Europe/London in 3 seconds...");
        std::thread::sleep(Duration::from_secs(3));
        vec!["Europe/London".to_string(), "Asia/Kolkata".to_string()]
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
                if zones_from_cli {
                    eprintln!("Invalid time zone: {}", zone_str);
                    return Ok(());
                }

                eprintln!("Ignoring invalid saved time zone: {}", zone_str);
            }
        }
    }

    if clocks.is_empty() {
        println!("No valid saved time zones found.");
        println!("To customize, run: cargo run -- <TimeZones...>");
        println!("Example: cargo run -- America/New_York Europe/London");
        println!("Defaulting to Europe/London and Asia/Kolkata in 3 seconds...");
        std::thread::sleep(Duration::from_secs(3));
        clocks = vec![
            Clock {
                name: "Europe/London".to_string(),
                timezone: "Europe/London"
                    .parse::<Tz>()
                    .expect("default timezone should be valid"),
            },
            Clock {
                name: "Asia/Kolkata".to_string(),
                timezone: "Asia/Kolkata"
                    .parse::<Tz>()
                    .expect("default timezone should be valid"),
            },
        ];
    }

    if args.tui && !args.gui {
        tui::run(&clocks, &alarms, app_config)?;
    } else {
        gui::run(clocks, alarms, app_config.always_on_top)?;
    }

    Ok(())
}
