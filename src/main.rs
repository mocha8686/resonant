use resonant::track::{self, Track};
use iced::widget::{Column, column};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Message {
    Track(track::Message, u64),
}

#[derive(Default)]
struct State {
    track1: Track,
    track2: Track,
}

impl State {
    fn update(&mut self, msg: Message) {
        match msg {
            Message::Track(msg, id) => {
                if id == 1 {
                    self.track1.update(msg);
                } else {
                    self.track2.update(msg);
                }
            },
        }
    }

    fn view(&self) -> Column<'_, Message> {
        column![
            self.track1.view().map(|m| Message::Track(m, 1)),
            self.track2.view().map(|m| Message::Track(m, 2)),
        ]
    }
}

fn main() -> iced::Result {
    iced::run(State::update, State::view)
}
