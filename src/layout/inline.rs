use super::font::Font;
use super::{BoxType, Dimensions, LayoutBox, Rect, TextNode};
use crate::dom::NodeType;
use std::collections::VecDeque;
use std::mem;
use std::iter::Iterator;
use std::ops::Range;

#[derive(Clone)]
struct Line {
    range: Range<usize>,
    bounds: Dimensions,
    green_zone: Rect,
    metrics: LineMetrics,
}

impl Line {
    pub fn new(bounds: Dimensions) -> Line {
        Line {
            range: 0..0,
            bounds,
            green_zone: Default::default(),
            metrics: LineMetrics::new(),
        }
    }
}

struct LineBreaker<'a> {
    work_list: VecDeque<LayoutBox<'a>>,
    new_boxes: Vec<LayoutBox<'a>>,
    lines: Vec<Line>,
    pending_line: Line,
    // Largest width in each lines
    max_width: f32,
    cur_height: f32,
    // This value is express index of text range
    // If this value has index, line is broken
    last_known_line_breaking_opportunity: Option<usize>,
}

impl<'a> LineBreaker<'a> {
    fn new() -> LineBreaker<'a> {
        LineBreaker {
            work_list: VecDeque::new(),
            new_boxes: vec![],
            lines: vec![],
            pending_line: Line::new(Default::default()),
            max_width: 0.0,
            cur_height: 0.0,
            last_known_line_breaking_opportunity: None,
        }
    }

    fn scan_for_line<I>(&mut self, root: &LayoutBox<'a>, iter_old_boxes: &mut I)
    where
        I: Iterator<Item = LayoutBox<'a>>
    {
        self.layout_boxes(root, iter_old_boxes);
    }

    fn next_layout_box<I>(&mut self, iter_old_boxes: &mut I) -> Option<LayoutBox<'a>>
    where
        I: Iterator<Item = LayoutBox<'a>>
    {
        self.work_list.pop_front().or_else(|| iter_old_boxes.next())
    }

    fn layout_boxes<I>(&mut self, root: &LayoutBox<'a>, iter_old_boxes: &mut I)
    where
        I: Iterator<Item = LayoutBox<'a>>
    {
        while let Some(layout_box) = self.next_layout_box(iter_old_boxes) {
            self.layout(root, &layout_box);
        }

        if !self.pending_line_is_empty() {
            self.lines.push(self.pending_line.clone());
            self.pending_line.range = 0..0;
        }
    }

    fn layout(&mut self, root: &LayoutBox<'a>, layout_box: &LayoutBox<'a>) {
        if self.pending_line_is_empty() {
            let line_bounds = self.initial_line_placement(root, layout_box);
            self.pending_line.bounds.content.x = line_bounds.content.x;
            self.pending_line.bounds.content.y = line_bounds.content.y;
            self.pending_line.green_zone.width = line_bounds.margin_horizontal_box().width;
        }

        // TODO: Check inline box is fit in green_zone
        self.pending_line.range.end += 1;

        match &layout_box.box_type {
            BoxType::InlineNode(_) => self.layout_inline(root, layout_box),
            BoxType::TextNode(node) => self.layout_text(node, layout_box),
            BoxType::BlockNode(_) => unimplemented!(),
            _ => unreachable!(),
        }
    }

    fn layout_inline(&mut self, root: &LayoutBox<'a>, layout_box: &LayoutBox<'a>) {
        layout_box.assign_horizontal_margin_box();
        layout_box.assign_vertical_margin_box();

        for child in &layout_box.children {
            self.layout(root, child);
        }

        self.calculate_inline_descendant_position(layout_box);

        self.new_boxes.push(layout_box.clone());
    }

    fn calculate_inline_descendant_position(&mut self, layout_box: &LayoutBox<'a>) {
        let mut total_width = 0.;
        let mut containing_block = layout_box.dimensions.borrow_mut();
        for child in &layout_box.children {
            let mut d = child.dimensions.borrow_mut();
            let margin_box = d.margin_horizontal_box();
            d.content.x = total_width + containing_block.margin_left_offset();
            total_width += margin_box.width;

            // Remove descendant from new_boxes
            self.pending_line.range.end -= 1;
            self.new_boxes.pop();
        }

        containing_block.content.width = total_width;
        containing_block.content.height = Font::new_from_style(layout_box.get_style_node()).ascent;
    }

    fn layout_text(&mut self, node: &TextNode, layout_box: &LayoutBox<'a>) {
        let mut d = layout_box.dimensions.borrow_mut();
        d.content.width = self.text_width(node);
        let metrics = self
            .pending_line
            .metrics
            .calc_space(node, node.styled_node.line_height());
        d.content.height = node.font.ascent + node.font.descent;
        // Maybe, this calculation is specific case for `iced`
        d.content.y -= metrics.leading / 2.;

        self.new_boxes.push(layout_box.clone());
    }

    fn initial_line_placement(&self, root: &LayoutBox, _layout_box: &LayoutBox) -> Dimensions {
        // refer: https://github.com/servo/servo/blob/3f7697690aabd2d8c31bc880fcae21250244219a/components/layout/inline.rs#L500
        // let width = if layout_box.can_split() {
        //   self.minimum_splittable_inline_width(&layout_box)
        // } else {
        //   // TODO: for `block box` and `inline block box`
        //   unimplemented!();
        // };

        // TODO: calculate float dimensions
        root.dimensions.borrow().clone()
    }

    fn text_width(&self, node: &TextNode<'a>) -> f32 {
        let styled_node = node.styled_node;
        let text = if let NodeType::Text(text) = &styled_node.node.node_type {
            text
        } else {
            unreachable!();
        };
        // TODO: optimize to load font only once
        node.font.width(text)
    }

    fn pending_line_is_empty(&self) -> bool {
        self.pending_line.range.end == 0
    }

    fn calculate_split_position(&self) {
        // 1. 一文字づつadvanced_widthを確認していく
        // 2. そのadvanced_widthを基に残りのwidthを計算していく
        // 3. advanced_widthが、残りのwidthより大きい場合、そこがbreak pointとなる(inline_start)
        //    ただし、foo<span>bar</span>のような場合、fooとbarの間で改行することはしない
        // 4. 残りのwidthを超えた文字列の位置から最後までの位置(inline_end)を記憶しておく
        // 5. inline_start、inline_endのrangeをTextNodeのrangeに入れて、新しいTextNodeを作る
        // 6. inline_startとinline_endが存在する場合は、inline_startのrightのborder, paddingを0に、
        //    inline_endのleftのborder, paddingを0にする
        // 7. inline_startをLineBreaker::linesに入れて、inline_endをwork_listにいれる
    }
}

pub struct InlineBox<'a> {
    pub root: LayoutBox<'a>,
    pub boxes: Vec<LayoutBox<'a>>,
    pub width: f32,
    pub height: f32,
}

impl<'a> InlineBox<'a> {
    pub fn new(root: LayoutBox<'a>, boxes: Vec<LayoutBox<'a>>) -> InlineBox<'a> {
        InlineBox {
            root,
            boxes,
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn process(&mut self) {
        let mut line_breaker = LineBreaker::new();
        let old_boxes = mem::replace(&mut self.boxes, Vec::new());
        let mut iter_old_boxes = old_boxes.into_iter();
        line_breaker.scan_for_line(&self.root, &mut iter_old_boxes);
        self.assign_position(&mut line_breaker);
        self.boxes = line_breaker.new_boxes;
        self.width = line_breaker.max_width;
        self.height = line_breaker.cur_height;
    }

    fn assign_position(&self, line_breaker: &mut LineBreaker<'a>) {
        for line in &line_breaker.lines {
            let mut line_box_x = line.bounds.content.x;
            for item in &mut line_breaker.new_boxes[line.range.clone()] {
                let new_rect_y = line.bounds.content.y + line.metrics.leading;
                {
                    let mut d = item.dimensions.borrow_mut();
                    d.content.x += line_box_x + d.margin_left_offset();
                    d.content.y += new_rect_y;
                }
                if let BoxType::InlineNode(_) = item.box_type {
                    self.recursive_position(item, line_box_x, new_rect_y);
                }
                let d = item.dimensions.borrow();
                let margin_box = d.margin_horizontal_box();
                let line_box_width = margin_box.width;
                line_box_x += line_box_width;
                line_breaker.max_width = line_box_width.max(line_breaker.max_width);
            }
            line_breaker.cur_height +=
                line.metrics.space_above_baseline + line.metrics.space_under_baseline;
        }
    }

    fn recursive_position(
        &self,
        layout_box: &mut LayoutBox<'a>,
        additional_rect_x: f32,
        additional_rect_y: f32,
    ) {
        let mut new_rect_x = additional_rect_x;
        for child in &mut layout_box.children {
            if let BoxType::InlineNode(_) = child.box_type {
                let new_rect_x = {
                    let d = child.dimensions.borrow();
                    new_rect_x + d.margin_left_offset()
                };
                self.recursive_position(child, new_rect_x, additional_rect_y);
            }

            let mut d = child.dimensions.borrow_mut();

            new_rect_x += d.margin_horizontal_box().width;

            d.content.x += additional_rect_x + d.margin_left_offset();
            d.content.y += additional_rect_y;
        }
    }
}

#[derive(Debug, Clone)]
struct LineMetrics {
    space_above_baseline: f32,
    space_under_baseline: f32,
    ascent: f32,
    leading: f32,
}

impl LineMetrics {
    fn new() -> LineMetrics {
        LineMetrics {
            space_above_baseline: 0.,
            space_under_baseline: 0.,
            ascent: 0.,
            leading: 0.,
        }
    }

    fn new_from_style(
        space_above_baseline: f32,
        space_under_baseline: f32,
        ascent: f32,
        leading: f32,
    ) -> LineMetrics {
        LineMetrics {
            space_above_baseline,
            space_under_baseline,
            ascent,
            leading,
        }
    }

    fn calc_space(&mut self, text_node: &TextNode, line_height: f32) -> LineMetrics {
        let font_metrics = &text_node.font;
        let ascent = font_metrics.ascent;
        let descent = font_metrics.descent;
        let leading = line_height - (ascent + descent);

        let half_leading = leading / 2.;
        let space_above_baseline = ascent + half_leading;
        let space_under_baseline = descent + leading - half_leading;

        self.space_above_baseline = self.space_above_baseline.max(space_above_baseline);
        self.space_under_baseline = self.space_under_baseline.max(space_under_baseline);
        self.ascent = self.ascent.max(ascent);
        self.leading = self.leading.max(leading);

        LineMetrics::new_from_style(space_above_baseline, space_under_baseline, ascent, leading)
    }
}
