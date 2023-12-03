use crate::{err::OperationError, util::IdGenerator};
use rpds::{HashTrieMap, HashTrieSet};
use std::{hash::Hash, rc::Rc};

enum TypeEnum<G: IdGenerator> {
    Symbol {
        id: G::Id,
        ref_self: Type<G>,
    },
    Reference {
        id: G::Id,
    },
    Quantification {
        has_symbol: bool,
        quantifier: Type<G>,
        predicate: Type<G>,
    },
}

struct TypeData<G: IdGenerator> {
    data: TypeEnum<G>,
    provides: HashTrieSet<G::Id>,
    requires: HashTrieSet<G::Id>,
}

pub struct Type<G: IdGenerator>(Rc<TypeData<G>>);

fn set_union<T: Clone + Hash + Eq>(a: &mut HashTrieSet<T>, b: &mut HashTrieSet<T>) {
    if a.size() < b.size() {
        std::mem::swap(a, b);
    }
    let mut a = a.clone();
    for x in b.iter() {
        a.insert_mut(x.clone());
    }
}

impl<G: IdGenerator> Type<G> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
    fn make_ref_self(id: G::Id) -> Self {
        let mut set = HashTrieSet::new();
        set.insert_mut(id.clone());
        Self(Rc::new(TypeData {
            data: TypeEnum::Reference { id },
            provides: HashTrieSet::new(),
            requires: set,
        }))
    }
    pub fn new_symbol(g: &mut G) -> Self {
        let id = g.new();
        let ref_self = Self::make_ref_self(id.clone());
        let set = ref_self.0.requires.clone();
        Self(Rc::new(TypeData {
            data: TypeEnum::Symbol { id, ref_self },
            provides: set,
            requires: HashTrieSet::new(),
        }))
    }
    fn has_symbol(&self) -> bool {
        match &self.0.data {
            TypeEnum::Symbol { .. } => true,
            TypeEnum::Reference { .. } => false,
            TypeEnum::Quantification { has_symbol, .. } => *has_symbol,
        }
    }
    pub fn new_quant(p: Self, q: Self) -> Self {
        let mut provides = p.0.provides.clone();
        set_union(&mut provides, &mut q.0.provides.clone());
        let mut requires = p.0.requires.clone();
        for x in q.0.requires.iter() {
            if !p.0.provides.contains(x) {
                requires.insert_mut(x.clone());
            }
        }
        let has_symbol = p.has_symbol() || q.has_symbol();
        Self(Rc::new(TypeData {
            data: TypeEnum::Quantification {
                has_symbol,
                quantifier: p,
                predicate: q,
            },
            provides,
            requires,
        }))
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
        if ty.0.requires.is_empty() {
            Ok(Self(ty))
        } else {
            Err(crate::err::OperationError::new("Type is not bounded"))
        }
    }
}

impl<G: IdGenerator> BoundedType<G> {
    fn dfs_check(a: &Type<G>, b: &Type<G>) -> bool {
        todo!()
    }
    fn dfs_deref(a: &Type<G>) -> Type<G> {
        if !a.has_symbol() {
            return a.clone();
        }
        use TypeEnum::*;
        match &a.0.data {
            Symbol { ref_self, .. } => ref_self.clone(),
            Reference { .. } => a.clone(),
            Quantification {
                quantifier: p,
                predicate: q,
                ..
            } => {
                let p2 = Self::dfs_deref(p);
                let q2 = Self::dfs_deref(q);
                if Rc::ptr_eq(&p.0, &p2.0) && Rc::ptr_eq(&q.0, &q2.0) {
                    a.clone()
                } else {
                    Type::new_quant(p2, q2)
                }
            }
        }
    }
    fn dfs_match(
        func: &Type<G>,
        param: &Type<G>,
        registry: &mut HashTrieMap<G::Id, Type<G>>,
        required: &mut HashTrieSet<G::Id>,
    ) -> crate::err::Result<()> {
        use TypeEnum::*;
        match &func.0.data {
            Symbol { id, .. } => {
                if required.remove_mut(id) {
                    todo!("Convert param to reference");
                    registry.insert_mut(id.clone(), param.clone());
                }
                Ok(())
            }
            Reference { id } => {
                let fulfilled = registry.get(id).unwrap();
                if Self::dfs_check(fulfilled, param) {
                    Ok(())
                } else {
                    Err(OperationError::new("Type mismatch inside reference"))
                }
            }
            Quantification {
                quantifier: p,
                predicate: q,
                ..
            } => match &param.0.data {
                Quantification {
                    quantifier: p2,
                    predicate: q2,
                    ..
                } => {
                    set_union(required, &mut q.0.requires.clone());
                    Self::dfs_match(p, p2, registry, required)?;
                    Self::dfs_match(q, q2, registry, required)?;
                    Ok(())
                }
                _ => Err(OperationError::new("Param type is not specific enough")),
            },
        }
    }
    fn dfs_apply(this: &Type<G>, registry: &HashTrieMap<G::Id, Type<G>>) -> Type<G> {
        if this.0.requires.size() == 0 {
            return this.clone();
        }
        use TypeEnum::*;
        match &this.0.data {
            Symbol { .. } => this.clone(),
            Reference { id } => registry.get(id).unwrap_or(&this).clone(),
            Quantification {
                quantifier: p,
                predicate: q,
                ..
            } => {
                let p2 = Self::dfs_apply(p, registry);
                let q2 = Self::dfs_apply(q, registry);
                if Rc::ptr_eq(&p.0, &p2.0) && Rc::ptr_eq(&q.0, &q2.0) {
                    this.clone()
                } else {
                    Type::new_quant(p2, q2)
                }
            }
        }
    }
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
