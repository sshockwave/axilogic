mod ty;

use std::{
    cell::{Ref, RefCell},
    cmp::max,
    collections::HashMap,
    num::NonZeroUsize,
    ops::Deref,
    rc::Rc,
};

use crate::{
    err::{OperationError, Result},
    isa::InstructionSet,
    util::{CountGenerator, IdGenerator},
};

enum Element<G: IdGenerator, P> {
    Argument { pos: NonZeroUsize },
    Object { id: G::Id, params: Vec<P> },
    Universal { body: P },
    Application { arg: P, body: P },
}

enum CacheEnum<G: IdGenerator, P> {
    Primitive(Element<G, P>),
    Bind { body: P, arg: P },
    RefShift(P, NonZeroUsize),
}
struct CacheElement<G: IdGenerator> {
    data: RefCell<CacheEnum<G, Rc<Self>>>,
    max_ref: usize,
    ty: ty::Type,
}
impl<G: IdGenerator> CacheElement<G> {
    fn unwrap_one(&self) -> Ref<'_, Element<G, Rc<Self>>> {
        use CacheEnum::*;
        Ref::map(self.data.borrow(), |x| match x {
            Primitive(el) => el,
            Bind { .. } => todo!(),
            RefShift(_, _) => todo!(),
        })
    }
    fn check_equal(a: &Rc<Self>, b: &Rc<Self>) -> bool {
        if Rc::ptr_eq(a, b) {
            return true;
        }
        use Element::*;
        match (a.unwrap_one().deref(), b.unwrap_one().deref()) {
            (Argument { pos: pos1, .. }, Argument { pos: pos2, .. }) => pos1 == pos2,
            (
                Object {
                    id: id1,
                    params: params1,
                },
                Object {
                    id: id2,
                    params: params2,
                },
            ) => {
                if id1 != id2 {
                    return false;
                }
                assert!(params1.len() == params2.len());
                params1
                    .iter()
                    .zip(params2.iter())
                    .all(|(x, y)| Self::check_equal(x, y))
            }
            (Universal { body: body1 }, Universal { body: body2 }) => {
                Self::check_equal(body1, body2)
            }
            (
                Application {
                    arg: arg1,
                    body: body1,
                },
                Application {
                    arg: arg2,
                    body: body2,
                },
            ) => Self::check_equal(arg1, arg2) && Self::check_equal(body1, body2),
            _ => false,
        }
    }

    fn new_primitive(el: Element<G, Rc<Self>>, max_ref: usize, ty: ty::Type) -> Rc<Self> {
        Rc::new(Self {
            data: RefCell::new(CacheEnum::Primitive(el)),
            max_ref,
            ty,
        })
    }

    fn new_argument(pos: NonZeroUsize, ty: ty::Type) -> Rc<Self> {
        Self::new_primitive(Element::Argument { pos }, pos.get(), ty)
    }

    fn new_application(arg: Rc<Self>, body: Rc<Self>) -> Rc<Self> {
        let ty = body.ty.clone();
        let max_ref = max(arg.max_ref, max(body.max_ref, 1) - 1);
        Self::new_primitive(Element::Application { arg, body }, max_ref, ty)
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
    syn_cnt: usize,
    arg_stack: Vec<ty::Type>,
    stack: Vec<StackElement<G>>,
    ty_reg: ty::Registry,
    imply_id: G::Id,
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
    fn init_l1(&mut self) -> Result<()> {
        self.syn()?;
        self.uni()?;
        self.var()?;
        self.var()?;
        self.qed()?;
        {
            self.req("sys::imply".into())?;
            self.syn()?;
            self.arg(2.try_into().unwrap())?;
            self.app()?;
            self.syn()?;
            {
                self.req("sys::imply".into())?;
                self.syn()?;
                self.arg(1.try_into().unwrap())?;
                self.app();
                self.syn()?;
                self.arg(2.try_into().unwrap())?;
                self.app();
            }
            self.app()?;
        }
        self.qed()?;
        self.qed()?;
        let name = "sys::l1";
        self.hyp(name.into())?;
        self.sym_table.get_mut(name).unwrap().0 = true;
        Ok(())
    }
    fn init_l2(&mut self) -> Result<()> {
        todo!()
    }
    fn init_l3(&mut self) -> Result<()> {
        todo!()
    }
    pub fn init_sys(&mut self) -> Result<()> {
        self.obj(1, "sys::not".into())?;
        self.add_obj(1, "sys::imply".into(), self.imply_id.clone())?;
        self.init_l1()?;
        self.init_l2()?;
        self.init_l3()?;
        Ok(())
    }

    pub fn new(mut obj_id: G) -> Self {
        let imply_id = obj_id.new();
        let mut vm = Self {
            obj_id,
            ty_reg: ty::Registry::new(),
            stack: Vec::new(),
            arg_stack: Vec::new(),
            syn_cnt: 0,
            imply_id,
            sym_table: HashMap::new(),
        };
        vm.init_sys().unwrap();
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

    fn add_sym(&mut self, s: String, is_real: bool, el: Rc<CacheElement<G>>) -> Result<()> {
        if let Some(_) = self.sym_table.insert(s, (is_real, el)) {
            return Err(OperationError::new("Symbol already exists"));
        }
        Ok(())
    }

    fn add_obj(&mut self, n: usize, s: String, id: G::Id) -> Result<()> {
        let arr = (1..=n)
            .rev()
            .map(|x| CacheElement::new_argument(x.try_into().unwrap(), self.ty_reg.symbol()))
            .collect();
        let mut el = self.new_object(id, arr);
        for _ in 0..n {
            el = self.new_universal(el);
        }
        self.add_sym(s, false, el)?;
        Ok(())
    }

    fn pop_imply(&mut self) -> Result<(Rc<CacheElement<G>>, Rc<CacheElement<G>>)> {
        let pq = if let StackElement::Element(pq) = self.pop()? {
            pq
        } else {
            return Err(OperationError::new("Imply statement not found"));
        };
        let pq = pq.unwrap_one();
        if let Element::Object { id, params } = pq.deref() {
            if id != &self.imply_id {
                return Err(OperationError::new("Object is not imply"));
            }
            assert!(params.len() == 2);
            Ok(match &params[..] {
                [a, b] => (a.clone(), b.clone()),
                _ => unreachable!(),
            })
        } else {
            Err(OperationError::new("Not imply object"))
        }
    }

    fn new_object(&mut self, id: G::Id, params: Vec<Rc<CacheElement<G>>>) -> Rc<CacheElement<G>> {
        let max_ref = params.iter().map(|x| x.max_ref).max().unwrap_or(0);
        CacheElement::new_primitive(
            Element::Object { id, params },
            max_ref,
            self.ty_reg.symbol(),
        )
    }

    fn new_universal(&mut self, body: Rc<CacheElement<G>>) -> Rc<CacheElement<G>> {
        let sym = self.ty_reg.symbol();
        let ty = self.ty_reg.infer(sym, body.ty.clone());
        let max_ref = max(body.max_ref, 1) - 1;
        CacheElement::new_primitive(Element::Universal { body: body }, max_ref, ty)
    }
}

impl<G: IdGenerator> super::isa::InstructionSet for Verifier<G> {
    fn syn(&mut self) -> Result<()> {
        self.syn_cnt += 1;
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
        let ty = f.ty.apply(&x.ty)?;
        let f_el = f.unwrap_one();
        let f_ref = f_el.deref();
        let el = StackElement::Element(match f_ref {
            Element::Universal { body } => {
                let max_ref = max(x.max_ref, f.max_ref);
                Rc::new(CacheElement {
                    data: RefCell::new(CacheEnum::Bind {
                        body: body.clone(),
                        arg: x,
                    }),
                    max_ref,
                    ty,
                })
            }
            Element::Application { .. } | Element::Argument { .. } => {
                std::mem::drop(f_el);
                CacheElement::new_application(x, f)
            }
            _ => return Err(OperationError::new("Using app on an invalid element")),
        });
        self.push(el);
        Ok(())
    }

    fn arg(&mut self, n: NonZeroUsize) -> Result<()> {
        if self.syn_cnt == 0 {
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
        self.push(StackElement::Element(CacheElement::new_argument(
            n,
            self.arg_stack[self.arg_stack.len() - n.get()].clone(),
        )));
        Ok(())
    }

    fn uni(&mut self) -> Result<()> {
        self.push(StackElement::Types(Vec::new()));
        Ok(())
    }

    fn def(&mut self, s: String) -> Result<()> {
        if self.syn_cnt > 0 {
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
        self.add_sym(s, true, el)?;
        Ok(())
    }

    fn hyp(&mut self, s: String) -> Result<()> {
        let el = if let StackElement::Element(el) = self.pop()? {
            el
        } else {
            return Err(OperationError::new("Exporting an invalid element"));
        };
        if let StackElement::Synthetic = self.pop()? {
            self.syn_cnt -= 1;
        } else {
            return Err(OperationError::new(
                "Exporting an element in non-synthetic mode",
            ));
        }
        if el.max_ref != 0 {
            return Err(OperationError::new("Exporting an unbounded element"));
        }
        self.add_sym(s, false, el)?;
        Ok(())
    }

    fn obj(&mut self, n: usize, s: String) -> Result<()> {
        let id = self.obj_id.new();
        self.add_obj(n, s, id)
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
                if let StackElement::Argument = self.pop()? {
                    self.arg_stack.pop().unwrap();
                } else {
                    return Err(OperationError::new("End of proof without an argument"));
                };
                let el = StackElement::Element(self.new_universal(el));
                self.stack.push(el);
            }
        }
        Ok(())
    }

    fn req(&mut self, s: String) -> Result<()> {
        let (is_real, el) = self
            .sym_table
            .get(&s)
            .ok_or_else(|| OperationError::new(format!("Symbol not found: {}", s)))?;
        if !is_real && self.syn_cnt == 0 {
            return Err(OperationError::new(format!(
                "Using imaginary symbol {} in imaginary mode",
                s
            )));
        }
        self.push(StackElement::Element(el.clone()));
        Ok(())
    }

    fn mp(&mut self) -> Result<()> {
        if self.syn_cnt > 0 {
            return Err(OperationError::new(
                "Use sat instead of mp in synthetic mode",
            ));
        }
        let p = if let StackElement::Element(p) = self.pop()? {
            p
        } else {
            return Err(OperationError::new("Using mp on an invalid element"));
        };
        let (p_ans, q) = self.pop_imply()?;
        if !CacheElement::check_equal(&p_ans, &p) {
            return Err(OperationError::new("Using mp but condition not met"));
        }
        self.push(StackElement::Element(q));
        Ok(())
    }

    fn sat(&mut self) -> Result<()> {
        if self.syn_cnt == 0 {
            return Err(OperationError::new("Using sat in non-synthetic mode"));
        }
        let (_, q) = self.pop_imply()?;
        self.push(StackElement::Element(q));
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
