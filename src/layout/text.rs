pub use xi_unicode::LineBreakLeafIter;

use super::font::{
    create_font_properties, Font, FontCacheKey, FontContext, FontProperties, GlyphBrushFont,
    PxScale, ScaleFont,
};
use super::{BoxType, LayoutBox};
use crate::dom::NodeType;
use crate::font_list::fallback_font_families;
use crate::str::char_is_whitespace;
use crate::style::StyledNode;
use std::ops::Range;
use unicode_script::Script;

#[derive(Debug, Clone, PartialEq)]
pub enum TextFlags {
    SuppressLineBreakBefore,
}

#[derive(Debug, Clone)]
pub struct TextNode<'a> {
    pub styled_node: &'a StyledNode<'a>,
    pub range: Range<usize>,
    pub text_run: TextRun,
    pub flags: Vec<TextFlags>,
}

impl<'a> TextNode<'a> {
    fn new(
        styled_node: &'a StyledNode<'a>,
        text_run: TextRun,
        flags: Vec<TextFlags>,
    ) -> TextNode<'a> {
        TextNode {
            styled_node,
            range: 0..text_run.glyphs.len(),
            text_run,
            flags,
        }
    }

    pub fn get_text(&self) -> String {
        let mut s = String::new();
        for glyph in &self.text_run.glyphs[self.range.clone()] {
            s.push_str(&self.text_run.text[glyph.range.clone()]);
        }
        s
    }

    pub fn calculate_split_position(
        &mut self,
        max_width: f32,
        remaining_width: f32,
        font: &Font,
        font_context: &mut FontContext,
    ) -> Option<(Option<SplitInfo>, Option<SplitInfo>)> {
        let mut total_width = 0.0;

        let font_ref = font.as_ref(font_context);

        let glyphs_iter = self.text_run.glyphs[self.range.clone()].iter();
        let glyphs_length = glyphs_iter.len();

        let mut break_normal_position: Option<usize> = None;

        // No break because it calculate total width
        for (i, glyph) in glyphs_iter.enumerate() {
            let text = &self.text_run.text[glyph.range.clone()];
            let scaled_font = font_ref.as_scaled(PxScale::from(font.size));
            for c in text.chars() {
                let advanced_width = scaled_font.h_advance(scaled_font.glyph_id(c));
                total_width += advanced_width;
                if total_width > remaining_width && break_normal_position.is_none() {
                    break_normal_position = Some(i);
                }
            }
            if break_normal_position.is_some() {
                break;
            }
        }

        let idx = match break_normal_position {
            Some(idx) => idx,
            None => {
                return Some((
                    None,
                    Some(SplitInfo::new(self.range.start..self.range.end, true)),
                ))
            }
        };

        if idx == 0 && total_width > max_width {
            return Some((
                Some(SplitInfo::new(self.range.start..self.range.end, true)),
                None,
            ));
        }

        if idx == 0 && self.text_run.has_start && self.flags.contains(&TextFlags::SuppressLineBreakBefore) {
            return None;
        }

        let break_point = idx + self.range.start;

        if idx == glyphs_length - 1 {
            if let Some(glyph) = self.text_run.glyphs.get(break_point) {
                if glyph.glyph_store.is_whitespace {
                    return Some((
                        Some(SplitInfo::new(self.range.start..break_point, false)),
                        None,
                    ));
                }
            }
        }

        if idx == 0 {
            if let Some(glyph) = self.text_run.glyphs.get(break_point) {
                if glyph.glyph_store.is_whitespace {
                    return Some((
                        None,
                        Some(SplitInfo::new(break_point + 1..self.range.end, true)),
                    ));
                }
            }
        }

        if idx == 0 && self.text_run.has_start {
            return Some((
                None,
                Some(SplitInfo::new(self.range.start..self.range.end, true)),
            ));
        }

        let inline_start = SplitInfo::new(self.range.start..break_point, false);
        let mut inline_end = None;

        if break_point != self.range.end {
            inline_end = Some(SplitInfo::new(break_point..self.range.end, false));
        }

        Some((Some(inline_start), inline_end))
    }
}

#[derive(Clone)]
pub struct SplitInfo {
    pub range: Range<usize>,
    pub is_hidden: bool,
}

impl SplitInfo {
    pub fn new(range: Range<usize>, is_hidden: bool) -> SplitInfo {
        SplitInfo { range, is_hidden }
    }
}

impl Default for SplitInfo {
    fn default() -> SplitInfo {
        SplitInfo::new(0..0, false)
    }
}

struct RunInfo {
    pub text: String,
    pub font: Font,
}

#[derive(Debug, Clone)]
pub struct GlyphStore {
    pub is_whitespace: bool,
}

impl GlyphStore {
    fn new(is_whitespace: bool) -> GlyphStore {
        GlyphStore { is_whitespace }
    }
}

#[derive(Debug, Clone)]
pub struct GlyphRun {
    pub glyph_store: GlyphStore,
    pub range: Range<usize>,
}

impl GlyphRun {
    fn new(glyph_store: GlyphStore, range: Range<usize>) -> GlyphRun {
        GlyphRun { glyph_store, range }
    }
}

#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub descriptor: FontProperties,
    pub size: f32,
    pub font: Font,
    pub cache_key: FontCacheKey,
    pub glyphs: Vec<GlyphRun>,
    pub has_start: bool,
}

impl TextRun {
    pub fn new(
        text: String,
        size: f32,
        descriptor: FontProperties,
        font: Font,
        breaker: &mut Option<LineBreakLeafIter>,
        has_start: bool,
    ) -> (TextRun, bool) {
        let (glyphs, break_at_zero) = TextRun::split_with_line_break_opportunity(&text, breaker);
        (
            TextRun {
                cache_key: FontCacheKey::new(size, descriptor, font.family_name.clone()),
                text,
                size,
                descriptor,
                font,
                glyphs,
                has_start,
            },
            break_at_zero,
        )
    }

    fn split_with_line_break_opportunity(
        text: &str,
        breaker: &mut Option<LineBreakLeafIter>,
    ) -> (Vec<GlyphRun>, bool) {
        let mut glyphs = vec![];
        let mut slice = 0..0;

        let mut finished = false;
        let mut break_at_zero = false;

        if breaker.is_none() {
            if text.is_empty() {
                return (glyphs, true);
            }
            *breaker = Some(LineBreakLeafIter::new(text, 0));
        }

        let breaker = breaker.as_mut().unwrap();

        while !finished {
            let (idx, _is_hard_break) = breaker.next(text);
            if idx == text.len() {
                finished = true;
            }
            if idx == 0 {
                break_at_zero = true;
            }

            // Extend the slice to the next UAX#14 line break opportunity.
            slice.end = idx;
            let word = &text[slice.clone()];

            // Split off any trailing whitespace into a separate glyph run.
            let mut whitespace = slice.end..slice.end;

            if let Some((i, _)) = word
                .char_indices()
                .rev()
                .take_while(|&(_, c)| char_is_whitespace(c))
                .last()
            {
                whitespace.start = slice.start + i;
                slice.end = whitespace.start;
            } else {
                // TODO: Support break-word: keep-all;
            }

            if !slice.is_empty() {
                glyphs.push(GlyphRun::new(GlyphStore::new(false), slice.clone()));
            }
            if !whitespace.is_empty() {
                glyphs.push(GlyphRun::new(GlyphStore::new(true), whitespace.clone()));
            }

            slice.start = whitespace.end;
        }

        (glyphs, break_at_zero)
    }

    pub fn scan_for_runs<'a>(
        layout_box: &mut LayoutBox<'a>,
        styled_node: &'a StyledNode<'a>,
        font_context: &mut FontContext,
        last_whitespace: &mut bool,
        breaker: &mut Option<LineBreakLeafIter>,
    ) {
        let content = match &styled_node.node.node_type {
            NodeType::Text(text) => text,
            _ => unreachable!(),
        };
        let descriptor = create_font_properties(styled_node);
        let size = styled_node.font_size();
        let families = styled_node.font_family();

        let mut script = Script::Common;
        let mut font: Option<Font> = None;
        let mut run_info_list = vec![];

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
                        if let Some(font) = font {
                            run_info_list.push(RunInfo {
                                text: transform_text(
                                    content,
                                    &mut start_pos,
                                    end_pos,
                                    last_whitespace,
                                ),
                                font,
                            });
                        }
                    }
                    font = new_font;
                    script = new_script;
                }
            }
            end_pos += ch.len_utf8();
        }

        if start_pos != end_pos {
            run_info_list.push(RunInfo {
                text: transform_text(content, &mut start_pos, end_pos, last_whitespace),
                font: font.unwrap(),
            });
        }

        for (i, run) in run_info_list.into_iter().enumerate() {
            let mut flags = vec![];
            let (text_run, break_at_zero) =
                TextRun::new(run.text, size, descriptor, run.font, breaker, i == 0);
            if !break_at_zero && i == 0 {
                flags.push(TextFlags::SuppressLineBreakBefore);
            }

            let child = LayoutBox::new(BoxType::TextNode(TextNode::new(
                styled_node,
                text_run,
                flags,
            )));

            layout_box.get_inline_container().children.push(child);
        }
    }
}

fn is_compatible(new: Script, old: Script) -> bool {
    new == old || !is_specific(new) || !is_specific(old)
}

fn is_specific(script: Script) -> bool {
    script != Script::Common && script != Script::Inherited
}

fn transform_text(
    content: &str,
    start_pos: &mut usize,
    end_pos: usize,
    last_whitespace: &mut bool,
) -> String {
    let mut text = String::new();
    let sliced_content = &content[(*start_pos)..end_pos];
    for ch in sliced_content.chars() {
        let is_whitespace = is_in_whitespace(ch);
        if !is_whitespace {
            text.push(ch);
        } else if !*last_whitespace {
            text.push(' ');
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
