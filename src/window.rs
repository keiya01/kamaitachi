use iced::{Sandbox, Element, Settings};

use std::{env, io, fs};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::cell::RefCell;
use std::io::prelude::*;

use crate::parser::{html, css};
use html::HTMLParser;
use css::CSSParser;
use crate::cssom::{Origin, Stylesheet};
use crate::style::create_style_tree;
use crate::layout::{Dimensions, layout_tree};
use crate::painter::{build_display_list, DisplayList, DisplayCommand};
use crate::painter::block::Block;
use crate::painter::wrapper::Wrapper;

#[derive(Debug)]
pub enum Message {}

pub struct Window {
  items: DisplayList,
}

impl<'a> Sandbox for Window {
  type Message = Message;

  fn new() -> Self {
      Window { items: prepare() }
  }

  fn title(&self) -> String {
      String::from("kamaitachi")
  }

  fn update(&mut self, message: Message) {
      match message {}
  }

  fn view(&mut self) -> Element<Message> {
    let mut wrapper = Wrapper::new();

    for item in &self.items {
      wrapper.items.push(match item {
        DisplayCommand::SolidColor(color, rect) => Block::new(color.clone(), rect.clone()),
      });
    }

    wrapper.into()
  }
}

fn prepare() -> DisplayList {
  let args: Vec<String> = env::args().collect();
  if args.len() < 1 {
    panic!("You need to specify entry path.");
  }
  let path = Path::new(&args[1]);

  let mut paths = vec![];
  visit_dirs(path, &mut paths).unwrap();

  let mut html = String::new();
  let mut css_list = vec![];

  for path in paths {
    let ext = path.extension().unwrap();
    if html.is_empty() && ext == "html" {
      fs::File::open(path).unwrap().read_to_string(&mut html).unwrap();
      continue;
    }
    if ext == "css" {
      let mut css = String::new();
      fs::File::open(path).unwrap().read_to_string(&mut css).unwrap();
      css_list.push(css);
      continue;
    }
  }

  paint(html, css_list)
}

fn paint(html: String, css_list: Vec<String>) -> DisplayList {
  let dom = HTMLParser::new(html).run();
  let mut author_rules = vec![];

  for css in css_list {
    author_rules.extend(CSSParser::new(css).parse_rules(Origin::Author));
  }

  let cssom = Stylesheet::new(author_rules);

  let styled_node = create_style_tree(Rc::new(&dom), &cssom, None);

  let mut viewport: Dimensions = Default::default();
  viewport.content.width  = 800.0;
  viewport.content.height = 600.0;

  let layout_root = layout_tree(&styled_node, Rc::new(RefCell::new(viewport)));

  build_display_list(&layout_root)
}

// one possible implementation of walking a directory only visiting files
fn visit_dirs(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
  if dir.is_dir() {
    for entry in fs::read_dir(dir)? {
      let entry = entry?;
      let path = entry.path();
      if path.is_dir() {
        visit_dirs(&path, paths)?;
      } else {
        paths.push(path);
      }
    }
  } else {
    paths.push(dir.to_path_buf());
  }
  Ok(())
}

pub fn main() -> iced::Result {
  Window::run(Settings::default())
}
