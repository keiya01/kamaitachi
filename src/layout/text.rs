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

    // TODO: check if character is splittable(Script::Common is not splittable).
    pub fn scan_for_text(styled_node: &StyledNode, font_context: &mut FontContext, last_whitespace: &mut bool) -> Vec<TextRun> {
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
            if !ch.is_control() {
                let has_glyph = |font: &Font| font.glyph_index(ch).is_some();

                let new_script = Script::from(ch);
                let compatible_script = is_compatible(new_script, script);
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
                    Some(font) => match &new_font {
                        Some(new_font) => font.family_name == new_font.family_name,
                        None => false,
                    },
                    None => false,
                };

                let is_flush = !has_font || !compatible_script;

                if is_flush {
                    if end_pos > 0 {
                        text_runs.push(TextRun::new(
                            transform_text(content, &mut start_pos, end_pos, last_whitespace),
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

        if start_pos != end_pos {
            text_runs.push(TextRun::new(
                transform_text(content, &mut start_pos, end_pos, last_whitespace),
                size,
                descriptor,
                font.unwrap(),
            ));
        }

        text_runs
    }
}

fn is_compatible(new: Script, old: Script) -> bool {
    new == old || !is_specific(new) || !is_specific(old)
}

fn is_specific(script: Script) -> bool {
    script != Script::Common && script != Script::Inherited
}

fn transform_text(content: &str, start_pos: &mut usize, end_pos: usize, last_whitespace: &mut bool) -> String {
    let mut text = String::new();
    let sliced_content = &content[(*start_pos)..end_pos];
    for ch in sliced_content.chars() {
        let is_whitespace = is_in_whitespace(ch);
        if !is_whitespace {
            text.push(ch);
        } else {
            if !*last_whitespace {
                text.push(' ');
            }
        }
        *last_whitespace = is_whitespace;
    }
    *start_pos = end_pos;
    text
}

// TODO: check white_space property value
fn is_in_whitespace(ch: char) -> bool {
    match ch {
        ' ' => true,
        '\t' => true,
        '\n' => true,
        '　' => true,
        _ => false,
    }
}
