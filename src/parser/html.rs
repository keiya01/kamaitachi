use std::collections::HashMap;

use super::Parser;
use crate::dom::{AttrMap, Node};

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
            if let Some(node) = self.parse_node() {
                nodes.push(node);
            }
        }
        nodes
    }

    fn parse_node(&mut self) -> Option<Node> {
        match self.next_char() {
            '<' => self.parse_element(),
            _ => self.parse_text(),
        }
    }

    fn parse_text(&mut self) -> Option<Node> {
        Some(Node::new_text(self.consume_while(|c| c != '<')))
    }

    fn parse_element(&mut self) -> Option<Node> {
        // Consumed character should be '<' in here.
        self.consume_char();

        let tag_name = self.parse_tag_name();
        let attrs = self.parse_attributes();

        if self.eof() {
            return Some(Node::new_element(tag_name, attrs, vec![]));
        }

        if self.next_char() != '>' {
            self.consume_while(|c| c != '>');
        } else {
            self.consume_char();
        }

        let children = self.parse_nodes();

        if self.eof() {
            return Some(Node::new_element(tag_name, attrs, children));
        }

        if self.next_char() != '<' {
            return Some(Node::new_element(tag_name, attrs, children));
        } else {
            self.consume_char();
        }

        if self.next_char() != '/' {
            return Some(Node::new_element(tag_name, attrs, children));
        } else {
            self.consume_char();
        }

        if tag_name != self.parse_tag_name() {
            return Some(Node::new_element(tag_name, attrs, children));
        }

        if self.next_char() != '>' {
            loop {
                if self.eof() || self.next_char() == '<' {
                    break;
                }
                if self.next_char() == '>' {
                    self.consume_char();
                    break;
                }
                self.consume_char();
            }
        } else {
            self.consume_char();
        }

        Some(Node::new_element(tag_name, attrs, children))
    }

    fn parse_attributes(&mut self) -> AttrMap {
        let mut attrs = HashMap::new();
        loop {
            if self.eof() {
                break;
            }
            self.consume_whitespace();
            if self.next_char() == '/' {
                self.consume_char();
                continue;
            }

            if self.next_char() == '>' {
                break;
            }
            let (name, value) = self.parse_attr();
            attrs.insert(name, value);
        }
        attrs
    }

    fn parse_attr(&mut self) -> (String, String) {
        let name = self.consume_while(|c| match c {
            '>' => false,
            '=' => false,
            c if c.is_whitespace() => false,
            _ => true,
        });
        if self.next_char() != '=' {
            return (name, "".into());
        } else {
            self.consume_char();
        }
        let val = self.parse_attr_value();
        (name, val)
    }

    fn parse_attr_value(&mut self) -> String {
        let open_quote = self.next_char();
        let is_quote = open_quote == '"' || open_quote == '\'';
        if is_quote {
            self.consume_char();
        }

        let val = self.consume_while(|c| {
            if is_quote {
                return c != open_quote;
            }
            if c == '>' || c.is_whitespace() {
                return false;
            }
            return true;
        });

        if !self.eof() && is_quote {
            self.consume_char();
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

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-error-end-tag-with-attributes
    #[test]
    fn test_parse_end_tag_with_attributes() {
        let input = "<body><div></div attr=\"test\"><p></p></body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        if let NodeType::Element(elm) = &body.children[0].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &0);
        }

        if let NodeType::Element(elm) = &body.children[1].node_type {
            assert_eq!(&elm.tag_name, "p");
            assert_eq!(&elm.attributes.len(), &0);
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-error-end-tag-with-trailing-solidus
    #[test]
    fn test_parse_end_tag_with_trailing_solidus() {
        let input = "<body><div></div/><p></p></body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        if let NodeType::Element(elm) = &body.children[0].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &0);
        }

        if let NodeType::Element(elm) = &body.children[1].node_type {
            assert_eq!(&elm.tag_name, "p");
            assert_eq!(&elm.attributes.len(), &0);
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-error-missing-end-tag-name
    #[test]
    fn test_parse_missing_end_tag_name() {
        let input = "<body><div></><p></p></body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        if let NodeType::Element(elm) = &body.children[0].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &0);
        }

        if let NodeType::Element(elm) = &body.children[1].node_type {
            assert_eq!(&elm.tag_name, "p");
            assert_eq!(&elm.attributes.len(), &0);
        }
    }

    #[test]
    fn test_parse_missing_close() {
        let input = "<body><div></div attr=\"test\" </body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        assert_eq!(&body.children.len(), &1);
        if let NodeType::Element(elm) = &body.children[0].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &0);
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-error-non-void-html-element-start-tag-with-trailing-solidus
    #[test]
    fn test_parse_non_void_html_element_start_tag_with_trailing_solidus() {
        let input = "<body><div /><p></p><span></span></body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        let div = &body.children[0];
        if let NodeType::Element(elm) = &div.node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &0);
        }

        if let NodeType::Element(elm) = &div.children[0].node_type {
            assert_eq!(&elm.tag_name, "p");
            assert_eq!(&elm.attributes.len(), &0);
        }

        if let NodeType::Element(elm) = &div.children[1].node_type {
            assert_eq!(&elm.tag_name, "span");
            assert_eq!(&elm.attributes.len(), &0);
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-error-unexpected-character-in-attribute-name
    #[test]
    fn test_parse_unexpected_character_in_attribute_name() {
        let input = "<body><div foo<div><div id'bar'></div></body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        if let NodeType::Element(elm) = &body.children[0].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &1);
            assert_eq!(&elm.attributes.get("foo<div").unwrap(), &"");
        }

        if let NodeType::Element(elm) = &body.children[1].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &1);
            assert_eq!(&elm.attributes.get("id'bar'").unwrap(), &"");
        }
    }

    /// https://html.spec.whatwg.org/multipage/parsing.html#parse-error-unexpected-character-in-unquoted-attribute-value
    #[test]
    fn test_parse_unexpected_character_in_unquoted_attribute_value() {
        let input = "<body><div id=b'ar'></div><div id=\"></body>";

        let mut p = HTMLParser::new(input.into());

        let body = p.run();
        if let NodeType::Element(elm) = &body.children[0].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &1);
            assert_eq!(&elm.attributes.get("id").unwrap(), &"b'ar'");
        }

        if let NodeType::Element(elm) = &body.children[1].node_type {
            assert_eq!(&elm.tag_name, "div");
            assert_eq!(&elm.attributes.len(), &1);
            assert_eq!(&elm.attributes.get("id").unwrap(), &"></body>");
        }
    }
}
