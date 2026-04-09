use crate::Clock;
use chrono::{Local, NaiveTime, Timelike, Utc};
use iced::{
    Color, Element, Fill, Font, Subscription, Task, Theme, application, border,
    widget::{column, container, row, text},
};
use std::time::Duration;

pub fn run(clocks: Vec<Clock>, alarms: Vec<NaiveTime>) -> iced::Result {
    let initial = WorldClockApp {
        clocks,
        alarms,
        local_time: Local::now().time(),
    };

    application(
        move || (initial.clone(), Task::none()),
        update,
        view,
    )
    .title(app_title)
    .theme(app_theme)
    .subscription(subscription)
    .centered()
    .run()
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
