use crate::Clock;
use chrono::{Local, NaiveTime, Timelike, Utc};
use iced::{
    Color, Element, Fill, Font, Subscription, Task, Theme, application, border,
    widget::{column, container, row, text},
    window,
};
use std::time::Duration;

pub fn run(clocks: Vec<Clock>, alarms: Vec<NaiveTime>) -> iced::Result {
    let initial = WorldClockApp {
        clocks,
        alarms,
        local_time: Local::now().time(),
    };

    application(move || (initial.clone(), Task::none()), update, view)
        .title(app_title)
        .theme(app_theme)
        .window(app_window_settings())
        .subscription(subscription)
        .centered()
        .run()
}

fn app_window_settings() -> window::Settings {
    window::Settings {
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
    local_time: NaiveTime,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(NaiveTime),
}

fn update(app: &mut WorldClockApp, message: Message) {
    match message {
        Message::Tick(time) => {
            app.local_time = time;
        }
    }
}

fn subscription(_app: &WorldClockApp) -> Subscription<Message> {
    iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick(Local::now().time()))
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
        |row, clock| row.push(clock_card(clock, is_alarm_active)),
    );

    container(content)
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .style(|_| {
            container::Style::default()
                .background(Color::BLACK)
                .color(Color::WHITE)
        })
        .into()
}

fn clock_card(clock: &Clock, is_alarm_active: bool) -> Element<'static, Message> {
    let time = Utc::now().with_timezone(&clock.timezone);
    let time_str = time.format("%H:%M:%S").to_string();
    let date_str = time.format("%Y-%m-%d").to_string();

    container(
        column![
            text(clock.name.clone()).size(24),
            text(time_str).size(72).font(Font::MONOSPACE),
            text(date_str).size(28).font(Font::MONOSPACE),
        ]
        .spacing(10)
        .width(Fill)
        .height(Fill)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Fill)
    .padding(20)
    .style(move |_| {
        let border_color = if is_alarm_active {
            Color::from_rgb(1.0, 0.25, 0.25)
        } else {
            Color::from_rgb(0.35, 0.35, 0.35)
        };

        container::Style {
            background: Some(Color::from_rgb(0.08, 0.08, 0.08).into()),
            text_color: Some(Color::WHITE),
            border: border::rounded(10).width(2).color(border_color),
            ..container::Style::default()
        }
    })
    .into()
}
