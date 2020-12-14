pub struct Stylesheet {
  pub rules: Vec<Rule>,
}

impl Stylesheet {
  pub fn new(rules: Vec<Rule>) -> Stylesheet {
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
  pub fn new(tag_name: Option<String>, id: Option<String>, class: Vec<String>) -> SimpleSelector {
    SimpleSelector { tag_name, id, class }
  }
}

pub struct Declaration {
  pub name: String,
  pub value: Value,
}

impl Declaration {
  pub fn new(name: String, value: Value) -> Declaration {
    Declaration { name, value }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
  Keyword(String),
  Length(f32, Unit),
  ColorValue(Color),
  None,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Unit {
  Px,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Color {
  r: u8,
  g: u8,
  b: u8,
  a: u8,
}

impl Color {
  pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color { r, g, b, a }
  }
}
