pub mod block;
pub mod wrapper;

use crate::cssom::{Color, Value};
use crate::layout::{LayoutBox, Rect, BoxType};

pub type DisplayList = Vec<DisplayCommand>;

#[derive(Debug)]
pub enum DisplayCommand {
  SolidColor(Color, Rect),
}

pub fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
  let mut list = vec![];
  render_layout_box(&mut list, layout_root);
  list
}

fn render_layout_box(list: &mut DisplayList, layout_box: &LayoutBox) {
  render_background(list, layout_box);
  render_borders(list, layout_box);
  // TODO: render text

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

fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
  match layout_box.box_type {
    BoxType::BlockNode(node) | BoxType::InlineNode(node) => match node.value(name){
      Some(Value::ColorValue(color)) => Some(color),
      _ => None,
    },
    BoxType::AnonymousBlock => None,
  }
}
