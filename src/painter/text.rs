use iced_graphics::Primitive;
use iced_native::{
    Color as GraphicsColor, Font, HorizontalAlignment, Point, Rectangle, Size, VerticalAlignment,
};

use super::Rect;
use crate::cssom::Color;
use crate::layout::font as layout_font;

pub fn create_text(
    content: String,
    color: Color,
    rect: Rect,
    font: layout_font::Font,
    font_context: &mut layout_font::FontContext,
) -> Primitive {
    Primitive::Text {
        content,
        bounds: Rectangle::new(
            Point::new(rect.x, rect.y),
            Size::new(rect.width, rect.height),
        ),
        color: GraphicsColor::from_rgba8(color.r, color.g, color.b, color.a),
        size: font.size,
        font: Font::External {
            bytes: font.get_static_font_data(font_context),
            name: font.get_static_font_family(),
        },
        horizontal_alignment: HorizontalAlignment::Left,
        vertical_alignment: VerticalAlignment::Top,
    }
}
