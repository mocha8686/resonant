use iced::{
    Element,
    widget::{button, column, container, row, stack},
};
use resonant::{
    soundscape::Soundscape,
    track::{self, Track},
};
use rfd::FileDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum MainMessage {
    Track(track::Message, usize),
    AddTrack,
    RemoveTrack,
    None,
}

struct State {
    tracks: Vec<Track>,
    soundscape: Soundscape,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            soundscape: Soundscape::new(50.0),
        }
    }
}

impl State {
    fn update(&mut self, msg: MainMessage) {
        match msg {
            MainMessage::Track(msg, index) => {
                let Some(track) = self.tracks.get_mut(index) else {
                    return;
                };
                track.update(msg);
            }
            MainMessage::AddTrack => {
                let Some(path) = FileDialog::new()
                    .add_filter("audio", &["mp3", "ogg", "wav", "m4a"])
                    .pick_file()
                else {
                    return;
                };
                let track = Track::new(path);
                self.tracks.push(track);
            }
            MainMessage::RemoveTrack => {
                self.tracks.pop();
            }
            MainMessage::None => {}
        }
    }

    fn view(&self) -> Element<'_, MainMessage> {
        let tracks = column(
            self.tracks
                .iter()
                .enumerate()
                .map(|(index, track)| track.view().map(move |msg| MainMessage::Track(msg, index))),
        );

        let track_menu = container(row![
            tracks,
            button("+").on_press(MainMessage::AddTrack),
            button("-").on_press(MainMessage::RemoveTrack),
        ])
        .style(container::primary);
        let canvas = self.soundscape.view().map(|_| MainMessage::None);

        stack![canvas, track_menu].into()
    }
}

fn main() -> iced::Result {
    iced::run(State::update, State::view)
}
