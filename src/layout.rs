use std::rc::Rc;
use std::cell::RefCell;
use crate::style::*;
use crate::cssom::{Value, Unit};

// CSS box model. All sizes are in px.

#[derive(Default, Debug)]
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
}

#[derive(Default, Debug)]
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
}

impl<'a> LayoutBox<'a> {
  pub fn new(box_type: BoxType<'a>) -> LayoutBox<'a> {
    LayoutBox { 
      box_type,
      dimensions: Rc::new(RefCell::new(Default::default())),
      children: vec![],
    }
  }

  fn get_style_node(&self) -> &'a StyledNode<'a> {
    match self.box_type {
      BoxType::BlockNode(node) | BoxType::InlineNode(node) => node,
      BoxType::AnonymousBlock => panic!("Anonymous block box has no style node"),
    }
  }

  pub fn layout(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
    match self.box_type {
      BoxType::BlockNode(_) => self.layout_block(containing_block),
      BoxType::InlineNode(_) => {}, // TODO
      BoxType::AnonymousBlock => {}, // TODO,
    }
  }

  fn layout_block(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
    // Child width depends on parent width,
    // so parent width need to be calculated before child width.
    self.calculate_block_width(containing_block.clone());

    self.calculate_block_position(containing_block.clone());

    self.layout_block_children();

    // Parent height is affected by child layout,
    // so parent height need to be calculated after children are laid out.
    self.calculate_block_height();
  }

  fn calculate_block_width(&mut self, containing_block: Rc<RefCell<Dimensions>>) {
    let style = self.get_style_node();

    // `width` has initial value `auto`.
    let auto = Value::Keyword("auto".into());
    let mut width = style.value("width").unwrap_or(auto.clone());

    // `margin`, `border`, `padding` has initial value `0`.
    let zero = Value::Length(0.0, Unit::Px);

    let mut margin_left = style.lookup("margin-left", "margin", &zero);
    let mut margin_right = style.lookup("margin-right", "margin", &zero);

    let border_left = style.lookup("border-left-width", "border", &zero);
    let border_right = style.lookup("border-right-width", "border", &zero);

    let padding_left = style.lookup("padding-left", "padding", &zero);
    let padding_right = style.lookup("padding-right", "padding", &zero);

    let total: f32 = [
      margin_left.to_px(), margin_right.to_px(),
      border_left.to_px(), border_right.to_px(),
      padding_left.to_px(), padding_right.to_px(),
      width.to_px(),
    ].iter().sum();

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
        if margin_left == auto { margin_left = Value::Length(0.0, Unit::Px) }
        if margin_right == auto { margin_right = Value::Length(0.0, Unit::Px) }

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
    let style = self.get_style_node();
    let mut d = self.dimensions.borrow_mut();

    let containing_block = containing_block.borrow();

    let zero = Value::Length(0.0, Unit::Px);

    d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
    d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

    d.border.top = style.lookup("border-top", "border", &zero).to_px();
    d.border.bottom = style.lookup("border-bottom", "border", &zero).to_px();

    d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
    d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

    d.content.x = containing_block.content.x +
                  d.margin.left + d.border.left + d.padding.left;

    d.content.y = containing_block.content.height + containing_block.content.y +
                  d.margin.top + d.border.top + d.padding.top;
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

  fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
    match self.box_type {
      BoxType::InlineNode(_) | BoxType::AnonymousBlock => self,
      BoxType::BlockNode(_) => {
        match self.children.last() {
          Some(&LayoutBox { box_type: BoxType::AnonymousBlock, .. }) => {},
          _ => self.children.push(LayoutBox::new(BoxType::AnonymousBlock)),
        };
        self.children.last_mut().unwrap()
      },
    }
  }
}

#[derive(Debug)]
pub enum BoxType<'a> {
  BlockNode(&'a StyledNode<'a>),
  InlineNode(&'a StyledNode<'a>),
  AnonymousBlock,
}

pub fn layout_tree<'a>(node: &'a StyledNode<'a>, containing_block: Rc<RefCell<Dimensions>>) -> LayoutBox<'a> {
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
    Display::Inline => BoxType::InlineNode(style_node),
    Display::None => panic!("Root node must has `display: none;`."),
  });

  for child in &style_node.children {
    match child.display() {
      Display::Block => root.children.push(build_layout_tree(child)),
      // TODO: Support the case where inline include block element(https://www.w3.org/TR/CSS2/visuren.html#box-gen) 
      Display::Inline => root.get_inline_container().children.push(build_layout_tree(child)),
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

    let styled_node = create_style_tree(Rc::new(&dom), &cssom, None);

    let mut viewport: Dimensions = Default::default();
    viewport.content.width  = 800.0;
    viewport.content.height = 600.0;

    layout_tree(&styled_node, Rc::new(RefCell::new(viewport)));
  }
}
