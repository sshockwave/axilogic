use std::{rc::Rc, cell::RefCell};

#[derive(Eq, Ord)]
pub struct Info<T: Ord> {
  key: RefCell<usize>,
  value: T,
}

impl<T: Ord> PartialEq for Info<T> {
  fn eq(&self, other: &Self) -> bool {
    self.key == other.key
  }
}

impl<T: Ord> PartialOrd for Info<T> {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.key.partial_cmp(&other.key)
  }
}

struct Node<T: Ord> {
  left: Tree<T>,
  right: Tree<T>,
  info: Rc<Info<T>>,
  size: usize,
}

impl<T: Ord> Node<T> {
  fn balanced(&self, delta_l: usize, delta_r: usize) -> bool {
    let l = self.left.root.as_ref().map(|v| v.size).unwrap_or(0) + delta_l;
    let r = self.right.root.as_ref().map(|v| v.size).unwrap_or(0) + delta_r;
    self.size * 3 > l * 5 && self.size * 3 > r * 5
  }
}

pub struct Tree<T: Ord> {
  root: Option<Box<Node<T>>>,
}

impl<T: Ord> Tree<T> {
  pub fn new() -> Self {
    Tree { root: None }
  }
  fn mid_point(l: usize, r: usize) -> usize {
    l + (r - l) / 2
  }
  fn rebuild(&mut self, intv: (usize, usize)) {
    todo!()
  }
  fn insert_node(&mut self, value: T, intv: (usize, usize), will_rebuild: bool) -> Rc<Info<T>> {
    if let Some(v) = self.root.as_mut() {
      use std::cmp::Ordering::*;
      let t = value.cmp(&v.info.value);
      let (delta_l, delta_r, child, intv) = match t {
        Equal => return v.info.clone(),
        Less => (1, 0, &mut v.left, (intv.0, *v.info.key.borrow())),
        Greater => (0, 1, &mut v.right, (*v.info.key.borrow(), intv.1)),
      };
      let rebuild = !will_rebuild && !v.balanced(delta_l, delta_r);
      let info = Self::insert_node(child, value, intv, will_rebuild || rebuild);
      if rebuild {
        self.rebuild(intv);
      }
      info
    } else {
      let info = Rc::new(Info {
        key: RefCell::new(Self::mid_point(intv.0, intv.1)),
        value,
      });
      self.root = Some(Box::new(Node {
        left: Tree{ root:None },
        right: Tree{ root:None },
        info: info.clone(),
        size: 1,
      }));
      info
    }
  }
  pub fn insert(&mut self, value: T) -> Rc<Info<T>> {
    self.insert_node(value, (0, usize::MAX), false)
  }
}
