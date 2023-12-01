use std::hash::Hash;

pub trait IdGenerator {
  type Id: Clone + Hash + Ord;
  fn new(&mut self) -> Self::Id;
}

pub fn rc_take<T: Clone>(rc: std::rc::Rc<T>) -> T {
  std::rc::Rc::try_unwrap(rc).unwrap_or_else(|rc| rc.as_ref().clone())
}
