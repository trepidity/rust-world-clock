use crate::Clock;
use chrono::{Local, NaiveTime, Timelike, Utc};
use iced::{
    executor,
    widget::{container, column, row, text, Container},
    Application, Command, Element, Length, Settings, Subscription, Theme, Color, Alignment,
};
use std::time::{Duration};

pub fn run(clocks: Vec<Clock>, alarms: Vec<NaiveTime>) -> iced::Result {
    WorldClockApp::run(Settings {
        flags: (clocks, alarms),
        ..Settings::default()
    })
}

struct WorldClockApp {
    clocks: Vec<Clock>,
    alarms: Vec<NaiveTime>,
    local_time: NaiveTime,
}

#[derive(Debug, Clone)]
enum Message {
    Tick(NaiveTime),
}

impl Application for WorldClockApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = (Vec<Clock>, Vec<NaiveTime>);

    fn new(flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            WorldClockApp {
                clocks: flags.0,
                alarms: flags.1,
                local_time: Local::now().time(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Rust World Clock")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick(time) => {
                self.local_time = time;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let is_alarm_active = self.alarms.iter().any(|&alarm| {
            self.local_time.hour() == alarm.hour() && self.local_time.minute() == alarm.minute()
        });

        let clock_content = self.clocks.iter().map(|clock| {
            let time = Utc::now().with_timezone(&clock.timezone);
            let time_str = time.format("%H:%M:%S").to_string();
            let date_str = time.format("%Y-%m-%d").to_string();

            container(
                column![
                    text(&clock.name).size(20).style(Color::from_rgb(1.0, 1.0, 0.0)), // Yellow-ish
                    text(time_str).size(40).style(Color::from_rgb(0.0, 1.0, 1.0)), // Cyan-ish
                    text(date_str).size(15).style(Color::from_rgb(0.5, 0.5, 0.5)), // Gray
                ]
                .align_items(Alignment::Center)
                .spacing(10)
            )
            .padding(20)
            .style(if is_alarm_active {
                // simple hack: generic theme style doesn't easily support custom borders without boilerplate
                // so we just use a different "built-in" usage if possible, or just ignore red border for now
                // to make it compile.
                // But wait, we can wrap it in ANOTHER container with a red background to simulate a border?
                // Or just use `iced::theme::Container::Box`
                iced::theme::Container::Box
            } else {
                iced::theme::Container::Transparent
            })
            .into()
        });

        let content = row(clock_content).spacing(20).padding(20).align_items(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(iced::theme::Container::Custom(Box::new(DarkBackground)))
            .into()
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_millis(500)).map(|_| {
            Message::Tick(Local::now().time())
        })
    }
}

struct DarkBackground;
impl container::StyleSheet for DarkBackground {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(Color::BLACK.into()),
            text_color: Some(Color::WHITE),
            ..Default::default()
        }
    }
}
