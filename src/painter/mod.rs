pub mod block;
pub mod text;
pub mod wrapper;

use crate::cssom::{Color, Value};
use crate::dom::{NodeType};
use crate::layout::{LayoutBox, Rect, BoxType, font};

pub type DisplayList = Vec<DisplayCommand>;

#[derive(Debug)]
pub enum DisplayCommand {
  SolidColor(Color, Rect),
  Text(String, Color, Rect, font::Font),
}

pub fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
  let mut list = vec![];
  render_layout_box(&mut list, layout_root);
  list
}

fn render_layout_box(list: &mut DisplayList, layout_box: &LayoutBox) {
  render_background(list, layout_box);
  render_borders(list, layout_box);
  render_text(list, layout_box);

  for child in &layout_box.children {
    render_layout_box(list, child);
  }
}

fn render_background(list: &mut DisplayList, layout_box: &LayoutBox) {
  get_color(layout_box, "background").map(|color| {
    list.push(DisplayCommand::SolidColor(color, layout_box.dimensions.borrow().border_box()))
  });
}

fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox) {
  let color = match get_color(layout_box, "border-color") {
    Some(color) => color,
    None => return, // render nothing
  };

  let d = layout_box.dimensions.borrow();
  let border_box = d.border_box();

  // border-left
  list.push(DisplayCommand::SolidColor(color.clone(), Rect {
    x: border_box.x,
    y: border_box.y,
    width: d.border.left,
    height: border_box.height,
  }));

  // border-right
  list.push(DisplayCommand::SolidColor(color.clone(), Rect {
    x: border_box.x + border_box.width - d.border.right,
    y: border_box.y,
    width: d.border.right,
    height: border_box.height,
  }));

  // border-top
  list.push(DisplayCommand::SolidColor(color.clone(), Rect {
    x: border_box.x,
    y: border_box.y,
    width: border_box.width,
    height: d.border.top,
  }));

  // border-top
  list.push(DisplayCommand::SolidColor(color.clone(), Rect {
    x: border_box.x,
    y: border_box.y + border_box.height - d.border.bottom,
    width: border_box.width,
    height: d.border.bottom,
  }));
}

fn render_text(list: &mut DisplayList, layout_box: &LayoutBox) {
  let node = match &layout_box.box_type {
    BoxType::TextNode(node) => node,
    _ => return,
  };

  let text = match &(*node.styled_node.node).node_type {
    NodeType::Text(text) => text,
    _ => unreachable!(),
  };

  let color = get_color(layout_box, "color").unwrap_or(Color::new(0, 0, 0, 1.0));
  list.push(DisplayCommand::Text(text.into(), color, layout_box.dimensions.borrow().content.clone(), node.font.clone()))
}

fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
  match &layout_box.box_type {
    BoxType::BlockNode(node) | BoxType::InlineNode(node) => match node.value(name){
      Some(Value::ColorValue(color)) => Some(color),
      _ => None,
    },
    BoxType::TextNode(node) => match node.styled_node.value(name){
      Some(Value::ColorValue(color)) => Some(color),
      _ => None,
    },
    BoxType::AnonymousBlock => None,
  }
}
