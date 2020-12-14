use std::collections::HashMap;

use crate::{cssom, dom};
use dom::{Node, ElementData, NodeType};
use cssom::*;

// Map from CSS property names to values.
type PropertyMap = HashMap<String, Value>;

// A node with associated style data.
pub struct StyledNode<'a> {
  pub node: &'a Node,
  pub specified_values: PropertyMap,
  pub children: Vec<StyledNode<'a>>,
}

impl<'a> StyledNode<'a> {
  pub fn new(node: &'a Node, specified_values: PropertyMap, children: Vec<StyledNode<'a>>) -> StyledNode<'a> {
    return StyledNode { node, specified_values, children }
  }
}

pub fn create_style_tree<'a>(root: &'a Node, stylesheet: &'a Stylesheet) -> StyledNode<'a> {
  StyledNode::new(
    root,
    match &root.node_type {
      NodeType::Element(elm) => specified_values(elm, stylesheet),
      NodeType::Text(_) => HashMap::new(),
    },
    root.children.iter().map(|node| create_style_tree(node, stylesheet)).collect(),
  )
}

fn specified_values(elm: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
  let mut values = HashMap::new();
  let mut rules = match_rules(elm, stylesheet);
  
  rules.sort_by(|&(a, _), &(b, _)| a.cmp(&b));
  for (_, rule) in rules {
    for declaration in &rule.declarations {
      values.insert(declaration.name.clone(), declaration.value.clone());
    }
  }
  values
}

type MatchedRule<'a> = (Specificity, &'a Rule);

fn match_rules<'a>(elm: &ElementData, stylesheet: &'a Stylesheet) -> Vec<MatchedRule<'a>> {
  stylesheet.rules.iter().filter_map(|rule| match_rule(elm, rule)).collect()
}

fn match_rule<'a>(elm: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    rule.selectors.iter()
      .find(|selector| matches(elm, *selector))
      .map(|selector| (selector.specificity(), rule))
}

fn matches(elm: &ElementData, selector: &Selector) -> bool {
  match selector {
    Selector::Simple(simple) => matches_simple_selector(elm, simple),
  }
}

fn matches_simple_selector(elm: &ElementData, selector: &SimpleSelector) -> bool {
  if selector.tag_name.iter().any(|name| *name != elm.tag_name) {
    return false;
  }

  if selector.id.iter().any(|id| Some(id) != elm.id()) {
    return false;
  }

  if selector.class.iter().any(|class| !elm.classes().contains(&**class)) {
    return false;
  }

  return true;
}

#[cfg(test)]
mod tests {
  use crate::parser::html::HTMLParser;
  use crate::parser::css::CSSParser;
  use super::*;

  fn test_element(node_type: &NodeType, expected: &str) {
    if let NodeType::Element(elm) = node_type {
      assert_eq!(&elm.tag_name, expected);
    } else {
      panic!("node should has Element");
    }
  }

  #[test]
  fn test_compound() {
      let html = "
<body class='bar'>
  <div id='foo' class='bar'></div>
  <div>test</div>
</body>
";
    let css = "
.bar {
  height: 100px;
}

div {
  color: #cc0000;
  display: block;
}

div#foo.bar {
  height: auto;
}

div#foo {
  color: red;
}
";
    let mut html_parser = HTMLParser::new(html.into());
    let mut css_parser = CSSParser::new(css.into());

    let dom = html_parser.run();
    let cssom = css_parser.run();

    let styled_node = create_style_tree(&dom, &cssom);

    test_element(&styled_node.node.node_type, &"body");
    assert_eq!(&styled_node.specified_values.len(), &1);
    assert_eq!(
      *styled_node.specified_values.get("height").unwrap(),
      Value::Length(100.0, Unit::Px),
    );

    assert_eq!(&styled_node.node.children.len(), &2);

    let div = &styled_node.children[0];
    test_element(&div.node.node_type, &"div");
    assert_eq!(&div.specified_values.len(), &3);
    assert_eq!(
      *div.specified_values.get("color").unwrap(),
      Value::Keyword("red".into()),
    );
    assert_eq!(
      *div.specified_values.get("height").unwrap(),
      Value::Keyword("auto".into()),
    );
    assert_eq!(
      *div.specified_values.get("display").unwrap(),
      Value::Keyword("block".into()),
    );

    let div = &styled_node.children[1];
    test_element(&div.node.node_type, &"div");
    assert_eq!(&div.specified_values.len(), &2);
    assert_eq!(
      *div.specified_values.get("color").unwrap(),
      Value::ColorValue(Color::new(204, 0, 0, 255)),
    );
    assert_eq!(
      *div.specified_values.get("display").unwrap(),
      Value::Keyword("block".into()),
    );    
  }
}
