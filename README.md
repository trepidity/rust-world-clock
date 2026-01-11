# Rust World Clock

A terminal-based world clock application written in Rust. It displays the current time for multiple time zones in a tiled layout, supports daily alarms, and persists your configuration between sessions.

<img width="1007" height="463" alt="image" src="https://github.com/user-attachments/assets/dccf73b1-3dd9-4a9e-87ef-d8776379b389" />


## Features

*   **Multi-Timezone Support**: Display any number of world clocks side-by-side.
*   **Tiled TUI Layout**: Automatically arranges clocks in a grid based on available space.
*   **Alarms**: Set multiple daily alarms (in your local time).
    *   **Visual Alert**: Clock borders turn red when an alarm is active.
    *   **Dismissal**: Dismiss alarms with a key press.
*   **Persistence**: Automatically saves your configured time zones and alarms.
*   **Default Fallback**: Defaults to `Europe/London` if no configuration is found.

## Installation

Ensure you have [Rust and Cargo installed](https://rustup.rs/).

```bash
git clone https://github.com/yourusername/rust-world-clock.git
cd rust-world-clock
cargo build --release
```

## Usage

Run the application using `cargo run`.

### Basic Usage

Specify the time zones you want to display:

```bash
cargo run -- America/New_York Europe/London Asia/Tokyo
```

### Setting Alarms

Use the `--alarms` flag to set daily alarms (in 24-hour format, local time):

```bash
cargo run --alarms 09:00 17:30 -- America/New_York
```

### Persistence

The application automatically saves your settings to your user configuration directory (e.g., `~/.config/rust-world-clock/` on Linux/macOS).
- Running `cargo run` without arguments will load your last used configuration.
- Running with new arguments will update the saved configuration.

### Controls

| Key | Action |
| :--- | :--- |
| `q` or `Ctrl+C` | Quit the application |
| `Space` or `d` | Dismiss an active alarm |

## License

MIT
