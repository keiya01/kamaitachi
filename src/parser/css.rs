use super::Parser;
use crate::cssom::*;

pub struct CSSParser {
    pos: usize,
    input: String,
}

impl CSSParser {
    pub fn new(input: String) -> CSSParser {
        CSSParser { pos: 0, input }
    }

    pub fn parse_rules(&mut self, level: Origin) -> Vec<Rule> {
        let mut rules = vec![];
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            rules.push(self.parse_rule(level.clone()));
        }
        rules
    }

    fn parse_rule(&mut self, level: Origin) -> Rule {
        Rule::new(self.parse_selectors(), self.parse_declarations(), level)
    }

    fn parse_selectors(&mut self) -> Vec<Selector> {
        let mut selectors = vec![];
        loop {
            selectors.push(Selector::Simple(self.parse_simple_selector()));
            self.consume_whitespace();
            match self.next_char() {
                ',' => {
                    self.consume_char();
                    self.consume_whitespace();
                }
                '{' => break,
                _ => {
                    self.consume_while(|c| c != '}');
                    self.consume_char();
                    selectors.pop();
                }
            }
        }
        // Return selectors with highest specificity first, for use in matching.
        selectors.sort_by_key(|b| std::cmp::Reverse(b.specificity()));
        selectors
    }

    fn parse_simple_selector(&mut self) -> SimpleSelector {
        let mut selector = SimpleSelector::new(None, None, vec![]);
        loop {
            self.consume_whitespace();
            match self.next_char() {
                '#' => {
                    self.consume_char();
                    selector.id = Some(self.parse_identifier())
                }
                '.' => {
                    self.consume_char();
                    selector.class.push(self.parse_identifier());
                }
                '*' => {
                    // universal selector
                    self.consume_char();
                }
                c if valid_identifier_char(c) => {
                    selector.tag_name = Some(self.parse_identifier());
                }
                _ => break,
            }
        }
        selector
    }

    pub fn parse_declarations(&mut self) -> Vec<Declaration> {
        // Declaration should be started with '{'.
        if self.next_char() == '{' {
            self.consume_char();
        }

        let mut declarations = vec![];
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }
            if self.next_char() == '}' {
                self.consume_char();
                break;
            }
            declarations.push(self.parse_declaration());
        }
        declarations
    }

    /// Parse one `<property>: <value>;` declaration.
    fn parse_declaration(&mut self) -> Declaration {
        let name = self.parse_identifier();

        self.consume_whitespace();
        if !self.eof() && self.next_char() == ':' {
            self.consume_char();
        }

        self.consume_whitespace();
        if self.eof() {
            return Declaration::new(name, Value::None);
        }

        let value = self.parse_value();

        self.consume_whitespace();
        if !self.eof() && self.next_char() == ';' {
            self.consume_char();
        }

        Declaration::new(name, value)
    }

    fn parse_value(&mut self) -> Value {
        match self.next_char() {
            '0'..='9' => self.parse_length(),
            '#' => self.parse_color(),
            c if valid_identifier_char(c) => self.parse_keyword(),
            _ => {
                self.consume_while(|c| c != '}');
                Value::None
            }
        }
    }

    fn parse_keyword(&mut self) -> Value {
        let keyword = self.parse_identifier();
        self.consume_whitespace();
        if self.next_char() != ',' {
            return Value::Keyword(keyword);
        }
        let mut keyword_array = vec![keyword];
        while self.next_char() == ',' {
            self.consume_char();
            self.consume_whitespace();
            if self.next_char() == ';' {
                break;
            }
            let keyword = self.parse_identifier();
            keyword_array.push(keyword);
        }
        Value::KeywordArray(keyword_array)
    }

    fn parse_length(&mut self) -> Value {
        let float = self.parse_float();
        self.consume_whitespace();
        if self.next_char() == ';' {
            self.consume_char();
            return Value::Number(float);
        }
        Value::Length(float, self.parse_unit())
    }

    fn parse_float(&mut self) -> f32 {
        let s = self.consume_while(|c| matches!(c, '0'..='9' | '.'));
        s.parse().unwrap_or(0.0)
    }

    fn parse_unit(&mut self) -> Unit {
        match &*self.parse_identifier().to_ascii_lowercase() {
            "px" => Unit::Px,
            _ => panic!("unrecognized unit"),
        }
    }

    fn parse_color(&mut self) -> Value {
        // Color value should be started with '#'.
        self.consume_char();

        Value::ColorValue(Color::new(
            self.parse_hex_pair(),
            self.parse_hex_pair(),
            self.parse_hex_pair(),
            1.0,
        ))
    }

    fn parse_hex_pair(&mut self) -> u8 {
        let s = &self.input[self.pos..self.pos + 2];
        self.pos += 2;
        u8::from_str_radix(s, 16).unwrap()
    }

    fn parse_identifier(&mut self) -> String {
        self.consume_while(valid_identifier_char)
    }
}

impl Parser for CSSParser {
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

fn valid_identifier_char(c: char) -> bool {
    // TODO: Include U+00A0 and higher.
    matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_')
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_tag_name() {
        let input = "
h1,
h2,
h3 {
  margin: auto;
  color: #cc0000;
}
";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(selector.tag_name.as_ref().unwrap(), &"h1");
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class.len(), &0);

            let Selector::Simple(selector) = &rule.selectors[1];
            assert_eq!(&selector.tag_name.as_ref().unwrap(), &"h2");
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class.len(), &0);

            let Selector::Simple(selector) = &rule.selectors[2];
            assert_eq!(selector.tag_name.as_ref().unwrap(), &"h3");
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class.len(), &0);

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "margin");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, "auto");
            } else {
                panic!("declaration.value should has Keyword");
            };

            let declaration = &rule.declarations[1];
            assert_eq!(&declaration.name, "color");
            if let Value::ColorValue(color) = &declaration.value {
                assert_eq!(color, &Color::new(204, 0, 0, 1.0));
            } else {
                panic!("declaration.value should has ColorValue");
            };
        }
    }

    #[test]
    fn test_parse_class() {
        let input = "div.note { margin-bottom: 20px; padding: 5.5px; }";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(selector.tag_name.as_ref().unwrap(), &"div");
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class[0], &"note");

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "margin-bottom");
            if let Value::Length(len, unit) = &declaration.value {
                assert_eq!(len, &(20 as f32));
                assert_eq!(unit, &Unit::Px);
            } else {
                panic!("declaration.value should has Length");
            };

            let declaration = &rule.declarations[1];
            assert_eq!(&declaration.name, "padding");
            if let Value::Length(len, unit) = &declaration.value {
                assert_eq!(len, &(5.5 as f32));
                assert_eq!(unit, &Unit::Px);
            } else {
                panic!("declaration.value should has Length");
            };
        }
    }

    #[test]
    fn test_parse_id() {
        let input = "#answer { display: none; }";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(selector.tag_name, None);
            assert_eq!(selector.id.as_ref().unwrap(), "answer");
            assert_eq!(&selector.class.len(), &0);

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "display");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, &"none");
            } else {
                panic!("declaration.value should has Keyword");
            };
        }
    }

    #[test]
    fn test_parse_missing_start_bracket() {
        let input = "
#answer
  display: none;
}
.class {
  color: red;
}";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(selector.tag_name, None);
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class.len(), &1);
            assert_eq!(&selector.class[0], &"class");

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "color");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, &"red");
            } else {
                panic!("declaration.value should has Keyword");
            };
        }
    }

    #[test]
    fn test_parse_missing_end_bracket() {
        let input = "
.class {
  color: red;
h1 {}
";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        assert_eq!(&stylesheet.rules.len(), &1);
        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(selector.tag_name, None);
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class.len(), &1);
            assert_eq!(&selector.class[0], &"class");

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "color");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, &"red");
            } else {
                panic!("declaration.value should has Keyword");
            };
        }
    }

    #[test]
    fn test_parse_missing_declaration_string() {
        let input = "
.class {
  color: red
  display: block;
  height auto;
}
";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        assert_eq!(&stylesheet.rules.len(), &1);
        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(selector.tag_name, None);
            assert_eq!(selector.id, None);
            assert_eq!(&selector.class.len(), &1);
            assert_eq!(&selector.class[0], &"class");

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "color");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, &"red");
            } else {
                panic!("declaration.value should has Keyword");
            };

            let declaration = &rule.declarations[1];
            assert_eq!(&declaration.name, "display");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, &"block");
            } else {
                panic!("declaration.value should has Keyword");
            };

            let declaration = &rule.declarations[2];
            assert_eq!(&declaration.name, "height");
            if let Value::Keyword(keyword) = &declaration.value {
                assert_eq!(keyword, &"auto");
            } else {
                panic!("declaration.value should has Keyword");
            };
        }
    }

    #[test]
    fn test_parse_keyword_array() {
        let input = "
.class {
    font-family: serif,  sans-serif,cursive,;
}
";

        let mut p = CSSParser::new(input.into());

        let rules = p.parse_rules(Origin::Author);
        let stylesheet = Stylesheet::new(rules);

        assert_eq!(&stylesheet.rules.len(), &1);
        for rule in stylesheet.rules {
            let Selector::Simple(selector) = &rule.selectors[0];
            assert_eq!(&selector.class[0], &"class");

            let declaration = &rule.declarations[0];
            assert_eq!(&declaration.name, "font-family");
            if let Value::KeywordArray(arr) = &declaration.value {
                assert_eq!(
                    arr,
                    &[
                        "serif".to_string(),
                        "sans-serif".to_string(),
                        "cursive".to_string()
                    ]
                );
            } else {
                panic!("declaration.value should has KeywordArray");
            };
        }
    }
}
