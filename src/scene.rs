use std::sync::Arc;

use crate::{
    audio_cache::AudioData,
    soundscape::{self, Soundscape},
    track::{self, Track},
};
use iced::{
    Element, Subscription, Task,
    widget::{button, column, container, stack, text},
};
use log::info;
use ordermap::OrderMap;
use ulid::Ulid;

mod serde_impl;
pub use serde_impl::SceneData;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Track(track::Message, Ulid),
    Soundscape(soundscape::Message),
    AddTrack,
    TrackAdded {
        id: Ulid,
        name: String,
        data: Arc<AudioData>,
    },
    Loaded,
}

#[derive(Debug)]
pub enum Action {
    Run(Task<Message>),
    AddTrack,
}

pub struct Scene {
    name: String,
    tracks: OrderMap<Ulid, Track>,
    soundscape: Soundscape,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            name: String::from("New Scene"),
            tracks: OrderMap::new(),
            soundscape: Soundscape::new(),
        }
    }
}

impl Scene {
    pub fn update(&mut self, msg: Message) -> Option<Action> {
        match msg {
            Message::Track(msg, id) => {
                if let Some(track) = self.tracks.get_mut(&id)
                    && let Some(action) = track.update(msg)
                {
                    match action {
                        track::Action::Run(task) => {
                            let task = task.map(move |msg| Message::Track(msg, id));
                            Some(Action::Run(task))
                        }
                        track::Action::Remove => {
                            self.tracks.remove(&id);
                            let task = Task::done(Message::Soundscape(
                                soundscape::Message::TrackRemoved(id),
                            ));
                            Some(Action::Run(task))
                        }
                    }
                } else {
                    None
                }
            }
            Message::Soundscape(msg) => {
                if let Some(action) = self.soundscape.update(msg) {
                    match action {
                        soundscape::Action::MoveTrack(id, new_position) => {
                            let move_task = Task::done(Message::Track(
                                track::Message::Moved {
                                    new_position,
                                    listener_position: self.soundscape.listener_position(),
                                },
                                id,
                            ));
                            let select_task = Task::done(Message::Soundscape(
                                soundscape::Message::TrackSelected(Some(id)),
                            ));
                            Some(Action::Run(move_task.chain(select_task)))
                        }
                        soundscape::Action::ResizeTrack(id, new_radius) => {
                            let task = Task::done(Message::Track(
                                track::Message::Resized {
                                    new_radius,
                                    listener_position: self.soundscape.listener_position(),
                                },
                                id,
                            ));
                            Some(Action::Run(task))
                        }
                        soundscape::Action::MoveListener(new_position) => {
                            let tasks = self.tracks.keys().map(|id| {
                                Task::done(Message::Track(
                                    track::Message::ListenerMoved(new_position),
                                    *id,
                                ))
                            });
                            Some(Action::Run(Task::batch(tasks)))
                        }
                        soundscape::Action::ChangeSelection {
                            deselected,
                            selected,
                        } => {
                            let deselected = deselected.map_or_else(Task::none, |id| {
                                Task::done(Message::Track(track::Message::Selected(false), id))
                            });
                            let selected = selected.map_or_else(Task::none, |id| {
                                Task::done(Message::Track(track::Message::Selected(true), id))
                            });
                            Some(Action::Run(deselected.chain(selected)))
                        }
                    }
                } else {
                    None
                }
            }
            Message::AddTrack => Some(Action::AddTrack),
            Message::TrackAdded { id, name, data } => {
                let track = Track::new(id, name, data).expect("should be able to create track");
                let task = Task::done(Message::Soundscape((&track).into()));
                self.tracks.insert(track.id(), track);
                Some(Action::Run(task))
            }
            Message::Loaded => {
                info!("Scene loaded.");
                let tasks = self.tracks.keys().map(|id| {
                    Task::done(Message::Track(
                        track::Message::ListenerMoved(self.soundscape.listener_position()),
                        *id,
                    ))
                });
                Some(Action::Run(Task::batch(tasks)))
            }
        }
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

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
