use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use iced::{
    Element, Task,
    widget::{column, text},
};
use kira::{
    AudioManager, AudioManagerSettings, Easing, StartTime, Tween,
    sound::{
        PlaybackState,
        static_sound::{StaticSoundData, StaticSoundHandle},
    },
};

use crate::{
    components::Toggle,
    track::{looping::Loop, play_pause::PlayPause, progress::Progress},
};

mod looping;
mod play_pause;
mod progress;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    PlayPause(play_pause::Message),
    Progress(progress::Message),
    Loop(looping::Message),
}

pub struct Track {
    name: String,
    data: StaticSoundData,
    manager: AudioManager,
    handle: Option<StaticSoundHandle>,
    play_pause: PlayPause,
    progress: Progress,
    looping: Loop,
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

    pub fn new(path: PathBuf) -> Result<Self> {
        let name = path
            .with_extension("")
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown filename")
            .to_string();

        let data = StaticSoundData::from_file(path)?;
        let manager = AudioManager::new(AudioManagerSettings::default())?;
        let duration = data.unsliced_duration().as_secs_f32();

        Ok(Self {
            name,
            data,
            manager,
            handle: None,
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
                        self.play();
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
                    if let Some(handle) = &mut self.handle {
                        handle.seek_to(self.progress.offset());
                    } else {
                        let handle = self
                            .create_track(Some(self.progress.offset()))
                            .expect("should be able to start track");
                        handle.pause(Self::TWEEN_INSTANT);
                    }
                }

                Some(
                    self.progress
                        .update(m, self.play_pause.is_on())
                        .map(Message::Progress),
                )
            }
            Message::Loop(m) => {
                if let Some(handle) = &mut self.handle {
                    match m {
                        looping::Message::Press(true) => {
                            handle.set_loop_region(0.0..);
                        }
                        looping::Message::Press(false) => {
                            handle.set_loop_region(None);
                        }
                    }
                }
                self.looping.update(m);
                None
            }
        }
        .unwrap_or_else(Task::none)
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            text(self.name.clone()),
            self.progress
                .view(self.track_position())
                .map(Message::Progress),
            self.play_pause.view().map(Message::PlayPause),
            self.looping.view().map(Message::Loop),
        ]
        .into()
    }

    fn play(&mut self) {
        if let Some(handle) = &mut self.handle
            && handle.state() != PlaybackState::Stopping
            && handle.state() != PlaybackState::Stopped
        {
            handle.resume(Self::TWEEN_DEFAULT);
        } else {
            self.create_track(None)
                .expect("should be able to start track");
        }
    }

    fn pause(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.pause(Self::TWEEN_DEFAULT);
        }
    }

    fn create_track(&mut self, offset: Option<f64>) -> Result<&mut StaticSoundHandle> {
        let mut handle = self.manager.play(self.data.clone())?;
        handle.seek_to(offset.unwrap_or(0.0));

        if self.looping.is_on() {
            handle.set_loop_region(0.0..);
        }

        self.handle.replace(handle);
        Ok(self.handle.as_mut().unwrap())
    }

    fn track_position(&self) -> f32 {
        self.handle.as_ref().map_or(0.0, |h| h.position() as f32)
    }
}
