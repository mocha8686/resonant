use iced::{
    Element, Subscription, widget::{button, column, container, row, stack}
};
use resonant::{
    soundscape::{self, Soundscape},
    track::{self, Track},
};
use rfd::FileDialog;

#[derive(Debug, Clone, Copy, PartialEq)]
enum MainMessage {
    Track(track::Message, usize),
    Soundscape(soundscape::Message),
    AddTrack,
    RemoveTrack,
}

struct State {
    tracks: Vec<Track>,
    soundscape: Soundscape,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tracks: Vec::new(),
            soundscape: Soundscape::new(),
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
            MainMessage::Soundscape(msg) => {
                self.soundscape.update(msg);
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
        let canvas = self.soundscape.view().map(MainMessage::Soundscape);

        stack![canvas, track_menu].into()
    }

    fn subscription(&self) -> Subscription<MainMessage> {
        self.soundscape.subscription().map(MainMessage::Soundscape)
    }
}

fn main() -> iced::Result {
    iced::application(State::default, State::update, State::view)
        .subscription(State::subscription)
        .run()
}
