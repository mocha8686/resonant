use crate::components::Toggle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Pressed(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Enable,
    Disable,
}

pub struct Loop {
    is_looping: bool,
}

impl Default for Loop {
    fn default() -> Self {
        Self { is_looping: true }
    }
}

impl Loop {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Pressed(is_looping) => {
                self.is_looping = is_looping;
                if is_looping {
                    Action::Enable
                } else {
                    Action::Disable
                }
            }
        }
    }
}

impl Toggle<'_, Message> for Loop {
    const TOGGLE_MESSAGE: fn(bool) -> Message = Message::Pressed;

    fn is_on(&self) -> bool {
        self.is_looping
    }

    fn icon(&self, _is_on: bool) -> &'static [u8] {
        include_bytes!("../icons/loop.svg").as_slice()
    }
}
