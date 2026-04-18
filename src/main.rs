use iced::{
    Element, Subscription, Task,
    widget::{button, column, container, stack, text},
};
use ordermap::OrderMap;
use resonant::{
    soundscape::{self, Soundscape},
    track::{self, Track},
};
use rfd::FileDialog;
use ulid::Ulid;

#[derive(Debug, Clone, Copy, PartialEq)]
enum MainMessage {
    Track(track::Message, Ulid),
    Soundscape(soundscape::Message),
    AddTrack,
}

struct State {
    tracks: OrderMap<Ulid, Track>,
    soundscape: Soundscape,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tracks: OrderMap::new(),
            soundscape: Soundscape::new(),
        }
    }
}

impl State {
    fn update(&mut self, msg: MainMessage) -> Task<MainMessage> {
        match msg {
            MainMessage::Track(msg, id) => {
                if let Some(track) = self.tracks.get_mut(&id) {
                    if msg == track::Message::Remove {
                        self.tracks.remove(&id);
                        Some(
                            self.soundscape
                                .update(soundscape::Message::TrackRemoved(id))
                                .map(MainMessage::Soundscape),
                        )
                    } else {
                        Some(track.update(msg).map(move |m| MainMessage::Track(m, id)))
                    }
                } else {
                    None
                }
            }
            MainMessage::Soundscape(msg) => {
                let task = match msg {
                    soundscape::Message::ListenerMoved(new_position) => {
                        let tasks = self.tracks.values_mut().map(|t| {
                            let id = t.id();
                            t.update(track::Message::ListenerMoved(new_position))
                                .map(move |m| MainMessage::Track(m, id))
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
                    let track = Track::new(Ulid::new(), path)
                        .expect("should be able to create track");
                    let task = self
                        .soundscape
                        .update((&track).into())
                        .map(MainMessage::Soundscape);
                    self.tracks.insert(track.id(), track);

                    Some(task)
                } else {
                    None
                }
            }
        }
        .unwrap_or_else(Task::none)
    }

    fn view(&self) -> Element<'_, MainMessage> {
        let tracks = column(self.tracks.values().map(|track| {
            track
                .view()
                .map(move |msg| MainMessage::Track(msg, track.id()))
        }));

        let track_menu = container(
            column![
                text("Tracklist"),
                tracks,
                button("+").on_press(MainMessage::AddTrack),
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
