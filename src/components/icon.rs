use iced::{Element, Theme, widget::svg};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Icon {
    svg_data: &'static [u8],
}

impl Icon {
    const SIZE: u32 = 16;

    #[must_use]
    pub const fn new(svg_data: &'static [u8]) -> Self {
        Self { svg_data }
    }

    pub fn view<'a, Message>(&self) -> Element<'a, Message> {
        let handle = svg::Handle::from_memory(self.svg_data);

        svg(handle)
            .style(|theme: &Theme, _| svg::Style {
                color: Some(theme.palette().text),
            })
            .width(Self::SIZE)
            .height(Self::SIZE)
            .into()
    }
}
