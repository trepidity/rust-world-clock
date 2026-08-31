use crate::Clock;
use crate::time_conversion::{self, TimeConversion};
use crate::timezone_search::{self, TimezoneSearchMatch};
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use iced::{
    Background, Border, Color, Element, Fill, Font, Shadow, Size, Subscription, Task, Theme,
    Vector, application, border, keyboard,
    keyboard::key::Named,
    widget::{button, column, container, row, stack, text, text_input},
    window,
};
use std::time::Duration;

const INITIAL_WINDOW_SIZE: Size = Size::new(1024.0, 768.0);
const MIN_WINDOW_SIZE: Size = Size::new(360.0, 220.0);

pub fn run(clocks: Vec<Clock>, alarms: Vec<NaiveTime>, always_on_top: bool) -> iced::Result {
    let initial = WorldClockApp {
        clocks,
        added_clock_count: 0,
        alarms,
        home_timezone: detect_home_timezone(),
        local_time: Local::now().time(),
        local_date: Local::now().date_naive(),
        converter: ConverterState::default(),
        window_size: INITIAL_WINDOW_SIZE,
    };

    application(move || (initial.clone(), Task::none()), update, view)
        .title(app_title)
        .theme(app_theme)
        .window(app_window_settings(always_on_top))
        .subscription(subscription)
        .centered()
        .run()
}

fn app_window_settings(always_on_top: bool) -> window::Settings {
    window::Settings {
        size: INITIAL_WINDOW_SIZE,
        min_size: Some(MIN_WINDOW_SIZE),
        level: if always_on_top {
            window::Level::AlwaysOnTop
        } else {
            window::Level::Normal
        },
        icon: app_icon(),
        ..window::Settings::default()
    }
}

fn app_icon() -> Option<window::Icon> {
    const SIZE: u32 = 64;

    window::icon::from_rgba(render_app_icon(SIZE), SIZE, SIZE).ok()
}

fn render_app_icon(size: u32) -> Vec<u8> {
    let mut pixels = vec![0; (size * size * 4) as usize];
    let size_f = size as f32;
    let center = (size_f - 1.0) * 0.5;
    let radius = size_f * 0.295;
    let ring_radius = size_f * 0.34;
    let ring_thickness = size_f * 0.0625;
    let corner_radius = size_f * 0.2;

    for y in 0..size {
        for x in 0..size {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            let dx = xf - center;
            let dy = yf - center;

            let bg_mask = rounded_rect_mask(
                xf,
                yf,
                size_f * 0.08,
                size_f * 0.08,
                size_f * 0.84,
                size_f * 0.84,
                corner_radius,
            );
            let bg = lerp_color(
                [0x1B, 0x22, 0x2D, 0xFF],
                [0x10, 0x14, 0x1D, 0xFF],
                yf / size_f,
            );
            blend_pixel(&mut pixels, size, x, y, bg, bg_mask);

            let dist = (dx * dx + dy * dy).sqrt();
            let ring_mask = stroke_circle_mask(dist, ring_radius, ring_thickness);
            let ring = lerp_color(
                [0xF5, 0xA6, 0x23, 0xFF], // Gold
                [0xFF, 0x66, 0xAA, 0xFF], // Pink
                yf / size_f,
            );
            blend_pixel(&mut pixels, size, x, y, ring, ring_mask);

            let inner_clip = smoothstep(radius + size_f * 0.012, radius - size_f * 0.012, dist);

            let parallel_main = line_mask(yf, center, size_f * 0.045) * inner_clip;
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0xFF, 0xFF, 0xFF, 0xFF], // White
                parallel_main,
            );

            let parallel_top = line_mask(yf, center - size_f * 0.085, size_f * 0.032) * inner_clip;
            let parallel_bottom =
                line_mask(yf, center + size_f * 0.085, size_f * 0.032) * inner_clip;
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0x00, 0xE5, 0xFF, 0xFF], // Cyan
                parallel_top,
            );
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0x00, 0xE5, 0xFF, 0xFF], // Cyan
                parallel_bottom,
            );

            let meridian_left =
                ellipse_mask(dx + size_f * 0.085, dy, size_f * 0.12, radius) * inner_clip;
            let meridian_right =
                ellipse_mask(dx - size_f * 0.085, dy, size_f * 0.12, radius) * inner_clip;
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0x00, 0xE5, 0xFF, 0xFF], // Cyan
                meridian_left * 0.95,
            );
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0x00, 0xE5, 0xFF, 0xFF], // Cyan
                meridian_right * 0.95,
            );

            let hour_hand = segment_mask(dx, dy, 0.0, 0.0, 0.0, -size_f * 0.17, size_f * 0.032);
            let minute_hand = segment_mask(
                dx,
                dy,
                0.0,
                0.0,
                size_f * 0.13,
                size_f * 0.065,
                size_f * 0.032,
            );
            blend_pixel(&mut pixels, size, x, y, [0xF8, 0xFA, 0xFC, 0xFF], hour_hand);
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0xF8, 0xFA, 0xFC, 0xFF],
                minute_hand,
            );

            let hub = filled_circle_mask(dx, dy, size_f * 0.042);
            blend_pixel(&mut pixels, size, x, y, [0xF8, 0xFA, 0xFC, 0xFF], hub);

            let accent = filled_circle_mask(dx, dy + size_f * 0.27, size_f * 0.026);
            blend_pixel(&mut pixels, size, x, y, [0xF9, 0x73, 0x16, 0xFF], accent);
        }
    }

    pixels
}

fn blend_pixel(pixels: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4], alpha: f32) {
    if alpha <= 0.0 {
        return;
    }

    let index = ((y * width + x) * 4) as usize;
    let src_alpha = alpha.clamp(0.0, 1.0) * (color[3] as f32 / 255.0);
    let dst_alpha = pixels[index + 3] as f32 / 255.0;
    let out_alpha = src_alpha + dst_alpha * (1.0 - src_alpha);

    if out_alpha <= 0.0 {
        return;
    }

    for channel in 0..3 {
        let src = color[channel] as f32 / 255.0;
        let dst = pixels[index + channel] as f32 / 255.0;
        let out = (src * src_alpha + dst * dst_alpha * (1.0 - src_alpha)) / out_alpha;
        pixels[index + channel] = (out * 255.0).round() as u8;
    }

    pixels[index + 3] = (out_alpha * 255.0).round() as u8;
}

fn rounded_rect_mask(
    x: f32,
    y: f32,
    left: f32,
    top: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> f32 {
    let cx = x.clamp(left + radius, left + width - radius);
    let cy = y.clamp(top + radius, top + height - radius);
    let dx = x - cx;
    let dy = y - cy;

    smoothstep(1.0, -1.0, (dx * dx + dy * dy).sqrt() - radius)
}

fn stroke_circle_mask(distance: f32, radius: f32, thickness: f32) -> f32 {
    smoothstep(1.0, -1.0, (distance - radius).abs() - thickness * 0.5)
}

fn filled_circle_mask(dx: f32, dy: f32, radius: f32) -> f32 {
    smoothstep(1.0, -1.0, (dx * dx + dy * dy).sqrt() - radius)
}

fn ellipse_mask(dx: f32, dy: f32, radius_x: f32, radius_y: f32) -> f32 {
    let normalized = ((dx * dx) / (radius_x * radius_x) + (dy * dy) / (radius_y * radius_y)).sqrt();

    smoothstep(0.04, -0.04, normalized - 1.0)
}

fn line_mask(value: f32, center: f32, thickness: f32) -> f32 {
    smoothstep(1.0, -1.0, (value - center).abs() - thickness * 0.5)
}

fn segment_mask(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32, thickness: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let ab_len_sq = abx * abx + aby * aby;
    let t = ((apx * abx + apy * aby) / ab_len_sq).clamp(0.0, 1.0);
    let closest_x = ax + abx * t;
    let closest_y = ay + aby * t;
    let dx = px - closest_x;
    let dy = py - closest_y;

    smoothstep(1.0, -1.0, (dx * dx + dy * dy).sqrt() - thickness * 0.5)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_color(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);

    [
        lerp_channel(a[0], b[0], t),
        lerp_channel(a[1], b[1], t),
        lerp_channel(a[2], b[2], t),
        lerp_channel(a[3], b[3], t),
    ]
}

fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[derive(Clone)]
struct WorldClockApp {
    clocks: Vec<Clock>,
    added_clock_count: usize,
    alarms: Vec<NaiveTime>,
    home_timezone: Option<Tz>,
    local_time: NaiveTime,
    local_date: NaiveDate,
    converter: ConverterState,
    window_size: Size,
}

#[derive(Debug, Clone, Default)]
struct ConverterState {
    is_open: bool,
    mode: ConverterMode,
    input: String,
    outcome: Option<ConverterOutcome>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ConverterMode {
    #[default]
    TimeConversion,
    AddZone,
}

#[derive(Debug, Clone)]
enum ConverterOutcome {
    Converted(TimeConversion),
    ZoneAdded(String),
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    Tick(NaiveDateTime),
    Keyboard(keyboard::Event),
    WindowResized(Size),
    ConverterInputChanged(String),
    SubmitConversion,
    SelectZone(String),
    AddClock,
    RemoveClock,
}

fn update(app: &mut WorldClockApp, message: Message) {
    match message {
        Message::Tick(now) => {
            app.local_time = now.time();
            app.local_date = now.date();
        }
        Message::Keyboard(event) => {
            handle_keyboard(app, event);
        }
        Message::WindowResized(size) => {
            app.window_size = size;
        }
        Message::ConverterInputChanged(input) => {
            app.converter.input = input;
            app.converter.outcome = None;
        }
        Message::SubmitConversion => {
            submit_overlay(app);
        }
        Message::SelectZone(zone_name) => {
            app.converter.input = zone_name;
            submit_overlay(app);
        }
        Message::AddClock => {
            open_add_zone(app);
        }
        Message::RemoveClock => {
            if app.added_clock_count > 0 && !app.clocks.is_empty() {
                app.clocks.pop();
                app.added_clock_count -= 1;
                persist_clocks(app);
            }
        }
    }
}

fn subscription(_app: &WorldClockApp) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(Duration::from_millis(500))
            .map(|_| Message::Tick(Local::now().naive_local())),
        keyboard::listen().map(Message::Keyboard),
        window::resize_events().map(|(_id, size)| Message::WindowResized(size)),
    ])
}

fn app_title(app: &WorldClockApp) -> String {
    format!(
        "Rust World Clock \u{2014} {}",
        format_title_date(app.local_date)
    )
}

/// Weekday abbreviation plus the long local date, e.g. `Mon, August 31, 2026`.
fn format_title_date(date: NaiveDate) -> String {
    date.format("%a, %B %-d, %Y").to_string()
}

fn app_theme(_app: &WorldClockApp) -> Theme {
    Theme::Dark
}

fn view(app: &WorldClockApp) -> Element<'_, Message> {
    let is_alarm_active = app.alarms.iter().any(|&alarm| {
        app.local_time.hour() == alarm.hour() && app.local_time.minute() == alarm.minute()
    });
    let metrics = ResponsiveMetrics::new(app.window_size, app.clocks.len());
    let can_remove_added_clock = app.added_clock_count > 0;

    let mut grid = column![].spacing(metrics.card_gap);
    let mut current_row = row![].spacing(metrics.card_gap);

    for (i, clock) in app.clocks.iter().enumerate() {
        let is_home_timezone = app
            .home_timezone
            .as_ref()
            .is_some_and(|home_timezone| clock.timezone == *home_timezone);

        current_row = current_row.push(clock_card(
            clock,
            is_alarm_active,
            is_home_timezone,
            metrics,
        ));

        if (i + 1) % 2 == 0 || (i + 1) == app.clocks.len() {
            grid = grid.push(current_row);
            current_row = row![].spacing(metrics.card_gap);
        }
    }

    let footer = row![
        button(
            container(text("+").size(metrics.badge_size * 1.2))
                .padding([metrics.badge_padding_y, metrics.badge_padding_x * 1.5])
                .center_x(Fill)
        )
        .on_press(Message::AddClock)
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => rgb8(75, 85, 99),
                _ => rgb8(55, 65, 81),
            };
            button::Style {
                background: Some(Background::Color(bg)),
                border: border::rounded(8),
                text_color: Color::WHITE,
                ..button::Style::default()
            }
        }),
        button(
            container(text("-").size(metrics.badge_size * 1.2))
                .padding([metrics.badge_padding_y, metrics.badge_padding_x * 1.5])
                .center_x(Fill)
        )
        .on_press(Message::RemoveClock)
        .style(move |_theme, status| {
            let (bg, text_color) = if can_remove_added_clock {
                let bg = match status {
                    button::Status::Hovered | button::Status::Pressed => rgb8(75, 85, 99),
                    _ => rgb8(55, 65, 81),
                };

                (bg, Color::WHITE)
            } else {
                (rgb8(31, 41, 55), rgb8(107, 114, 128))
            };

            button::Style {
                background: Some(Background::Color(bg)),
                border: border::rounded(8),
                text_color,
                ..button::Style::default()
            }
        }),
    ]
    .spacing(metrics.card_gap);

    let base = container(
        column![grid, footer]
            .spacing(metrics.card_gap)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Fill)
    .height(Fill)
    .padding(metrics.window_padding)
    .center_x(Fill)
    .center_y(Fill)
    .style(|_| {
        container::Style::default()
            .background(rgb8(16, 20, 29)) // Sophisticated charcoal
            .color(Color::WHITE)
    });

    if app.converter.is_open {
        stack![base, converter_overlay(app)]
            .width(Fill)
            .height(Fill)
            .into()
    } else {
        base.into()
    }
}

fn converter_overlay(app: &WorldClockApp) -> Element<'_, Message> {
    let (title, placeholder) = match app.converter.mode {
        ConverterMode::TimeConversion => (
            "Time Conversion",
            "I can meet at 7AM CT, what time is that in IST",
        ),
        ConverterMode::AddZone => ("Add Time Zone", "America/New_York"),
    };

    let input = text_input(placeholder, &app.converter.input)
        .on_input(Message::ConverterInputChanged)
        .on_submit(Message::SubmitConversion)
        .padding(14)
        .size(22)
        .width(Fill);

    let mut body = column![text(title).size(26).color(rgb8(248, 250, 252)), input,].spacing(16);

    if app.converter.mode == ConverterMode::AddZone {
        let suggestions = add_zone_suggestions(app);
        if !suggestions.is_empty() {
            body = body.push(add_zone_suggestion_list(suggestions));
        }
    }

    if let Some(outcome) = &app.converter.outcome {
        body = body.push(converter_outcome(outcome));
    }

    let panel = container(body)
        .width(Fill)
        .max_width(760)
        .padding(24)
        .style(|_| {
            container::Style::default()
                .background(rgb8(16, 24, 39))
                .color(Color::WHITE)
                .border(border::rounded(8).width(1).color(rgb8(71, 85, 105)))
                .shadow(Shadow {
                    color: rgba8(0, 0, 0, 0.45),
                    offset: Vector::new(0.0, 12.0),
                    blur_radius: 32.0,
                })
        });

    container(panel)
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .style(|_| {
            container::Style::default()
                .background(rgba8(0, 0, 0, 0.72))
                .color(Color::WHITE)
        })
        .into()
}

fn add_zone_suggestions(app: &WorldClockApp) -> Vec<TimezoneSearchMatch> {
    if app.converter.input.trim().is_empty() {
        return Vec::new();
    }

    timezone_search::search_timezones(&app.converter.input, 6)
        .into_iter()
        .filter(|matched| !timezone_already_shown(&matched.zone_name, &app.clocks))
        .collect()
}

fn add_zone_suggestion_list(suggestions: Vec<TimezoneSearchMatch>) -> Element<'static, Message> {
    let mut list = column![].spacing(8);

    for suggestion in suggestions {
        let zone_name = suggestion.zone_name.clone();
        list = list.push(
            button(
                container(text(suggestion.display).size(16).color(rgb8(226, 232, 240)))
                    .width(Fill)
                    .padding(10),
            )
            .width(Fill)
            .on_press(Message::SelectZone(zone_name))
            .style(|_theme, status| {
                let background = match status {
                    button::Status::Hovered | button::Status::Pressed => rgb8(30, 41, 59),
                    _ => rgb8(15, 23, 42),
                };

                button::Style {
                    background: Some(Background::Color(background)),
                    border: border::rounded(8).width(1).color(rgb8(51, 65, 85)),
                    text_color: rgb8(226, 232, 240),
                    ..button::Style::default()
                }
            }),
        );
    }

    list.into()
}

fn converter_outcome(outcome: &ConverterOutcome) -> Element<'_, Message> {
    match outcome {
        ConverterOutcome::Converted(conversion) => {
            let source = format!(
                "{} {}",
                time_conversion::format_time(conversion.source_time),
                conversion.source_label
            );
            let target = format!(
                "{} {}",
                time_conversion::format_time(conversion.target_time),
                conversion.target_label
            );
            let route = format!(
                "{} -> {}",
                conversion.source_time.timezone().name(),
                conversion.target_time.timezone().name()
            );
            let date = if conversion.source_time.date_naive() == conversion.target_time.date_naive()
            {
                conversion.source_time.format("%Y-%m-%d").to_string()
            } else {
                format!(
                    "{} -> {}",
                    conversion.source_time.format("%Y-%m-%d"),
                    conversion.target_time.format("%Y-%m-%d")
                )
            };

            container(
                column![
                    text(format!("{source} = {target}"))
                        .size(34)
                        .font(Font::MONOSPACE)
                        .color(rgb8(125, 249, 255)),
                    text(format!("{route} on {date}"))
                        .size(16)
                        .color(rgb8(203, 213, 225)),
                ]
                .spacing(8),
            )
            .padding(18)
            .style(|_| {
                container::Style::default()
                    .background(rgb8(7, 24, 36))
                    .border(border::rounded(8).width(1).color(rgb8(34, 211, 238)))
            })
            .into()
        }
        ConverterOutcome::ZoneAdded(zone_name) => container(
            text(format!("Added {zone_name}"))
                .size(18)
                .color(rgb8(125, 211, 252)),
        )
        .padding(18)
        .style(|_| {
            container::Style::default()
                .background(rgb8(7, 24, 36))
                .border(border::rounded(8).width(1).color(rgb8(34, 211, 238)))
        })
        .into(),
        ConverterOutcome::Error(error) => {
            container(text(error.clone()).size(18).color(rgb8(248, 113, 113)))
                .padding(18)
                .style(|_| {
                    container::Style::default()
                        .background(rgb8(45, 17, 24))
                        .border(border::rounded(8).width(1).color(rgb8(248, 113, 113)))
                })
                .into()
        }
    }
}

fn handle_keyboard(app: &mut WorldClockApp, event: keyboard::Event) {
    let keyboard::Event::KeyPressed {
        key,
        modifiers,
        text,
        ..
    } = event
    else {
        return;
    };

    if modifiers.command() && is_key(&key, "k") {
        open_converter(app);
        return;
    }

    if !app.converter.is_open {
        return;
    }

    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => {
            app.converter.is_open = false;
        }
        keyboard::Key::Named(Named::Enter) => {
            submit_overlay(app);
        }
        keyboard::Key::Named(Named::Backspace) => {
            app.converter.input.pop();
            app.converter.outcome = None;
        }
        _ if !modifiers.command() && !modifiers.control() => {
            if let Some(text) = text {
                app.converter.input.push_str(&text);
                app.converter.outcome = None;
            }
        }
        _ => {}
    }
}

fn is_key(key: &keyboard::Key, expected: &str) -> bool {
    match key.as_ref() {
        keyboard::Key::Character(character) => character.eq_ignore_ascii_case(expected),
        _ => false,
    }
}

fn open_converter(app: &mut WorldClockApp) {
    app.converter.is_open = true;
    if app.converter.mode != ConverterMode::TimeConversion {
        app.converter.input.clear();
    }
    app.converter.mode = ConverterMode::TimeConversion;
    app.converter.outcome = None;
}

fn open_add_zone(app: &mut WorldClockApp) {
    app.converter.is_open = true;
    app.converter.mode = ConverterMode::AddZone;
    app.converter.input.clear();
    app.converter.outcome = None;
}

fn submit_overlay(app: &mut WorldClockApp) {
    match app.converter.mode {
        ConverterMode::TimeConversion => submit_conversion(app),
        ConverterMode::AddZone => submit_add_zone(app),
    }
}

fn submit_conversion(app: &mut WorldClockApp) {
    app.converter.outcome = Some(
        match time_conversion::convert_query(&app.converter.input, Local::now().date_naive()) {
            Ok(conversion) => ConverterOutcome::Converted(conversion),
            Err(error) => ConverterOutcome::Error(error.to_string()),
        },
    );
}

fn submit_add_zone(app: &mut WorldClockApp) {
    match parse_clock(&app.converter.input, &app.clocks) {
        Ok(clock) => {
            let zone_name = clock.name.clone();
            app.clocks.push(clock);
            app.added_clock_count += 1;
            app.converter.input.clear();
            app.converter.outcome = Some(ConverterOutcome::ZoneAdded(zone_name));
            persist_clocks(app);
        }
        Err(error) => {
            app.converter.outcome = Some(ConverterOutcome::Error(error));
        }
    }
}

fn parse_clock(input: &str, existing_clocks: &[Clock]) -> Result<Clock, String> {
    let zone_name = input.trim();
    if zone_name.is_empty() {
        return Err(
            "Search for a city or enter an IANA timezone like America/New_York.".to_string(),
        );
    }

    let timezone = timezone_search::resolve_timezone(zone_name)
        .ok_or_else(|| format!("No timezone match for: {zone_name}"))?;

    if timezone_already_shown(timezone.name(), existing_clocks) {
        return Err(format!("Timezone already shown: {}", timezone.name()));
    }

    Ok(Clock {
        name: timezone.name().to_string(),
        timezone,
    })
}

fn timezone_already_shown(zone_name: &str, existing_clocks: &[Clock]) -> bool {
    let Ok(timezone) = zone_name.parse::<Tz>() else {
        return false;
    };

    existing_clocks
        .iter()
        .any(|clock| clock.timezone == timezone || clock.name == timezone.name())
}

fn persist_clocks(app: &WorldClockApp) {
    let zones: Vec<String> = app.clocks.iter().map(|clock| clock.name.clone()).collect();
    crate::save_clocks(&zones);
}

fn clock_card(
    clock: &Clock,
    is_alarm_active: bool,
    is_home_timezone: bool,
    metrics: ResponsiveMetrics,
) -> Element<'static, Message> {
    let time = Utc::now().with_timezone(&clock.timezone);
    let time_str = time.format("%H:%M:%S %Z").to_string();
    let date_str = time.format("%Y-%m-%d").to_string();
    let home_style = HomeCardStyle::new(is_home_timezone, is_alarm_active);

    let content = column![
        row![
            text(clock.name.clone())
                .size(metrics.title_size_for(&clock.name, is_home_timezone))
                .color(home_style.title_color),
            home_badge(is_home_timezone, metrics)
        ]
        .spacing(metrics.title_gap)
        .align_y(iced::alignment::Vertical::Center),
        text(time_str)
            .size(metrics.time_size)
            .font(Font::MONOSPACE)
            .color(home_style.time_color),
        text(date_str)
            .size(metrics.date_size)
            .font(Font::MONOSPACE)
            .color(home_style.date_color),
    ]
    .spacing(if is_home_timezone {
        metrics.home_content_gap
    } else {
        metrics.content_gap
    })
    .width(Fill)
    .align_x(iced::alignment::Horizontal::Center);

    let card_content = if is_home_timezone {
        column![
            home_accent_bar(true, metrics),
            container(content)
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill)
        ]
    } else {
        column![
            container(content)
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill)
        ]
    };

    container(card_content)
        .width(Fill)
        .height(Fill)
        .padding(metrics.card_padding)
        .style(move |_| container::Style {
            background: Some(home_style.background.into()),
            text_color: Some(Color::WHITE),
            border: border::rounded(if is_home_timezone {
                metrics.home_border_radius
            } else {
                metrics.border_radius
            })
            .width(if is_home_timezone {
                metrics.home_border_width
            } else {
                metrics.border_width
            })
            .color(home_style.border_color),
            shadow: home_style.shadow,
            ..container::Style::default()
        })
        .into()
}

#[derive(Clone, Copy)]
struct ResponsiveMetrics {
    window_padding: f32,
    card_gap: f32,
    card_padding: f32,
    title_gap: f32,
    content_gap: f32,
    home_content_gap: f32,
    title_size: f32,
    time_size: f32,
    date_size: f32,
    badge_size: f32,
    badge_padding_y: f32,
    badge_padding_x: f32,
    accent_height: f32,
    border_radius: f32,
    home_border_radius: f32,
    border_width: f32,
    home_border_width: f32,
    inner_width: f32,
}

impl ResponsiveMetrics {
    fn new(window_size: Size, clock_count: usize) -> Self {
        let size = window_size.max(MIN_WINDOW_SIZE);
        let clock_count = clock_count.max(1) as f32;
        let window_area_scale = ((size.width * size.height)
            / (INITIAL_WINDOW_SIZE.width * INITIAL_WINDOW_SIZE.height))
            .sqrt()
            .clamp(0.45, 2.6);

        let window_padding = scaled(24.0, window_area_scale, 12.0, 64.0);
        let card_gap = scaled(24.0, window_area_scale, 12.0, 48.0);
        let card_padding = scaled(24.0, window_area_scale, 12.0, 48.0);

        let row_count = if clock_count > 2.0 {
            (clock_count / 2.0).ceil()
        } else {
            1.0
        };

        let card_width = if clock_count > 2.0 {
            ((size.width - window_padding * 2.0 - card_gap) / 2.0).max(1.0)
        } else {
            ((size.width - window_padding * 2.0 - card_gap * (clock_count - 1.0)) / clock_count)
                .max(1.0)
        };
        let card_height = if clock_count > 2.0 {
            ((size.height - window_padding * 2.0 - card_gap * (row_count - 1.0)) / row_count)
                .max(1.0)
        } else {
            (size.height - window_padding * 2.0).max(1.0)
        };
        let inner_width = (card_width - card_padding * 2.0).max(1.0);
        let inner_height = (card_height - card_padding * 2.0).max(1.0);

        let base_card_width = if clock_count > 2.0 {
            ((INITIAL_WINDOW_SIZE.width - 24.0 * 2.0 - 24.0) / 2.0).max(1.0)
        } else {
            ((INITIAL_WINDOW_SIZE.width - 24.0 * 2.0 - 24.0 * (clock_count - 1.0)) / clock_count)
                .max(1.0)
        };
        let base_card_height = if clock_count > 2.0 {
            ((INITIAL_WINDOW_SIZE.height - 24.0 * 2.0 - 24.0 * (row_count - 1.0)) / row_count)
                .max(1.0)
        } else {
            (INITIAL_WINDOW_SIZE.height - 24.0 * 2.0).max(1.0)
        };
        let card_scale = ((card_width * card_height) / (base_card_width * base_card_height))
            .sqrt()
            .clamp(0.32, 2.6);

        let desired_time_size = 84.0 * card_scale;
        let time_fit_width = inner_width / (8.0 * 0.62);
        let time_fit_height = inner_height / 2.4;
        let time_size = desired_time_size
            .min(time_fit_width)
            .min(time_fit_height)
            .clamp(16.0, 180.0);
        let date_size = (32.0 * card_scale)
            .min(inner_width / (10.0 * 0.62))
            .min(time_size * 0.45)
            .clamp(10.0, 72.0);
        let title_size = (28.0 * card_scale).min(time_size * 0.4).clamp(10.0, 56.0);

        Self {
            window_padding,
            card_gap,
            card_padding,
            title_gap: scaled(12.0, card_scale, 6.0, 28.0),
            content_gap: scaled(12.0, card_scale, 6.0, 32.0),
            home_content_gap: scaled(16.0, card_scale, 8.0, 40.0),
            title_size,
            time_size,
            date_size,
            badge_size: scaled(16.0, card_scale, 10.0, 28.0),
            badge_padding_y: scaled(5.0, card_scale, 3.0, 10.0),
            badge_padding_x: scaled(12.0, card_scale, 6.0, 20.0),
            accent_height: scaled(10.0, card_scale, 4.0, 24.0),
            border_radius: scaled(12.0, card_scale, 8.0, 28.0),
            home_border_radius: scaled(18.0, card_scale, 10.0, 36.0),
            border_width: scaled(2.5, card_scale, 1.0, 6.0),
            home_border_width: scaled(4.5, card_scale, 2.0, 10.0),
            inner_width,
        }
    }

    fn title_size_for(self, title: &str, is_home_timezone: bool) -> f32 {
        let badge_width = if is_home_timezone {
            (4.0 * self.badge_size) + self.badge_padding_x * 2.0 + self.title_gap
        } else {
            0.0
        };
        let available_width = (self.inner_width - badge_width).max(1.0);

        fit_text_size(
            title.chars().count(),
            available_width,
            self.title_size,
            9.0,
            0.58,
        )
    }
}

fn scaled(base: f32, scale: f32, min: f32, max: f32) -> f32 {
    (base * scale).clamp(min, max)
}

fn fit_text_size(
    character_count: usize,
    available_width: f32,
    preferred_size: f32,
    min_size: f32,
    character_width_ratio: f32,
) -> f32 {
    if character_count == 0 {
        return preferred_size;
    }

    preferred_size
        .min(available_width / (character_count as f32 * character_width_ratio))
        .max(min_size)
}

fn detect_home_timezone() -> Option<Tz> {
    let timezone_name = iana_time_zone::get_timezone().ok()?;

    timezone_name.parse::<Tz>().ok()
}

fn home_accent_bar(
    is_home_timezone: bool,
    metrics: ResponsiveMetrics,
) -> Element<'static, Message> {
    if !is_home_timezone {
        return container(column![]).height(0).into();
    }

    row![
        accent_segment(rgb8(245, 166, 35), metrics.accent_height), // Gold
        accent_segment(rgb8(255, 102, 170), metrics.accent_height), // Pink
        accent_segment(rgb8(0, 229, 255), metrics.accent_height),  // Cyan
    ]
    .spacing(0)
    .width(Fill)
    .into()
}

fn accent_segment(color: Color, height: f32) -> Element<'static, Message> {
    container(column![])
        .width(Fill)
        .height(height)
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            border: Border::default(),
            shadow: Shadow::default(),
            text_color: None,
            snap: false,
        })
        .into()
}

fn home_badge(is_home_timezone: bool, metrics: ResponsiveMetrics) -> Element<'static, Message> {
    if !is_home_timezone {
        return container(column![]).width(0).into();
    }

    container(
        text("HOME")
            .size(metrics.badge_size)
            .font(Font::MONOSPACE)
            .color(rgb8(12, 18, 28)),
    )
    .padding([metrics.badge_padding_y, metrics.badge_padding_x])
    .style(|_| {
        container::Style::default()
            .background(rgb8(251, 191, 36))
            .border(border::rounded(999))
    })
    .into()
}

#[derive(Clone, Copy)]
struct HomeCardStyle {
    background: Color,
    border_color: Color,
    title_color: Color,
    time_color: Color,
    date_color: Color,
    shadow: Shadow,
}

impl HomeCardStyle {
    fn new(is_home_timezone: bool, is_alarm_active: bool) -> Self {
        if is_home_timezone {
            let border_color = if is_alarm_active {
                rgb8(248, 113, 113)
            } else {
                rgb8(245, 166, 35) // Amber
            };

            return Self {
                background: rgb8(27, 34, 45), // Subtly lighter charcoal
                border_color,
                title_color: rgb8(255, 255, 255),
                time_color: rgb8(0, 229, 255),   // Cyan
                date_color: rgb8(125, 211, 252), // Soft cyan-tinted light grey
                shadow: Shadow {
                    color: rgba8(245, 166, 35, 0.15),
                    offset: Vector::new(0.0, 10.0),
                    blur_radius: 28.0,
                },
            };
        }

        let border_color = if is_alarm_active {
            rgb8(255, 64, 64)
        } else {
            rgb8(55, 65, 81) // Desaturated charcoal border
        };

        Self {
            background: rgb8(27, 34, 45), // Subtly lighter charcoal
            border_color,
            title_color: Color::WHITE,
            time_color: Color::WHITE,
            date_color: rgb8(156, 163, 175), // Neutral grey
            shadow: Shadow::default(),
        }
    }
}

fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn rgba8(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responsive_metrics_scale_with_window_size() {
        let default = ResponsiveMetrics::new(INITIAL_WINDOW_SIZE, 2);
        let small = ResponsiveMetrics::new(Size::new(512.0, 384.0), 2);
        let large = ResponsiveMetrics::new(Size::new(2048.0, 1536.0), 2);

        assert!(small.time_size < default.time_size);
        assert!(large.time_size > default.time_size);
    }

    #[test]
    fn responsive_metrics_fit_more_clocks_into_the_same_window() {
        let two_clocks = ResponsiveMetrics::new(INITIAL_WINDOW_SIZE, 2);
        let eight_clocks = ResponsiveMetrics::new(INITIAL_WINDOW_SIZE, 8);

        assert!(eight_clocks.time_size < two_clocks.time_size);
    }

    #[test]
    fn title_date_uses_day_abbreviation_and_long_local_date() {
        let date = NaiveDate::from_ymd_opt(2026, 8, 31).unwrap();

        assert_eq!(format_title_date(date), "Mon, August 31, 2026");
    }

    #[test]
    fn parse_clock_accepts_iana_timezone() {
        let clock = parse_clock("America/New_York", &[]).expect("timezone should parse");

        assert_eq!(clock.name, "America/New_York");
        assert_eq!(clock.timezone, chrono_tz::America::New_York);
    }

    #[test]
    fn parse_clock_accepts_pasted_turkiye_display_name() {
        let clock = parse_clock(
            "T\u{00fc}rkiye Standard Time\nTime zone in T\u{00fc}rkiye (GMT+3)",
            &[],
        )
        .expect("timezone display label should parse");

        assert_eq!(clock.name, "Europe/Istanbul");
        assert_eq!(clock.timezone, chrono_tz::Europe::Istanbul);
    }

    #[test]
    fn parse_clock_accepts_fuzzy_city_search() {
        let clock = parse_clock("istnbul", &[]).expect("fuzzy city search should parse");

        assert_eq!(clock.name, "Europe/Istanbul");
        assert_eq!(clock.timezone, chrono_tz::Europe::Istanbul);
    }

    #[test]
    fn parse_clock_rejects_duplicate_timezone() {
        let existing = [Clock {
            name: "America/New_York".to_string(),
            timezone: chrono_tz::America::New_York,
        }];

        let error = parse_clock("America/New_York", &existing).expect_err("duplicate should fail");

        assert_eq!(error, "Timezone already shown: America/New_York");
    }
}
