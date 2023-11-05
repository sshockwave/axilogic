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
    let l = self.left.len() + delta_l;
    let r = self.right.len() + delta_r;
    self.size * 3 > l * 5 && self.size * 3 > r * 5
  }
  fn update(&mut self) {
    self.size = self.left.len() + self.right.len() + 1;
  }
}

pub struct Tree<T: Ord> {
  root: Option<Box<Node<T>>>,
}

impl<T: Ord> Tree<T> {
  pub fn new() -> Self {
    Tree { root: None }
  }
  pub fn len(&self) -> usize {
    return self.root.as_ref().map_or(0, |v| v.size);
  }
  fn mid_point(l: usize, r: usize) -> usize {
    l + (r - l) / 2
  }
  fn dfs_push(node: &mut Option<Box<Node<T>>>, vec: &mut Vec<Rc<Info<T>>>) {
    if let Some(mut x) = node.take() {
      Self::dfs_push(&mut x.left.root, vec);
      if Rc::strong_count(&x.info) > 1 {
        vec.push(x.info);
      }
      Self::dfs_push(&mut x.right.root, vec);
    }
  }
  fn dfs_build(vec: &[Rc<Info<T>>], (key_l, key_r): (usize, usize)) -> Tree<T> {
    if vec.is_empty() {
      return Tree::new();
    }
    let m = vec.len() / 2;
    let key_m = Self::mid_point(key_l, key_r);
    let rc = vec[m].clone();
    rc.as_ref().key.replace(key_m);
    Tree {
      root: Some(Box::new(Node {
        left: Self::dfs_build(&vec[..m], (key_l, key_m - 1)),
        right: Self::dfs_build(&vec[m + 1..], (key_m + 1, key_r)),
        info: rc,
        size: vec.len(),
      })),
    }
  }
  fn rebuild(&mut self, intv: (usize, usize)) {
    let mut vec = Vec::new();
    Self::dfs_push(&mut self.root, &mut vec);
    *self = Self::dfs_build(&vec, intv);
  }
  fn insert_node(&mut self, value: T, (l, r): (usize, usize), will_rebuild: bool) -> Rc<Info<T>> {
    if let Some(v) = self.root.as_mut() {
      use std::cmp::Ordering::*;
      let t = value.cmp(&v.info.value);
      let (balanced, child, intv) = match t {
        Equal => return v.info.clone(),
        Less => (v.balanced(1, 0), &mut v.left, (l, *v.info.key.borrow() - 1)),
        Greater => (v.balanced(0, 1), &mut v.right, (*v.info.key.borrow() + 1, r)),
      };
      let info = Self::insert_node(child, value, intv, !balanced);
      if !will_rebuild {
        if !balanced {
          self.rebuild(intv);
        } else if !will_rebuild {
          v.update();
        }
      }
      info
    } else {
      let info = Rc::new(Info {
        key: RefCell::new(Self::mid_point(l, r)),
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
