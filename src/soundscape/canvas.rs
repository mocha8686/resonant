use std::{
    cmp::max_by_key,
    f32::consts::{FRAC_PI_4, FRAC_PI_8},
};

use iced::{
    Event, Rectangle, Renderer, Theme, Vector,
    alignment::Vertical,
    keyboard,
    mouse::{self, Cursor},
    widget::{canvas, text::Alignment},
};
use ulid::Ulid;

use super::{Message, Soundscape};
use crate::Vector2;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum State {
    #[default]
    None,
    Pending {
        cursor_pos: Vector2,
    },
    Panning {
        cursor_start: Vector2,
        original_position: Vector2,
    },
    MovingTrack {
        id: Ulid,
        cursor_start: Vector2,
        original_position: Vector2,
    },
    ResizingTrack {
        id: Ulid,
        track_position: Vector2,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    Vertical,
    Horizontal,
}

impl Soundscape {
    const GRID_LINE_STROKE_WIDTH: f32 = 1.0;
    const GRID_LINE_ALPHA_NORMAL: f32 = 0.3;
    const GRID_LINE_ALPHA_HIGHLIGHT: f32 = 0.8;
    const GRID_LINE_LABEL_PAD: f32 = 10.0;
    const GRID_LINE_LABEL_SIZE: u32 = 14;
    const GRID_TRACK_LABEL_SIZE: u32 = 48;

    const TRACK_SELECTED_ALPHA: f32 = 0.6;
    const TRACK_ALPHA: f32 = 0.3;

    const CURSOR_MOVE_THRESHOLD: f32 = 5.0;

    const MIN_SCALE: f32 = 0.25;
    const MAX_SCALE: f32 = 2.0;

    const SCROLL_SENSITIVITY: f32 = 1.0 / 100.0;

    const SPACING: f32 = 100.0;
    const MIN_SPACING_WIDTH: f32 = 75.0;
    const MAX_SPACING_WIDTH: f32 = 125.0;

    const LISTENER_RADIUS: f32 = 25.0;
    const WAYPOINT_RADIUS: f32 = 5.0;

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

            let top_left =
                self.screen_to_world(Vector2::new(bounds.x, bounds.y), bounds.center().into());
            let start = match direction {
                Direction::Vertical => top_left.x,
                Direction::Horizontal => top_left.y,
            };
            let start_rounded = (start / world_spacing).trunc() * world_spacing;
            let world_position = start_rounded + world_spacing * i as f32;

            let is_highlight = world_position.abs() % (world_spacing * 4.0) <= f32::EPSILON;
            let alpha = if is_highlight {
                Self::GRID_LINE_ALPHA_HIGHLIGHT
            } else {
                Self::GRID_LINE_ALPHA_NORMAL
            };

            let stroke = canvas::Stroke::default()
                .with_width(Self::GRID_LINE_STROKE_WIDTH)
                .with_color(theme.palette().text.scale_alpha(alpha));

            frame.stroke(&path, stroke);
            let text = canvas::Text {
                content: world_position.abs().to_string(),
                position: match direction {
                    Direction::Vertical => {
                        iced::Point::new(c + 4.0, cross_length - Self::GRID_LINE_LABEL_PAD)
                    }
                    Direction::Horizontal => iced::Point::new(Self::GRID_LINE_LABEL_PAD, c),
                },
                color: theme.palette().text.scale_alpha(alpha),
                align_x: Alignment::Left,
                align_y: Vertical::Bottom,
                size: iced::Pixels::from(Self::GRID_LINE_LABEL_SIZE),
                ..Default::default()
            };
            frame.fill_text(text);
        }
    }

    fn calculate_pan(&self, delta: Vector2, original_position: Vector2) -> canvas::Action<Message> {
        let new_position = original_position + delta / self.scale;
        canvas::Action::publish(Message::Translated { new_position }).and_capture()
    }

    fn calculate_zoom(
        &self,
        offset_to_center: Option<Vector2>,
        scroll_y: f32,
    ) -> canvas::Action<Message> {
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

    fn calculate_track_move(
        &self,
        id: Ulid,
        delta: Vector2,
        original_position: Vector2,
    ) -> canvas::Action<Message> {
        let new_position = original_position + delta / self.scale;
        canvas::Action::publish(Message::TrackMoved { id, new_position }).and_capture()
    }

    fn calculate_track_resize(
        id: Ulid,
        cursor_pos: Vector2,
        track_position: Vector2,
    ) -> canvas::Action<Message> {
        let delta = cursor_pos - track_position;
        let new_radius = delta.magnitude();
        canvas::Action::publish(Message::TrackResized { id, new_radius }).and_capture()
    }

    fn find_track_at_point(&self, point: Vector2) -> Option<&super::TrackZone> {
        self.tracks.values().find(|t| t.contains(point))
    }

    fn handle_mouse_event(
        &self,
        state: &mut State,
        event: &mouse::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            mouse::Event::CursorMoved { position } => match state {
                State::None => None,
                State::Pending { cursor_pos } => {
                    let cursor_pos = *cursor_pos;
                    let delta = cursor_pos - position.into();
                    if delta.square_magnitude()
                        <= Self::CURSOR_MOVE_THRESHOLD * Self::CURSOR_MOVE_THRESHOLD
                    {
                        return None;
                    }

                    let world_cursor_pos = self.screen_to_world(cursor_pos, bounds.center().into());

                    let position = Vector2::from(position);
                    let world_position = self.screen_to_world(position, bounds.center().into());

                    if let Some((id, track)) = self.selected_track()
                        && dbg!(track.is_on_border(world_cursor_pos))
                    {
                        *state = State::ResizingTrack {
                            id,
                            track_position: track.position,
                        };
                        Some(Self::calculate_track_resize(id, world_position, track.position))
                    } else if let Some(track) = self.find_track_at_point(world_position) {
                        let id = track.id;
                        *state = State::MovingTrack {
                            id,
                            cursor_start: cursor_pos,
                            original_position: track.position,
                        };
                        Some(self.calculate_track_move(id, position - cursor_pos, track.position))
                    } else {
                        *state = State::Panning {
                            cursor_start: cursor_pos,
                            original_position: self.camera,
                        };
                        Some(self.calculate_pan(position - cursor_pos, self.camera))
                    }
                }
                State::Panning {
                    cursor_start,
                    original_position,
                } => {
                    let delta = Vector2::from(position) - *cursor_start;
                    Some(self.calculate_pan(delta, *original_position))
                }
                State::MovingTrack {
                    id,
                    cursor_start,
                    original_position,
                } => {
                    let delta = Vector2::from(position) - *cursor_start;
                    Some(self.calculate_track_move(*id, delta, *original_position))
                }
                State::ResizingTrack { id, track_position } => {
                    let cursor_pos = self.screen_to_world(position.into(), bounds.center().into());
                    Some(Self::calculate_track_resize(*id, cursor_pos, *track_position))
                }
            },
            mouse::Event::ButtonPressed(mouse::Button::Left)
                if !cursor.is_levitating()
                    && let Some(position) = cursor.position() =>
            {
                *state = State::Pending {
                    cursor_pos: position.into(),
                };
                None
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let action = match state {
                    State::Pending { cursor_pos } => {
                        let id = self
                            .find_track_at_point(
                                self.screen_to_world(*cursor_pos, bounds.center().into()),
                            )
                            .map(|t| t.id);
                        Some(canvas::Action::publish(Message::TrackSelected(id)))
                    }
                    _ => None,
                };
                *state = State::None;
                action
            }
            mouse::Event::WheelScrolled { delta } if !cursor.is_levitating() => match state {
                State::None | State::Pending { .. } | State::Panning { .. } => match *delta {
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
            _ => None,
        }
    }

    fn handle_default_mouse_interaction(
        &self,
        bounds: Rectangle,
        cursor_pos: Vector2,
    ) -> mouse::Interaction {
        let position = self.screen_to_world(cursor_pos, bounds.center().into());
        if let Some((_, track)) = self.selected_track()
            && track.is_on_border(position)
        {
            let mut delta = position - track.position;
            if delta.y >= 0.0 {
                delta.x *= -1.0;
            }
            let res = delta.normalized().dot(Vector2::RIGHT).acos();
            // res ∈ [0, π]

            let a = FRAC_PI_8;
            let b = a + FRAC_PI_4;
            let c = b + FRAC_PI_4;
            let d = c + FRAC_PI_4;

            if res <= a {
                mouse::Interaction::ResizingHorizontally
            } else if res <= b {
                mouse::Interaction::ResizingDiagonallyUp
            } else if res <= c {
                mouse::Interaction::ResizingVertically
            } else if res <= d {
                mouse::Interaction::ResizingDiagonallyDown
            } else {
                mouse::Interaction::ResizingHorizontally
            }
        } else {
            mouse::Interaction::None
        }
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
                theme.extended_palette().primary.weak.color.scale_alpha(
                    if Some(track.id) == self.selected_track {
                        Self::TRACK_SELECTED_ALPHA
                    } else {
                        Self::TRACK_ALPHA
                    },
                ),
            );
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_width(2.0)
                    .with_color(theme.extended_palette().primary.strong.color),
            );

            let text = canvas::Text {
                content: track.name.clone(),
                position: track.position.into(),
                color: theme.extended_palette().primary.strong.color,
                align_x: Alignment::Center,
                align_y: Vertical::Center,
                size: iced::Pixels::from(Self::GRID_TRACK_LABEL_SIZE),
                ..Default::default()
            };

            frame.fill_text(text);
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
    ) -> Option<canvas::Action<Message>> {
        match event {
            Event::Mouse(event) => self.handle_mouse_event(state, event, bounds, cursor),
            Event::Keyboard(event) if !cursor.is_levitating() => match event {
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

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state {
            State::None if let Some(cursor_pos) = cursor.position() => {
                self.handle_default_mouse_interaction(bounds, cursor_pos.into())
            }
            State::Pending { cursor_pos } => {
                self.handle_default_mouse_interaction(bounds, *cursor_pos)
            }
            _ => mouse::Interaction::None,
        }
    }
}
