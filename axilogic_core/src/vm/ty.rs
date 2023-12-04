use std::ops::Deref;

use crate::ds::dedup::{Dedup, HashDedup};
use crate::err::{OperationError, Result};

#[derive(Hash, PartialEq, Eq)]
enum TypeEnum {
    Symbol,
    Inference(
        <HashDedup<Self> as Dedup>::Ptr,
        <HashDedup<Self> as Dedup>::Ptr,
    ),
}

pub struct Registry {
    dedup: HashDedup<TypeEnum>,
    symbol: Type,
}

impl Registry {
    pub fn new() -> Self {
        let mut dedup = HashDedup::new();
        let symbol = dedup.get(TypeEnum::Symbol);
        Self {
            dedup,
            symbol: Type { data: symbol },
        }
    }
    pub fn symbol(&mut self) -> Type {
        self.symbol.clone()
    }
    pub fn infer(&mut self, a: Type, b: Type) -> Type {
        Type {
            data: self.dedup.get(TypeEnum::Inference(a.data, b.data)),
        }
    }
}

type Ptr = <HashDedup<TypeEnum> as Dedup>::Ptr;
#[derive(Clone)]
pub struct Type {
    data: Ptr,
}

impl Type {
    fn dfs_check(a: &Ptr, b: &Ptr) -> bool {
        if a == b {
            return true;
        }
        use TypeEnum::*;
        match (a.deref(), b.deref()) {
            (Symbol, _) => true,
            (Inference(p1, q1), Inference(p2, q2)) => {
                Self::dfs_check(p1, p2) && Self::dfs_check(q1, q2)
            }
            (Inference(..), Symbol) => false,
        }
    }
    pub fn apply(&self, spec: &Self) -> Result<Type> {
        use TypeEnum::*;
        match self.data.deref() {
            Symbol => Err(OperationError::new("cannot apply symbol type")),
            Inference(p, q) => {
                if Self::dfs_check(p, &spec.data) {
                    Ok(Type { data: q.clone() })
                } else {
                    Err(OperationError::new("Type mismatch for application"))
                }
            }
        }
    }
}
