use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use futures_time::task;
use iced::{
    Element, Task, Theme,
    mouse::Interaction,
    widget::{button, column, mouse_area, progress_bar, svg, text},
};
use kira::{
    AudioManager, AudioManagerSettings, Easing, StartTime, Tween,
    sound::{
        PlaybackState,
        static_sound::{StaticSoundData, StaticSoundHandle},
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Play,
    Pause,
    ProgressMove(f32),
    ProgressHold,
    ProgressRelease,
    Seeked,
}

pub struct Track {
    name: String,
    data: StaticSoundData,
    manager: AudioManager,
    handle: Option<StaticSoundHandle>,
    offset: f32,
    cursor_pos: f32,
    cursor_held: bool,
    seeking: bool,
}

impl Track {
    const PROGRESS_LENGTH: u32 = 200;
    const PLAY_PAUSE_TWEEN: Tween = Tween {
        start_time: StartTime::Immediate,
        duration: Duration::from_secs(1),
        easing: Easing::InPowi(2),
    };
    const PROGRESS_DEBOUNCE_INTERVAL: u64 = 15;

    pub fn new(path: PathBuf) -> Result<Self> {
        let name = path
            .with_extension("")
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown filename")
            .to_string();

        let data = StaticSoundData::from_file(path)?;
        let manager = AudioManager::new(AudioManagerSettings::default())?;

        Ok(Self {
            name,
            data,
            manager,
            handle: None,
            offset: 0.0,
            cursor_pos: 0.0,
            cursor_held: false,
            seeking: false,
        })
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Play => {
                if let Some(handle) = &mut self.handle {
                    handle.resume(Self::PLAY_PAUSE_TWEEN);
                } else {
                    self.start_track().expect("should be able to start track");
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
            Message::ProgressMove(v) => {
                self.cursor_pos = v;
                None
            }
            Message::ProgressHold => {
                self.cursor_held = true;
                self.seeking = true;
                None
            }
            Message::ProgressRelease => {
                self.cursor_held = false;
                self.offset = self.cursor_pos;

                if let Some(handle) = &mut self.handle {
                    handle.seek_to(self.cursor_pos as f64);
                } else {
                    self.start_track().expect("should be able to start track");
                }

                Some(Task::perform(
                    task::sleep(Duration::from_millis(Self::PROGRESS_DEBOUNCE_INTERVAL).into()),
                    |_| Message::Seeked,
                ))
            }
            Message::Seeked => {
                if !self.cursor_held {
                    self.seeking = false;
                }
                None
            }
        }
        .unwrap_or_else(Task::none)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let duration = self.data.unsliced_duration().as_secs_f32();

        let value = if self.seeking {
            self.cursor_pos
        } else {
            self.handle.as_ref().map_or(0.0, |h| h.position() as f32)
        };

        let progress = mouse_area(
            progress_bar(0.0..=duration, value)
                .length(Self::PROGRESS_LENGTH)
                .girth(8),
        )
        .interaction(Interaction::Pointer)
        .on_move(move |p| Message::ProgressMove(p.x * duration / Self::PROGRESS_LENGTH as f32))
        .on_press(Message::ProgressHold)
        .on_release(Message::ProgressRelease);

        column![text(self.name.clone()), progress, self.play_pause()].into()
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

    fn start_track(&mut self) -> Result<()> {
        let mut handle = self.manager.play(self.data.clone())?;
        handle.seek_to(self.offset as f64);
        self.handle.replace(handle);
        Ok(())
    }
}
