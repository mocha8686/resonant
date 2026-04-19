use crate::{
    soundscape::{self, Soundscape},
    track::{self, Track},
};
use iced::{
    Element, Subscription, Task,
    widget::{button, column, container, stack, text},
};
use ordermap::OrderMap;
use rfd::FileDialog;
use ulid::Ulid;

mod serde_impl;
pub use serde_impl::SceneData;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Track(track::Message, Ulid),
    Soundscape(soundscape::Message),
    AddTrack,
    Loaded,
}

pub struct Scene {
    tracks: OrderMap<Ulid, Track>,
    soundscape: Soundscape,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            tracks: OrderMap::new(),
            soundscape: Soundscape::new(),
        }
    }
}

impl Scene {
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Track(msg, id) => {
                if let Some(track) = self.tracks.get_mut(&id) {
                    if msg == track::Message::Remove {
                        self.tracks.remove(&id);
                        Some(
                            self.soundscape
                                .update(soundscape::Message::TrackRemoved(id))
                                .map(Message::Soundscape),
                        )
                    } else {
                        Some(track.update(msg).map(move |m| Message::Track(m, id)))
                    }
                } else {
                    None
                }
            }
            Message::Soundscape(msg) => {
                let task = match msg {
                    soundscape::Message::TrackMoved { id, new_position } => {
                        if let Some((_, track)) = self
                            .tracks
                            .iter_mut()
                            .find(|(track_id, _)| id == **track_id)
                        {
                            track
                                .update(track::Message::Moved {
                                    new_position,
                                    listener_position: self.soundscape.listener_position(),
                                })
                                .map(move |msg| Message::Track(msg, id))
                        } else {
                            Task::none()
                        }
                    }
                    soundscape::Message::ListenerMoved(new_position) => {
                        let tasks = self.tracks.values_mut().map(|t| {
                            let id = t.id();
                            t.update(track::Message::ListenerMoved(new_position))
                                .map(move |m| Message::Track(m, id))
                        });
                        Task::batch(tasks)
                    }
                    _ => Task::none(),
                };

                let task = task.chain(self.soundscape.update(msg).map(Message::Soundscape));
                Some(task)
            }
            Message::AddTrack => {
                if let Some(path) = FileDialog::new()
                    .add_filter("audio", &["flac", "mp3", "ogg", "wav", "webm"])
                    .pick_file()
                {
                    let track =
                        Track::new(Ulid::new(), &path).expect("should be able to create track");
                    let task = self
                        .soundscape
                        .update((&track).into())
                        .map(Message::Soundscape);
                    self.tracks.insert(track.id(), track);

                    Some(task)
                } else {
                    None
                }
            }
            Message::Loaded => {
                let tasks = self.tracks.iter_mut().map(|(id, track)| {
                    let id = *id;
                    track
                        .update(track::Message::ListenerMoved(
                            self.soundscape.listener_position(),
                        ))
                        .map(move |msg| Message::Track(msg, id))
                });
                Some(Task::batch(tasks))
            }
        }
        .unwrap_or_else(Task::none)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let tracks = column(
            self.tracks
                .values()
                .map(|track| track.view().map(move |msg| Message::Track(msg, track.id()))),
        );

        let track_menu = container(
            column![
                text("Tracklist"),
                tracks,
                button("+").on_press(Message::AddTrack),
            ]
            .spacing(16),
        )
        .padding(16)
        .style(container::bordered_box);

        let canvas = self.soundscape.view().map(Message::Soundscape);

        stack![canvas, container(track_menu).padding(4),].into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.soundscape.subscription().map(Message::Soundscape)
    }
}
