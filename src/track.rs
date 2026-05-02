use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures_time::task;
use iced::{
    Element, Task,
    widget::{button, column, container, text},
};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, Easing, StartTime, Tween, Tweenable,
    sound::{PlaybackPosition, PlaybackState, Region},
};
use log::{debug, info, trace};
use ulid::Ulid;

use crate::{
    Vector2,
    audio_cache::{AudioData, FileHash, FileStreamingSoundData, FileStreamingSoundHandle},
    components::Toggle,
};
use looping::Loop;
use play_pause::PlayPause;
use progress::Progress;

mod looping;
mod play_pause;
mod progress;
mod serde_impl;

pub use serde_impl::TrackData;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    PlayPause(play_pause::Message),
    Progress(progress::Message),
    Loop(looping::Message),
    Selected(bool),
    ListenerMoved(Vector2),
    Moved {
        new_position: Vector2,
        listener_position: Vector2,
    },
    Resized {
        new_radius: f32,
        listener_position: Vector2,
    },
    RemoveRequested,
}

#[derive(Debug)]
pub enum Action {
    Run(Task<Message>),
    Remove,
}

pub struct Track {
    id: Ulid,
    name: String,
    audio_data: Arc<AudioData>,
    position: Vector2,
    radius: f32,
    selected: bool,
    manager: AudioManager,
    handle: Handle,
    play_pause: PlayPause,
    progress: Progress,
    looping: Loop,
}

enum Handle {
    Uninitialized(Option<FileStreamingSoundData>),
    Initialized(FileStreamingSoundHandle),
}

impl Default for Handle {
    fn default() -> Self {
        Self::Uninitialized(None)
    }
}

impl Track {
    const TWEEN_DEFAULT: Tween = Tween {
        start_time: StartTime::Immediate,
        duration: Duration::from_secs(1),
        easing: Easing::InPowi(2),
    };
    const TWEEN_INSTANT: Tween = Tween {
        start_time: StartTime::Immediate,
        duration: Duration::from_millis(0),
        easing: Easing::Linear,
    };
    const ATTENUATION_STRENGTH: f64 = 10.0;
    const DEFAULT_RADIUS: f32 = 200.0;

    pub fn new(id: Ulid, name: String, data: Arc<AudioData>) -> Result<Self> {
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        let stream = data.load()?;
        let duration = stream.unsliced_duration().as_secs_f32();

        Ok(Self {
            id,
            name,
            audio_data: data,
            position: Vector2::default(),
            radius: Self::DEFAULT_RADIUS,
            selected: false,
            manager,
            handle: Handle::Uninitialized(Some(stream)),
            progress: Progress::new(duration),
            play_pause: PlayPause::new(),
            looping: Loop::new(),
        })
    }

    pub fn update(&mut self, msg: Message) -> Option<Action> {
        match msg {
            Message::PlayPause(msg) => {
                match self.play_pause.update(msg) {
                    play_pause::Action::Play => {
                        info!("Playing track {}.", self.id.to_string());
                        self.play().expect("should be able to play");
                        self.progress.stop_seeking();
                    }
                    play_pause::Action::Pause => {
                        info!("Pausing track {}.", self.id.to_string());
                        self.pause();
                    }
                }
                None
            }
            Message::Progress(msg) => {
                if let Some(action) = self.progress.update(msg, self.play_pause.is_on()) {
                    match action {
                        progress::Action::Release => {
                            info!(
                                "Seeking track {} to {:.2}s.",
                                self.id.to_string(),
                                self.progress.offset()
                            );
                            if let Handle::Initialized(handle) = &mut self.handle {
                                handle.seek_to(self.progress.offset());
                            } else {
                                self.create_track().expect("should be able to start track");
                            }

                            let debounce = Task::perform(
                                task::sleep(
                                    Duration::from_millis(Progress::DEBOUNCE_INTERVAL).into(),
                                ),
                                |_| Message::Progress(progress::Message::Seeked),
                            );

                            Some(Action::Run(debounce))
                        }
                    }
                } else {
                    None
                }
            }
            Message::Loop(msg) => {
                let loop_region: Option<Region> = match self.looping.update(msg) {
                    looping::Action::Enable => {
                        info!("Enabling looping for track {}.", self.id.to_string());
                        Some((0.0..).into())
                    },
                    looping::Action::Disable => {
                        info!("Disabling looping for track {}.", self.id.to_string());
                        None
                    },
                };

                match &mut self.handle {
                    Handle::Initialized(handle) => {
                        handle.set_loop_region(loop_region);
                    }
                    Handle::Uninitialized(data) => {
                        if let Some(data) = data {
                            data.settings.loop_region = loop_region;
                        }
                    }
                }

                None
            }
            Message::Selected(selected) => {
                if selected {
                    debug!("Selecting track {}.", self.id.to_string());
                } else {
                    debug!("Deselecting track {}.", self.id.to_string());
                }
                self.selected = selected;
                None
            }
            Message::Moved {
                new_position,
                listener_position,
            } => {
                trace!("Moving track {} to {new_position}.", self.id.to_string());
                self.position = new_position;
                self.recalculate_volume(listener_position);
                None
            }
            Message::Resized {
                new_radius,
                listener_position,
            } => {
                trace!("Resizing track {} to {new_radius}.", self.id.to_string());
                self.radius = new_radius;
                self.recalculate_volume(listener_position);
                None
            }
            Message::ListenerMoved(listener_position) => {
                self.recalculate_volume(listener_position);
                None
            }
            Message::RemoveRequested => Some(Action::Remove),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let position = match &self.handle {
            Handle::Uninitialized(Some(data)) => match data.settings.start_position {
                PlaybackPosition::Seconds(time) => time as f32,
                PlaybackPosition::Samples(_) => 0.0,
            },
            Handle::Initialized(handle) => handle.position() as f32,
            Handle::Uninitialized(_) => 0.0,
        };

        let info = column![
            text(self.name.clone()),
            self.progress.view(position).map(Message::Progress),
            self.play_pause.view().map(Message::PlayPause),
            self.looping.view().map(Message::Loop),
            button("-").on_press(Message::RemoveRequested),
        ];

        let style = if self.selected {
            container::primary
        } else {
            container::secondary
        };
        container(info).style(style).into()
    }

    fn play(&mut self) -> Result<()> {
        let data = match &mut self.handle {
            Handle::Initialized(handle)
                if handle.state() != PlaybackState::Stopping
                    && handle.state() != PlaybackState::Stopped =>
            {
                handle.resume(Self::TWEEN_DEFAULT);
                return Ok(());
            }
            Handle::Initialized(_) => {
                self.create_track()?;
                let Handle::Uninitialized(data) = &mut self.handle else {
                    unreachable!()
                };
                data.take().unwrap()
            }
            Handle::Uninitialized(data) => data.take().map_or_else(
                || {
                    self.create_track()?;
                    let Handle::Uninitialized(data) = &mut self.handle else {
                        unreachable!()
                    };
                    Ok(data.take().unwrap())
                },
                anyhow::Result::<FileStreamingSoundData>::Ok,
            )?,
        };

        let handle = self.manager.play(data)?;
        self.handle = Handle::Initialized(handle);
        Ok(())
    }

    fn pause(&mut self) {
        if let Handle::Initialized(handle) = &mut self.handle {
            handle.pause(Self::TWEEN_DEFAULT);
        }
    }

    fn create_track(&mut self) -> Result<&mut FileStreamingSoundData> {
        let loop_region = if self.looping.is_on() {
            Some(Region::from(0.0..))
        } else {
            None
        };

        let stream = self
            .audio_data
            .load()?
            .loop_region(loop_region)
            .start_position(self.progress.offset());
        self.handle = Handle::Uninitialized(Some(stream));

        let Handle::Uninitialized(Some(data)) = &mut self.handle else {
            unreachable!()
        };
        Ok(data)
    }

    fn recalculate_volume(&mut self, listener_position: Vector2) {
        let t = 1.0 - (listener_position - self.position).magnitude() / self.radius;
        let t_log =
            (t as f64 * (Self::ATTENUATION_STRENGTH - 1.0) + 1.0).log(Self::ATTENUATION_STRENGTH);
        let volume = Decibels::interpolate(Decibels::SILENCE, Decibels::IDENTITY, t_log);

        match &mut self.handle {
            Handle::Initialized(handle) => {
                handle.set_volume(volume, Self::TWEEN_INSTANT);
            }
            Handle::Uninitialized(data) => {
                if let Some(data) = data {
                    data.settings.volume = volume.into();
                }
            }
        }
    }

    pub fn id(&self) -> Ulid {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn position(&self) -> Vector2 {
        self.position
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn hash(&self) -> FileHash {
        self.audio_data.hash()
    }
}
