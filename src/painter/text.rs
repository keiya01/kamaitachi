use iced_graphics::{Primitive};
use iced_native::{Rectangle, Size, Point, Color as GraphicsColor, Font, HorizontalAlignment, VerticalAlignment};

use crate::cssom::Color;
use super::Rect;

pub struct Text;

impl Text {
  pub fn new(content: String, color: Color, rect: Rect) -> Primitive {
    Primitive::Text {
      content,
      bounds: Rectangle::new(Point::new(rect.x, rect.y), Size::new(rect.width, rect.height)),
      color: GraphicsColor::from_rgba8(
        color.r,
        color.g,
        color.b,
        color.a,
      ),
      size: 16.0,
      font: Font::Default,
      horizontal_alignment: HorizontalAlignment::Left,
      vertical_alignment: VerticalAlignment::Top,
    }
  }
}
