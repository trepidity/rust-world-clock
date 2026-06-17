use crate::Clock;
use crate::time_conversion::{self, TimeConversion};
use chrono::{Local, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use iced::{
    Background, Border, Color, Element, Fill, Font, Shadow, Subscription, Task, Theme, Vector,
    application, border, keyboard,
    keyboard::key::Named,
    widget::{column, container, row, stack, text, text_input},
    window,
};
use std::time::Duration;

pub fn run(clocks: Vec<Clock>, alarms: Vec<NaiveTime>, always_on_top: bool) -> iced::Result {
    let initial = WorldClockApp {
        clocks,
        alarms,
        home_timezone: detect_home_timezone(),
        local_time: Local::now().time(),
        converter: ConverterState::default(),
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
                [0x15, 0x20, 0x33, 0xFF],
                [0x0A, 0x0F, 0x18, 0xFF],
                yf / size_f,
            );
            blend_pixel(&mut pixels, size, x, y, bg, bg_mask);

            let dist = (dx * dx + dy * dy).sqrt();
            let ring_mask = stroke_circle_mask(dist, ring_radius, ring_thickness);
            let ring = lerp_color(
                [0xF9, 0x73, 0x16, 0xFF],
                [0xFB, 0x71, 0x85, 0xFF],
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
                [0xDD, 0xEA, 0xF7, 0xFF],
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
                [0x5B, 0xC0, 0xEB, 0xFF],
                parallel_top,
            );
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0x5B, 0xC0, 0xEB, 0xFF],
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
                [0x63, 0xE6, 0xBE, 0xFF],
                meridian_left * 0.95,
            );
            blend_pixel(
                &mut pixels,
                size,
                x,
                y,
                [0x63, 0xE6, 0xBE, 0xFF],
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
    alarms: Vec<NaiveTime>,
    home_timezone: Option<Tz>,
    local_time: NaiveTime,
    converter: ConverterState,
}

#[derive(Debug, Clone, Default)]
struct ConverterState {
    is_open: bool,
    input: String,
    outcome: Option<ConverterOutcome>,
}

#[derive(Debug, Clone)]
enum ConverterOutcome {
    Converted(TimeConversion),
    Error(String),
}

#[derive(Debug, Clone)]
enum Message {
    Tick(NaiveTime),
    Keyboard(keyboard::Event),
    ConverterInputChanged(String),
    SubmitConversion,
}

fn update(app: &mut WorldClockApp, message: Message) {
    match message {
        Message::Tick(time) => {
            app.local_time = time;
        }
        Message::Keyboard(event) => {
            handle_keyboard(app, event);
        }
        Message::ConverterInputChanged(input) => {
            app.converter.input = input;
            app.converter.outcome = None;
        }
        Message::SubmitConversion => {
            submit_conversion(app);
        }
    }
}

fn subscription(_app: &WorldClockApp) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick(Local::now().time())),
        keyboard::listen().map(Message::Keyboard),
    ])
}

fn app_title(_app: &WorldClockApp) -> String {
    String::from("Rust World Clock")
}

fn app_theme(_app: &WorldClockApp) -> Theme {
    Theme::Dark
}

fn view(app: &WorldClockApp) -> Element<'_, Message> {
    let is_alarm_active = app.alarms.iter().any(|&alarm| {
        app.local_time.hour() == alarm.hour() && app.local_time.minute() == alarm.minute()
    });

    let content = app.clocks.iter().fold(
        row!().spacing(20).padding(20).width(Fill).height(Fill),
        |row, clock| {
            let is_home_timezone = app
                .home_timezone
                .as_ref()
                .is_some_and(|home_timezone| clock.timezone == *home_timezone);

            row.push(clock_card(clock, is_alarm_active, is_home_timezone))
        },
    );

    let base = container(content)
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .style(|_| {
            container::Style::default()
                .background(Color::BLACK)
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
    let input = text_input(
        "I can meet at 7AM CT, what time is that in IST",
        &app.converter.input,
    )
    .on_input(Message::ConverterInputChanged)
    .on_submit(Message::SubmitConversion)
    .padding(14)
    .size(22)
    .width(Fill);

    let mut body = column![
        text("Time Conversion").size(26).color(rgb8(248, 250, 252)),
        input,
    ]
    .spacing(16);

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
            submit_conversion(app);
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
}

fn submit_conversion(app: &mut WorldClockApp) {
    app.converter.outcome = Some(
        match time_conversion::convert_query(&app.converter.input, Local::now().date_naive()) {
            Ok(conversion) => ConverterOutcome::Converted(conversion),
            Err(error) => ConverterOutcome::Error(error.to_string()),
        },
    );
}

fn clock_card(
    clock: &Clock,
    is_alarm_active: bool,
    is_home_timezone: bool,
) -> Element<'static, Message> {
    let time = Utc::now().with_timezone(&clock.timezone);
    let time_str = time.format("%H:%M:%S").to_string();
    let date_str = time.format("%Y-%m-%d").to_string();
    let home_style = HomeCardStyle::new(is_home_timezone, is_alarm_active);

    container(
        column![
            home_accent_bar(is_home_timezone),
            row![
                text(clock.name.clone())
                    .size(24)
                    .color(home_style.title_color),
                home_badge(is_home_timezone)
            ]
            .spacing(10),
            text(time_str)
                .size(72)
                .font(Font::MONOSPACE)
                .color(home_style.time_color),
            text(date_str)
                .size(28)
                .font(Font::MONOSPACE)
                .color(home_style.date_color),
        ]
        .spacing(if is_home_timezone { 14 } else { 10 })
        .width(Fill)
        .height(Fill)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Fill)
    .padding(20)
    .style(move |_| container::Style {
        background: Some(home_style.background.into()),
        text_color: Some(Color::WHITE),
        border: border::rounded(if is_home_timezone { 16 } else { 10 })
            .width(if is_home_timezone { 4 } else { 2 })
            .color(home_style.border_color),
        shadow: home_style.shadow,
        ..container::Style::default()
    })
    .into()
}

fn detect_home_timezone() -> Option<Tz> {
    let timezone_name = iana_time_zone::get_timezone().ok()?;

    timezone_name.parse::<Tz>().ok()
}

fn home_accent_bar(is_home_timezone: bool) -> Element<'static, Message> {
    if !is_home_timezone {
        return container(column![]).height(0).into();
    }

    row![
        accent_segment(rgb8(249, 115, 22)),
        accent_segment(rgb8(236, 72, 153)),
        accent_segment(rgb8(34, 211, 238)),
    ]
    .spacing(0)
    .width(Fill)
    .into()
}

fn accent_segment(color: Color) -> Element<'static, Message> {
    container(column![])
        .width(Fill)
        .height(8)
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            border: Border::default(),
            shadow: Shadow::default(),
            text_color: None,
            snap: false,
        })
        .into()
}

fn home_badge(is_home_timezone: bool) -> Element<'static, Message> {
    if !is_home_timezone {
        return container(column![]).width(0).into();
    }

    container(
        text("HOME")
            .size(14)
            .font(Font::MONOSPACE)
            .color(rgb8(12, 18, 28)),
    )
    .padding([4, 10])
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
                rgb8(251, 191, 36)
            };

            return Self {
                background: rgb8(18, 26, 40),
                border_color,
                title_color: rgb8(255, 213, 128),
                time_color: rgb8(125, 249, 255),
                date_color: rgb8(244, 114, 182),
                shadow: Shadow {
                    color: rgba8(251, 191, 36, 0.28),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 24.0,
                },
            };
        }

        let border_color = if is_alarm_active {
            rgb8(255, 64, 64)
        } else {
            rgb8(89, 89, 89)
        };

        Self {
            background: rgb8(20, 20, 20),
            border_color,
            title_color: Color::WHITE,
            time_color: Color::WHITE,
            date_color: rgb8(214, 214, 214),
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
