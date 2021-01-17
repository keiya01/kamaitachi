pub use font_kit::family_name::FamilyName;
pub use font_kit::handle::Handle;
pub use font_kit::properties::{Stretch as FontStretch, Style as FontStyle, Weight as FontWeight};
pub use glyph_brush::ab_glyph::{Font as GlyphBrushFont, PxScale, ScaleFont};

use core_text::font::CTFont;
use font_kit::font;
use font_kit::properties::Properties;
use font_kit::source::SystemSource;
use glyph_brush::ab_glyph::FontRef;
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::style::StyledNode;

#[derive(Clone)]
pub struct FontCacheKey {
    size: f32,
    properties: Properties,
    family_name: FamilyName,
}

impl Hash for FontCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.size as i32).hash(state);
        format!("{:?}", self.properties.style).hash(state);
        (self.properties.stretch.0 as i32).hash(state);
        (self.properties.weight.0 as i32).hash(state);
    }
}

impl PartialEq for FontCacheKey {
    fn eq(&self, other: &Self) -> bool {
        (self.size as i32) == (other.size as i32)
            && (self.properties.stretch.0 as i32) == (other.properties.stretch.0 as i32)
            && (self.properties.weight.0 as i32) == (other.properties.weight.0 as i32)
            && format!("{:?}", self.properties.style) == format!("{:?}", other.properties.style)
    }
}

impl Eq for FontCacheKey {}

impl FontCacheKey {
    pub fn new(styled_node: &StyledNode, family_name: FamilyName) -> FontCacheKey {
        FontCacheKey {
            size: styled_node.font_size(),
            properties: Properties {
                style: styled_node.font_style(),
                weight: styled_node.font_weight(),
                stretch: FontStretch::NORMAL,
            },
            family_name,
        }
    }
}

pub(crate) struct FontContext {
    font_caches: HashMap<FontCacheKey, Font>,
}

impl FontContext {
    pub fn new() -> FontContext {
        FontContext {
            font_caches: HashMap::new(),
        }
    }

    pub fn get_or_create_by(&mut self, cache_key: FontCacheKey) -> Font {
        let font = self.font_caches.get(&cache_key);
        if let Some(font) = font {
            return font.clone();
        }
        let font = Font::new(cache_key.clone());
        self.font_caches.insert(cache_key, font.clone());
        font
    }
}

thread_local! {
    static FONT_CONTEXT: RefCell<Option<FontContext>> = RefCell::new(None);
}

pub(crate) fn with_thread_local_font_context<F, R>(f: F) -> R
where
    F: FnOnce(&mut FontContext) -> R,
{
    FONT_CONTEXT.with(|font_context| {
        f(font_context
            .borrow_mut()
            .get_or_insert_with(|| FontContext::new()))
    })
}

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
    pub fn new(descriptor: FontCacheKey) -> Font {
        let size = descriptor.size;
        let font_families = &[descriptor.family_name];
        let font = load_font_family(Some(font_families), &descriptor.properties);

        let ctfont = font.native_font().clone_with_font_size(size as f64);

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

fn load_font_family(font_families: Option<&[FamilyName]>, properties: &Properties) -> font::Font {
    match font_families {
        Some(font_families) => SystemSource::new()
            .select_best_match(font_families, properties)
            .unwrap()
            .load()
            .unwrap(),
        None => load_default_font_family(),
    }
}

fn load_default_font_family() -> font::Font {
    SystemSource::new()
        .select_best_match(&[FamilyName::Serif], &Properties::new())
        .unwrap()
        .load()
        .unwrap()
}
