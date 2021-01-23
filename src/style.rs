// TODO
// - [ ] Computed Values
// - [ ] Initial

use std::collections::HashMap;

use crate::{cssom, dom, font_list, layout, parser};
use cssom::*;
use dom::{ElementData, Node, NodeType};
use font_list::{get_generic_fonts, DEFAULT_FONT_FAMILY_NAME};
use layout::font::{FontStyle, FontWeight};
use parser::css::CSSParser;

// Map from CSS property names to values.
type PropertyMap = HashMap<String, Value>;

// A node with associated style data.
#[derive(Debug)]
pub struct StyledNode<'a> {
    pub node: &'a Node,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

pub enum Display {
    Inline,
    Block,
    None,
}

const XX_LARGE: f32 = 2.5;
const X_LARGE: f32 = 1.8;
const LARGE: f32 = 1.5;
const MEDIUM: f32 = 1.3;
const SMALL: f32 = 1.1;
const X_SMALL: f32 = 1.;

const INHERITABLE_PROPERTY_LIST: [&str; 7] = [
    "font-size",
    "color",
    "line-height",
    "font-family",
    "font-weight",
    "font-style",
    "word-break",
];

pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}

impl WordBreak {
    pub fn is_break_all(&self) -> bool {
        if let WordBreak::BreakAll = self {
            true
        } else {
            false
        }
    }
}

impl<'a> StyledNode<'a> {
    pub fn new(
        node: &'a Node,
        specified_values: PropertyMap,
        children: Vec<StyledNode<'a>>,
    ) -> StyledNode<'a> {
        StyledNode {
            node,
            specified_values,
            children,
        }
    }

    pub fn value(&self, name: &str) -> Option<Value> {
        self.specified_values.get(name).cloned()
    }

    pub fn display(&self) -> Display {
        match self.value("display") {
            Some(Value::Keyword(s)) => match &*s {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }

    pub fn lookup(&self, name: &str, fallback_name: &str, default: &Value) -> Value {
        self.value(name)
            .unwrap_or_else(|| self.value(fallback_name).unwrap_or_else(|| default.clone()))
    }

    /// Font families are supported only for macos.
    pub fn font_family(&self) -> Vec<String> {
        let generic_fonts = get_generic_fonts();
        let default_families = vec![DEFAULT_FONT_FAMILY_NAME.to_string(); 1];
        let value = match self.value("font-family") {
            Some(val) => val,
            None => return default_families,
        };

        match value {
            Value::Keyword(val) => match generic_fonts.get(&val) {
                Some(val) => val.clone(),
                None => default_families,
            },
            Value::KeywordArray(arr) => {
                let mut families = vec![];
                for item in arr.into_iter() {
                    match generic_fonts.get(&item) {
                        Some(val) => {
                            for name in val {
                                families.push(name.clone());
                            }
                        }
                        None => continue,
                    }
                }
                if families.is_empty() {
                    return default_families;
                }
                families
            }
            _ => default_families,
        }
    }

    pub fn font_size(&self) -> f32 {
        let default_font_size = Value::Length(16.0, Unit::Px);
        self.value("font-size")
            .unwrap_or_else(|| default_font_size)
            .to_px()
            * MEDIUM
    }

    pub fn font_style(&self) -> FontStyle {
        let default_font_style = Value::Keyword("normal".to_string());
        let val = self
            .value("font-style")
            .unwrap_or_else(|| default_font_style);
        let keyword = match val {
            Value::Keyword(s) => s,
            _ => return FontStyle::Normal,
        };
        match &keyword[..] {
            "italic" => FontStyle::Italic,
            "oblique" => FontStyle::Oblique,
            _ => FontStyle::Normal,
        }
    }

    pub fn font_weight(&self) -> FontWeight {
        let normal = 400.;
        let default_font_weight = Value::Number(normal);
        let val = self
            .value("font-weight")
            .unwrap_or_else(|| default_font_weight);
        let num = match val {
            Value::Number(n) => n,
            _ => return FontWeight(normal),
        };
        FontWeight(num)
    }

    pub fn line_height(&self) -> f32 {
        let default_line_height = Value::Number(1.2);
        let line_height = self
            .value("line-height")
            .unwrap_or_else(|| default_line_height)
            .to_px();
        self.font_size() * line_height
    }

    pub fn word_break(&self) -> WordBreak {
        let word_break = self.value("word-break");
        let value = match word_break {
            Some(val) => val,
            None => return WordBreak::Normal,
        };

        let keyword = match value {
            Value::Keyword(keyword) => keyword,
            _ => return WordBreak::Normal,
        };

        match &keyword[..] {
            "break-all" => WordBreak::BreakAll,
            "keep-all" => WordBreak::KeepAll,
            _ => WordBreak::Normal,
        }
    }
}

pub fn create_style_tree<'a>(
    root: &'a Node,
    stylesheet: &'a Stylesheet,
    inherited_specified_values: Option<PropertyMap>,
) -> StyledNode<'a> {
    let inherited_specified_values = inherited_specified_values.unwrap_or_default();
    let root_specified_values = match &root.node_type {
        NodeType::Element(elm) => specified_values(elm, stylesheet, inherited_specified_values),
        NodeType::Text(_) => inherited_specified_values,
    };

    let new_inherited_specified_values: PropertyMap = root_specified_values
        .iter()
        .filter(|(k, _)| INHERITABLE_PROPERTY_LIST.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    StyledNode::new(
        root,
        root_specified_values,
        root.children
            .iter()
            .map(|node| {
                create_style_tree(
                    node,
                    stylesheet,
                    Some(new_inherited_specified_values.clone()),
                )
            })
            .collect(),
    )
}

fn specified_values(
    elm: &ElementData,
    stylesheet: &Stylesheet,
    inherited_specified_values: PropertyMap,
) -> PropertyMap {
    let mut values = inherited_specified_values;
    let mut rules = match_rules(elm, stylesheet);

    rules.sort_by(|&(specificity1, rule1), &(specificity2, rule2)| {
        if rule1.level != rule2.level {
            return rule1.level.to_index().cmp(&rule2.level.to_index());
        }
        specificity1.cmp(&specificity2)
    });
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
    stylesheet
        .rules
        .iter()
        .filter_map(|rule| match_rule(elm, rule))
        .collect()
}

fn match_rule<'a>(elm: &ElementData, rule: &'a Rule) -> Option<MatchedRule<'a>> {
    rule.selectors
        .iter()
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

    if selector
        .class
        .iter()
        .any(|class| !elm.classes().contains(&**class))
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::css::CSSParser;
    use crate::parser::html::HTMLParser;

    fn test_element(node_type: &NodeType, expected: &str) {
        if let NodeType::Element(elm) = node_type {
            assert_eq!(&elm.tag_name, expected);
        } else {
            panic!("node should has Element");
        }
    }

    fn test_text(node_type: &NodeType, expected: &str) {
        if let NodeType::Text(text) = node_type {
            assert_eq!(text.as_str(), expected);
        } else {
            panic!("node should has Text");
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

        let styled_node = create_style_tree(&dom, &cssom, None);

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
            Value::ColorValue(Color::new(204, 0, 0, 1.0)),
        );
        assert_eq!(
            *div.specified_values.get("display").unwrap(),
            Value::Keyword("block".into()),
        );

        let text = &div.children[0];
        assert_eq!(&text.specified_values.len(), &1,);
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

        let styled_node = create_style_tree(&dom, &cssom, None);

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

        let styled_node = create_style_tree(&dom, &cssom, None);

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

    #[test]
    fn test_inheritance() {
        let html = "
<body>
  <div class='foo'>
    <p class='bar'>test</p>
  </div>
</body>
";

        let author_css = "
.foo {
  color: green;
}

.bar {
  font-size: 16px;
}
";

        let mut html_parser = HTMLParser::new(html.into());
        let mut css_parser = CSSParser::new(author_css.into());

        let dom = html_parser.run();

        let author_rules = css_parser.parse_rules(Origin::Author);

        let cssom = Stylesheet::new(author_rules);

        let styled_node = create_style_tree(&dom, &cssom, None);

        test_element(&styled_node.node.node_type, &"body");
        assert_eq!(&styled_node.specified_values.len(), &0);

        assert_eq!(&styled_node.node.children.len(), &1);

        let div = &styled_node.children[0];
        test_element(&div.node.node_type, &"div");
        assert_eq!(&div.specified_values.len(), &1);
        assert_eq!(
            *div.specified_values.get("color").unwrap(),
            Value::Keyword("green".into()),
        );

        assert_eq!(&div.children.len(), &1);

        let p = &div.children[0];
        test_element(&p.node.node_type, &"p");
        assert_eq!(&p.specified_values.len(), &2);
        assert_eq!(
            *p.specified_values.get("color").unwrap(),
            Value::Keyword("green".into()),
        );
        assert_eq!(
            *p.specified_values.get("font-size").unwrap(),
            Value::Length(16.0, Unit::Px),
        );

        assert_eq!(&p.children.len(), &1);

        let text = &p.children[0];
        test_text(&text.node.node_type, &"test");
        assert_eq!(&text.specified_values.len(), &2);
        assert_eq!(
            *text.specified_values.get("color").unwrap(),
            Value::Keyword("green".into()),
        );
        assert_eq!(
            *text.specified_values.get("font-size").unwrap(),
            Value::Length(16.0, Unit::Px),
        );
    }
}
