use super::font::{with_thread_local_font_context, Font, FontCacheKey, FontContext};
use super::text::{TextNode};
use super::{BoxType, Dimensions, LayoutBox, Rect};
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
    is_suppress_line_break_before: bool,
}

impl Line {
    pub fn new(bounds: Dimensions) -> Line {
        Line {
            range: 0..0,
            bounds,
            green_zone: Default::default(),
            metrics: LineMetrics::new(),
            is_line_broken: false,
            is_suppress_line_break_before: false,
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
    metrics: LineMetrics,
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
            metrics: LineMetrics::new(),
            last_known_line_breaking_opportunity: None,
        }
    }

    fn scan_for_line<I>(&mut self, root: &Dimensions, iter_old_boxes: &mut I)
    where
        I: Iterator<Item = LayoutBox<'a>>,
    {
        with_thread_local_font_context(|font_context| {
            self.layout_boxes(root, iter_old_boxes, font_context)
        });
    }

    fn next_layout_box<I>(&mut self, iter_old_boxes: &mut I) -> Option<LayoutBox<'a>>
    where
        I: Iterator<Item = LayoutBox<'a>>,
    {
        let item = self.work_list.pop_front();
        match item {
            Some(item) => Some(item),
            None => iter_old_boxes.next(),
        }
    }

    fn layout_boxes<I>(
        &mut self,
        root: &Dimensions,
        iter_old_boxes: &mut I,
        font_context: &mut FontContext,
    ) where
        I: Iterator<Item = LayoutBox<'a>>,
    {
        while let Some(layout_box) = &mut self.next_layout_box(iter_old_boxes) {
            self.layout(root, layout_box, font_context);

            self.pending_line.range.end += 1;

            if self.pending_line.is_line_broken
                && self.pending_line.is_suppress_line_break_before
            {
                if let Some(idx) = &self.last_known_line_breaking_opportunity {
                    for _ in (*idx..self.pending_line.range.end - 1).rev() {
                        let item = self.new_boxes.pop().unwrap();
                        self.work_list.push_front(item);
                    }

                    self.recursive_split_suppress_line(layout_box, font_context);
                } else {
                    self.new_boxes.push(layout_box.clone());
                }
            } else {
                if !layout_box.is_hidden {
                    self.new_boxes.push(layout_box.clone());
                } else {
                    self.pending_line.range.end -= 1;
                }
                self.last_known_line_breaking_opportunity = Some(self.pending_line.range.end);
            }

            if self.pending_line.is_line_broken {
                self.flush_current_line();
            }
        }

        if !self.pending_line_is_empty() {
            self.flush_current_line();
        }
    }

    fn recursive_split_suppress_line(
        &mut self,
        layout_box: &mut LayoutBox<'a>,
        font_context: &mut FontContext,
    ) {
        match self.split_suppressed_line(layout_box, font_context) {
            (Some(mut result), false) => {
                result.reset_all_edge_left();
                self.work_list.push_front(result);
                layout_box.reset_all_edge_right();
                self.new_boxes.push(layout_box.clone());
            }
            // If splitting position is inline box, search splittable position recursively.
            (Some(result), true) => {
                self.work_list.push_front(result);
                self.pending_line.range.end -= 1;
                if let Some(mut layout_box) = self.new_boxes.pop() {
                    self.recursive_split_suppress_line(&mut layout_box, font_context);
                }
            }
            // If splitting position is not found, search splittable position from new_boxes.
            (None, _) => {
                self.pending_line.range.end -= 1;
                let mut i = self.new_boxes.len();
                let mut new_boxes = vec![];
                while let Some(mut item) = self.new_boxes.pop() {
                    i -= 1;
                    match self.split_suppressed_line(&mut item, font_context) {
                        (Some(mut result), false) => {
                            result.reset_all_edge_left();
                            self.work_list.push_front(result);
                            item.reset_all_edge_right();
                            new_boxes.insert(0, item);
                            break;
                        }
                        (Some(result), true) => {
                            self.work_list.push_front(result);
                            self.pending_line.range.end -= 1;
                        }
                        (None, _) => {
                            self.work_list.push_front(item);
                            self.pending_line.range.end -= 1;
                        }
                    }
                    if self.pending_line.range.start == i {
                        break;
                    }
                }
                if self.new_boxes.len() == 0 {
                    // TODO: Support the case of all layout_box are line box
                    unimplemented!("Not supported if all layout_box are line box");
                } else {
                    self.new_boxes.append(&mut new_boxes);
                }
            }
        }
    }

    fn split_suppressed_line(
        &self,
        layout_box: &mut LayoutBox<'a>,
        font_context: &mut FontContext,
    ) -> (Option<LayoutBox<'a>>, bool) {
        if layout_box.is_hidden {
            return (None, false);
        }
        if let BoxType::TextNode(node) = &mut layout_box.box_type {
            let mut slice = node.range.clone();
            let (idx, glyph) = node.text_run.glyphs[node.range.clone()]
                .iter()
                .enumerate()
                .last()
                .unwrap();
            slice.start = node.range.start + idx;

            let is_inline_box = node.text_run.has_start && slice.start == 0;
            if is_inline_box {
                return (None, true);
            }
            node.range.end = slice.start;

            if glyph.glyph_store.is_whitespace {
                slice.start += 1;
                let font = font_context.get_or_create_by(&node.text_run.cache_key);
                let width = self.text_width(node, &font, font_context);
                layout_box.dimensions.borrow_mut().content.width = width;
            }

            let mut next_layout_box = layout_box.clone();

            if let BoxType::TextNode(node) = &mut next_layout_box.box_type {
                node.range = slice;
                let font = font_context.get_or_create_by(&node.text_run.cache_key);
                let width = self.text_width(node, &font, font_context);
                next_layout_box.dimensions.borrow_mut().content.width = width;
            }
            return (Some(next_layout_box), false);
        }

        let mut end_layout_box = layout_box.clone();
        end_layout_box.children = vec![];

        let mut old_children = mem::replace(&mut layout_box.children, vec![]);
        old_children.reverse();
        while let Some(mut child) = old_children.pop() {
            let result = self.split_suppressed_line(&mut child, font_context);
            match result {
                (Some(result), true) => {
                    if child.is_hidden {
                        layout_box.is_hidden = true;
                    }
                    end_layout_box.children.push(result);
                }
                (Some(result), false) => {
                    let mut d = layout_box.dimensions.borrow_mut();
                    // Sync inline box with layout_box width
                    d.content.width = child.dimensions.borrow().content.width;
                    if let BoxType::TextNode(_) = result.box_type {
                        // Remove splitted node width
                        d.content.width -= result.dimensions.borrow().content.width;
                    }
                    end_layout_box.children.push(result);
                    layout_box.children.push(child);

                    let is_first_line = self.lines.is_empty();
                    if !is_first_line {
                        while let Some(old_child) = old_children.pop() {
                            end_layout_box.children.push(old_child);
                        }
                    }

                    return (Some(end_layout_box), false);
                }
                (None, true) => {
                    layout_box.is_hidden = true;
                    end_layout_box.children.push(child);
                    return (Some(end_layout_box), true);
                }
                (None, false) => end_layout_box.children.push(child),
            }
        }

        if !end_layout_box.children.is_empty() {
            (Some(end_layout_box), true)
        } else {
            (None, true)
        }
    }

    fn layout(
        &mut self,
        root: &Dimensions,
        layout_box: &mut LayoutBox<'a>,
        font_context: &mut FontContext,
    ) {
        if self.pending_line_is_empty() {
            let line_bounds = self.initial_line_placement(root, layout_box);
            self.pending_line.bounds.content.x = line_bounds.content.x;
            self.pending_line.bounds.content.y = line_bounds.content.y;
            self.pending_line.green_zone.width = line_bounds.content.width;
            if !self.lines.is_empty() {
                let last_line_index = self.lines.len() - 1;
                let end_last_line_range = self.lines[last_line_index].range.end;
                self.pending_line.range = end_last_line_range..end_last_line_range;
            }
        }

        match &layout_box.box_type {
            BoxType::InlineNode(_) => self.layout_inline(root, layout_box, font_context),
            BoxType::TextNode(_) => self.layout_text(layout_box, font_context),
            BoxType::BlockNode(_) => unimplemented!(),
            _ => unreachable!(),
        }
    }

    fn layout_inline(
        &mut self,
        root: &Dimensions,
        layout_box: &mut LayoutBox<'a>,
        font_context: &mut FontContext,
    ) {
        if !layout_box.is_splitted {
            layout_box.assign_horizontal_margin_box();
        }
        layout_box.assign_vertical_margin_box();

        {
            let d = layout_box.dimensions.borrow();
            self.pending_line.bounds.content.width +=
                d.margin_left_offset() + d.margin_right_offset();
        }

        self.calculate_inline_descendant_position(root, layout_box, font_context);
    }

    fn calculate_inline_descendant_position(
        &mut self,
        root: &Dimensions,
        layout_box: &mut LayoutBox<'a>,
        font_context: &mut FontContext,
    ) {
        let mut total_width = 0.;
        let mut new_children: Vec<LayoutBox> = vec![];
        let mut broken_line_children = vec![];
        let mut end_layout_box: LayoutBox<'a> = layout_box.clone();

        for (i, child) in layout_box.children.iter_mut().enumerate() {
            let is_first_line_box = i == 0;
            if self.pending_line.is_line_broken {
                broken_line_children.push(child.clone());
                continue;
            }

            self.layout(root, child, font_context);

            // Child is text node in here
            if self.pending_line.is_line_broken {
                if let Some(item) = self.work_list.pop_front() {
                    broken_line_children.push(item);
                }
            }

            // TODO: Remove whitespace when splitted text node is CJK.
            // |------| <= available width
            // <span>" " <= line 1. This should be removed.
            // "日本語"</span> <= line 2.
            if let BoxType::TextNode(_) = child.box_type {
                if child.is_hidden {
                    // Remove splitted inline start node
                    layout_box.is_hidden = true;
                }
            }

            if child.is_hidden {
                if is_first_line_box {
                    layout_box.is_hidden = true;
                }
                continue;
            }

            if let BoxType::TextNode(node) = &child.box_type {
                let ascent = font_context
                    .get_or_create_by(&node.text_run.cache_key)
                    .ascent;
                {
                    let mut d = layout_box.dimensions.borrow_mut();
                    if ascent > d.content.height {
                        d.content.height = ascent;
                    }
                }
            }

            {
                let mut d = child.dimensions.borrow_mut();
                let mut margin_box = d.margin_horizontal_box();
                if self.pending_line.is_line_broken {
                    margin_box.width -= d.margin_right_offset();
                    d.content.width -= d.margin_right_offset();
                    d.reset_edge_right();
                }
                // calculate position in inline box
                d.content.x = total_width;
                total_width += margin_box.width;
            }

            new_children.push(child.clone());
        }

        if self.pending_line.is_line_broken {
            let mut d = layout_box.dimensions.borrow_mut();
            d.reset_edge_right();
            if !layout_box.is_hidden {
                end_layout_box.is_splitted = true;
                end_layout_box.dimensions.borrow_mut().reset_edge_left();
            }
        }

        // inline_end
        if self.pending_line.is_line_broken && !broken_line_children.is_empty() {
            end_layout_box.children = broken_line_children;
            self.work_list.push_front(end_layout_box);
        }

        if !new_children.is_empty() {
            // inline_start
            layout_box.children = new_children;
            {
                let mut containing_block = layout_box.dimensions.borrow_mut();
                containing_block.content.width = total_width;
                let styled_node = layout_box.get_style_node();
                let ascent = font_context
                    .get_or_create_by(&FontCacheKey::new_from_style(styled_node))
                    .ascent;
                if ascent > containing_block.content.height {
                    containing_block.content.height = ascent;
                }
            }
        }
    }

    fn layout_text(&mut self, layout_box: &mut LayoutBox<'a>, font_context: &mut FontContext) {
        let node = match &mut layout_box.box_type {
            BoxType::TextNode(node) => node,
            _ => unreachable!(),
        };

        let font = font_context.get_or_create_by(&node.text_run.cache_key);

        let metrics = self
            .pending_line
            .metrics
            .calc_space(node.styled_node.line_height(), &font);
        self.metrics.space_above_baseline = self
            .metrics
            .space_above_baseline
            .max(metrics.space_above_baseline);
        self.metrics.space_under_baseline = self
            .metrics
            .space_under_baseline
            .max(metrics.space_under_baseline);
        self.metrics.leading = self.metrics.leading.max(metrics.leading);

        let text_width = self.text_width(node, &font, font_context);

        let remaining_width =
            self.pending_line.green_zone.width - self.pending_line.bounds.content.width;

        if text_width >= remaining_width || self.pending_line.is_line_broken {
            self.pending_line.is_line_broken = true;

            let (inline_start, inline_end) = match node.calculate_split_position(
                self.pending_line.green_zone.width,
                remaining_width,
                &font,
                font_context,
            ) {
                Some(result) => result,
                None => {
                    self.pending_line.is_suppress_line_break_before = true;
                    let next_layout_box = layout_box.clone();
                    self.work_list.push_front(next_layout_box);
                    layout_box.is_hidden = true;
                    return;
                }
            };

            if let Some(inline_start) = &inline_start {
                let mut node = match &mut layout_box.box_type {
                    BoxType::TextNode(node) => node,
                    _ => unreachable!(),
                };
                node.range = inline_start.range.clone();

                if node.range.end != 0 {
                    // remove whitespace on line end.
                    let last_glyph = &node.text_run.glyphs[node.range.end - 1];
                    if last_glyph.glyph_store.is_whitespace {
                        node.range.end -= 1;
                    }
                } else {
                    layout_box.is_hidden = true;
                }

                let text_width = self.text_width(node, &font, font_context);
                {
                    let mut d = layout_box.dimensions.borrow_mut();
                    d.content.height = font.ascent + font.descent;
                    // Maybe, this calculation is specific case for `iced`
                    d.content.y -= metrics.leading / 2.;
                    d.content.width = text_width;
                }
                self.pending_line.bounds.content.width += text_width;
            }

            if let Some(inline_end) = &inline_end {
                let mut new_layout_box = layout_box.clone();
                let mut node = match &mut new_layout_box.box_type {
                    BoxType::TextNode(node) => node,
                    _ => unreachable!(),
                };
                node.range = inline_end.range.clone();
                // Splitted node should not have start position
                node.text_run.has_start = false;
                layout_box.is_hidden = inline_end.is_hidden;
                new_layout_box.dimensions.borrow_mut().content.y = 0.;
                self.work_list.push_front(new_layout_box);
            }
        } else {
            let metrics = self
                .pending_line
                .metrics
                .calc_space(node.styled_node.line_height(), &font);
            {
                let mut d = layout_box.dimensions.borrow_mut();
                d.content.height = font.ascent + font.descent;
                // Maybe, this calculation is specific case for `iced`
                d.content.y -= metrics.leading / 2.;
                d.content.width = text_width;
            }
            self.pending_line.bounds.content.width += text_width;
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

    fn text_width(&self, node: &TextNode<'a>, font: &Font, font_context: &mut FontContext) -> f32 {
        let text = node.get_text();
        font.width(&text, font_context)
    }

    fn pending_line_is_empty(&self) -> bool {
        let rng = &self.pending_line.range;
        rng.end - rng.start == 0
    }

    fn flush_current_line(&mut self) {
        self.lines.push(mem::replace(
            &mut self.pending_line,
            Line::new(Default::default()),
        ));
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
        self.assign_position(&mut line_breaker);
        self.boxes = line_breaker.new_boxes;
        self.width = line_breaker.max_width;
        self.height = line_breaker.cur_height;
    }

    /// calculate inline position in line box
    fn assign_position(&self, line_breaker: &mut LineBreaker<'a>) {
        for line in &line_breaker.lines {
            let mut line_box_x = line.bounds.content.x;
            for item in &mut line_breaker.new_boxes[line.range.clone()] {
                let new_rect_y =
                    line_breaker.cur_height + line.bounds.content.y + line_breaker.metrics.leading;
                {
                    let mut d = item.dimensions.borrow_mut();
                    d.content.x += line_box_x + d.margin_left_offset();
                    d.content.y += new_rect_y;
                }
                if let BoxType::InlineNode(_) = item.box_type {
                    let line_box_x = { line_box_x + item.dimensions.borrow().margin_left_offset() };
                    self.calculate_child_position(item, line_box_x, new_rect_y);
                }
                let d = item.dimensions.borrow();
                let margin_box = d.margin_horizontal_box();
                let line_box_width = margin_box.width;
                line_box_x += line_box_width;
                line_breaker.max_width = line_box_width.max(line_breaker.max_width);
            }
            line_breaker.cur_height += line_breaker.metrics.space_above_baseline
                + line_breaker.metrics.space_under_baseline;
        }
    }

    fn calculate_child_position(
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
                self.calculate_child_position(child, new_rect_x, additional_rect_y);
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

    fn calc_space(&mut self, line_height: f32, font: &Font) -> LineMetrics {
        let ascent = font.ascent;
        let descent = font.descent;
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
