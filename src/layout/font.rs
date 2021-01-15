pub use font_kit::family_name::FamilyName;
pub use font_kit::handle::Handle;
pub use glyph_brush::ab_glyph::{Font as GlyphBrushFont, PxScale, ScaleFont};

use core_text::font::CTFont;
use font_kit::font;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use glyph_brush::ab_glyph::FontRef;

use crate::style::StyledNode;

#[derive(Debug, Clone)]
pub struct Font {
    pub font: font::Font,
    pub ascent: f32,
    pub descent: f32,
    pub size: f32,
    ctfont: CTFont,
    units_per_em: f32,
}

fn px_to_pt(px: f64) -> f64 {
    px / 96. * 72.
}

fn pt_to_px(pt: f64) -> f64 {
    pt / 72. * 96.
}

impl Font {
    // TODO: specify property. ex: `font-weight`, `font-style`, etc
    pub fn new(font_families: Option<&[FamilyName]>, size: f32) -> Font {
        let font = load_font_family(font_families);

        let ctfont = font.native_font();
        let ctfont = ctfont.clone_with_font_size(size as f64);

        let ascent = ctfont.ascent() as f64;
        let descent = ctfont.descent() as f64;

        let scale = px_to_pt(ctfont.pt_size() as f64) / (ascent + descent);

        Font {
            font,
            ascent: pt_to_px(ascent * scale) as f32,
            descent: pt_to_px(descent * scale) as f32,
            size,
            units_per_em: ctfont.units_per_em() as f32,
            ctfont,
        }
    }

    pub fn as_ref(&self) -> FontRef {
        // TODO: optimize memory leak
        FontRef::try_from_slice(self.get_static_font_data()).unwrap()
    }

    // TODO: cache font data with font-size and font-family
    pub fn new_from_style(style: &StyledNode) -> Font {
        Font::new(style.font_family(), style.font_size())
    }

    fn leading(&self, line_height: f32) -> f32 {
        line_height - (self.ascent - self.descent)
    }

    pub fn height(&self, line_height: f32) -> (f32, f32) {
        let leading = self.leading(line_height);
        let above_baseline = self.ascent + leading / 2.0;
        let under_baseline = self.descent - leading / 2.0;
        (above_baseline, under_baseline)
    }

    pub fn width(&self, text: &str) -> f32 {
        let font_ref = self.as_ref();
        let scaled_font = font_ref.as_scaled(PxScale::from(self.size));
        let mut total_width = 0.;
        for c in text.chars() {
            let advanced_width = scaled_font.h_advance(scaled_font.glyph_id(c));
            total_width += advanced_width;
        }
        total_width
    }

    pub fn get_static_font_data(&self) -> &'static [u8] {
        let font_data = &*self.font.copy_font_data().unwrap();
        let boxed_slice = font_data.clone().into_boxed_slice();
        Box::leak(boxed_slice)
    }

    pub fn get_static_font_family(&self) -> &'static str {
        Box::leak(self.font.family_name().into_boxed_str())
    }
}

fn load_font_family(font_families: Option<&[FamilyName]>) -> font::Font {
    match font_families {
        Some(font_families) => SystemSource::new()
            .select_best_match(font_families, &Properties::new())
            .unwrap()
            .load()
            .unwrap(),
        None => load_default_font_family(),
    }
}

fn load_default_font_family() -> font::Font {
    SystemSource::new()
        .select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap()
}
