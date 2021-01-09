use iced_graphics::{Backend, Defaults, Primitive, Renderer};
use iced_native::{layout, mouse, Element, Hasher, Layout, Length, Point, Rectangle, Size, Widget};

pub struct Wrapper {
    pub items: Vec<Primitive>,
}

impl Wrapper {
    pub fn new() -> Wrapper {
        Wrapper { items: vec![] }
    }
}

impl<Message, B> Widget<Message, Renderer<B>> for Wrapper
where
    B: Backend,
{
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, _renderer: &Renderer<B>, _limits: &layout::Limits) -> layout::Node {
        layout::Node::new(Size::new(0.0, 0.0))
    }

    fn hash_layout(&self, _state: &mut Hasher) {}

    fn draw(
        &self,
        _renderer: &mut Renderer<B>,
        _defaults: &Defaults,
        _layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) -> (Primitive, mouse::Interaction) {
        (
            Primitive::Group {
                primitives: self.items.clone(),
            },
            mouse::Interaction::default(),
        )
    }
}

impl<'a, Message, B> Into<Element<'a, Message, Renderer<B>>> for Wrapper
where
    B: Backend,
{
    fn into(self) -> Element<'a, Message, Renderer<B>> {
        Element::new(self)
    }
}
