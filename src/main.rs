/*
 * Rust World Clock
 * Author: Jared L Jennings
 * Description: A terminal-based world clock application that displays multiple time zones
 *              in a tiled layout, supports local-time alarms, and persists user configuration.
 */

mod tui;
mod gui;

use chrono::{NaiveTime, Utc};
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

    /// Run in GUI mode
    #[arg(long)]
    gui: bool,
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
        load_clocks()
    };
    
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

    if args.gui {
        gui::run(clocks, alarms)?;
    } else {
        tui::run(&clocks, &alarms)?;
    }

    Ok(())
}
