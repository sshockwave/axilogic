mod ty;

use std::{cell::RefCell, collections::HashMap, num::NonZeroUsize, rc::Rc};

use crate::{
    err::{OperationError, Result},
    util::IdGenerator,
};

enum Element<G: IdGenerator, P> {
    Argument { pos: NonZeroUsize, ty: ty::Type },
    Object { id: G::Id, params: Vec<P> },
    Universal { ty: ty::Type, body: P },
    Implication(P, P),
    Application { ty: ty::Type, arg: P, body: P },
}
impl<G: IdGenerator, P> Element<G, P> {
    fn ty(&self, reg: &mut ty::Registry) -> ty::Type {
        match self {
            Element::Argument { ty, .. } => ty.clone(),
            Element::Object { .. } => reg.symbol(),
            Element::Universal { ty, .. } => ty.clone(),
            Element::Implication(p, q) => reg.symbol(),
            Element::Application { ty, .. } => ty.clone(),
        }
    }
}

enum CacheEnum<G: IdGenerator, P> {
    Primitive(Element<G, P>),
    Bind { body: P, arg: P, ty: ty::Type },
    RefShift(P, NonZeroUsize),
}
struct CacheElement<G: IdGenerator> {
    data: RefCell<CacheEnum<G, Rc<Self>>>,
    max_ref: usize,
}
impl<G: IdGenerator> CacheElement<G> {
    fn ty(&self, reg: &mut ty::Registry) -> ty::Type {
        match &*self.data.borrow() {
            CacheEnum::Primitive(el) => el.ty(reg),
            CacheEnum::Bind { ty, .. } => ty.clone(),
            CacheEnum::RefShift(el, _) => el.ty(reg),
        }
    }
}

enum StackElement<G: IdGenerator> {
    Argument,
    Synthetic,
    Types(Vec<ty::Type>),
    Element(Rc<CacheElement<G>>),
}

pub struct Verifier<G: IdGenerator> {
    obj_id: G,
    is_syn: bool,
    arg_stack: Vec<ty::Type>,
    stack: Vec<StackElement<G>>,
    ty_reg: ty::Registry,
    sym_table: HashMap<String, (bool, Rc<CacheElement<G>>)>, // is_real, element
}

fn s_top<T>(s: &mut Vec<T>) -> Result<&mut T> {
    match s.last_mut() {
        Some(x) => Ok(x),
        None => Err(OperationError::new("Stack underflow")),
    }
}

fn s_pop<T>(s: &mut Vec<T>) -> Result<T> {
    match s.pop() {
        Some(x) => Ok(x),
        None => Err(OperationError::new("Stack underflow")),
    }
}

impl<G: IdGenerator> Verifier<G> {
    pub fn init_sys(&mut self) {
        todo!("proposition logic")
    }
    pub fn new(obj_id: G) -> Self {
        let mut vm = Self {
            obj_id,
            ty_reg: ty::Registry::new(),
            stack: Vec::new(),
            arg_stack: Vec::new(),
            is_syn: false,
            sym_table: HashMap::new(),
        };
        vm.init_sys();
        vm
    }
    pub fn has(&self, s: String) -> bool {
        self.sym_table.contains_key(&s)
    }
    fn push(&mut self, el: StackElement<G>) {
        self.stack.push(el)
    }
    fn pop(&mut self) -> Result<StackElement<G>> {
        s_pop(&mut self.stack)
    }
}

impl<G: IdGenerator> super::isa::InstructionSet for Verifier<G> {
    fn syn(&mut self) -> Result<()> {
        if self.is_syn {
            return Err(OperationError::new("Already in synthetic mode"));
        }
        self.is_syn = true;
        self.stack.push(StackElement::Synthetic);
        Ok(())
    }

    fn app(&mut self) -> Result<()> {
        todo!()
    }

    fn arg(&mut self, n: NonZeroUsize) -> Result<()> {
        if !self.is_syn {
            return Err(OperationError::new(
                "Using argument of function in non-synthetic mode",
            ));
        }
        if n.get() > self.arg_stack.len() {
            return Err(OperationError::new(format!(
                "Argument index out of range: {}",
                n.get()
            )));
        }
        self.push(StackElement::Element(Rc::new(CacheElement {
            data: RefCell::new(CacheEnum::Primitive(Element::Argument {
                pos: n,
                ty: self.arg_stack[self.arg_stack.len() - n.get()].clone(),
            })),
            max_ref: n.get(),
        })));
        Ok(())
    }

    fn uni(&mut self) -> Result<()> {
        self.push(StackElement::Types(Vec::new()));
        Ok(())
    }

    fn def(&mut self, s: String) -> Result<()> {
        if self.is_syn {
            return Err(OperationError::new(
                "Exporting an element in synthetic mode but calling `def`",
            ));
        }
        let el = if let StackElement::Element(el) = self.pop()? {
            el
        } else {
            return Err(OperationError::new("Exporting an invalid element"));
        };
        if el.max_ref != 0 {
            return Err(OperationError::new("Exporting an unbounded element"));
        }
        self.sym_table.insert(s, (true, el));
        Ok(())
    }

    fn hyp(&mut self, s: String) -> Result<()> {
        let el = if let StackElement::Element(el) = self.pop()? {
            el
        } else {
            return Err(OperationError::new("Exporting an invalid element"));
        };
        if let StackElement::Synthetic = self.pop()? {
            self.is_syn = false;
        } else {
            return Err(OperationError::new(
                "Exporting an element in non-synthetic mode",
            ));
        }
        if el.max_ref != 0 {
            return Err(OperationError::new("Exporting an unbounded element"));
        }
        self.sym_table.insert(s, (false, el));
        Ok(())
    }

    fn obj(&mut self, n: usize, s: String) -> Result<()> {
        todo!()
    }

    fn hkt(&mut self) -> Result<()> {
        todo!()
    }
    fn qed(&mut self) -> Result<()> {
        todo!()
    }
    fn req(&mut self, s: String) -> Result<()> {
        let (is_real, el) = self
            .sym_table
            .get(&s)
            .ok_or_else(|| OperationError::new(format!("Symbol not found: {}", s)))?;
        if !is_real && !self.is_syn {
            return Err(OperationError::new(format!(
                "Using imaginary symbol {} in imaginary mode",
                s
            )));
        }
        self.push(StackElement::Element(el.clone()));
        Ok(())
    }
    fn mp(&mut self) -> Result<()> {
        if self.is_syn {
            return Err(OperationError::new(
                "Use sat instead of mp in synthetic mode",
            ));
        }
        let p_pred = self.pop()?;
        let pq = self.pop()?;
        todo!()
    }
    fn sat(&mut self) -> Result<()> {
        if !self.is_syn {
            return Err(OperationError::new("Using sat in non-synthetic mode"));
        }
        todo!()
    }
    fn var(&mut self) -> Result<()> {
        let el = self
            .stack
            .last_mut()
            .ok_or_else(|| OperationError::new("Using var without uni"))?;
        let vec = if let StackElement::Types(vec) = el {
            vec
        } else {
            return Err(OperationError::new("Using var without uni"));
        };
        vec.push(self.ty_reg.symbol());
        Ok(())
    }
}
