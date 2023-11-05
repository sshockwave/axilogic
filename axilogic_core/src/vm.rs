use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub usize);
static SYMBOL_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Symbol {
    pub fn new() -> Self {
        Self(SYMBOL_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum Element {
    Symbol(Symbol),
    Conditional(Rc<Element>, Rc<Element>),
    Not(Rc<Element>),
    Universal(Symbol, Rc<Element>),
    Conjunction(Symbol, Rc<Element>, Rc<Element>),
}
