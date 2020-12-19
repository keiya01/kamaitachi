use iced_graphics::{Primitive};
use iced_native::{Rectangle, Size, Background, Point, Color as GraphicsColor};

use crate::cssom::Color;
use super::Rect;

pub struct Block;

impl Block {
  pub fn new(color: Color, rect: Rect) -> Primitive {
    Primitive::Quad {
      bounds: Rectangle::new(Point::new(rect.x, rect.x), Size::new(rect.width, rect.height)),
      background: Background::Color(GraphicsColor::from_rgba8(
        color.r,
        color.g,
        color.b,
        color.a,
      )),
      border_radius: 0.0,
      border_width: 0.0,
      border_color: GraphicsColor::TRANSPARENT,
  }
  }
}
