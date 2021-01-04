use font_kit::source::SystemSource;
use font_kit::properties::Properties;
use font_kit::font;
pub use font_kit::family_name::FamilyName;
pub use font_kit::handle::Handle;

#[derive(Debug, Clone)]
pub struct Font {
  pub ascent: f32,
  pub descent: f32,
  font: font::Font,
  pub size: f32,
}

impl Font {
  // TODO: specify property. ex: `font-weight`, `font-style`, etc
  pub fn new(font_families: Option<&[FamilyName]>, size: f32) -> Font {
    let font = match font_families {
      Some(font_families) => {
        SystemSource::new().select_best_match(font_families, &Properties::new())
          .unwrap()
          .load()
          .unwrap()
      },
      None => load_default_font_family(),
    };
    let metrics = font.metrics();

    Font { font, ascent: metrics.ascent, descent: metrics.descent, size }
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
    let metrics = self.font.metrics();
    let mut width = 0.;
    for c in text.chars() {
        if let Some(glyph_id) = self.font.glyph_for_char(c) {
            if let Ok(advance) = self.font.advance(glyph_id) {
                width += advance.x() * self.size / metrics.units_per_em as f32;
            }
        }
    }
    width
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

fn load_default_font_family() -> font::Font {
  SystemSource::new().select_best_match(&[FamilyName::SansSerif], &Properties::new())
        .unwrap()
        .load()
        .unwrap()
}
