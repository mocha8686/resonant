use iced::{
    Element, Length::Fill, Rectangle, Renderer, Theme, mouse::Cursor, widget::canvas::{Frame, Geometry, Path, Program, Stroke}
};

const STROKE_WIDTH: f32 = 1.0;
const STROKE_ALPHA: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {}

#[derive(Debug, Clone, Copy)]
pub struct Soundscape {
    radius: f32,
}

impl Soundscape {
    #[must_use]
    pub fn new(radius: f32) -> Self {
        Self { radius }
    }
}

impl Program<Message> for Soundscape {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());
        let path = Path::circle(frame.center(), self.radius);

        let w = frame.width();
        let w2 = frame.width() / 2.0;
        let h = frame.height();
        let h2 = frame.height() / 2.0;

        let ns = Path::line((w2, 0.0).into(), (w2, h).into());
        let ew = Path::line((0.0, h2).into(), (w, h2).into());

        let stroke = Stroke::default().with_width(STROKE_WIDTH).with_color(theme.palette().text.scale_alpha(STROKE_ALPHA));

        frame.stroke(&ns, stroke);
        frame.stroke(&ew, stroke);
        frame.fill(&path, theme.palette().primary);

        vec![frame.into_geometry()]
    }
}

impl Soundscape {
    #[must_use]
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::canvas(self).width(Fill).height(Fill).into()
    }
}
