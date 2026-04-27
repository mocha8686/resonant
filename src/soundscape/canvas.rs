use iced::{
    Event, Rectangle, Renderer, Theme, Vector,
    alignment::Vertical,
    keyboard,
    widget::{canvas, text::Alignment},
};
use ulid::Ulid;

use super::{Message, Soundscape};
use crate::Vector2;

mod mouse;
mod grid;

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

impl Soundscape {
    const TRACK_LABEL_SIZE: u32 = 48;
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
}

impl canvas::Program<Message> for Soundscape {
    type State = State;

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
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
                size: iced::Pixels::from(Self::TRACK_LABEL_SIZE),
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
        cursor: iced::mouse::Cursor,
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
        cursor: iced::mouse::Cursor,
    ) -> iced::mouse::Interaction {
        match state {
            State::None if let Some(cursor_pos) = cursor.position() => {
                self.handle_default_mouse_interaction(bounds, cursor_pos.into())
            }
            State::Pending { cursor_pos } => {
                self.handle_default_mouse_interaction(bounds, *cursor_pos)
            }
            _ => iced::mouse::Interaction::None,
        }
    }
}
