use crate::error;

pub struct Stylesheet {
  pub rules: Vec<Rule>,
}

impl Stylesheet {
  fn new(rules: Vec<Rule>) -> Stylesheet {
    Stylesheet { rules }
  }
}

pub struct Rule {
  pub selectors: Vec<Selector>,
  pub declarations: Vec<Declaration>,
}

impl Rule {
  pub fn new(selectors: Vec<Selector>, declarations: Vec<Declaration>) -> Rule {
    Rule { selectors, declarations }
  }
}

pub enum Selector {
  Simple(SimpleSelector),
}

pub type Specificity = (usize, usize, usize);

impl Selector {
  pub fn specificity(&self) -> Specificity {
    // http://www.w3.org/TR/selectors/#specificity
    let Selector::Simple(simple) = self;
    let a = simple.id.iter().count();
    let b = simple.class.len();
    let c = simple.tag_name.iter().count();
    (a, b, c)
  }
}

pub struct SimpleSelector {
  pub tag_name: Option<String>,
  pub id: Option<String>,
  pub class: Vec<String>,
}

impl SimpleSelector {
  fn new(tag_name: Option<String>, id: Option<String>, class: Vec<String>) -> SimpleSelector {
    SimpleSelector { tag_name, id, class }
  }
}

pub struct Declaration {
  pub name: String,
  pub value: Value,
}

impl Declaration {
  fn new(name: String, value: Value) -> Declaration {
    Declaration { name, value }
  }
}

pub enum Value {
  Keyword(String),
  Length(f32, Unit),
  ColorValue(Color),
}

#[derive(Debug, PartialEq)]
pub enum Unit {
  Px,
}

#[derive(Debug, PartialEq)]
pub struct Color {
  r: u8,
  g: u8,
  b: u8,
  a: u8,
}

impl Color {
  fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
  }
}

pub struct Parser {
  pos: usize,
  input: String,
}

impl Parser {
  pub fn new(input: String) -> Parser {
    Parser { pos: 0, input }
  }

  pub fn run(&mut self) -> Stylesheet {
    Stylesheet::new(self.parse_rules())
  }

  fn parse_rules(&mut self) -> Vec<Rule> {
    let mut rules = vec![];
    loop {
      self.consume_whitespace();
      if self.eof() {
        break;
      }
      rules.push(self.parse_rule());
    }
    rules
  }

  fn parse_rule(&mut self) -> Rule {
    Rule::new(self.parse_selectors(), self.parse_declarations())
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
        },
        '{' => break,
        c => self.new_internal_error(&format!("unexpected character {} in selector list", c)),
      }
    }
    // Return selectors with highest specificity first, for use in matching.
    selectors.sort_by(|a, b| b.specificity().cmp(&a.specificity()));
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
        },
        '.' => {
          self.consume_char();
          selector.class.push(self.parse_identifier());
        },
        '*' => {
          // universal selector
          self.consume_char();
        },
        c if valid_identifier_char(c) => {
          selector.tag_name = Some(self.parse_identifier());
        },
        _ => break
      }
    }
    selector
  }

  fn parse_declarations(&mut self) -> Vec<Declaration> {
    if self.consume_char() != '{' {
      self.new_internal_error("Declaration must be started with '{'");
    }

    let mut declarations = vec![];
    loop {
      self.consume_whitespace();
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
    if self.consume_char() != ':' {
      self.new_internal_error("The next character in the property must be ':'");
    }
    
    self.consume_whitespace();
    let value = self.parse_value();

    self.consume_whitespace();
    if self.consume_char() != ';' {
      self.new_internal_error("The property must be ended with ';'");
    }

    Declaration::new(name, value)
  }

  fn parse_value(&mut self) -> Value {
    match self.next_char() {
      '0'..='9' => self.parse_length(),
      '#' => self.parse_color(),
      _ => Value::Keyword(self.parse_identifier()),
    }
  }
  
  fn parse_length(&mut self) -> Value {
    Value::Length(self.parse_float(), self.parse_unit())
  }

  fn parse_float(&mut self) -> f32 {
    let s  = self.consume_while(|c| match c {
      '0'..='9' | '.' => true,
      _ => false
    });
    s.parse().unwrap_or(0.0)
  }

  fn parse_unit(&mut self) -> Unit {
    match &*self.parse_identifier().to_ascii_lowercase() {
      "px" => Unit::Px,
      _ => panic!("unrecognized unit"),
    }
  }

  fn parse_color(&mut self) -> Value {
    if self.consume_char() != '#' {
      self.new_internal_error("color value should be started with '#'");
    }

    Value::ColorValue(
      Color::new(
        self.parse_hex_pair(),
        self.parse_hex_pair(),
        self.parse_hex_pair(),
        255
      )
    )
  }

  fn parse_hex_pair(&mut self) -> u8 {
    let s = &self.input[self.pos..self.pos+2];
    self.pos += 2;
    u8::from_str_radix(s, 16).unwrap()
  }

  fn parse_identifier(&mut self) -> String {
    self.consume_while(valid_identifier_char)
  }

  fn next_char(&self) -> char {
    self.input[self.pos..].chars().next().unwrap()
  }

  fn eof(&self) -> bool {
    self.pos >= self.input.len()
  }

  fn consume_char(&mut self) -> char {
    let mut iter = self.input[self.pos..].char_indices();
    let (_, cur_char) = iter.next().unwrap();
    let (next_pos, _) = iter.next().unwrap_or((1, ' '));
    self.pos += next_pos;
    cur_char
  }

  fn consume_while<F>(&mut self, test: F) -> String
      where F: Fn(char) -> bool {
    let mut result = String::new();
    while !self.eof() && test(self.next_char()) {
      result.push(self.consume_char());
    }
    result
  }

  fn consume_whitespace(&mut self) {
    self.consume_while(|c| c.is_whitespace());
  }

  fn new_internal_error(&self, msg: &str) {
    error::new_internal_error("CSS Parser", msg);
  }
}

fn valid_identifier_char(c: char) -> bool {
  match c {
      // TODO: Include U+00A0 and higher.
      'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => true,
      _ => false,
  }
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

    let mut p = Parser::new(input.into());

    let stylesheet = p.run();

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
        assert_eq!(color, &Color::new(204, 0, 0, 255));
      } else {
        panic!("declaration.value should has ColorValue");
      };
    }
  }

  #[test]
  fn test_parse_class() {
    let input = "div.note { margin-bottom: 20px; padding: 5.5px; }";

    let mut p = Parser::new(input.into());

    let stylesheet = p.run();

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

    let mut p = Parser::new(input.into());

    let stylesheet = p.run();

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
}
