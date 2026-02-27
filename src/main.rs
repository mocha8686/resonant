use iced::{Element, widget::{Column, button, row}};
use resonant::track::{self, Track};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Message {
    Track(track::Message, usize),
    AddTrack,
    RemoveTrack,
}

#[derive(Default)]
struct State {
    tracks: Vec<Track>,
}

impl State {
    fn update(&mut self, msg: Message) {
        match msg {
            Message::Track(msg, index) => {
                let Some(track) = self.tracks.get_mut(index) else {
                    return;
                };
                track.update(msg);
            }
            Message::AddTrack => self.tracks.push(Track::default()),
            Message::RemoveTrack => {
                self.tracks.pop();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let tracks = Column::from_vec(
            self.tracks
                .iter()
                .enumerate()
                .map(|(index, track)| track.view().map(move |msg| Message::Track(msg, index)))
                .collect(),
        );

        row![
            tracks,
            button("+").on_press(Message::AddTrack),
            button("-").on_press(Message::RemoveTrack),
        ].into()
    }
}

fn main() -> iced::Result {
    iced::run(State::update, State::view)
}
