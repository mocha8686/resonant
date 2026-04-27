use std::f32::consts::{FRAC_PI_4, FRAC_PI_8};

use iced::{Rectangle, mouse, widget::canvas};

use super::{Message, Soundscape, State};
use crate::Vector2;

impl Soundscape {
    const CURSOR_MOVE_THRESHOLD: f32 = 5.0;

    pub(super) fn handle_mouse_event(
        &self,
        state: &mut State,
        event: &mouse::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        match event {
            mouse::Event::CursorMoved {
                position: cursor_current,
            } => self.handle_mouse_moved(state, bounds, cursor_current.into()),
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

    fn handle_mouse_moved(
        &self,
        state: &mut State,
        bounds: Rectangle,
        cursor_current: Vector2,
    ) -> Option<canvas::Action<Message>> {
        match state {
            State::None => None,
            State::Pending {
                cursor_pos: cursor_start,
            } => {
                let cursor_start = *cursor_start;
                self.handle_mouse_moved_pending(state, bounds, cursor_start, cursor_current)
            }
            State::Panning {
                cursor_start,
                original_position,
            } => {
                let delta = cursor_current - *cursor_start;
                Some(self.calculate_pan(delta, *original_position))
            }
            State::MovingTrack {
                id,
                cursor_start,
                original_position,
            } => {
                let delta = cursor_current - *cursor_start;
                Some(self.calculate_track_move(*id, delta, *original_position))
            }
            State::ResizingTrack { id, track_position } => {
                let cursor_pos = self.screen_to_world(cursor_current, bounds.center().into());
                Some(Self::calculate_track_resize(
                    *id,
                    cursor_pos,
                    *track_position,
                ))
            }
        }
    }

    fn handle_mouse_moved_pending(
        &self,
        state: &mut State,
        bounds: Rectangle,
        cursor_start: Vector2,
        cursor_current: Vector2,
    ) -> Option<canvas::Action<Message>> {
        let delta = cursor_start - cursor_current;
        if delta.square_magnitude() <= Self::CURSOR_MOVE_THRESHOLD * Self::CURSOR_MOVE_THRESHOLD {
            return None;
        }

        let world_cursor_start = self.screen_to_world(cursor_start, bounds.center().into());
        let world_cursor_current = self.screen_to_world(cursor_current, bounds.center().into());

        if let Some((id, track)) = self.selected_track()
            && track.is_on_border(world_cursor_start)
        {
            *state = State::ResizingTrack {
                id,
                track_position: track.position,
            };
            Some(Self::calculate_track_resize(
                id,
                world_cursor_current,
                track.position,
            ))
        } else if let Some(track) = self.find_track_at_point(world_cursor_start) {
            let id = track.id;
            *state = State::MovingTrack {
                id,
                cursor_start,
                original_position: track.position,
            };
            Some(self.calculate_track_move(id, cursor_current - track.position, track.position))
        } else {
            *state = State::Panning {
                cursor_start,
                original_position: self.camera,
            };
            Some(self.calculate_pan(cursor_current - cursor_start, self.camera))
        }
    }

    pub(super) fn handle_default_mouse_interaction(
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
        } else if self.find_track_at_point(position).is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}
