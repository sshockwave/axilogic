use crate::{
    ds::pds::{set_diff_mut, set_union, set_union_own},
    err::OperationError,
    util::IdGenerator,
};
use rpds::{HashTrieMap, HashTrieSet};
use std::rc::Rc;

pub struct Symbol<G: IdGenerator> {
    id: G::Id,
    provides: HashTrieSet<G::Id>,
    ref_ptr: Type<G>,
}

impl<G: IdGenerator> Symbol<G> {
    pub fn new(g: &mut G) -> Self {
        let id = g.new();
        let mut provides = HashTrieSet::new();
        provides.insert_mut(id.clone());
        let mut self_ptr = Type(Rc::new(TypeEnum::Reference {
            id: id.clone(),
            requires: provides.clone(),
        }));
        Self {
            id,
            provides,
            ref_ptr: self_ptr,
        }
    }
    pub fn get_ref(&self) -> Type<G> {
        self.ref_ptr.clone()
    }
}

struct Reference<G: IdGenerator>(Type<G>);

impl<G: IdGenerator> Clone for Reference<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

enum TypeEnum<G: IdGenerator> {
    Symbol {
        id: G::Id,
        provides: HashTrieSet<G::Id>,
        ref_ptr: Type<G>,
    },
    Reference {
        id: G::Id,
        requires: HashTrieSet<G::Id>,
    },
    Quantification {
        has_symbol: bool,
        quantifier: Type<G>,
        predicate: Type<G>,
        provides: HashTrieSet<G::Id>,
        requires: HashTrieSet<G::Id>,
    },
}

impl<G: IdGenerator> From<Symbol<G>> for Type<G> {
    fn from(s: Symbol<G>) -> Self {
        Self(Rc::new(TypeEnum::Symbol {
            id: s.id,
            provides: s.provides,
            ref_ptr: s.ref_ptr,
        }))
    }
}
impl<G: IdGenerator> From<Reference<G>> for Type<G> {
    fn from(r: Reference<G>) -> Self {
        r.0
    }
}

pub struct Type<G: IdGenerator>(Rc<TypeEnum<G>>);

impl<G: IdGenerator> Type<G> {
    pub fn new_quant(p: Self, q: Self) -> Self {
        let mut provides = set_union_own(p.provides(), q.provides());
        let mut requires = q.requires();
        set_diff_mut(&mut requires, &p.provides());
        let requires = set_union_own(requires, p.requires());
        let has_symbol = p.has_symbol() || q.has_symbol();
        Self(Rc::new(TypeEnum::Quantification {
            has_symbol,
            quantifier: p,
            predicate: q,
            provides,
            requires,
        }))
    }
    fn requires(&self) -> HashTrieSet<G::Id> {
        use TypeEnum::*;
        match self.0.as_ref() {
            Symbol { .. } => HashTrieSet::new(),
            Reference { requires, .. } | Quantification { requires, .. } => requires.clone(),
        }
    }
    fn provides(&self) -> HashTrieSet<G::Id> {
        use TypeEnum::*;
        match self.0.as_ref() {
            Symbol { provides, .. } | Quantification { provides, .. } => provides.clone(),
            Reference { .. } => HashTrieSet::new(),
        }
    }
    fn has_symbol(&self) -> bool {
        use TypeEnum::*;
        match self.0.as_ref() {
            Symbol { .. } => true,
            Reference { .. } => false,
            Quantification { has_symbol, .. } => *has_symbol,
        }
    }
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
    fn dfs_deref(&self, symbols: &mut HashTrieMap<G::Id, Type<G>>) -> Type<G> {
        use TypeEnum::*;
        match self.0.as_ref() {
            Symbol { id, ref_ptr, .. } => {
                symbols.insert_mut(id.clone(), self.clone());
                ref_ptr.clone()
            }
            Reference { .. } => self.clone(),
            Quantification {
                quantifier: p,
                predicate: q,
                has_symbol,
                ..
            } => {
                if !has_symbol {
                    return self.clone();
                }
                let p2 = Self::dfs_deref(p, symbols);
                let q2 = Self::dfs_deref(q, symbols);
                if Rc::ptr_eq(&p.0, &p2.0) && Rc::ptr_eq(&q.0, &q2.0) {
                    self.clone()
                } else {
                    Self::new_quant(p2, q2)
                }
            }
        }
    }
    fn dfs_check(a: &Type<G>, b: &Type<G>) -> bool {
        todo!()
    }
    fn dfs_match(
        &self,
        param: &Type<G>,
        registry: &HashTrieMap<G::Id, Type<G>>,
        required: &HashTrieSet<G::Id>,
        symbols: &mut HashTrieMap<G::Id, Type<G>>,
    ) -> crate::err::Result<()> {
        use TypeEnum::*;
        match self.0.as_ref() {
            Symbol { id, .. } => {
                if required.contains(id) {
                    registry.insert_mut(id.clone(), param.dfs_deref(symbols));
                }
                Ok(())
            }
            Reference { id, .. } => {
                let fulfilled = registry.get(id).unwrap();
                if Self::dfs_check(fulfilled, param) {
                    Ok(())
                } else {
                    Err(OperationError::new("Type mismatch inside reference"))
                }
            }
            Quantification {
                quantifier: p1,
                predicate: q1,
                ..
            } => match param.0.as_ref() {
                Quantification {
                    quantifier: p2,
                    predicate: q2,
                    ..
                } => {
                    Self::dfs_match(
                        p1,
                        p2,
                        registry,
                        &set_union(required, &q1.requires()),
                        symbols,
                    )?;
                    Self::dfs_match(q1, q2, registry, required, symbols)?;
                    Ok(())
                }
                _ => Err(OperationError::new("Param type is not specific enough")),
            },
        }
    }
    fn dfs_apply(this: &Type<G>, registry: &HashTrieMap<G::Id, Type<G>>) -> Type<G> {
        match this.0.as_ref() {
            TypeEnum::Symbol(_) => this.clone(),
            TypeEnum::Reference(v) => registry.get(&v.id).unwrap_or(&this).clone(),
            TypeEnum::Quantification(v) => {
                let (p, q) = (&v.quantifier, &v.predicate);
                let p2 = Self::dfs_apply(p, registry);
                let q2 = Self::dfs_apply(q, registry);
                if Rc::ptr_eq(&p.0, &p2.0) && Rc::ptr_eq(&q.0, &q2.0) {
                    this.clone()
                } else {
                    Quantification::new(p2, q2).into()
                }
            }
        }
    }
}

pub struct BoundedType<G: IdGenerator>(Type<G>);

impl<G: IdGenerator> Clone for BoundedType<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<G: IdGenerator> TryFrom<Type<G>> for BoundedType<G> {
    type Error = crate::err::OperationError;
    fn try_from(ty: Type<G>) -> crate::err::Result<Self> {
        if ty.requires().is_empty() {
            Ok(Self(ty))
        } else {
            Err(crate::err::OperationError::new("Type is not bounded"))
        }
    }
}

impl<G: IdGenerator> BoundedType<G> {
    pub fn apply(&self, param: &Self) -> crate::err::Result<BoundedType<G>> {
        use TypeEnum::*;
        match &self.0 .0.data {
            Symbol { .. } => Err(OperationError::new("Type is not quantified")),
            Quantification {
                quantifier: p,
                predicate: q,
                ..
            } => {
                let mut registry = HashTrieMap::new();
                Self::dfs_match(p, &param.0, &mut registry, &mut q.0.requires.clone())?;
                Ok(Self::dfs_apply(q, &registry).try_into().unwrap())
            }
            Reference { .. } => unreachable!(),
        }
    }
}
