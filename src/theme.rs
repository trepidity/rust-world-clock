use crate::config::CustomThemeConfig;
use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub border: Color,
    pub border_alarm: Color,
    pub title: Color,
    pub time_text: Color,
    pub date_text: Color,
    pub label_text: Color,
    pub edit_selected: Color,
    pub edit_empty: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_name("default")
    }
}

impl Theme {
    pub fn from_name(name: &str) -> Self {
        match name {
            "dracula" => Self::dracula(),
            "nord" => Self::nord(),
            "solarized_dark" => Self::solarized_dark(),
            "gruvbox" => Self::gruvbox(),
            "monokai" => Self::monokai(),
            _ => Self::default_theme(),
        }
    }

    pub fn from_custom(config: &CustomThemeConfig) -> Self {
        let base = Self::default_theme();
        Theme {
            background: config.background.as_deref().map(parse_color).unwrap_or(base.background),
            border: config.border.as_deref().map(parse_color).unwrap_or(base.border),
            border_alarm: config.border_alarm.as_deref().map(parse_color).unwrap_or(base.border_alarm),
            title: config.title.as_deref().map(parse_color).unwrap_or(base.title),
            time_text: config.time_text.as_deref().map(parse_color).unwrap_or(base.time_text),
            date_text: config.date_text.as_deref().map(parse_color).unwrap_or(base.date_text),
            label_text: config.label_text.as_deref().map(parse_color).unwrap_or(base.label_text),
            edit_selected: base.edit_selected,
            edit_empty: base.edit_empty,
        }
    }

    pub fn available_themes() -> &'static [&'static str] {
        &["default", "dracula", "nord", "solarized_dark", "gruvbox", "monokai"]
    }

    fn default_theme() -> Self {
        Theme {
            background: Color::Reset,
            border: Color::White,
            border_alarm: Color::Red,
            title: Color::Yellow,
            time_text: Color::Cyan,
            date_text: Color::Gray,
            label_text: Color::Yellow,
            edit_selected: Color::LightYellow,
            edit_empty: Color::DarkGray,
        }
    }

    fn dracula() -> Self {
        Theme {
            background: Color::Rgb(40, 42, 54),
            border: Color::Rgb(98, 114, 164),
            border_alarm: Color::Rgb(255, 85, 85),
            title: Color::Rgb(241, 250, 140),
            time_text: Color::Rgb(139, 233, 253),
            date_text: Color::Rgb(98, 114, 164),
            label_text: Color::Rgb(80, 250, 123),
            edit_selected: Color::Rgb(255, 121, 198),
            edit_empty: Color::Rgb(68, 71, 90),
        }
    }

    fn nord() -> Self {
        Theme {
            background: Color::Rgb(46, 52, 64),
            border: Color::Rgb(76, 86, 106),
            border_alarm: Color::Rgb(191, 97, 106),
            title: Color::Rgb(235, 203, 139),
            time_text: Color::Rgb(136, 192, 208),
            date_text: Color::Rgb(76, 86, 106),
            label_text: Color::Rgb(163, 190, 140),
            edit_selected: Color::Rgb(180, 142, 173),
            edit_empty: Color::Rgb(59, 66, 82),
        }
    }

    fn solarized_dark() -> Self {
        Theme {
            background: Color::Rgb(0, 43, 54),
            border: Color::Rgb(88, 110, 117),
            border_alarm: Color::Rgb(220, 50, 47),
            title: Color::Rgb(181, 137, 0),
            time_text: Color::Rgb(38, 139, 210),
            date_text: Color::Rgb(88, 110, 117),
            label_text: Color::Rgb(133, 153, 0),
            edit_selected: Color::Rgb(211, 54, 130),
            edit_empty: Color::Rgb(7, 54, 66),
        }
    }

    fn gruvbox() -> Self {
        Theme {
            background: Color::Rgb(40, 40, 40),
            border: Color::Rgb(146, 131, 116),
            border_alarm: Color::Rgb(204, 36, 29),
            title: Color::Rgb(250, 189, 47),
            time_text: Color::Rgb(131, 165, 152),
            date_text: Color::Rgb(146, 131, 116),
            label_text: Color::Rgb(184, 187, 38),
            edit_selected: Color::Rgb(211, 134, 155),
            edit_empty: Color::Rgb(60, 56, 54),
        }
    }

    fn monokai() -> Self {
        Theme {
            background: Color::Rgb(39, 40, 34),
            border: Color::Rgb(117, 113, 94),
            border_alarm: Color::Rgb(249, 38, 114),
            title: Color::Rgb(230, 219, 116),
            time_text: Color::Rgb(102, 217, 239),
            date_text: Color::Rgb(117, 113, 94),
            label_text: Color::Rgb(166, 226, 46),
            edit_selected: Color::Rgb(174, 129, 255),
            edit_empty: Color::Rgb(62, 61, 50),
        }
    }
}

pub fn parse_color(s: &str) -> Color {
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex[0..2], 16),
                u8::from_str_radix(&hex[2..4], 16),
                u8::from_str_radix(&hex[4..6], 16),
            ) {
                return Color::Rgb(r, g, b);
            }
        }
    }
    match s.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::White,
    }
}
