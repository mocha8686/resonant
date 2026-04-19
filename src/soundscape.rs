use std::{
    cmp::max_by_key,
    collections::{HashMap, VecDeque},
    time::Instant,
};

use iced::{
    Element, Event,
    Length::Fill,
    Rectangle, Renderer, Subscription, Task, Theme, Vector, keyboard,
    mouse::{self, Cursor},
    widget::{Action, canvas},
    window,
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::{Vector2, track::Track};

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
    ListenerMoved(Vector2),
    NewTrack {
        id: Ulid,
        position: Vector2,
        radius: f32,
    },
    TrackRemoved(Ulid),
    TrackMoved {
        id: Ulid,
        new_position: Vector2,
    },
}

impl From<&Track> for Message {
    fn from(track: &Track) -> Self {
        Self::NewTrack {
            id: track.id(),
            position: track.position(),
            radius: track.radius(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum State {
    #[default]
    None,
    Panning {
        cursor_start: Vector2,
        original_position: Vector2,
    },
    MovingTrack {
        id: Ulid,
        cursor_start: Vector2,
        original_position: Vector2,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct TrackInfo {
    id: Ulid,
    position: Vector2,
    radius: f32,
}

impl TrackInfo {
    fn contains(&self, point: Vector2) -> bool {
        let delta = self.position - point;
        delta.square_magnitude() <= self.radius * self.radius
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soundscape {
    listener: Listener,
    tracks: HashMap<Ulid, TrackInfo>,
    camera: Vector2,
    scale: f32,
    #[serde(skip, default = "Instant::now")]
    current: Instant,
    waypoints: VecDeque<Vector2>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
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
            tracks: HashMap::new(),
            camera: Vector2::ZERO,
            scale: 1.0,
            current: Instant::now(),
            waypoints: VecDeque::new(),
        }
    }

    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::Translated { new_position } => {
                self.camera = new_position;
                None
            }
            Message::Scaled {
                new_scale,
                new_position,
            } => {
                self.scale = new_scale;
                if let Some(new_position) = new_position {
                    self.camera = new_position;
                }
                None
            }
            Message::NewFrame(instant) => {
                let dt = instant - self.current;
                self.current = instant;

                if let Some(next_waypoint) = self.waypoints.front() {
                    let velocity =
                        (*next_waypoint - self.listener.position).normalized() * Self::SPEED;
                    let dv = velocity * dt.as_secs_f32();
                    self.listener.position += dv;

                    while let Some(next_waypoint) = self.waypoints.front()
                        && (*next_waypoint - self.listener.position).square_magnitude()
                            < dv.square_magnitude()
                    {
                        self.waypoints.pop_front();
                    }

                    Some(Task::done(Message::ListenerMoved(self.listener.position)))
                } else {
                    None
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
                None
            }
            Message::ListenerMoved(_) => None,
            Message::NewTrack {
                id,
                position,
                radius,
            } => {
                self.tracks.insert(
                    id,
                    TrackInfo {
                        id,
                        position,
                        radius,
                    },
                );
                None
            }
            Message::TrackRemoved(id) => {
                self.tracks.remove(&id);
                None
            }
            Message::TrackMoved { id, new_position } => {
                if let Some(track) = self.tracks.get_mut(&id) {
                    track.position = new_position;
                }
                None
            }
        }
        .unwrap_or_else(Task::none)
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        canvas(self).width(Fill).height(Fill).into()
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

    fn screen_to_world(&self, screen: Vector2, screen_center: Vector2) -> Vector2 {
        (screen - screen_center) / self.scale - self.camera
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn draw_grid(&self, frame: &mut canvas::Frame, theme: &Theme, bounds: Rectangle) {
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
        frame: &mut canvas::Frame,
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
        let amount = (main_length / spacing).ceil() as u32 + 1;
        let offset = (main_length / 2.0 + position * self.scale) % spacing;

        for i in 0..amount {
            let c = i as f32 * spacing + offset;
            let path = match direction {
                Direction::Vertical => {
                    canvas::Path::line((c, 0.0).into(), (c, cross_length).into())
                }
                Direction::Horizontal => {
                    canvas::Path::line((0.0, c).into(), (cross_length, c).into())
                }
            };

            let top_left = self.screen_to_world(Vector2::new(bounds.x, bounds.y), bounds.center().into());
            let start = match direction {
                Direction::Vertical => top_left.x,
                Direction::Horizontal => top_left.y,
            };
            let start_rounded = (start / world_spacing).trunc() * world_spacing;
            let world_position = start_rounded + world_spacing * i as f32;

            let is_highlight = world_position.abs() % (world_spacing * 4.0) <= f32::EPSILON;
            let alpha = if is_highlight {
                Self::GRID_ALPHA_HIGHLIGHT
            } else {
                Self::GRID_ALPHA_NORMAL
            };

            let stroke = canvas::Stroke::default()
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

    #[must_use]
    pub fn listener_position(&self) -> Vector2 {
        self.listener.position
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

impl canvas::Program<Message> for Soundscape {
    type State = State;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center_origin_transform = Vector::new(bounds.width, bounds.height) / 2.0;
        frame.translate(center_origin_transform);
        frame.scale(self.scale);
        frame.translate(self.camera.into());

        for point in &self.waypoints {
            let path = canvas::Path::circle(point.into(), Self::WAYPOINT_RADIUS);
            frame.fill(&path, theme.palette().text);
        }

        let path = canvas::Path::new(|p| {
            p.move_to(self.listener.position.into());
            for point in &self.waypoints {
                p.line_to(point.into());
            }
        });
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(1.0)
                .with_color(theme.palette().text.scale_alpha(0.8)),
        );

        let path = canvas::Path::circle(
            (self.listener.position.x, self.listener.position.y).into(),
            Self::LISTENER_RADIUS,
        );
        frame.fill(&path, theme.palette().primary);

        for track in self.tracks.values() {
            let path = canvas::Path::circle(track.position.into(), track.radius);
            frame.fill(
                &path,
                theme.extended_palette().primary.weak.color.scale_alpha(0.3),
            );
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_width(2.0)
                    .with_color(theme.extended_palette().primary.strong.color),
            );
        }

        let mut grid_frame = canvas::Frame::new(renderer, bounds.size());
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
        if cursor.is_levitating() {
            return None;
        }

        match event {
            Event::Mouse(event) => match event {
                mouse::Event::CursorMoved { position } => match state {
                    State::None => None,
                    State::Panning {
                        cursor_start,
                        original_position,
                    } => {
                        let delta = Vector2::from(position) - *cursor_start;
                        let new_position = *original_position + delta / self.scale;
                        let action = canvas::Action::publish(Message::Translated { new_position })
                            .and_capture();
                        Some(action)
                    }
                    State::MovingTrack {
                        id,
                        cursor_start,
                        original_position,
                    } => {
                        let delta = Vector2::from(position) - *cursor_start;
                        let new_position = *original_position + delta / self.scale;
                        let action = canvas::Action::publish(Message::TrackMoved {
                            id: *id,
                            new_position,
                        });
                        Some(action)
                    }
                },
                mouse::Event::ButtonPressed(button) => match button {
                    mouse::Button::Left => {
                        if let Some(position) = cursor.position() {
                            let position = Vector2::from(position);
                            let world_position =
                                self.screen_to_world(position, bounds.center().into());

                            *state = if let Some((id, track)) = self
                                .tracks
                                .iter()
                                .find(|(_, t)| t.contains(world_position))
                            {
                                State::MovingTrack {
                                    id: *id,
                                    cursor_start: position,
                                    original_position: track.position,
                                }
                            } else {
                                State::Panning {
                                    cursor_start: position,
                                    original_position: self.camera,
                                }
                            }
                        }
                        None
                    }
                    _ => None,
                },
                mouse::Event::ButtonReleased(_) => {
                    *state = State::None;
                    None
                }
                mouse::Event::WheelScrolled { delta } => match state {
                    State::None | State::Panning { .. } => match *delta {
                        mouse::ScrollDelta::Lines { y, .. }
                        | mouse::ScrollDelta::Pixels { y, .. } => {
                            let action = self.calculate_zoom(
                                cursor.position_from(bounds.center()).map(Vector2::from),
                                y,
                            );
                            Some(action)
                        }
                    },
                    State::MovingTrack { .. } => None,
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
