use crate::{
    ds::dedup::{Dedup, HashDedup},
    err::OperationError,
};

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
}

impl Registry {
    fn new() -> Self {
        Self {
            dedup: HashDedup::new(),
        }
    }
    fn new_symbol(&mut self) -> Type {
        Type {
            data: self.dedup.get(TypeEnum::Symbol),
        }
    }
    fn new_infer(&mut self, a: Type, b: Type) -> Result<Type, OperationError> {
        Ok(Type {
            data: self.dedup.get(TypeEnum::Inference(a.data, b.data)),
        })
    }
}

pub struct Type {
    data: <HashDedup<TypeEnum> as Dedup>::Ptr,
}
