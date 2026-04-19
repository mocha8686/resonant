use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use iced::{
    Element, Task,
    widget::{button, column, text},
};
use kira::{
    AudioManager, AudioManagerSettings, Decibels, Easing, StartTime, Tween, Tweenable,
    sound::{
        PlaybackPosition, PlaybackState, Region,
        streaming::{StreamingSoundData, StreamingSoundHandle},
    },
};
use ulid::Ulid;

use crate::{PROJECT_DIRS, Vector2, components::Toggle};
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
    ListenerMoved(Vector2),
    Moved {
        new_position: Vector2,
        listener_position: Vector2,
    },
    Remove,
}

type FileStreamingSoundData = StreamingSoundData<kira::sound::FromFileError>;
type FileStreamingSoundHandle = StreamingSoundHandle<kira::sound::FromFileError>;

pub struct Track {
    id: Ulid,
    name: String,
    path: PathBuf,
    position: Vector2,
    radius: f32,
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

    pub fn new(id: Ulid, original_path: &Path) -> Result<Self> {
        let name = original_path
            .with_extension("")
            .file_name()
            .map_or("Unknown filename".into(), |s| {
                s.to_string_lossy().to_string()
            });

        let manager = AudioManager::new(AudioManagerSettings::default())?;

        let cache_dir = PROJECT_DIRS.cache_dir();

        std::fs::create_dir_all(cache_dir)?;
        let cache_dest = cache_dir.join(id.to_string()).with_extension(
            original_path
                .extension()
                .expect("file should have extension"),
        );
        std::fs::copy(original_path, &cache_dest)?;

        let data = FileStreamingSoundData::from_file(&cache_dest)?;
        let duration = data.unsliced_duration().as_secs_f32();

        Ok(Self {
            id,
            name,
            path: cache_dest,
            position: Vector2::default(),
            radius: 500.0,
            manager,
            handle: Handle::Uninitialized(Some(data)),
            progress: Progress::new(duration),
            play_pause: PlayPause::new(),
            looping: Loop::new(),
        })
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::PlayPause(m) => {
                match m {
                    play_pause::Message::Press(true) => {
                        self.play().expect("should be able to play");
                        self.progress.stop_seeking();
                    }
                    play_pause::Message::Press(false) => {
                        self.pause();
                    }
                }
                self.play_pause.update(m);
                None
            }
            Message::Progress(m) => {
                if m == progress::Message::Release {
                    if let Handle::Initialized(handle) = &mut self.handle {
                        handle.seek_to(self.progress.offset());
                    } else {
                        self.create_track().expect("should be able to start track");
                    }
                }

                Some(
                    self.progress
                        .update(m, self.play_pause.is_on())
                        .map(Message::Progress),
                )
            }
            Message::Loop(m) => {
                let loop_region: Option<Region> = match m {
                    looping::Message::Press(true) => Some((0.0..).into()),
                    looping::Message::Press(false) => None,
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

                self.looping.update(m);
                None
            }
            Message::Moved {
                new_position,
                listener_position,
            } => {
                self.position = new_position;
                self.recalculate_volume(listener_position);
                None
            }
            Message::ListenerMoved(listener_position) => {
                self.recalculate_volume(listener_position);
                None
            }
            Message::Remove => None,
        }
        .unwrap_or(Task::none())
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

        column![
            text(self.name.clone()),
            self.progress.view(position).map(Message::Progress),
            self.play_pause.view().map(Message::PlayPause),
            self.looping.view().map(Message::Loop),
            button("-").on_press(Message::Remove),
        ]
        .into()
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

        let data = FileStreamingSoundData::from_file(&self.path)?
            .loop_region(loop_region)
            .start_position(self.progress.offset());
        self.handle = Handle::Uninitialized(Some(data));

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
}

impl Drop for Track {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).ok();
    }
}
