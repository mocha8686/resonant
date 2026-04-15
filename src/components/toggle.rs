use iced::{Element, widget::button};

use crate::components::Icon;

type ButtonStyler = fn(&iced::Theme, button::Status) -> button::Style;

pub trait Toggle<'a, Message: 'a + Copy> {
    const TOGGLE_MESSAGE: fn(bool) -> Message;

    fn is_on(&self) -> bool;

    fn icon(&self, is_on: bool) -> &'static [u8];

    fn message(&self, is_on: bool) -> Message {
        Self::TOGGLE_MESSAGE(!is_on)
    }

    fn style(&self, is_on: bool) -> ButtonStyler {
        if is_on {
            button::primary
        } else {
            button::background
        }
    }

    fn view(&self) -> Element<'a, Message> {
        let is_on = self.is_on();
        let icon = Icon::new(self.icon(is_on));

        button(icon.view())
            .on_press(self.message(is_on))
            .style(self.style(is_on))
            .into()
    }
}
