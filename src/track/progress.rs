use std::time::Duration;

use futures_time::task;
use iced::{
    Element, Task,
    mouse::Interaction,
    widget::{mouse_area, progress_bar},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Move(f32),
    Hold,
    Release,
    Seeked,
}

#[derive(Default)]
pub struct Progress {
    duration: f32,
    cursor_offset: f32,
    cursor_holding: bool,
    seeking: bool,
}

impl Progress {
    const LENGTH: u32 = 200;
    const DEBOUNCE_INTERVAL: u64 = 15;
    const GIRTH: u32 = 8;

    #[must_use]
    pub fn new(duration: f32) -> Self {
        Self {
            duration,
            ..Default::default()
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Move(v) => {
                self.cursor_offset = v;
                None
            }
            Message::Hold => {
                self.cursor_holding = true;
                self.seeking = true;
                None
            }
            Message::Release => {
                self.cursor_holding = false;

                Some(Task::perform(
                    task::sleep(Duration::from_millis(Self::DEBOUNCE_INTERVAL).into()),
                    |_| Message::Seeked,
                ))
            }
            Message::Seeked => {
                if !self.cursor_holding {
                    self.seeking = false;
                }
                None
            }
        }
        .unwrap_or_else(Task::none)
    }

    pub fn view(&self, track_position: f32) -> Element<'_, Message> {
        let value = if self.seeking {
            self.cursor_offset
        } else {
            track_position
        };

        mouse_area(
            progress_bar(0.0..=self.duration, value)
                .length(Self::LENGTH)
                .girth(Self::GIRTH),
        )
        .interaction(Interaction::Pointer)
        .on_move(move |p| Message::Move(p.x * self.duration / Self::LENGTH as f32))
        .on_press(Message::Hold)
        .on_release(Message::Release)
        .into()
    }

    pub fn offset(&self) -> f64 {
        self.cursor_offset as f64
    }
}
