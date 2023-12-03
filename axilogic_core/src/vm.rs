mod ty;

use std::{cell::RefCell, cmp::max, collections::HashMap, num::NonZeroUsize, rc::Rc};

use crate::{
    err::{OperationError, Result},
    util::{CountGenerator, IdGenerator},
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
            Element::Implication(..) => reg.symbol(),
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
    fn unwrap_one(&self) -> &Element<G, Rc<Self>> {
        todo!()
    }
    fn check_equal(a: &Rc<Self>, b: &Rc<Self>) -> bool {
        todo!()
    }
}
impl<G: IdGenerator> From<Element<G, Rc<Self>>> for CacheElement<G> {
    fn from(el: Element<G, Rc<Self>>) -> Self {
        use Element::*;
        let max_ref = match &el {
            Argument { pos, .. } => pos.get(),
            Object { params, .. } => params.iter().map(|x| x.max_ref).max().unwrap_or(0),
            Universal { body, .. } => max(body.max_ref, 1) - 1,
            Implication(p, q) => max(p.max_ref, q.max_ref),
            Application { arg, body, .. } => max(arg.max_ref, max(body.max_ref, 1) - 1),
        };
        CacheElement {
            data: RefCell::new(CacheEnum::Primitive(el)),
            max_ref,
        }
    }
}

enum StackElement<G: IdGenerator> {
    Argument,
    Synthetic,
    Types(Vec<ty::Type>),
    Element(Rc<CacheElement<G>>),
}

pub struct Verifier<G: IdGenerator = CountGenerator> {
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
        let x = if let StackElement::Element(x) = self.pop()? {
            x
        } else {
            return Err(OperationError::new("Using app on an invalid element"));
        };
        let f = if let StackElement::Element(f) = self.pop()? {
            f
        } else {
            return Err(OperationError::new("Using app on an invalid element"));
        };
        let el = StackElement::Element(Rc::new(match f.unwrap_one() {
            Element::Universal { body, ty: f_ty } => {
                let ty = f_ty.apply(&x.ty(&mut self.ty_reg))?;
                CacheElement {
                    data: RefCell::new(CacheEnum::Bind {
                        body: body.clone(),
                        arg: x,
                        ty,
                    }),
                    max_ref: f.max_ref,
                }
            }
            Element::Application { ty: f_ty, .. } | Element::Argument { ty: f_ty, .. } => {
                let ty = f_ty.apply(&x.ty(&mut self.ty_reg))?;
                Element::Application {
                    ty: ty,
                    arg: x,
                    body: f,
                }
                .into()
            }
            _ => return Err(OperationError::new("Using app on an invalid element")),
        }));
        self.push(el);
        Ok(())
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
        let mut el = Rc::new(
            Element::Object {
                id: self.obj_id.new(),
                params: (1..=n)
                    .map(|x| {
                        Rc::new(
                            Element::Argument {
                                pos: x.try_into().unwrap(),
                                ty: self.ty_reg.symbol(),
                            }
                            .into(),
                        )
                    })
                    .collect(),
            }
            .into(),
        );
        for _ in 0..n {
            el = Rc::new(
                Element::Universal {
                    ty: self.ty_reg.symbol(),
                    body: el,
                }
                .into(),
            );
        }
        self.sym_table.insert(s, (false, el));
        Ok(())
    }

    fn hkt(&mut self) -> Result<()> {
        let el = s_top(&mut self.stack)?;
        let vec = if let StackElement::Types(vec) = el {
            vec
        } else {
            return Err(OperationError::new("Using hkt without uni"));
        };
        let q = s_pop(vec)?;
        let p = s_pop(vec)?;
        vec.push(self.ty_reg.infer(p, q));
        Ok(())
    }
    fn qed(&mut self) -> Result<()> {
        match self.pop()? {
            StackElement::Argument | StackElement::Synthetic => {
                return Err(OperationError::new("Calling qed without uni"))
            }
            StackElement::Types(vec) => {
                for ty in vec.into_iter() {
                    self.arg_stack.push(ty);
                    self.stack.push(StackElement::Argument);
                }
            }
            StackElement::Element(el) => {
                let param_ty = if let StackElement::Argument = self.pop()? {
                    self.arg_stack.pop().unwrap()
                } else {
                    return Err(OperationError::new("End of proof without an argument"));
                };
                let body_ty = el.ty(&mut self.ty_reg);
                let ty = self.ty_reg.infer(param_ty, body_ty);
                self.stack.push(StackElement::Element(Rc::new(
                    Element::Universal { body: el, ty }.into(),
                )));
            }
        }
        Ok(())
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
        let p = if let StackElement::Element(p) = self.pop()? {
            p
        } else {
            return Err(OperationError::new("Using mp on an invalid element"));
        };
        let pq = if let StackElement::Element(pq) = self.pop()? {
            pq
        } else {
            return Err(OperationError::new("Using mp on an invalid element"));
        };
        let (p_ans, q) = if let Element::Implication(p_ans, q) = pq.unwrap_one() {
            (p_ans, q)
        } else {
            return Err(OperationError::new(
                "Using mp on an non-implication element",
            ));
        };
        if !CacheElement::check_equal(p_ans, &p) {
            return Err(OperationError::new("Using mp but condition not met"));
        }
        self.push(StackElement::Element(q.clone()));
        Ok(())
    }

    fn sat(&mut self) -> Result<()> {
        if !self.is_syn {
            return Err(OperationError::new("Using sat in non-synthetic mode"));
        }
        let el = self.pop();
        let el = if let StackElement::Element(el) = el? {
            el
        } else {
            return Err(OperationError::new("Using sat on an invalid element"));
        };
        let q = if let Element::Implication(_, q) = el.unwrap_one() {
            q
        } else {
            return Err(OperationError::new(
                "Using sat on an non-implication element",
            ));
        };
        self.push(StackElement::Element(q.clone()));
        Ok(())
    }

    fn var(&mut self) -> Result<()> {
        let el = s_top(&mut self.stack)?;
        let vec = if let StackElement::Types(vec) = el {
            vec
        } else {
            return Err(OperationError::new("Using var without uni"));
        };
        vec.push(self.ty_reg.symbol());
        Ok(())
    }
}
