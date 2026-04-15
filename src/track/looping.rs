use iced::{Element, widget::button};

use crate::components::Icon;

type ButtonStyler = fn(&iced::Theme, button::Status) -> button::Style;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Loop,
    Unloop,
}

#[derive(Default)]
pub struct Loop {
    is_looping: bool,
}

impl Loop {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Loop => {
                self.is_looping = true;
            }
            Message::Unloop => {
                self.is_looping = false;
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let data = include_bytes!("../icons/loop.svg").as_slice();
        let icon = Icon::new(data);

        button(icon.view())
            .on_press(self.message())
            .style(self.style())
            .into()
    }

    pub fn is_looping(&self) -> bool {
        self.is_looping
    }

    fn message(&self) -> Message {
        if self.is_looping {
            Message::Unloop
        } else {
            Message::Loop
        }
    }

    fn style(&self) -> ButtonStyler {
        if self.is_looping {
            button::primary
        } else {
            button::background
        }
    }
}
