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
    pub position: Vector,
    pub scale: f32,
}

impl Soundscape {
    const STROKE_WIDTH: f32 = 1.0;
    const STROKE_ALPHA: f32 = 0.3;

    const MIN_SCALE: f32 = 0.1;
    const MAX_SCALE: f32 = 1.5;

    const SCROLL_SENSITIVITY: f32 = 1.0 / 60.0;

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
            },
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::canvas(self).width(Fill).height(Fill).into()
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
        frame.translate(self.position);

        let path = Path::circle((0.0, 0.0).into(), self.radius);

        let w2 = frame.width() / 2.0;
        let h2 = frame.height() / 2.0;

        let ns = Path::line((0.0, -h2).into(), (0.0, h2).into());
        let ew = Path::line((-w2, 0.0).into(), (w2, 0.0).into());

        let stroke = Stroke::default()
            .with_width(Self::STROKE_WIDTH)
            .with_color(theme.palette().text.scale_alpha(Self::STROKE_ALPHA));

        frame.stroke(&ns, stroke);
        frame.stroke(&ew, stroke);
        frame.fill(&path, theme.palette().primary);

        vec![frame.into_geometry()]
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
                        let delta = *position - *cursor_start;
                        Some(Message::Translated(*original_position + delta / self.scale))
                            .map(canvas::Action::publish)
                            .map(canvas::Action::and_capture)
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
                        if y < 0.0 && self.scale > Self::MIN_SCALE
                            || y > 0.0 && self.scale < Self::MAX_SCALE
                        {
                            let new_scale = (self.scale * 1.0 + y * Self::SCROLL_SENSITIVITY).clamp(Self::MIN_SCALE, Self::MAX_SCALE);
                            let translation = if let Some(offset) = cursor.position_from(bounds.center()) {
                                let factor = (new_scale / self.scale - 1.0) / new_scale;
                                let offset = Vector::new(offset.x, offset.y);
                                Some(self.position - offset * factor)
                            } else {
                                None
                            };

                            Some(canvas::Action::publish(Message::Scaled(new_scale, translation)).and_capture())
                        } else {
                            Some(canvas::Action::capture())
                        }
                    }
                },
                _ => None,
            },
            _ => None,
        }
    }
}
