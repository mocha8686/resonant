use iced::{Element, widget::button};

use crate::components::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Play,
    Pause,
}

#[derive(Default)]
pub struct PlayPause {
    is_playing: bool,
}

type ButtonStyler = fn(&iced::Theme, button::Status) -> button::Style;

impl PlayPause {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Play => {
                self.is_playing = true;
            }
            Message::Pause => {
                self.is_playing = false;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let icon = Icon::new(self.icon());

        button(icon.view())
            .on_press(self.message())
            .style(self.style())
            .into()
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn message(&self) -> Message {
        if self.is_playing {
            Message::Pause
        } else {
            Message::Play
        }
    }

    fn style(&self) -> ButtonStyler {
        if self.is_playing {
            button::primary
        } else {
            button::background
        }
    }

    fn icon(&self) -> &'static [u8] {
        if self.is_playing {
            include_bytes!("../icons/pause.svg").as_slice()
        } else {
            include_bytes!("../icons/play.svg").as_slice()
        }
    }
}
