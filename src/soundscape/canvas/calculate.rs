use iced::widget::canvas;
use ulid::Ulid;

use super::{Message, Soundscape};
use crate::Vector2;

impl Soundscape {
    const MIN_SCALE: f32 = 0.25;
    const MAX_SCALE: f32 = 2.0;
    const SCROLL_SENSITIVITY: f32 = 1.0 / 100.0;

    pub(super) fn calculate_pan(
        &self,
        delta: Vector2,
        original_position: Vector2,
    ) -> canvas::Action<Message> {
        let new_position = original_position + delta / self.scale;
        canvas::Action::publish(Message::Translated { new_position }).and_capture()
    }

    pub(super) fn calculate_zoom(
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

    pub(super) fn calculate_track_move(
        &self,
        id: Ulid,
        delta: Vector2,
        original_position: Vector2,
    ) -> canvas::Action<Message> {
        let new_position = original_position + delta / self.scale;
        canvas::Action::publish(Message::TrackMoved { id, new_position }).and_capture()
    }

    pub(super) fn calculate_track_resize(
        id: Ulid,
        cursor_pos: Vector2,
        track_position: Vector2,
    ) -> canvas::Action<Message> {
        let delta = cursor_pos - track_position;
        let new_radius = delta.magnitude();
        canvas::Action::publish(Message::TrackResized { id, new_radius }).and_capture()
    }
}
