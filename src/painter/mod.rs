mod block;
mod text;
pub mod wrapper;

pub use block::create_block;
pub use text::create_text;

use crate::cssom::{Color, Value};
use crate::layout::{font, BoxType, LayoutBox, Rect};
use font::{with_thread_local_font_context, FontCacheKey, FontContext};

pub type DisplayList = Vec<DisplayCommand>;

#[derive(Debug)]
pub enum DisplayCommand {
    SolidColor(Color, Rect),
    Text(String, Color, Rect, font::Font),
}

pub fn build_display_list(layout_root: &LayoutBox) -> DisplayList {
    let mut list = vec![];
    with_thread_local_font_context(|font_context| {
        render_layout_box(&mut list, layout_root, font_context)
    });
    list
}

fn render_layout_box(
    list: &mut DisplayList,
    layout_box: &LayoutBox,
    font_context: &mut FontContext,
) {
    render_background(list, layout_box);
    render_borders(list, layout_box);
    render_text(list, layout_box, font_context);

    for child in &layout_box.children {
        render_layout_box(list, child, font_context);
    }
}

fn render_background(list: &mut DisplayList, layout_box: &LayoutBox) {
    if let Some(color) = get_color(layout_box, "background") {
        list.push(DisplayCommand::SolidColor(
            color,
            layout_box.dimensions.borrow().border_box(),
        ))
    }
}

fn render_borders(list: &mut DisplayList, layout_box: &LayoutBox) {
    let color = match get_color(layout_box, "border-color") {
        Some(color) => color,
        None => return, // render nothing
    };

    let d = layout_box.dimensions.borrow();
    let border_box = d.border_box();

    if d.border.left != 0. {
        // border-left
        list.push(DisplayCommand::SolidColor(
            color.clone(),
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: d.border.left,
                height: border_box.height,
            },
        ));
    }

    if d.border.right != 0. {
        // border-right
        list.push(DisplayCommand::SolidColor(
            color.clone(),
            Rect {
                x: border_box.x + border_box.width - d.border.right,
                y: border_box.y,
                width: d.border.right,
                height: border_box.height,
            },
        ));
    }

    if d.border.top != 0. {
        // border-top
        list.push(DisplayCommand::SolidColor(
            color.clone(),
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: border_box.width,
                height: d.border.top,
            },
        ));
    }

    if d.border.bottom != 0. {
        // border-bottom
        list.push(DisplayCommand::SolidColor(
            color,
            Rect {
                x: border_box.x,
                y: border_box.y + border_box.height - d.border.bottom,
                width: border_box.width,
                height: d.border.bottom,
            },
        ));
    }
}

// TODO: remove font_context
fn render_text(list: &mut DisplayList, layout_box: &LayoutBox, font_context: &mut FontContext) {
    let node = match &layout_box.box_type {
        BoxType::TextNode(node) => node,
        _ => return,
    };

    let text = node.get_text();

    let color = get_color(layout_box, "color").unwrap_or_else(|| Color::new(0, 0, 0, 1.0));

    // TODO: node.run_info_listのitemをloopしながら入れていく
    // 座標やsizeはinline boxのlayout処理で行っておく
    // layout_boxのdimensionsをrun_info.dimensionsで置き換える
    // fontはrun_info.fontに持っておく

    let font = font_context.get_or_create_by(&node.text_run.cache_key);
    list.push(DisplayCommand::Text(
        text.into(),
        color,
        layout_box.dimensions.borrow().content.clone(),
        font,
    ))
}

fn get_color(layout_box: &LayoutBox, name: &str) -> Option<Color> {
    match &layout_box.box_type {
        BoxType::BlockNode(node) | BoxType::InlineNode(node) => match node.value(name) {
            Some(Value::ColorValue(color)) => Some(color),
            _ => None,
        },
        BoxType::TextNode(node) => match node.styled_node.value(name) {
            Some(Value::ColorValue(color)) => Some(color),
            _ => None,
        },
        BoxType::AnonymousBlock => None,
    }
}
