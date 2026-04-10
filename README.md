# Rust World Clock

A terminal-based world clock application written in Rust. Display the current time for multiple time zones in a customizable grid layout, with built-in themes, an interactive edit mode, daily alarms, and full configuration persistence.

<img width="1007" height="463" alt="image" src="https://github.com/user-attachments/assets/dccf73b1-3dd9-4a9e-87ef-d8776379b389" />

<!-- TODO: Replace with updated screenshot showing grid layout + theme -->

## Features

- **Customizable Grid Layout** — Arrange clocks in a configurable row/column grid
- **6 Built-in Themes** — Default, Dracula, Nord, Solarized Dark, Gruvbox, Monokai (plus custom theme support)
- **Interactive Edit Mode** — Drag-and-drop widgets, add/remove rows and columns, pick themes, all with mouse and keyboard
- **3 Widget Display Modes** — Time only, date only, or time and date
- **Daily Alarms** — Set local-time alarms with visual alerts (red borders)
- **TOML Configuration** — Full grid layout, theme, and widget config persisted automatically
- **GUI Mode** — Optional graphical interface via iced

## Installation

Ensure you have [Rust and Cargo installed](https://rustup.rs/).

```bash
git clone https://github.com/yourusername/rust-world-clock.git
cd rust-world-clock
cargo build --release
```

## Development

Enable the repo's pre-push hook so pushes are blocked unless formatting, Clippy, and tests all pass:

```bash
git config core.hooksPath .githooks
chmod +x .githooks/pre-push
```

## Usage

### Basic Usage

Specify the time zones you want to display:

```bash
cargo run -- America/New_York Europe/London Asia/Tokyo
```

### Setting Alarms

Use the `--alarms` flag to set daily alarms (24-hour format, local time):

```bash
cargo run -- --alarms 09:00 17:30 America/New_York Europe/London
```

### Custom Config File

```bash
cargo run -- --config ~/my-clocks.toml
```

### GUI Mode

```bash
cargo run -- --gui
```

### Persistence

Settings are saved to a platform-specific config directory:

| Platform | Config Directory |
| :--- | :--- |
| macOS | `~/Library/Application Support/rust-world-clock/` |
| Linux | `~/.config/rust-world-clock/` |
| Windows | `%APPDATA%\rust-world-clock\config\` |

Files stored in this directory:

| File | Contents |
| :--- | :--- |
| `config.toml` | Grid layout, themes, and widget configuration |
| `clocks.json` | Timezone list |
| `alarms.json` | Alarm times |

Running without arguments loads your last configuration. Running with new arguments updates the saved config.

## Controls

### Normal Mode

| Key | Action |
| :--- | :--- |
| `q` / `Ctrl+C` | Quit |
| `Space` / `d` | Dismiss active alarm |
| `e` | Enter edit mode |

### Edit Mode

<img width="1116" height="582" alt="image" src="https://github.com/user-attachments/assets/beb66828-f0bd-4a2e-a996-f65e0fbb0d0f" />

<img width="1116" height="582" alt="image" src="https://github.com/user-attachments/assets/c8a29aa6-1050-4c0c-9799-de06698af272" />

<img width="1116" height="582" alt="image" src="https://github.com/user-attachments/assets/e0964bb2-9573-407b-8d31-0973131d1e58" />

<img width="1116" height="582" alt="image" src="https://github.com/user-attachments/assets/96ca94b0-04d8-4f19-928c-dee9fab5f233" />

| Key | Action |
| :--- | :--- |
| Arrow keys | Move cell selection |
| Mouse drag | Swap widgets between cells |
| `a` | Add new widget (opens timezone search) |
| `x` / `Delete` | Remove widget from selected cell |
| `t` | Cycle display mode (Time+Date / Time / Date) |
| `h` | Open theme picker |
| `]` / `[` | Add / remove column |
| `}` / `{` | Add / remove row |
| `s` | Save configuration |
| `Esc` | Exit edit mode |

## Themes

Six built-in themes are available, selectable via the theme picker (`h` in edit mode) or in `config.toml`:

<img width="1032" height="378" alt="image" src="https://github.com/user-attachments/assets/57b1148a-cf8e-48dd-ae3b-5c5dbcfd8395" />


| Theme | Description |
| :--- | :--- |
| `default` | Classic terminal colors |
| `dracula` | Purple and pink accents on dark background |
| `nord` | Cool blue and teal tones |
| `solarized_dark` | Ethan Schoonover's dark palette |
| `gruvbox` | Warm retro colors |
| `monokai` | Vibrant syntax-inspired colors |

### Custom Themes

Set `theme = "custom"` in your config and define colors under `[custom_theme]`:

```toml
theme = "custom"

[custom_theme]
background = "#1a1b26"
border = "#565f89"
border_alarm = "#f7768e"
title = "#c0caf5"
time_text = "#9ece6a"
date_text = "#565f89"
label_text = "#7aa2f7"
```

Colors accept hex values (`#ff5555`) or named colors (`red`, `cyan`, etc.).

## Configuration

The full grid layout is configured via TOML at `config.toml` in your config directory (see [Persistence](#persistence) for the path on your platform):

```toml
theme = "dracula"
always_on_top = true

[grid]
rows = 2
cols = 3

[[cells]]
row = 0
col = 0
timezone = "America/New_York"
widget = "time_and_date"
label = "New York"

[[cells]]
row = 0
col = 1
timezone = "Europe/London"
widget = "time_only"
label = "London"

[[cells]]
row = 1
col = 0
timezone = "Asia/Tokyo"
widget = "date_only"
label = "Tokyo"
```

Widget types: `time_and_date`, `time_only`, `date_only`

Set `always_on_top = false` if you want the GUI window to behave like a normal window instead of staying above other apps.

## License

MIT
