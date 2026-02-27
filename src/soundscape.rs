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

const STROKE_WIDTH: f32 = 1.0;
const STROKE_ALPHA: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Message {
    Translated(Vector),
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
}

impl Soundscape {
    #[must_use]
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            position: Vector::ZERO,
        }
    }
}

impl Soundscape {
    pub fn update(&mut self, msg: Message) {
        match msg {
            Message::Translated(vector) => self.position = vector,
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
        frame.translate(self.position);
        let path = Path::circle(frame.center(), self.radius);

        let w = frame.width();
        let w2 = frame.width() / 2.0;
        let h = frame.height();
        let h2 = frame.height() / 2.0;

        let ns = Path::line((w2, 0.0).into(), (w2, h).into());
        let ew = Path::line((0.0, h2).into(), (w, h2).into());

        let stroke = Stroke::default()
            .with_width(STROKE_WIDTH)
            .with_color(theme.palette().text.scale_alpha(STROKE_ALPHA));

        frame.stroke(&ns, stroke);
        frame.stroke(&ew, stroke);
        frame.fill(&path, theme.palette().primary);

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        _bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(event) => match event {
                mouse::Event::CursorMoved {
                    position,
                } => match state {
                    State::Panning { cursor_start, original_position } => {
                        let delta = *position - *cursor_start;
                        Some(Message::Translated(*original_position + delta))
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
                mouse::Event::WheelScrolled { delta } => todo!(),
                _ => None,
            },
            _ => None,
        }
    }
}
