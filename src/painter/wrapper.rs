use iced_graphics::{Backend, Defaults, Primitive, Renderer};
use iced_native::{layout, mouse, Element, Hasher, Layout, Length, Point, Rectangle, Size, Widget};

pub struct Wrapper {
    pub items: Vec<Primitive>,
    size: Size,
}

impl Wrapper {
    pub fn new(height: f32, width: f32) -> Wrapper {
        Wrapper {
            items: vec![],
            size: Size::new(width, height),
        }
    }
}

impl Default for Wrapper {
    fn default() -> Wrapper {
        Wrapper::new(0., 0.)
    }
}

impl<Message, B> Widget<Message, Renderer<B>> for Wrapper
where
    B: Backend,
{
    fn width(&self) -> Length {
        Length::Units(self.size.width as u16)
    }

    fn height(&self) -> Length {
        Length::Units(self.size.height as u16)
    }

    fn layout(&self, _renderer: &Renderer<B>, _limits: &layout::Limits) -> layout::Node {
        layout::Node::new(self.size)
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
