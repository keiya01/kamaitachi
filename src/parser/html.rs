use std::collections::HashMap;

use crate::dom::{Node, AttrMap};
use crate::error;

use super::Parser;

pub struct HTMLParser {
  pos: usize,
  input: String,
}

impl HTMLParser {
  pub fn new(input: String) -> HTMLParser {
    HTMLParser { pos: 0, input }
  }

  pub fn run(&mut self) -> Node {
    let mut nodes = self.parse_nodes();
    if nodes.len() == 1 {
      nodes.swap_remove(0)
    } else {
      Node::new_element("html".into(), HashMap::new(), nodes)
    }
  }

  fn parse_nodes(&mut self) -> Vec<Node> {
    let mut nodes = vec![];
    loop {
      self.consume_whitespace();
      if self.eof() || self.starts_with("</") {
        break;
      }
      nodes.push(self.parse_node());
    }
    nodes
  }

  fn parse_node(&mut self) -> Node {
    match self.next_char() {
      '<' => self.parse_element(),
      _ => self.parse_text(),
    }
  }

  fn parse_text(&mut self) -> Node {
    Node::new_text(self.consume_while(|c| c != '<'))
  }

  fn parse_element(&mut self) -> Node {
    if self.consume_char() != '<' {
      self.new_internal_error("Opening tag must start with '<'");
    }

    let tag_name = self.parse_tag_name();
    let attrs = self.parse_attributes();

    if self.consume_char() != '>' {
      self.new_internal_error("Opening tag should end with '>'");
    }

    let children = self.parse_nodes();

    if self.consume_char() != '<' || self.consume_char() != '/' {
      self.new_internal_error("Closing tag must start with '</'");
    }

    if tag_name != self.parse_tag_name() {
      self.new_internal_error(&format!("Closing tag name must be same with {}", tag_name));
    }

    if self.consume_char() != '>' {
      self.new_internal_error("Closing tag should end with '>'");
    }

    Node::new_element(tag_name, attrs, children)
  }

  fn parse_attributes(&mut self) -> AttrMap {
    let mut attrs = HashMap::new();
    loop {
      self.consume_whitespace();
      if self.next_char() == '>' {
        break;
      }
      let (name, value) = self.parse_attr();
      attrs.insert(name, value);
    }
    attrs
  }

  fn parse_attr(&mut self) -> (String, String) {
    let name = self.parse_tag_name();
    if self.consume_char() != '=' {
      self.new_internal_error("Attribute should has '=' keyword");
    }
    let val = self.parse_attr_value();
    (name, val)
  }

  fn parse_attr_value(&mut self) -> String {
    let open_quote = self.consume_char();
    if open_quote != '"' && open_quote != '\'' {
      self.new_internal_error("Attribute should be wrapped with '\"' keyword");
    }

    let val = self.consume_while(|c| c != open_quote);
    if self.consume_char() != open_quote  {
      self.new_internal_error("Attribute should be wrapped with '\"' keyword");
    }
    val
  }

  fn parse_tag_name(&mut self) -> String {
    self.consume_while(|c| match c {
      'a'..='z' | 'A'..='Z' | '0'..='9' => true,
      _ => false,
    })
  }
}

impl Parser for HTMLParser {
  fn input(&self) -> &str {
    &self.input
  }

  fn pos(&self) -> usize {
    self.pos
  }

  fn set_pos(&mut self, next_pos: usize) {
    self.pos += next_pos;
  }

  fn new_internal_error(&self, msg: &str) {
    error::new_internal_error("HTML Parser", msg);
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::dom::*;
  #[test]
  fn test_parse_node() {
    let input = "
<div id=\"main\" class=\"test\">
  <p>Hello <em>world</em>!</p>
  <p>Test</p>
</div>
";

    let mut p = HTMLParser::new(input.into());

    let div = p.run();
    if let NodeType::Element(elm) = div.node_type {
      assert_eq!(&elm.tag_name, "div");
      assert_eq!(&elm.attributes.get("id").unwrap(), &"main");
      assert_eq!(&elm.attributes.get("class").unwrap(), &"test");
    }
    
    assert_eq!(div.children.len(), 2);

    let p_tag = &div.children[0];
    if let NodeType::Element(elm) = &p_tag.node_type {
      assert_eq!(&elm.tag_name, "p");
      assert_eq!(&elm.attributes.len(), &0);
    }

    let text_node = &p_tag.children[0];
    if let NodeType::Text(s) = &text_node.node_type {
      assert_eq!(s, "Hello ");
    }

    let em_tag = &p_tag.children[1];
    if let NodeType::Element(elm) = &em_tag.node_type {
      assert_eq!(&elm.tag_name, "em");
      assert_eq!(&elm.attributes.len(), &0);
    }

    let text_node = &em_tag.children[0];
    if let NodeType::Text(s) = &text_node.node_type {
      assert_eq!(s, "world");
    }

    let text_node = &p_tag.children[2];
    if let NodeType::Text(s) = &text_node.node_type {
      assert_eq!(s, "!");
    }

    let p_tag = &div.children[1];
    if let NodeType::Element(elm) = &p_tag.node_type {
      assert_eq!(&elm.tag_name, "p");
      assert_eq!(&elm.attributes.len(), &0);
    }

    let text_node = &p_tag.children[0];
    if let NodeType::Text(s) = &text_node.node_type {
      assert_eq!(s, "Test");
    }
  }
}
