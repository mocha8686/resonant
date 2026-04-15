use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use iced::{
    Element, Task,
    widget::{column, text},
};
use kira::{
    AudioManager, AudioManagerSettings, Easing, StartTime, Tween,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};

use crate::track::{play_pause::PlayPause, progress::Progress};

mod play_pause;
mod progress;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    PlayPause(play_pause::Message),
    Progress(progress::Message),
}

pub struct Track {
    name: String,
    data: StaticSoundData,
    manager: AudioManager,
    handle: Option<StaticSoundHandle>,
    play_pause: PlayPause,
    progress: Progress,
}

impl Track {
    const PLAY_PAUSE_TWEEN: Tween = Tween {
        start_time: StartTime::Immediate,
        duration: Duration::from_secs(1),
        easing: Easing::InPowi(2),
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
        })
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::PlayPause(m) => {
                match m {
                    play_pause::Message::Play => self.play(),
                    play_pause::Message::Pause => self.pause(),
                }
                self.play_pause.update(m);
                None
            }
            Message::Progress(m) => {
                if m == progress::Message::Release {
                    if let Some(handle) = &mut self.handle {
                        handle.seek_to(self.progress.offset());
                    } else {
                        self.start_track(Some(self.progress.offset()))
                            .expect("should be able to start track");
                    }
                }

                Some(self.progress.update(m).map(Message::Progress))
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
        ]
        .into()
    }

    fn play(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.resume(Self::PLAY_PAUSE_TWEEN);
        } else {
            self.start_track(None)
                .expect("should be able to start track");
        }
    }

    fn pause(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.pause(Self::PLAY_PAUSE_TWEEN);
        }
    }

    fn start_track(&mut self, offset: Option<f64>) -> Result<()> {
        let mut handle = self.manager.play(self.data.clone())?;
        handle.seek_to(offset.unwrap_or(0.0));
        self.handle.replace(handle);
        Ok(())
    }

    fn track_position(&self) -> f32 {
        self.handle.as_ref().map_or(0.0, |h| h.position() as f32)
    }
}
