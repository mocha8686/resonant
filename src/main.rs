use iced::{
    Element, Subscription, Task,
    widget::{button, column, container, row, stack, text},
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
    fn update(&mut self, msg: MainMessage) -> Task<MainMessage> {
        match msg {
            MainMessage::Track(msg, index) => {
                let Some(track) = self.tracks.get_mut(index) else {
                    return Task::none();
                };
                Some(track.update(msg).map(move |m| MainMessage::Track(m, index)))
            }
            MainMessage::Soundscape(msg) => {
                let task = match msg {
                    soundscape::Message::ListenerMoved(new_position) => {
                        let tasks = self.tracks.iter_mut().enumerate().map(|(i, t)| {
                            t.update(track::Message::ListenerMoved(new_position))
                                .map(move |m| MainMessage::Track(m, i))
                        });
                        Task::batch(tasks)
                    }
                    _ => Task::none(),
                };

                let task = task.chain(self.soundscape.update(msg).map(MainMessage::Soundscape));
                Some(task)
            }
            MainMessage::AddTrack => {
                if let Some(path) = FileDialog::new()
                    .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                    .pick_file()
                {
                    let track = Track::new(path).expect("should be able to create track");
                    let task = self.soundscape.update(soundscape::Message::NewTrack {
                        position: track.position(),
                        radius: track.radius(),
                    }).map(MainMessage::Soundscape);
                    self.tracks.push(track);

                    Some(task)
                } else {
                    None
                }
            }
            MainMessage::RemoveTrack => {
                self.tracks.pop();
                None
            }
        }
        .unwrap_or_else(Task::none)
    }

    fn view(&self) -> Element<'_, MainMessage> {
        let tracks = column(
            self.tracks
                .iter()
                .enumerate()
                .map(|(index, track)| track.view().map(move |msg| MainMessage::Track(msg, index))),
        );

        let track_menu = container(
            column![
                text("Tracklist"),
                tracks,
                row![
                    button("+").on_press(MainMessage::AddTrack),
                    button("-").on_press(MainMessage::RemoveTrack),
                ],
            ]
            .spacing(16),
        )
        .padding(16)
        .style(container::bordered_box);

        let canvas = self.soundscape.view().map(MainMessage::Soundscape);

        stack![canvas, container(track_menu).padding(4),].into()
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
