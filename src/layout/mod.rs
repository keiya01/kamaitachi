pub mod font;
mod inline;

use crate::cssom::{Unit, Value};
use crate::dom::NodeType;
use crate::style::*;
use font::{GlyphBrushFont, PxScale, ScaleFont};
use inline::InlineBox;
use std::cell::RefCell;
use std::mem;
use std::ops::Range;
use std::rc::Rc;

// CSS box model. All sizes are in px.

#[derive(Default, Debug, Clone)]
pub struct Dimensions {
    pub content: Rect,

    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    pub fn padding_box(&self) -> Rect {
        self.content.expanded_by(&self.padding)
    }

    pub fn border_box(&self) -> Rect {
        self.padding_box().expanded_by(&self.border)
    }

    pub fn margin_box(&self) -> Rect {
        self.border_box().expanded_by(&self.margin)
    }

    pub fn padding_horizontal_box(&self) -> Rect {
        self.content.expanded_horizontal_by(&self.padding)
    }

    pub fn border_horizontal_box(&self) -> Rect {
        self.padding_horizontal_box()
            .expanded_horizontal_by(&self.border)
    }

    pub fn margin_horizontal_box(&self) -> Rect {
        self.border_horizontal_box()
            .expanded_horizontal_by(&self.margin)
    }

    pub fn padding_left_offset(&self) -> f32 {
        self.padding.left
    }

    pub fn border_left_offset(&self) -> f32 {
        self.padding_left_offset() + self.border.left
    }

    pub fn margin_left_offset(&self) -> f32 {
        self.border_left_offset() + self.margin.left
    }

    pub fn padding_right_offset(&self) -> f32 {
        self.padding.right
    }

    pub fn border_right_offset(&self) -> f32 {
        self.padding_right_offset() + self.border.right
    }

    pub fn margin_right_offset(&self) -> f32 {
        self.border_right_offset() + self.margin.right
    }
}

#[derive(Default, Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    fn expanded_by(&self, edge: &EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }

    fn expanded_horizontal_by(&self, edge: &EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y,
            width: self.width + edge.left + edge.right,
            height: self.height,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug)]
pub struct LayoutBox<'a> {
    pub dimensions: Rc<RefCell<Dimensions>>,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
    /// used for line breaking
    pub is_splitted: bool,
}

impl<'a> Clone for LayoutBox<'a> {
    fn clone(&self) -> LayoutBox<'a> {
        let mut layout_box = LayoutBox::new(self.box_type.clone());
        let d = self.dimensions.borrow();
        layout_box.dimensions = Rc::new(RefCell::new(d.clone()));
        layout_box.children = self.children.clone();
        layout_box
    }
}

impl<'a> LayoutBox<'a> {
    pub fn new(box_type: BoxType<'a>) -> LayoutBox<'a> {
        LayoutBox {
            box_type,
            dimensions: Rc::new(RefCell::new(Default::default())),
            children: vec![],
            is_splitted: false,
        }
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match &self.box_type {
            BoxType::BlockNode(node) | BoxType::InlineNode(node) => node,
            BoxType::TextNode(node) => node.styled_node,
            BoxType::AnonymousBlock => panic!("Anonymous block box has no style node"),
        }
    }

    pub fn layout(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
        match self.box_type {
            BoxType::BlockNode(_) => self.layout_block(containing_block),
            BoxType::AnonymousBlock => {
                let containing_block = containing_block.borrow();
                {
                    let mut d = self.dimensions.borrow_mut();
                    d.content.x = containing_block.content.x;
                    d.content.y = containing_block.content.y;
                }
                // Anonymous block is including only inline box in children
                let mut inline_box = InlineBox::new(
                    containing_block.clone(),
                    mem::replace(&mut self.children, Vec::new()),
                );
                inline_box.process();
                let mut d = self.dimensions.borrow_mut();
                d.content.width = inline_box.width;
                d.content.height = inline_box.height;
                self.children = inline_box.boxes;
            }
            // All inline node and text node are contained in anonymous box.
            BoxType::InlineNode(_) | BoxType::TextNode(_) => unreachable!(),
        }
    }

    fn layout_block(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
        // Child width depends on parent width,
        // so parent width need to be calculated before child width.
        self.calculate_block_width(containing_block.clone());

        self.calculate_block_position(containing_block);

        self.layout_block_children();

        // Parent height is affected by child layout,
        // so parent height need to be calculated after children are laid out.
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
        let style = self.get_style_node();

        // `width` has initial value `auto`.
        let auto = Value::Keyword("auto".into());
        let mut width = style.value("width").unwrap_or_else(|| auto.clone());

        // `margin`, `border`, `padding` has initial value `0`.
        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border", &zero);
        let border_right = style.lookup("border-right-width", "border", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total: f32 = [
            margin_left.to_px(),
            margin_right.to_px(),
            border_left.to_px(),
            border_right.to_px(),
            padding_left.to_px(),
            padding_right.to_px(),
            width.to_px(),
        ]
        .iter()
        .sum();

        let containing_block = containing_block.borrow();

        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Value::Length(0.0, Unit::Px);
            }
            if margin_right == auto {
                margin_right = Value::Length(0.0, Unit::Px);
            }
        }

        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            (false, false, false) => {
                margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
            }
            (false, false, true) => {
                margin_right = Value::Length(underflow, Unit::Px);
            }
            (false, true, false) => {
                margin_left = Value::Length(underflow, Unit::Px);
            }
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = Value::Length(0.0, Unit::Px)
                }
                if margin_right == auto {
                    margin_right = Value::Length(0.0, Unit::Px)
                }

                if underflow > 0.0 {
                    width = Value::Length(underflow, Unit::Px);
                } else {
                    width = Value::Length(0.0, Unit::Px);
                    margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
                }
            }
            (false, true, true) => {
                margin_left = Value::Length(underflow / 2.0, Unit::Px);
                margin_right = Value::Length(underflow / 2.0, Unit::Px);
            }
        }

        let mut d = self.dimensions.borrow_mut();
        d.content.width = width.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();
    }

    fn calculate_block_position(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
        let containing_block = containing_block.borrow();

        self.assign_vertical_margin_box();

        let mut d = self.dimensions.borrow_mut();

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;

        d.content.y = containing_block.content.height
            + containing_block.content.y
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_block_children(&mut self) {
        let parent_dimensions = &self.dimensions;
        for child in &mut self.children {
            child.layout(parent_dimensions.clone());
            let mut d = parent_dimensions.borrow_mut();
            let child_d = child.dimensions.borrow();
            d.content.height += child_d.margin_box().height;
        }
    }

    fn calculate_block_height(&mut self) {
        if let Some(Value::Length(height, Unit::Px)) = self.get_style_node().value("height") {
            self.dimensions.borrow_mut().content.height = height;
        }
    }

    /// When a anonymous box has some node, this node will be placed horizontally.
    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            BoxType::TextNode(_) | BoxType::InlineNode(_) | BoxType::AnonymousBlock => self,
            BoxType::BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: BoxType::AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(BoxType::AnonymousBlock)),
                };
                self.children.last_mut().unwrap()
            }
        }
    }

    fn assign_vertical_margin_box(&self) {
        let node = self.get_style_node();
        let mut d = self.dimensions.borrow_mut();
        let zero = Value::Length(0.0, Unit::Px);

        d.margin.top = node.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = node.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = node.lookup("border-top-width", "border", &zero).to_px();
        d.border.bottom = node.lookup("border-bottom-width", "border", &zero).to_px();

        d.padding.top = node.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = node.lookup("padding-bottom", "padding", &zero).to_px();
    }

    fn assign_horizontal_margin_box(&self) {
        let node = self.get_style_node();
        let mut d = self.dimensions.borrow_mut();
        let zero = Value::Length(0.0, Unit::Px);

        d.margin.left = node.lookup("margin-left", "margin", &zero).to_px();
        d.margin.right = node.lookup("margin-right", "margin", &zero).to_px();

        d.border.left = node.lookup("border-left-width", "border", &zero).to_px();
        d.border.right = node.lookup("border-right-width", "border", &zero).to_px();

        d.padding.left = node.lookup("padding-left", "padding", &zero).to_px();
        d.padding.right = node.lookup("padding-right", "padding", &zero).to_px();
    }

    fn reset_edge_left(&mut self) {
        self.is_splitted = true;
        let mut d = self.dimensions.borrow_mut();
        d.margin.left = 0.;
        d.border.left = 0.;
        d.padding.left = 0.;
        if self.children.len() != 0 {
            self.children[0].reset_edge_left();
        }
    }

    fn reset_edge_right(&mut self) {
        self.is_splitted = true;
        let mut d = self.dimensions.borrow_mut();
        d.margin.right = 0.;
        d.border.right = 0.;
        d.padding.right = 0.;
        let len = self.children.len();
        if len != 0 {
            self.children[len - 1].reset_edge_right();
        }
    }
}

#[derive(Debug, Clone)]
pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    TextNode(TextNode<'a>),
    AnonymousBlock,
}

#[derive(Debug, Clone)]
pub struct TextNode<'a> {
    pub styled_node: &'a StyledNode<'a>,
    pub font: font::Font,
    pub range: Range<usize>,
}

impl<'a> TextNode<'a> {
    fn new(styled_node: &'a StyledNode<'a>, content: &str) -> TextNode<'a> {
        let font = font::Font::new(None, styled_node.font_size());
        TextNode {
            styled_node,
            font,
            range: 0..content.len(),
        }
    }

    pub fn get_text(&self) -> &str {
        let text = match &self.styled_node.node.node_type {
            NodeType::Text(text) => text,
            _ => unreachable!(),
        };
        &text[self.range.clone()]
    }

    // TODO: stop splitting just before inline box
    // TODO: consider if inline_start is NONE
    fn calculate_split_position(
        &self,
        text_node: &TextNode,
        remaining_width: f32,
    ) -> (Option<SplitInfo>, Option<SplitInfo>) {
        let text = text_node.get_text();

        let mut total_width = 0.0;
        // priority: 1
        let mut space_position: Option<usize> = None;
        // priority: 2
        let mut break_point: usize = text.len();

        let font_ref = text_node.font.as_ref();
        let scaled_font = font_ref.as_scaled(PxScale::from(text_node.font.size));
        for (i, c) in text.chars().enumerate() {
            let advanced_width = scaled_font.h_advance(scaled_font.glyph_id(c));
            total_width += advanced_width;
            if total_width > remaining_width {
                break;
            }
            if c.is_whitespace() {
                space_position = Some(i);
            }
            break_point = i;
        }

        if let Some(pos) = space_position {
            break_point = pos;
        }

        break_point += text_node.range.start + 1;

        let inline_start = SplitInfo::new(text_node.range.start..break_point);
        let mut inline_end = None;

        if break_point != text_node.range.end {
            inline_end = Some(SplitInfo::new(break_point..text_node.range.end));
        }

        (Some(inline_start), inline_end)
    }
}

#[derive(Clone)]
pub struct SplitInfo {
    range: Range<usize>,
}

impl SplitInfo {
    pub fn new(range: Range<usize>) -> SplitInfo {
        SplitInfo { range }
    }
}

impl Default for SplitInfo {
    fn default() -> SplitInfo {
        SplitInfo::new(0..0)
    }
}

pub fn layout_tree<'a>(
    node: &'a StyledNode<'a>,
    containing_block: Rc<RefCell<Dimensions>>,
) -> LayoutBox<'a> {
    // The layout algorithm expects the container height to start at 0.
    // TODO: Save the initial containing block height, for calculating percent heights.
    containing_block.borrow_mut().content.height = 0.0;

    let mut root_box = build_layout_tree(node);
    root_box.layout(containing_block);
    root_box
}

pub fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BoxType::BlockNode(style_node),
        Display::Inline => match &style_node.node.node_type {
            NodeType::Element(_) => BoxType::InlineNode(style_node),
            NodeType::Text(content) => BoxType::TextNode(TextNode::new(style_node, content)),
        },
        Display::None => panic!("Root node must has `display: none;`."),
    });

    for child in &style_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root
                .get_inline_container()
                .children
                .push(build_layout_tree(child)),
            Display::None => {}
        }
    }

    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cssom::*;
    use crate::parser::css::*;
    use crate::parser::html::*;

    #[test]
    fn test_block() {
        let html = "
    <body class='bar'>
      <div id='foo' class='bar'></div>
      <div>test</div>
      <div>test</div>
    </body>
    ";
        let ua_css = "body, div { display: block; }";
        let css = "
    .bar {
      height: auto;
      width: 1000px;
    }
    
    div {
      width: 100px;
      height: 200px;
      margin: auto;
    }
    
    div#foo.bar {
      height: auto;
    }
    
    div#foo {
      color: red;
    }
    ";

        let mut html_parser = HTMLParser::new(html.into());
        let mut ua_css_parser = CSSParser::new(ua_css.into());
        let mut css_parser = CSSParser::new(css.into());

        let dom = html_parser.run();

        let ua_rules = ua_css_parser.parse_rules(Origin::UA);
        let mut rules = css_parser.parse_rules(Origin::Author);
        rules.extend(ua_rules);
        let cssom = Stylesheet::new(rules);

        let styled_node = create_style_tree(&dom, &cssom, None);

        let mut viewport: Dimensions = Default::default();
        viewport.content.width = 800.0;
        viewport.content.height = 600.0;

        layout_tree(&styled_node, Rc::new(RefCell::new(viewport)));
    }
}
