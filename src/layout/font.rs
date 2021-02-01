pub use font_kit::family_name::FamilyName;
pub use font_kit::handle::Handle;
pub use font_kit::properties::{
    Properties as FontProperties, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight,
};
pub use glyph_brush::ab_glyph::{Font as GlyphBrushFont, PxScale, ScaleFont};

use crate::style::StyledNode;
use core_foundation::string::UniChar;
use core_graphics::font::CGGlyph;
use core_text::font::CTFont;
use font_kit::font;
use font_kit::source::SystemSource;
use glyph_brush::ab_glyph::FontRef;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub fn create_font_properties(styled_node: &StyledNode) -> FontProperties {
    FontProperties {
        style: styled_node.font_style(),
        weight: styled_node.font_weight(),
        // TODO: support font stretch
        stretch: FontStretch::NORMAL,
    }
}

#[derive(Clone, Debug)]
pub struct FontCacheKey {
    size: f32,
    properties: FontProperties,
    family_name: String,
}

impl Hash for FontCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.size as i32).hash(state);
        match self.properties.style {
            FontStyle::Normal => "normal".hash(state),
            FontStyle::Italic => "italic".hash(state),
            FontStyle::Oblique => "oblique".hash(state),
        }
        (self.properties.stretch.0 as i32).hash(state);
        (self.properties.weight.0 as i32).hash(state);
        self.family_name.hash(state);
    }
}

impl PartialEq for FontCacheKey {
    fn eq(&self, other: &Self) -> bool {
        (self.size as i32) == (other.size as i32)
            && (self.properties.stretch.0 as i32) == (other.properties.stretch.0 as i32)
            && (self.properties.weight.0 as i32) == (other.properties.weight.0 as i32)
            && match (self.properties.style, other.properties.style) {
                (FontStyle::Normal, FontStyle::Normal)
                | (FontStyle::Italic, FontStyle::Italic)
                | (FontStyle::Oblique, FontStyle::Oblique) => true,
                _ => false,
            }
            && self.family_name == other.family_name
    }
}

impl Eq for FontCacheKey {}

impl FontCacheKey {
    pub fn new(size: f32, properties: FontProperties, family_name: String) -> FontCacheKey {
        FontCacheKey {
            size,
            properties,
            family_name,
        }
    }

    pub fn new_from_style(styled_node: &StyledNode) -> FontCacheKey {
        FontCacheKey {
            size: styled_node.font_size(),
            properties: create_font_properties(styled_node),
            // TODO: Fix to find appropriate family name
            family_name: styled_node.font_family().pop().unwrap(),
        }
    }
}

pub struct FontContext {
    font_caches: HashMap<FontCacheKey, Font>,
    font_data_caches: HashMap<FontCacheKey, &'static [u8]>,
}

impl FontContext {
    pub fn new() -> FontContext {
        FontContext {
            font_caches: HashMap::new(),
            font_data_caches: HashMap::new(),
        }
    }

    pub fn get_or_create_by(&mut self, cache_key: &FontCacheKey) -> Font {
        let font = self.font_caches.get(&cache_key);
        if let Some(font) = font {
            return font.clone();
        }
        let font = Font::new(cache_key);
        self.font_caches.insert(cache_key.clone(), font.clone());
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
            .get_or_insert_with(FontContext::new))
    })
}

#[derive(Debug, Clone)]
pub struct Font {
    pub font: font::Font,
    pub ascent: f32,
    pub descent: f32,
    pub size: f32,
    pub family_name: String,
    ctfont: CTFont,
    units_per_em: f32,
    cache_key: FontCacheKey,
}

fn px_to_pt(px: f64) -> f64 {
    px / 96. * 72.
}

fn pt_to_px(pt: f64) -> f64 {
    pt / 72. * 96.
}

impl Font {
    pub fn new(descriptor: &FontCacheKey) -> Font {
        let size = descriptor.size;
        let font_families = &[FamilyName::Title(descriptor.family_name.clone())];
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
            family_name: descriptor.family_name.clone(),
            cache_key: descriptor.clone(),
        }
    }

    pub fn as_ref(&self, font_context: &mut FontContext) -> FontRef {
        FontRef::try_from_slice(self.get_static_font_data(font_context)).unwrap()
    }

    pub fn glyph_index(&self, codepoint: char) -> Option<u32> {
        let characters: [UniChar; 1] = [codepoint as UniChar];
        let mut glyphs: [CGGlyph; 1] = [0 as CGGlyph];

        let result = unsafe {
            self.ctfont
                .get_glyphs_for_characters(characters.as_ptr(), glyphs.as_mut_ptr(), 1)
        };

        if !result || glyphs[0] == 0 {
            // No glyph for this character
            return None;
        }

        Some(glyphs[0] as u32)
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

    pub fn width(&self, text: &str, font_context: &mut FontContext) -> f32 {
        let font_ref = self.as_ref(font_context);
        let scaled_font = font_ref.as_scaled(PxScale::from(self.size));
        let mut total_width = 0.;
        for c in text.chars() {
            let advanced_width = scaled_font.h_advance(scaled_font.glyph_id(c));
            total_width += advanced_width;
        }
        total_width
    }

    pub fn get_static_font_data(&self, font_context: &mut FontContext) -> &'static [u8] {
        if let Some(data) = font_context.font_data_caches.get(&self.cache_key) {
            return data;
        }
        let font_data = &*self.font.copy_font_data().unwrap();
        let boxed_slice = font_data.clone().into_boxed_slice();
        let leaked_slice = Box::leak(boxed_slice);
        font_context
            .font_data_caches
            .insert(self.cache_key.clone(), leaked_slice);
        leaked_slice
    }

    pub fn get_static_hashed_family_name(&self) -> &'static str {
        let mut hasher = DefaultHasher::new();
        self.cache_key.hash(&mut hasher);
        Box::leak(hasher.finish().to_string().into_boxed_str())
    }
}

fn load_font_family(
    font_families: Option<&[FamilyName]>,
    properties: &FontProperties,
) -> font::Font {
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
        .select_best_match(&[FamilyName::Serif], &FontProperties::new())
        .unwrap()
        .load()
        .unwrap()
}
