use std::num::NonZeroUsize;

use crate::{err::Result, isa::InstructionSet};

pub trait Expression<T: InstructionSet> {
    fn add_to(&self, vm: &mut T) -> Result<()>;
    fn export(&self, vm: &mut T, name: String, is_real: bool) -> Result<()> {
        if !is_real {
            vm.syn()?;
        }
        self.add_to(vm)?;
        if is_real {
            vm.def(name)?;
        } else {
            vm.hyp(name)?;
        }
        Ok(())
    }
}
type Ptr<'a, T> = Box<dyn Expression<T> + 'a>;

#[derive(Debug, Clone, Copy)]
pub struct ForallArg(NonZeroUsize);
impl<T: InstructionSet> Expression<T> for ForallArg {
    fn add_to(&self, vm: &mut T) -> Result<()> {
        vm.arg(self.0)?;
        Ok(())
    }
}
impl<'a, T: InstructionSet> From<ForallArg> for Ptr<'a, T> {
    fn from(arg: ForallArg) -> Self {
        Box::new(arg)
    }
}

/// Does not support nesting!
pub struct Forall<'a, T: InstructionSet> {
    num_vars: NonZeroUsize,
    expr: Ptr<'a, T>,
}
impl<'a, T: InstructionSet> Forall<'a, T> {
    pub fn new<F: Fn(Vec<ForallArg>) -> Ptr<'a, T>>(num_vars: usize, f: F) -> Self {
        Self {
            num_vars: num_vars.try_into().unwrap(),
            expr: f((1..=num_vars)
                .rev()
                .map(|i| ForallArg(NonZeroUsize::new(i).unwrap()))
                .collect()),
        }
    }
}

impl<'a, T: InstructionSet> Expression<T> for Forall<'a, T> {
    fn add_to(&self, vm: &mut T) -> Result<()> {
        vm.uni()?;
        for _ in 0..self.num_vars.get() {
            vm.var()?;
        }
        vm.qed()?;
        self.expr.add_to(vm)?;
        vm.qed()?;
        Ok(())
    }
}

pub struct Concept<'a, T> {
    name: &'static str,
    args: Vec<Ptr<'a, T>>,
}
impl<'a, T: InstructionSet> Concept<'a, T> {
    pub fn new(name: &'static str, args: Vec<Ptr<'a, T>>) -> Self {
        Self { name, args }
    }
}
impl<'a, T: InstructionSet> Expression<T> for Concept<'a, T> {
    fn add_to(&self, vm: &mut T) -> Result<()> {
        vm.req(&self.name)?;
        for arg in &self.args {
            vm.syn()?;
            arg.add_to(vm)?;
            vm.app()?;
        }
        Ok(())
    }
}

pub fn imply<'a, T: InstructionSet + 'a>(a: Ptr<'a, T>, b: Ptr<'a, T>) -> Ptr<'a, T> {
    Box::new(Concept::new("sys::imply", vec![a, b]))
}

pub fn not<'a, T: InstructionSet + 'a>(a: Ptr<'a, T>) -> Ptr<'a, T> {
    Box::new(Concept::new("sys::not", vec![a]))
}
