// TODO
// - [ ] Computed Values
// - [ ] Inheritance
// - [ ] Initial

use std::collections::HashMap;
use std::rc::{Rc, Weak};

use crate::{cssom, dom, parser};
use dom::{Node, ElementData, NodeType};
use cssom::*;
use parser::css::CSSParser;

// Map from CSS property names to values.
type PropertyMap = HashMap<String, Value>;

// A node with associated style data.
pub struct StyledNode<'a> {
  pub node: Rc<&'a Node>,
  pub specified_values: PropertyMap,
  pub children: Vec<StyledNode<'a>>,
  pub parent: Option<Weak<&'a Node>>,
}

impl<'a> StyledNode<'a> {
  pub fn new(node: Rc<&'a Node>, specified_values: PropertyMap, children: Vec<StyledNode<'a>>, parent: Option<Weak<&'a Node>>) -> StyledNode<'a> {
    return StyledNode { node, specified_values, children, parent }
  }
}

pub fn create_style_tree<'a>(root: Rc<&'a Node>, stylesheet: &'a Stylesheet, parent: Option<Weak<&'a Node>>) -> StyledNode<'a> {
  StyledNode::new(
    root.clone(),
    match &root.node_type {
      NodeType::Element(elm) => specified_values(elm, stylesheet),
      NodeType::Text(_) => HashMap::new(),
    },
    root.children.iter().map(|node| {
      create_style_tree(Rc::new(node), stylesheet, Some(Rc::downgrade(&root)))
    }).collect(),
    parent,
  )
}

fn specified_values(elm: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
  let mut values = HashMap::new();
  let mut rules = match_rules(elm, stylesheet);
  
  rules.sort_by(
    |&(specificity1, rule1), &(specificity2, rule2)| {
      if rule1.level != rule2.level {
        return rule1.level.to_index().cmp(&rule2.level.to_index());
      }
      specificity1.cmp(&specificity2)
    }
  );
  for (_, rule) in rules {
    for declaration in &rule.declarations {
      values.insert(declaration.name.clone(), declaration.value.clone());
    }
  }

  if let Some(style) = elm.attributes.get("style") {
    let mut p = CSSParser::new(style.clone());
    let declarations = p.parse_declarations();
    for declaration in declarations {
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

    let rules = css_parser.parse_rules(Origin::Author);
    let cssom = Stylesheet::new(rules);

    let styled_node = create_style_tree(Rc::new(&dom), &cssom, None);

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
    
    let text = &div.children[0];
    assert_eq!(
      &text.specified_values.len(),
      &0,
    );    
  }

  #[test]
  fn test_cascade_level() {
      let html = "
<body class='bar'>
  <div id='foo' class='baz'></div>
</body>
";
    let ua_css = "
div#foo.baz {
  color: #cc0000;
  margin: auto;
}

body {
  display: block;
  margin: 5px;
}

div {
  display: block;
  margin: 5px;
}
";

    let author_css = "
body {
  margin: 0px;
}

.bar {
  height: 100px;
}

div {
  display: inline;
}

div#foo.baz {
  color: red;
}
";

    let mut html_parser = HTMLParser::new(html.into());
    let mut ua_css_parser = CSSParser::new(ua_css.into());
    let mut author_css_parser = CSSParser::new(author_css.into());

    let dom = html_parser.run();

    let ua_rules = ua_css_parser.parse_rules(Origin::UA);
    let mut author_rules = author_css_parser.parse_rules(Origin::Author);

    author_rules.extend(ua_rules);

    let cssom = Stylesheet::new(author_rules);

    let styled_node = create_style_tree(Rc::new(&dom), &cssom, None);

    test_element(&styled_node.node.node_type, &"body");
    assert_eq!(&styled_node.specified_values.len(), &3);
    assert_eq!(
      *styled_node.specified_values.get("display").unwrap(),
      Value::Keyword("block".into()),
    );
    assert_eq!(
      *styled_node.specified_values.get("margin").unwrap(),
      Value::Length(0.0, Unit::Px),
    );
    assert_eq!(
      *styled_node.specified_values.get("height").unwrap(),
      Value::Length(100.0, Unit::Px),
    );

    assert_eq!(&styled_node.node.children.len(), &1);

    let div = &styled_node.children[0];
    test_element(&div.node.node_type, &"div");
    assert_eq!(&div.specified_values.len(), &3);
    assert_eq!(
      *div.specified_values.get("color").unwrap(),
      Value::Keyword("red".into()),
    );
    assert_eq!(
      *div.specified_values.get("display").unwrap(),
      Value::Keyword("inline".into()),
    );
    assert_eq!(
      *div.specified_values.get("margin").unwrap(),
      Value::Keyword("auto".into()),
    );
  }

  #[test]
  fn test_style_attr() {
      let html = "
<body>
  <div id='foo' class='baz' style=\"color: green; display: block;\"></div>
</body>
";

    let author_css = "
div {
  display: inline;
}

div#foo.baz {
  color: red;
  height: auto;
}
";

    let mut html_parser = HTMLParser::new(html.into());
    let mut css_parser = CSSParser::new(author_css.into());

    let dom = html_parser.run();

    let author_rules = css_parser.parse_rules(Origin::Author);

    let cssom = Stylesheet::new(author_rules);

    let styled_node = create_style_tree(Rc::new(&dom), &cssom, None);

    test_element(&styled_node.node.node_type, &"body");
    assert_eq!(&styled_node.specified_values.len(), &0);

    assert_eq!(&styled_node.node.children.len(), &1);

    let div = &styled_node.children[0];
    test_element(&div.node.node_type, &"div");
    assert_eq!(&div.specified_values.len(), &3);
    assert_eq!(
      *div.specified_values.get("color").unwrap(),
      Value::Keyword("green".into()),
    );
    assert_eq!(
      *div.specified_values.get("display").unwrap(),
      Value::Keyword("block".into()),
    );
    assert_eq!(
      *div.specified_values.get("height").unwrap(),
      Value::Keyword("auto".into()),
    );
  }
}
