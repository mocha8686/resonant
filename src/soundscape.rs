use std::{cmp::max_by_key, collections::VecDeque, time::Instant};

use iced::{
    Element, Event,
    Length::Fill,
    Rectangle, Renderer, Subscription, Theme, Vector, keyboard,
    mouse::{self, Cursor},
    widget::{
        Action,
        canvas::{self, Frame, Geometry, Path, Program, Stroke},
    },
    window,
};

use crate::Vector2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Translated {
        new_position: Vector2,
    },
    Scaled {
        new_scale: f32,
        new_position: Option<Vector2>,
    },
    NewFrame(Instant),
    NewWaypoint(Vector2),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum State {
    #[default]
    None,
    Panning {
        cursor_start: Vector2,
        original_position: Vector2,
    },
}

#[derive(Debug, Clone)]
pub struct Soundscape {
    listener: Listener,
    camera: Vector2,
    scale: f32,
    current: Instant,
    waypoints: VecDeque<Vector2>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Listener {
    position: Vector2,
}

impl Soundscape {
    const GRID_STROKE_WIDTH: f32 = 1.0;
    const GRID_TEXT_PAD: f32 = 10.0;
    const GRID_TEXT_SIZE: u32 = 14;
    const GRID_ALPHA_NORMAL: f32 = 0.3;
    const GRID_ALPHA_HIGHLIGHT: f32 = 0.8;

    const MIN_SCALE: f32 = 0.25;
    const MAX_SCALE: f32 = 2.0;

    const SCROLL_SENSITIVITY: f32 = 1.0 / 100.0;

    const SPACING: f32 = 100.0;
    const MIN_SPACING_WIDTH: f32 = 75.0;
    const MAX_SPACING_WIDTH: f32 = 125.0;

    const LISTENER_RADIUS: f32 = 25.0;
    const WAYPOINT_RADIUS: f32 = 5.0;
    const OVERLAP_THRESHOLD: f32 = 30.0;

    const SPEED: f32 = 100.0;

    #[must_use]
    pub fn new() -> Self {
        Self {
            listener: Listener::default(),
            camera: Vector2::ZERO,
            scale: 1.0,
            current: Instant::now(),
            waypoints: VecDeque::new(),
        }
    }

    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Translated { new_position } => self.camera = new_position,
            Message::Scaled {
                new_scale,
                new_position,
            } => {
                self.scale = new_scale;
                if let Some(new_position) = new_position {
                    self.camera = new_position;
                }
            }
            Message::NewFrame(instant) => {
                let dt = instant - self.current;
                self.current = instant;

                let Some(next_waypoint) = self.waypoints.front() else {
                    return;
                };

                let velocity = (*next_waypoint - self.listener.position).normalized() * Self::SPEED;
                let dv = velocity * dt.as_secs_f32();
                self.listener.position += dv;

                while let Some(next_waypoint) = self.waypoints.front()
                    && (*next_waypoint - self.listener.position).square_magnitude()
                        < dv.square_magnitude()
                {
                    self.waypoints.pop_front();
                }
            }
            Message::NewWaypoint(point) => {
                if let Some(waypoint) = self.waypoints.back()
                    && (point - *waypoint).square_magnitude()
                        < Self::OVERLAP_THRESHOLD * Self::OVERLAP_THRESHOLD
                {
                    self.waypoints.pop_back();
                }
                self.waypoints.push_back(point);
            }
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        let canvas = iced::widget::canvas(self).width(Fill).height(Fill);
        // let debug = container(text!(
        //     "vel: ({}, {})",
        //     self.listener.velocity.x,
        //     self.listener.velocity.y
        // ))
        // .align_bottom(Fill);

        canvas.into()
        // stack![debug, canvas].width(Fill).height(Fill).into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        window::frames().map(Message::NewFrame)
    }

    fn calculate_zoom(&self, offset_to_center: Option<Vector2>, scroll_y: f32) -> Action<Message> {
        if scroll_y < 0.0 && self.scale > Self::MIN_SCALE
            || scroll_y > 0.0 && self.scale < Self::MAX_SCALE
        {
            let new_scale = (self.scale * 1.0 + scroll_y * Self::SCROLL_SENSITIVITY)
                .clamp(Self::MIN_SCALE, Self::MAX_SCALE);
            let new_position = if let Some(offset) = offset_to_center {
                let factor = (new_scale / self.scale - 1.0) / new_scale;
                Some(self.camera - offset * factor)
            } else {
                None
            };

            canvas::Action::publish(Message::Scaled {
                new_scale,
                new_position,
            })
            .and_capture()
        } else {
            canvas::Action::capture()
        }
    }

    fn calculate_pan(&self, delta: Vector2, original_position: Vector2) -> Action<Message> {
        let new_position = original_position + delta / self.scale;
        let msg = Message::Translated { new_position };
        canvas::Action::publish(msg).and_capture()
    }

    fn screen_to_world(&self, screen: Vector2, screen_center: Vector2) -> Vector2 {
        (screen - screen_center) / self.scale - self.camera
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn draw_grid(&self, frame: &mut Frame, theme: &Theme, bounds: Rectangle) {
        let spacing = Self::SPACING * self.scale;

        let n_min = (Self::MIN_SPACING_WIDTH / spacing).log2().ceil() as i32;
        let n_max = (Self::MAX_SPACING_WIDTH / spacing).log2().floor() as i32;
        let n = max_by_key(n_min, n_max, |n: &i32| n.abs());

        self.draw_gridlines(frame, n, bounds, Direction::Vertical, theme);
        self.draw_gridlines(frame, n, bounds, Direction::Horizontal, theme);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn draw_gridlines(
        &self,
        frame: &mut Frame,
        n: i32,
        bounds: Rectangle,
        direction: Direction,
        theme: &Theme,
    ) {
        let (main_length, cross_length, position) = match direction {
            Direction::Vertical => (bounds.width, bounds.height, self.camera.x),
            Direction::Horizontal => (bounds.height, bounds.width, self.camera.y),
        };

        let world_spacing = Self::SPACING * (n as f32).exp2();
        let spacing = world_spacing * self.scale;
        let amount = (main_length / spacing).ceil() as u32;
        let offset = (main_length / 2.0 + position * self.scale) % spacing;

        for i in 0..amount {
            let c = i as f32 * spacing + offset;
            let path = match direction {
                Direction::Vertical => Path::line((c, 0.0).into(), (c, cross_length).into()),
                Direction::Horizontal => Path::line((0.0, c).into(), (cross_length, c).into()),
            };

            let top_left = self.screen_to_world(Vector2::new(0.0, 0.0), bounds.center().into());
            let start = match direction {
                Direction::Vertical => top_left.x,
                Direction::Horizontal => top_left.y,
            };
            let start_rounded = (start / world_spacing).trunc() * world_spacing;
            let world_position = start_rounded + world_spacing * i as f32;

            let is_highlight = (world_position % (world_spacing * 4.0)) == 0.0;
            let alpha = if is_highlight { Self::GRID_ALPHA_HIGHLIGHT } else { Self::GRID_ALPHA_NORMAL };

            let stroke = Stroke::default()
                .with_width(Self::GRID_STROKE_WIDTH)
                .with_color(theme.palette().text.scale_alpha(alpha));

            frame.stroke(&path, stroke);
            let text = canvas::Text {
                content: world_position.abs().to_string(),
                position: match direction {
                    Direction::Vertical => {
                        iced::Point::new(c + 4.0, cross_length - Self::GRID_TEXT_PAD)
                    }
                    Direction::Horizontal => iced::Point::new(Self::GRID_TEXT_PAD, c),
                },
                color: theme.palette().text.scale_alpha(alpha),
                align_x: iced::widget::text::Alignment::Left,
                align_y: iced::alignment::Vertical::Bottom,
                size: iced::Pixels::from(Self::GRID_TEXT_SIZE),
                ..Default::default()
            };
            frame.fill_text(text);
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Vertical,
    Horizontal,
}

impl Default for Soundscape {
    fn default() -> Self {
        Self::new()
    }
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
        frame.translate(self.camera.into());

        for point in &self.waypoints {
            let path = Path::circle(point.into(), Self::WAYPOINT_RADIUS);
            frame.fill(&path, theme.palette().text);
        }

        let path = Path::new(|p| {
            p.move_to(self.listener.position.into());
            for point in &self.waypoints {
                p.line_to(point.into());
            }
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_width(1.0)
                .with_color(theme.palette().text.scale_alpha(0.8)),
        );

        let path = Path::circle(
            (self.listener.position.x, self.listener.position.y).into(),
            Self::LISTENER_RADIUS,
        );
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
                        let action = self.calculate_pan(
                            Vector2::from(position) - *cursor_start,
                            *original_position,
                        );
                        Some(action)
                    }
                    State::None => None,
                },
                mouse::Event::ButtonPressed(button) => match button {
                    mouse::Button::Left => {
                        *state = State::Panning {
                            cursor_start: cursor
                                .position()
                                .unwrap_or_else(|| (0.0, 0.0).into())
                                .into(),
                            original_position: self.camera,
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
                        let action = self.calculate_zoom(
                            cursor.position_from(bounds.center()).map(Vector2::from),
                            y,
                        );
                        Some(action)
                    }
                },
                _ => None,
            },
            Event::Keyboard(event) => match event {
                keyboard::Event::KeyPressed {
                    physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyW),
                    repeat: false,
                    ..
                } => {
                    let position =
                        self.screen_to_world(cursor.position()?.into(), bounds.center().into());
                    let msg = Message::NewWaypoint(position);
                    Some(canvas::Action::publish(msg).and_capture())
                }
                _ => None,
            },
            _ => None,
        }
    }
}
