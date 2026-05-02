use crate::components::Toggle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    Pressed(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Play,
    Pause,
}

#[derive(Default)]
pub struct PlayPause {
    is_playing: bool,
}

impl PlayPause {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Pressed(is_playing) => {
                self.is_playing = is_playing;
                if is_playing {
                    Action::Play
                } else {
                    Action::Pause
                }
            }
        }
    }
}

impl Toggle<'_, Message> for PlayPause {
    const TOGGLE_MESSAGE: fn(bool) -> Message = Message::Pressed;

    fn is_on(&self) -> bool {
        self.is_playing
    }

    fn icon(&self, is_on: bool) -> &'static [u8] {
        if is_on {
            include_bytes!("../icons/pause.svg").as_slice()
        } else {
            include_bytes!("../icons/play.svg").as_slice()
        }
    }
}
