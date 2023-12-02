use bit_vec::BitVec;
use std::{cmp::max, num::NonZeroUsize, rc::Rc};

use crate::util::IdGenerator;

pub enum TypeData<G: IdGenerator> {
    Symbol,
    Reference(NonZeroUsize, BitVec),
    Quantification { quantifier: Type<G>, predicate: Type<G> },
}

pub struct Type<G: IdGenerator> {
    data: Rc<TypeData<G>>,
    requires: std::collections::HashSet<G::Id>,
}

impl<G: IdGenerator> Type<G> {
}

impl<G: IdGenerator> TypeData<G> {
}
