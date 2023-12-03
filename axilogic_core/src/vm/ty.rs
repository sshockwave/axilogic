use crate::ds::dedup::{Dedup, HashDedup};

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
    pub fn check(&mut self, spec: &Self) -> bool {
        todo!()
    }
}

#[derive(Clone)]
pub struct Type {
    data: <HashDedup<TypeEnum> as Dedup>::Ptr,
}
