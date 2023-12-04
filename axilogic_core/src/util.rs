use std::hash::Hash;

pub trait IdGenerator {
    type Id: Clone + Hash + Ord;
    fn new(&mut self) -> Self::Id;
}

pub struct CountGenerator(usize);
impl IdGenerator for CountGenerator {
    type Id = usize;
    fn new(&mut self) -> Self::Id {
        let id = self.0;
        self.0 += 1;
        id
    }
}
impl Default for CountGenerator {
    fn default() -> Self {
        Self(0)
    }
}

pub fn rc_take<T: Clone>(rc: std::rc::Rc<T>) -> T {
    std::rc::Rc::try_unwrap(rc).unwrap_or_else(|rc| rc.as_ref().clone())
}

pub fn vec_rev_get<T>(arr: &Vec<T>, index: usize) -> Option<&T> {
    let len = arr.len();
    if index <= len {
        Some(&arr[len - index])
    } else {
        None
    }
}

pub fn defer<T, F: FnOnce() -> T>(f: F) -> impl Drop {
    Defer(Some(f))
}

struct Defer<T, F: FnOnce() -> T>(Option<F>);
impl<T, F: FnOnce() -> T> Drop for Defer<T, F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}
