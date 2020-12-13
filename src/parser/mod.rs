pub mod html;
pub mod css;

trait Parser {
  fn input(&self) -> &str;

  fn pos(&self) -> usize;

  fn set_pos(&mut self, next_pos: usize);

  fn next_char(&self) -> char {
    self.input()[self.pos()..].chars().next().unwrap()
  }

  fn starts_with(&self, s: &str) -> bool {
    self.input()[self.pos()..].starts_with(s)
  }

  fn eof(&self) -> bool {
    self.pos() >= self.input().len()
  }

  fn consume_char(&mut self) -> char {
    let mut iter = self.input()[self.pos()..].char_indices();
    let (_, cur_char) = iter.next().unwrap();
    let (next_pos, _) = iter.next().unwrap_or((1, ' '));
    self.set_pos(next_pos);
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
}
