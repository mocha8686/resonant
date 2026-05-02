use iced::{
    Element,
    mouse::Interaction,
    widget::{mouse_area, progress_bar},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Moved(f32),
    Pressed,
    Released,
    Seeked,
}

#[derive(Debug)]
pub enum Action {
    Release,
}

#[derive(Default)]
pub struct Progress {
    duration: f32,
    offset: f32,
    cursor_pos: f32,
    cursor_holding: bool,
    seeking: bool,
}

impl Progress {
    pub const DEBOUNCE_INTERVAL: u64 = 500;
    const LENGTH: u32 = 200;
    const GIRTH: u32 = 8;

    #[must_use]
    pub fn new(duration: f32) -> Self {
        Self {
            duration,
            ..Default::default()
        }
    }

    pub fn update(&mut self, msg: Message, is_playing: bool) -> Option<Action> {
        match msg {
            Message::Moved(v) => {
                self.cursor_pos = v;
                if self.cursor_holding {
                    self.offset = self.cursor_pos;
                }
                None
            }
            Message::Pressed => {
                self.cursor_holding = true;
                self.seeking = true;
                self.offset = self.cursor_pos;
                None
            }
            Message::Released => {
                self.cursor_holding = false;

                Some(Action::Release)
            }
            Message::Seeked => {
                if !self.cursor_holding && is_playing {
                    self.stop_seeking();
                }
                None
            }
        }
    }

    pub fn view(&self, track_position: f32) -> Element<'_, Message> {
        let value = if self.seeking {
            self.offset
        } else {
            track_position
        };

        mouse_area(
            progress_bar(0.0..=self.duration, value)
                .length(Self::LENGTH)
                .girth(Self::GIRTH),
        )
        .interaction(Interaction::Pointer)
        .on_move(move |p| Message::Moved(p.x * self.duration / Self::LENGTH as f32))
        .on_press(Message::Pressed)
        .on_release(Message::Released)
        .into()
    }

    pub fn offset(&self) -> f64 {
        self.offset as f64
    }

    pub fn stop_seeking(&mut self) {
        self.seeking = false;
    }

    pub fn duration(&self) -> f32 {
        self.duration
    }
}
