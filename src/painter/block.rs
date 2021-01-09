use iced_graphics::Primitive;
use iced_native::{Background, Color as GraphicsColor, Point, Rectangle, Size};

use super::Rect;
use crate::cssom::Color;

pub fn create_block(color: Color, rect: Rect) -> Primitive {
    Primitive::Quad {
        bounds: Rectangle::new(
            Point::new(rect.x, rect.y),
            Size::new(rect.width, rect.height),
        ),
        background: Background::Color(GraphicsColor::from_rgba8(
            color.r, color.g, color.b, color.a,
        )),
        border_radius: 0.0,
        border_width: 0.0,
        border_color: GraphicsColor::TRANSPARENT,
    }
}
