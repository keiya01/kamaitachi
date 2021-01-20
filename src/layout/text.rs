use super::font::{create_font_properties, Font, FontCacheKey, FontContext, FontProperties};
use crate::dom::NodeType;
use crate::font_list::fallback_font_families;
use crate::style::StyledNode;
use unicode_script::Script;

#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub descriptor: FontProperties,
    pub size: f32,
    pub font: Font,
    pub cache_key: FontCacheKey,
}

impl TextRun {
    pub fn new(text: String, size: f32, descriptor: FontProperties, font: Font) -> TextRun {
        TextRun {
            cache_key: FontCacheKey::new(size, descriptor, font.family_name.clone()),
            text,
            size,
            descriptor,
            font,
        }
    }

    pub fn scan_for_text(styled_node: &StyledNode, font_context: &mut FontContext) -> Vec<TextRun> {
        let content = match &styled_node.node.node_type {
            NodeType::Text(text) => text,
            _ => unreachable!(),
        };
        let descriptor = create_font_properties(styled_node);
        let size = styled_node.font_size();
        let families = styled_node.font_family();

        let mut script = Script::Common;
        let mut font: Option<Font> = None;
        let mut text_runs = vec![];

        let (mut start_pos, mut end_pos) = (0, 0);
        for (_, ch) in content.char_indices() {
            if !ch.is_whitespace() {
                let has_glyph = |font: &Font| font.glyph_index(ch).is_some();

                let new_script = Script::from(ch);
                let compatible_script = is_compatible(script, new_script);
                if compatible_script && !is_specific(script) && is_specific(new_script) {
                    // Initialize Script::Common to new_script, if new_script is specific
                    script = new_script;
                }

                let new_font = families
                    .iter()
                    .map(|family| {
                        let key = FontCacheKey::new(size, descriptor, family.clone());
                        font_context.get_or_create_by(&key)
                    })
                    .find(has_glyph);
                let new_font = if new_font.is_some() {
                    new_font
                } else {
                    fallback_font_families(Some(ch))
                        .into_iter()
                        .map(|family| {
                            let key = FontCacheKey::new(size, descriptor, family);
                            font_context.get_or_create_by(&key)
                        })
                        .find(has_glyph)
                };

                let has_font = match &font {
                    Some(font) => has_glyph(font),
                    None => false,
                };

                let is_flush = !has_font || !compatible_script;

                if is_flush {
                    if end_pos > 0 {
                        text_runs.push(TextRun::new(
                            transform_text(content, &mut start_pos, end_pos),
                            size,
                            descriptor,
                            font.unwrap(),
                        ));
                    }
                    font = new_font;
                    script = new_script;
                }
            }
            end_pos += ch.len_utf8();
        }

        text_runs.push(TextRun::new(
            transform_text(content, &mut start_pos, end_pos),
            size,
            descriptor,
            font.unwrap(),
        ));

        text_runs
    }
}

fn is_compatible(old: Script, new: Script) -> bool {
    old == new
}

fn is_specific(script: Script) -> bool {
    script != Script::Common && script != Script::Inherited
}

fn transform_text(content: &str, start_pos: &mut usize, end_pos: usize) -> String {
    let mut text = String::new();
    let sliced_content = &content[(*start_pos)..end_pos];
    let mut is_prev_whitespace = false;
    for ch in sliced_content.chars() {
        let is_whitespace = ch.is_whitespace();
        if !is_whitespace {
            text.push(ch);
        } else {
            if !is_prev_whitespace {
                text.push(' ');
            }
        }
        is_prev_whitespace = is_whitespace;
    }
    *start_pos = end_pos;
    text
}
