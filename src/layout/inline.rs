use super::font::Font;
use super::{BoxType, Dimensions, LayoutBox, Rect, TextNode};
use std::collections::VecDeque;
use std::iter::Iterator;
use std::mem;
use std::ops::Range;

#[derive(Clone)]
struct Line {
    range: Range<usize>,
    bounds: Dimensions,
    green_zone: Rect,
    metrics: LineMetrics,
    is_line_broken: bool,
}

impl Line {
    pub fn new(bounds: Dimensions) -> Line {
        Line {
            range: 0..0,
            bounds,
            green_zone: Default::default(),
            metrics: LineMetrics::new(),
            is_line_broken: false,
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

    fn scan_for_line<I>(&mut self, root: &Dimensions, iter_old_boxes: &mut I)
    where
        I: Iterator<Item = LayoutBox<'a>>,
    {
        self.layout_boxes(root, iter_old_boxes);
    }

    fn next_layout_box<I>(&mut self, iter_old_boxes: &mut I) -> Option<LayoutBox<'a>>
    where
        I: Iterator<Item = LayoutBox<'a>>,
    {
        self.work_list.pop_front().or_else(|| iter_old_boxes.next())
    }

    fn layout_boxes<I>(&mut self, root: &Dimensions, iter_old_boxes: &mut I)
    where
        I: Iterator<Item = LayoutBox<'a>>,
    {
        while let Some(layout_box) = &mut self.next_layout_box(iter_old_boxes) {
            self.layout(root, layout_box);
            if self.pending_line.is_line_broken {
                self.flush_current_line();
            }
        }

        if !self.pending_line_is_empty() {
            self.flush_current_line();
        }
    }

    fn layout(&mut self, root: &Dimensions, layout_box: &mut LayoutBox<'a>) {
        if self.pending_line_is_empty() {
            let line_bounds = self.initial_line_placement(root, layout_box);
            self.pending_line.bounds.content.x = line_bounds.content.x;
            self.pending_line.bounds.content.y = line_bounds.content.y;
            self.pending_line.green_zone.width = line_bounds.content.width;
            if self.lines.len() != 0 {
                let last_line_index = self.lines.len() - 1;
                let end_last_line_range = self.lines[last_line_index].range.end;
                self.pending_line.range = end_last_line_range..end_last_line_range;
            }
        }

        self.pending_line.range.end += 1;

        match &layout_box.box_type {
            BoxType::InlineNode(_) => self.layout_inline(root, layout_box),
            BoxType::TextNode(_) => self.layout_text(layout_box),
            BoxType::BlockNode(_) => unimplemented!(),
            _ => unreachable!(),
        }
    }

    fn layout_inline(&mut self, root: &Dimensions, layout_box: &mut LayoutBox<'a>) {
        layout_box.assign_horizontal_margin_box();
        layout_box.assign_vertical_margin_box();

        {
            let d = layout_box.dimensions.borrow();
            self.pending_line.bounds.content.width += d.margin_left_offset();
        }
        
        self.calculate_inline_descendant_position(root, layout_box);

        {
            let d = layout_box.dimensions.borrow();
            self.pending_line.bounds.content.width += d.margin_right_offset();
        }
    }

    fn calculate_inline_descendant_position(
        &mut self,
        root: &Dimensions,
        layout_box: &mut LayoutBox<'a>,
    ) {
        let mut total_width = 0.;
        let containing_block = &layout_box.dimensions;
        let mut new_children = vec![];
        let mut broken_line_children = vec![];

        let mut i = 0;
        for child in &mut layout_box.children {
            self.layout(root, child);

            if self.pending_line.is_line_broken && broken_line_children.len() != 0 {
                broken_line_children.push(child.clone());
                continue;
            }

            if self.pending_line.is_line_broken {
                if let Some(layout_box) = self.work_list.pop_front() {
                    broken_line_children.push(layout_box);
                } else {
                    break;
                };
            }

            {
                let mut d = child.dimensions.borrow_mut();
                let margin_box = d.margin_horizontal_box();
                if self.lines.len() != 0 {
                    d.content.x += total_width;
                    if i == 0 {
                        total_width += margin_box.width - d.margin_left_offset();
                    } else {
                        total_width += margin_box.width;
                    }
                } else {
                    d.content.x += total_width + containing_block.borrow().margin_left_offset();
                    total_width += margin_box.width;
                }
            }

            new_children.push(child.clone());

            // Remove descendant from new_boxes
            self.pending_line.range.end -= 1;
            self.new_boxes.pop();
            i += 1;
        }

        if self.pending_line.is_line_broken && broken_line_children.len() != 0 {
            let mut new_layout_box = layout_box.clone();
            new_layout_box.children = broken_line_children;
            self.work_list.push_front(new_layout_box);
        }

        if new_children.len() != 0 {
            layout_box.children = new_children;
            {
                let mut containing_block = containing_block.borrow_mut();
                containing_block.content.width = total_width;
                containing_block.content.height =
                    Font::new_from_style(layout_box.get_style_node()).ascent;
            }
            self.new_boxes.push(layout_box.clone());
        } else {
            self.pending_line.range.end -= 1;
        }
    }

    fn layout_text(&mut self, layout_box: &mut LayoutBox<'a>) {
        let mut node = match &mut layout_box.box_type {
            BoxType::TextNode(node) => node,
            _ => unreachable!(),
        };

        let text_width = self.text_width(node);

        let remaining_width =
            self.pending_line.green_zone.width - self.pending_line.bounds.content.width;

        if text_width > remaining_width || self.pending_line.is_line_broken {
            // TODO: assign remaining character
            // TODO: create new text node
            if self.pending_line.is_line_broken {
                self.pending_line.range.end -= 1;
                return;
            }

            self.pending_line.is_line_broken = true;

            let (inline_start, inline_end) = node.calculate_split_position(node, remaining_width);

            if let Some(inline_start) = inline_start {
                node.range = inline_start.range;
                let metrics = self
                    .pending_line
                    .metrics
                    .calc_space(node, node.styled_node.line_height());
                {
                    let mut d = layout_box.dimensions.borrow_mut();
                    d.content.height = node.font.ascent + node.font.descent;
                    // Maybe, this calculation is specific case for `iced`
                    d.content.y -= metrics.leading / 2.;
                    d.content.width = self.text_width(node);
                }
                self.new_boxes.push(layout_box.clone());
            }

            if let Some(inline_end) = inline_end {
                let mut new_layout_box = layout_box.clone();
                let mut node = match &mut new_layout_box.box_type {
                    BoxType::TextNode(node) => node,
                    _ => unreachable!(),
                };
                node.range = inline_end.range;
                self.work_list.push_front(new_layout_box);
            }
        } else {
            let metrics = self
                .pending_line
                .metrics
                .calc_space(node, node.styled_node.line_height());
            {
                let mut d = layout_box.dimensions.borrow_mut();
                d.content.height = node.font.ascent + node.font.descent;
                // Maybe, this calculation is specific case for `iced`
                d.content.y -= metrics.leading / 2.;
                d.content.width = text_width;
            }
            self.pending_line.bounds.content.width += text_width;
            self.new_boxes.push(layout_box.clone());
        }
    }

    fn initial_line_placement(&self, root: &Dimensions, _layout_box: &LayoutBox) -> Dimensions {
        // refer: https://github.com/servo/servo/blob/3f7697690aabd2d8c31bc880fcae21250244219a/components/layout/inline.rs#L500
        // let width = if layout_box.can_split() {
        //   self.minimum_splittable_inline_width(&layout_box)
        // } else {
        //   // TODO: for `block box` and `inline block box`
        //   unimplemented!();
        // };

        // TODO: calculate float dimensions
        root.clone()
    }

    fn text_width(&self, node: &TextNode<'a>) -> f32 {
        let text = node.get_text();
        // TODO: optimize to load font only once
        node.font.width(text)
    }

    fn pending_line_is_empty(&self) -> bool {
        self.pending_line.range.end == 0
    }

    fn flush_current_line(&mut self) {
        self.lines.push(self.pending_line.clone());
        self.pending_line = Line::new(Default::default());
    }

    fn reset_line_edge(&mut self) {
        if self.lines.len() == 0 {
            return;
        }

        let mut line_index = 0;
        let last_line_index = self.lines.len() - 1;
        for line in &self.lines {
            let box_list = &mut self.new_boxes[line.range.clone()];
            if box_list.len() == 0 {
                break;
            }
            if line_index != 0 {
                let item = &mut box_list[0];
                item.reset_edge_left();
            }
            if line_index != last_line_index {
                let item = &mut box_list[box_list.len() - 1];
                item.reset_edge_right();
            }
            line_index += 1;
        }
    }
}

pub struct InlineBox<'a> {
    pub root: Dimensions,
    pub boxes: Vec<LayoutBox<'a>>,
    pub width: f32,
    pub height: f32,
}

impl<'a> InlineBox<'a> {
    pub fn new(root: Dimensions, boxes: Vec<LayoutBox<'a>>) -> InlineBox<'a> {
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
        line_breaker.reset_line_edge();
        self.assign_position(&mut line_breaker);
        self.boxes = line_breaker.new_boxes;
        self.width = line_breaker.max_width;
        self.height = line_breaker.cur_height;
    }

    fn assign_position(&self, line_breaker: &mut LineBreaker<'a>) {
        for line in &line_breaker.lines {
            let mut line_box_x = line.bounds.content.x;
            for item in &mut line_breaker.new_boxes[line.range.clone()] {
                let new_rect_y =
                    line_breaker.cur_height + line.bounds.content.y + line.metrics.leading;
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
