use iced::{
    Element, Theme,
    widget::{button, svg},
};

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
        let icon = include_bytes!("../icons/loop.svg").as_slice();
        let handle = svg::Handle::from_memory(icon);
        let svg = svg(handle)
            .style(|theme: &Theme, _| svg::Style {
                color: Some(theme.palette().text),
            })
            .width(16)
            .height(16);

        button(svg)
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
