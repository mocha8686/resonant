use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use iced::{
    Element, Task, Theme,
    widget::{button, column, svg, text},
};
use kira::{
    AudioManager, AudioManagerSettings, Easing, StartTime, Tween,
    sound::{
        PlaybackState,
        static_sound::{StaticSoundData, StaticSoundHandle},
    },
};

use crate::track::progress::Progress;

mod progress;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Play,
    Pause,
    Progress(progress::Message),
}

pub struct Track {
    name: String,
    data: StaticSoundData,
    manager: AudioManager,
    handle: Option<StaticSoundHandle>,
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
        })
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Play => {
                if let Some(handle) = &mut self.handle {
                    handle.resume(Self::PLAY_PAUSE_TWEEN);
                } else {
                    self.start_track(None)
                        .expect("should be able to start track");
                }
                None
            }
            Message::Pause => {
                let Some(handle) = &mut self.handle else {
                    return Task::none();
                };
                handle.pause(Self::PLAY_PAUSE_TWEEN);
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
            self.play_pause()
        ]
        .into()
    }

    fn play_pause(&self) -> Element<'_, Message> {
        let icon = if self.is_paused() {
            include_bytes!("icons/play.svg").as_slice()
        } else {
            include_bytes!("icons/pause.svg").as_slice()
        };
        let handle = svg::Handle::from_memory(icon);
        let svg = svg(handle)
            .style(|theme: &Theme, _| svg::Style {
                color: Some(theme.palette().text),
            })
            .width(16)
            .height(16);

        let (message, style): (Message, fn(&Theme, button::Status) -> button::Style) =
            if self.is_paused() {
                (Message::Play, button::background)
            } else {
                (Message::Pause, button::primary)
            };

        button(svg).on_press(message).style(style).into()
    }

    fn is_paused(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| {
            h.state() == PlaybackState::Pausing || h.state() == PlaybackState::Paused
        })
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
