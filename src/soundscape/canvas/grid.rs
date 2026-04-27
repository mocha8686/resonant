use std::cmp::max_by_key;

use iced::{
    Rectangle, Theme,
    alignment::Vertical,
    widget::{canvas, text::Alignment},
};

use super::Soundscape;
use crate::Vector2;

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

    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub(super) fn draw_grid(&self, frame: &mut canvas::Frame, theme: &Theme, bounds: Rectangle) {
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
}
