use crate::grid::Grid;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WidgetKind {
    TimeOnly,
    DateOnly,
    #[default]
    TimeAndDate,
}

impl WidgetKind {
    pub fn cycle(&self) -> WidgetKind {
        match self {
            WidgetKind::TimeOnly => WidgetKind::DateOnly,
            WidgetKind::DateOnly => WidgetKind::TimeAndDate,
            WidgetKind::TimeAndDate => WidgetKind::TimeOnly,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            WidgetKind::TimeOnly => "Time Only",
            WidgetKind::DateOnly => "Date Only",
            WidgetKind::TimeAndDate => "Time and Date",
        }
    }

    pub fn all() -> &'static [WidgetKind] {
        &[
            WidgetKind::TimeAndDate,
            WidgetKind::TimeOnly,
            WidgetKind::DateOnly,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GridConfig {
    #[serde(default)]
    pub rows: u16,
    #[serde(default)]
    pub cols: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CellConfig {
    pub row: u16,
    pub col: u16,
    pub timezone: String,
    #[serde(default)]
    pub widget: WidgetKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomThemeConfig {
    pub background: Option<String>,
    pub border: Option<String>,
    pub border_alarm: Option<String>,
    pub title: Option<String>,
    pub time_text: Option<String>,
    pub date_text: Option<String>,
    pub label_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub grid: GridConfig,
    #[serde(default)]
    pub cells: Vec<CellConfig>,
    pub custom_theme: Option<CustomThemeConfig>,
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            theme: default_theme(),
            grid: GridConfig::default(),
            cells: Vec::new(),
            custom_theme: None,
        }
    }
}

pub fn config_path(custom: Option<&str>) -> PathBuf {
    if let Some(path) = custom {
        PathBuf::from(path)
    } else if let Some(config_dir) = crate::get_config_dir() {
        config_dir.join("config.toml")
    } else {
        PathBuf::from("config.toml")
    }
}

pub fn load_config(custom_path: Option<&str>) -> AppConfig {
    if custom_path.is_none() {
        write_default_config();
    }

    let path = config_path(custom_path);
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(config) => return config,
                Err(e) => {
                    eprintln!("Warning: Failed to parse config: {}", e);
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read config: {}", e);
            }
        }
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) {
    let path = config_path(None);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match toml::to_string_pretty(config) {
        Ok(content) => {
            if let Err(e) = fs::write(&path, content) {
                eprintln!("Failed to save config: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to serialize config: {}", e);
        }
    }
}

pub fn write_default_config() {
    let path = config_path(None);
    if !path.exists() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let template = include_str!("default_config.toml");
        let _ = fs::write(&path, template);
    }
}

pub fn config_to_cells(grid: &Grid) -> Vec<CellConfig> {
    let mut cells = Vec::new();
    for r in 0..grid.rows {
        for c in 0..grid.cols {
            if let Some(widget) = &grid.cells[r as usize][c as usize] {
                cells.push(CellConfig {
                    row: r,
                    col: c,
                    timezone: widget.timezone_name.clone(),
                    widget: widget.kind.clone(),
                    label: Some(widget.label.clone()),
                });
            }
        }
    }
    cells
}
