use std::cmp::max_by_key;

use iced::{
    Element, Event,
    Length::Fill,
    Point, Rectangle, Renderer, Theme, Vector,
    mouse::{self, Cursor},
    widget::{
        Action,
        canvas::{self, Frame, Geometry, Path, Program, Stroke},
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Translated(Vector),
    Scaled(f32, Option<Vector>),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum State {
    #[default]
    None,
    Panning {
        cursor_start: Point,
        original_position: Vector,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Soundscape {
    radius: f32,
    position: Vector,
    scale: f32,
}

impl Soundscape {
    const STROKE_WIDTH: f32 = 1.0;
    const STROKE_ALPHA: f32 = 0.3;

    const MIN_SCALE: f32 = 0.25;
    const MAX_SCALE: f32 = 2.0;

    const SCROLL_SENSITIVITY: f32 = 1.0 / 100.0;

    const MIN_SPACING: f32 = 75.0;
    const DEFAULT_SPACING: f32 = 100.0;
    const MAX_SPACING: f32 = 125.0;

    #[must_use]
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            position: Vector::ZERO,
            scale: 1.0,
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Translated(vector) => self.position = vector,
            Message::Scaled(scale, position) => {
                self.scale = scale;
                if let Some(position) = position {
                    self.position = position;
                }
            }
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        let canvas = iced::widget::canvas(self).width(Fill).height(Fill);
        canvas.into()
    }

    fn calculate_zoom(&self, offset_to_center: Option<Point>, scroll_y: f32) -> Action<Message> {
        if scroll_y < 0.0 && self.scale > Self::MIN_SCALE
            || scroll_y > 0.0 && self.scale < Self::MAX_SCALE
        {
            let new_scale = (self.scale * 1.0 + scroll_y * Self::SCROLL_SENSITIVITY)
                .clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let translation = if let Some(offset) = offset_to_center {
                let factor = (new_scale / self.scale - 1.0) / new_scale;
                let offset = Vector::new(offset.x, offset.y);
                Some(self.position - offset * factor)
            } else {
                None
            };

            canvas::Action::publish(Message::Scaled(new_scale, translation)).and_capture()
        } else {
            canvas::Action::capture()
        }
    }

    fn calculate_pan(
        &self,
        cursor_end: Point,
        cursor_start: Point,
        original_position: Vector,
    ) -> Action<Message> {
        let delta = cursor_end - cursor_start;
        let msg = Message::Translated(original_position + delta / self.scale);
        canvas::Action::publish(msg).and_capture()
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn draw_grid(&self, frame: &mut Frame, theme: &Theme, bounds: Rectangle) {
        let spacing = Self::DEFAULT_SPACING * self.scale;

        let n_min = (Self::MIN_SPACING / spacing).log2().ceil() as i32;
        let n_max = (Self::MAX_SPACING / spacing).log2().floor() as i32;
        let n = max_by_key(n_min, n_max, |n: &i32| n.abs());

        let spacing = spacing * (n as f32).exp2();

        let stroke = Stroke::default()
            .with_width(Self::STROKE_WIDTH)
            .with_color(theme.palette().text.scale_alpha(Self::STROKE_ALPHA));

        self.draw_gridlines(frame, spacing, bounds, Direction::Vertical, stroke);
        self.draw_gridlines(frame, spacing, bounds, Direction::Horizontal, stroke);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn draw_gridlines(
        &self,
        frame: &mut Frame,
        spacing: f32,
        bounds: Rectangle,
        direction: Direction,
        stroke: Stroke,
    ) {
        let main_length = match direction {
            Direction::Vertical => bounds.width,
            Direction::Horizontal => bounds.height,
        };
        let cross_length = match direction {
            Direction::Vertical => bounds.height,
            Direction::Horizontal => bounds.width,
        };

        let amount = (main_length / spacing).ceil() as u32;
        let position = match direction {
            Direction::Vertical => self.position.x,
            Direction::Horizontal => self.position.y,
        };
        let offset = (main_length / 2.0 + position * self.scale) % spacing;
        for i in 0..amount {
            let c = i as f32 * spacing + offset;
            let path = match direction {
                Direction::Vertical => Path::line((c, 0.0).into(), (c, cross_length).into()),
                Direction::Horizontal => Path::line((0.0, c).into(), (cross_length, c).into()),
            };
            frame.stroke(&path, stroke);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Vertical,
    Horizontal,
}

impl Program<Message> for Soundscape {
    type State = State;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center_origin_transform = Vector::new(bounds.width, bounds.height) / 2.0;
        frame.translate(center_origin_transform);
        frame.scale(self.scale);
        frame.translate(self.position);
        let path = Path::circle((0.0, 0.0).into(), self.radius);
        frame.fill(&path, theme.palette().primary);

        let mut grid_frame = Frame::new(renderer, bounds.size());
        self.draw_grid(&mut grid_frame, theme, bounds);

        vec![grid_frame.into_geometry(), frame.into_geometry()]
    }

    #[allow(clippy::collapsible_match, reason = "prototyping")]
    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(event) => match event {
                mouse::Event::CursorMoved { position } => match state {
                    State::Panning {
                        cursor_start,
                        original_position,
                    } => {
                        let action =
                            self.calculate_pan(*position, *cursor_start, *original_position);
                        Some(action)
                    }
                    State::None => None,
                },
                mouse::Event::ButtonPressed(button) => match button {
                    mouse::Button::Left => {
                        *state = State::Panning {
                            cursor_start: cursor.position().unwrap_or_else(|| (0.0, 0.0).into()),
                            original_position: self.position,
                        };
                        None
                    }
                    _ => None,
                },
                mouse::Event::ButtonReleased(_) => {
                    *state = State::None;
                    None
                }
                mouse::Event::WheelScrolled { delta } => match *delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        let action = self.calculate_zoom(cursor.position_from(bounds.center()), y);
                        Some(action)
                    }
                },
                _ => None,
            },
            _ => None,
        }
    }
}
