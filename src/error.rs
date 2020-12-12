pub fn new_internal_error(name: &str, msg: &str)  {
  panic!("[Internal {} Error]: {}", name, msg);
}
