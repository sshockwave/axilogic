use super::err::Result;
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

pub trait ISA {
    // stack bottom is 0, while stack top is -1
    fn push(&mut self, n: isize) -> Result<()>;
    fn pop(&mut self) -> Result<()>;

    // The function can be abstracted
    // only when the stack[-2] is a symbol
    // and also its last appearance
    fn symbol(&mut self) -> Result<()>;
    fn forall(&mut self) -> Result<()>;
    fn apply(&mut self) -> Result<()>;

    // When exporting, the stack top will be popped
    // and the symbol will be saved to the symbol table
    fn export(&mut self, name: String) -> Result<()>;
    fn import(&mut self, name: String) -> Result<()>;

    // A concept is a tuple (symbol, stack[-1], stack[-2])
    fn concept(&mut self) -> Result<()>;
    fn unwrap(&mut self) -> Result<()>;
}
