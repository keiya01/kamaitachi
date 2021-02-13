/// A static slice of characters.
pub type StaticCharVec = &'static [char];

pub static HTML_SPACE_CHARACTERS: StaticCharVec =
    &['\u{0020}', '\u{0009}', '\u{000a}', '\u{000c}', '\u{000d}'];

pub fn char_is_whitespace(c: char) -> bool {
    HTML_SPACE_CHARACTERS.contains(&c)
}
