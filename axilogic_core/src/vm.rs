mod ty;

use std::{
    cell::{Ref, RefCell},
    cmp::max,
    collections::HashMap,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    err::{OperationError, Result},
    isa::InstructionSet,
    kit::{imply, not, Expression, Forall},
    util::{defer, vec_rev_get, CountGenerator, IdGenerator},
};

enum Element<G: IdGenerator, P: Clone> {
    Argument { pos: NonZeroUsize },
    Object { id: G::Id, args: Vec<P> },
    Universal { body: P },
    Application { arg: P, body: P },
}

impl<G: IdGenerator, P: Clone> Clone for Element<G, P> {
    fn clone(&self) -> Self {
        match self {
            Self::Argument { pos } => Self::Argument { pos: *pos },
            Self::Object { id, args: params } => Self::Object {
                id: id.clone(),
                args: params.clone(),
            },
            Self::Universal { body } => Self::Universal { body: body.clone() },
            Self::Application { arg, body } => Self::Application {
                arg: arg.clone(),
                body: body.clone(),
            },
        }
    }
}

enum CacheEnum<G: IdGenerator, P: Clone> {
    Primitive(Element<G, P>),
    Bind { func: P, arg: P },
    RefShift(P, NonZeroUsize),
}

impl<G: IdGenerator, P: Clone> Clone for CacheEnum<G, P> {
    fn clone(&self) -> Self {
        match self {
            Self::Primitive(el) => Self::Primitive(el.clone()),
            Self::Bind { func, arg } => Self::Bind {
                func: func.clone(),
                arg: arg.clone(),
            },
            Self::RefShift(el, shift) => Self::RefShift(el.clone(), *shift),
        }
    }
}

struct TypedElement<G: IdGenerator> {
    data: RefCell<CacheEnum<G, Rc<Self>>>,
    max_ref: usize,
    ty: ty::Type,
}

fn max_ref_shift(max_ref: usize, delta: usize) -> usize {
    if max_ref > 0 {
        max_ref + delta
    } else {
        0
    }
}

struct CacheFlusher<'a, G: IdGenerator> {
    arg_stack: Vec<(Option<Rc<TypedElement<G>>>, usize)>,
    bind_stack: Vec<Rc<TypedElement<G>>>,
    ref_shift: usize,
    ty_reg: &'a mut ty::Registry,
}

impl<'a, G: IdGenerator> CacheFlusher<'a, G> {
    fn new(ty_reg: &'a mut ty::Registry) -> Self {
        Self {
            arg_stack: Vec::new(),
            bind_stack: Vec::new(),
            ref_shift: 0,
            ty_reg,
        }
    }

    fn flush_enum(&mut self, data: &CacheEnum<G, Rc<TypedElement<G>>>) -> Option<TypedElement<G>> {
        use CacheEnum::*;
        use Element::*;
        match data {
            Primitive(Argument { pos }) => {
                todo!()
            }
            Primitive(Object { id, args: params }) => {
                let results: Vec<_> = params.iter().map(|x| self.flush_ptr(x)).collect();
                if results.iter().all(|x| x.is_none()) {
                    return None;
                }
                let params = results
                    .into_iter()
                    .zip(params.iter())
                    .map(|(x, orig)| x.map_or_else(|| orig.clone(), |x| Rc::new(x)))
                    .collect();
                Some(new_object(&mut self.ty_reg, id.clone(), params))
            }
            Primitive(Universal { body }) => {
                self.arg_stack.push((None, self.tot_bind_cnt()));
                let _ = defer(|| self.arg_stack.pop().unwrap());
                self.flush_ptr(body)
            }
            Primitive(Application { arg, body }) => {
                let arg = self
                    .flush_ptr(arg)
                    .map_or_else(|| arg.clone(), |x| Rc::new(x));
                self.arg_stack.push((Some(arg), self.tot_bind_cnt() + 1));
                let _ = defer(|| self.arg_stack.pop().unwrap());
                self.flush_ptr(body)
            }
            Bind { func, arg } => {
                let arg = self
                    .flush_ptr(arg)
                    .map_or_else(|| arg.clone(), |x| Rc::new(x));
                self.bind_stack.push(arg);
                let _ = defer(|| self.bind_stack.pop().unwrap());
                self.flush_ptr(func)
            }
            RefShift(el, delta) => {
                self.ref_shift += delta.get();
                let _ = defer(|| self.ref_shift -= delta.get());
                self.flush_ptr(el)
            }
        }
    }

    fn tot_bind_cnt(&self) -> usize {
        self.arg_stack.last().map_or(0, |x| x.1)
    }

    fn calc_new_ref(&self, pos: usize) -> usize {
        let pos = max_ref_shift(pos, self.ref_shift);
        let binded_cnt = vec_rev_get(&self.arg_stack, pos + 1).map_or(0, |x| x.1);
        pos - (self.tot_bind_cnt() - binded_cnt)
    }

    fn flush_ptr(&mut self, ptr: &Rc<TypedElement<G>>) -> Option<TypedElement<G>> {
        if self.bind_stack.is_empty() && ptr.max_ref == self.calc_new_ref(ptr.max_ref) {
            return ptr.set_shift(self.ref_shift);
        }
        let data = ptr.data.borrow();
        self.flush_enum(data.deref())
    }
}

impl<'a, G: IdGenerator> Drop for CacheFlusher<'a, G> {
    fn drop(&mut self) {
        assert!(self.arg_stack.is_empty());
        assert!(self.bind_stack.is_empty());
        assert!(self.ref_shift == 0);
    }
}

impl<G: IdGenerator> TypedElement<G> {
    fn set_shift(self: &Rc<Self>, v: usize) -> Option<Self> {
        if let Ok(v) = v.try_into() {
            let v: NonZeroUsize = v;
            let data = self.data.borrow();
            let (p, v) = match data.deref() {
                CacheEnum::RefShift(p, delta) => (p, v.saturating_add(delta.get())),
                _ => (self, v),
            };
            Some(TypedElement {
                data: RefCell::new(CacheEnum::RefShift(p.clone(), v)),
                max_ref: max_ref_shift(p.max_ref, v.get()),
                ty: p.ty.clone(),
            })
        } else {
            None
        }
    }

    fn unwrap_one<'a>(
        self: &'a Rc<Self>,
        ty_reg: &mut ty::Registry,
    ) -> Ref<'a, Element<G, Rc<Self>>> {
        use CacheEnum::*;
        let mut data_mut = self.data.borrow_mut();
        match data_mut.deref() {
            Primitive(..) => (),
            _ => {
                let el = CacheFlusher::new(ty_reg).flush_enum(data_mut.deref_mut());
                if let Some(el) = el {
                    assert_eq!(el.ty, self.ty);
                    assert_eq!(el.max_ref, self.max_ref);
                    *data_mut = el.data.into_inner();
                }
            }
        }
        Ref::map(self.data.borrow(), |x| match x {
            Primitive(el) => el,
            _ => unreachable!(),
        })
    }

    fn check_equal(a: &Rc<Self>, b: &Rc<Self>, ty_reg: &mut ty::Registry) -> bool {
        if Rc::ptr_eq(a, b) {
            return true;
        }
        use Element::*;
        match (a.unwrap_one(ty_reg).deref(), b.unwrap_one(ty_reg).deref()) {
            (Argument { pos: pos1, .. }, Argument { pos: pos2, .. }) => pos1 == pos2,
            (
                Object {
                    id: id1,
                    args: params1,
                },
                Object {
                    id: id2,
                    args: params2,
                },
            ) => {
                if id1 != id2 {
                    return false;
                }
                assert!(params1.len() == params2.len());
                params1
                    .iter()
                    .zip(params2.iter())
                    .all(|(x, y)| Self::check_equal(x, y, ty_reg))
            }
            (Universal { body: body1 }, Universal { body: body2 }) => {
                Self::check_equal(body1, body2, ty_reg)
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
            ) => Self::check_equal(arg1, arg2, ty_reg) && Self::check_equal(body1, body2, ty_reg),
            _ => false,
        }
    }

    fn new_primitive(el: Element<G, Rc<Self>>, ty: ty::Type) -> Self {
        use Element::*;
        let max_ref = match &el {
            Argument { pos } => pos.get(),
            Object { args: params, .. } => params.iter().map(|x| x.max_ref).max().unwrap_or(0),
            Universal { body } => max(body.max_ref, 1) - 1,
            Application { arg, body } => max(arg.max_ref, max(body.max_ref, 1) - 1),
        };
        Self {
            data: RefCell::new(CacheEnum::Primitive(el)),
            ty,
            max_ref,
        }
    }

    fn new_argument(pos: NonZeroUsize, ty: ty::Type) -> Rc<Self> {
        Rc::new(Self::new_primitive(Element::Argument { pos }, ty))
    }

    fn new_application(arg: Rc<Self>, body: Rc<Self>) -> Rc<Self> {
        let ty = body.ty.clone();
        Rc::new(Self::new_primitive(Element::Application { arg, body }, ty))
    }

    fn new_bind(self: Rc<Self>, arg: Rc<Self>) -> Result<Self> {
        let ty = self.ty.apply(&arg.ty)?;
        let max_ref = max(self.max_ref, arg.max_ref);
        Ok(Self {
            data: RefCell::new(CacheEnum::Bind { func: self, arg }),
            max_ref,
            ty,
        })
    }
}

enum StackElement<G: IdGenerator> {
    Argument,
    Synthetic,
    Types(Vec<ty::Type>),
    Element(Rc<TypedElement<G>>),
}

pub struct Verifier<G: IdGenerator = CountGenerator> {
    obj_id: G,
    syn_cnt: usize,
    arg_stack: Vec<ty::Type>,
    stack: Vec<StackElement<G>>,
    ty_reg: ty::Registry,
    imply_id: G::Id,
    sym_table: HashMap<String, (bool, Rc<TypedElement<G>>)>, // is_real, element
}

fn s_pop<T>(s: &mut Vec<T>) -> Result<T> {
    match s.pop() {
        Some(x) => Ok(x),
        None => Err(OperationError::new("Stack underflow")),
    }
}

impl<G: IdGenerator> Verifier<G> {
    fn set_real(&mut self, name: &str) {
        self.sym_table.get_mut(name).unwrap().0 = true;
    }

    fn init_l1(&mut self) -> Result<()> {
        let name = "sys::l1";
        Forall::new(2, |args| {
            let a = args[0];
            let b = args[1];
            imply(a.into(), imply(b.into(), a.into()))
        })
        .export(self, name.into(), false)?;
        self.set_real(name);
        Ok(())
    }

    fn init_l2(&mut self) -> Result<()> {
        let name = "sys::l2";
        Forall::new(3, |args| {
            let a = args[0];
            let b = args[1];
            let c = args[2];
            imply(
                imply(a.into(), imply(b.into(), c.into())),
                imply(imply(a.into(), b.into()), imply(a.into(), c.into())),
            )
        })
        .export(self, name.into(), false)?;
        self.set_real(name);
        Ok(())
    }

    fn init_l3(&mut self) -> Result<()> {
        let name = "sys::l3";
        Forall::new(2, |args| {
            let a = args[0];
            let b = args[1];
            imply(
                imply(not(a.into()), not(b.into())),
                imply(b.into(), a.into()),
            )
        })
        .export(self, name.into(), false)?;
        self.set_real(name);
        Ok(())
    }

    fn init_sys(&mut self) -> Result<()> {
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

    pub fn has(&self, s: &str) -> bool {
        self.sym_table.contains_key(s)
    }

    fn push(&mut self, el: StackElement<G>) {
        self.stack.push(el)
    }

    fn pop(&mut self) -> Result<StackElement<G>> {
        s_pop(&mut self.stack)
    }

    fn add_sym(&mut self, s: String, is_real: bool, el: Rc<TypedElement<G>>) -> Result<()> {
        if let Some(_) = self.sym_table.insert(s, (is_real, el)) {
            return Err(OperationError::new("Symbol already exists"));
        }
        Ok(())
    }

    fn add_obj(&mut self, n: usize, s: String, id: G::Id) -> Result<()> {
        let arr = (1..=n)
            .rev()
            .map(|x| TypedElement::new_argument(x.try_into().unwrap(), self.ty_reg.symbol()))
            .collect();
        let mut el = Rc::new(new_object(&mut self.ty_reg, id, arr));
        for _ in 0..n {
            el = self.new_universal(el);
        }
        self.add_sym(s, false, el)?;
        Ok(())
    }

    fn pop_element(&mut self) -> Result<Rc<TypedElement<G>>> {
        if let StackElement::Element(el) = self.pop()? {
            Ok(el)
        } else {
            Err(OperationError::new("Expected element on stack top"))
        }
    }

    fn pop_syn(&mut self) -> Result<()> {
        if let StackElement::Synthetic = self.pop()? {
            self.syn_cnt -= 1;
            Ok(())
        } else {
            return Err(OperationError::new(
                "Exporting an element in non-synthetic mode",
            ));
        }
    }

    fn pop_imply(&mut self) -> Result<(Rc<TypedElement<G>>, Rc<TypedElement<G>>)> {
        let el = self.pop_element()?;
        let data = el.unwrap_one(&mut self.ty_reg);
        if let Element::Object { id, args: params } = data.deref() {
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

    fn expect_syn(&mut self) -> Result<()> {
        if self.syn_cnt == 0 {
            return Err(OperationError::new("Expected synthetic mode"));
        }
        Ok(())
    }

    fn expect_real(&mut self) -> Result<()> {
        if self.syn_cnt > 0 {
            return Err(OperationError::new("Expected non-synthetic mode"));
        }
        Ok(())
    }

    fn peek_types(&mut self) -> Result<(&mut Vec<ty::Type>, &mut ty::Registry)> {
        match self.stack.last_mut() {
            Some(StackElement::Types(vec)) => Ok((vec, &mut self.ty_reg)),
            _ => Err(OperationError::new("Expected types on stack top")),
        }
    }

    fn new_universal(&mut self, body: Rc<TypedElement<G>>) -> Rc<TypedElement<G>> {
        let sym = self.ty_reg.symbol();
        let ty = self.ty_reg.infer(sym, body.ty.clone());
        Rc::new(TypedElement::new_primitive(
            Element::Universal { body: body },
            ty,
        ))
    }
}

fn new_object<G: IdGenerator>(
    ty_reg: &mut ty::Registry,
    id: G::Id,
    params: Vec<Rc<TypedElement<G>>>,
) -> TypedElement<G> {
    TypedElement::new_primitive(Element::Object { id, args: params }, ty_reg.symbol())
}

impl<G: IdGenerator> super::isa::InstructionSet for Verifier<G> {
    fn syn(&mut self) -> Result<()> {
        self.syn_cnt += 1;
        self.stack.push(StackElement::Synthetic);
        Ok(())
    }

    fn app(&mut self) -> Result<()> {
        let x = self.pop_element()?;
        self.pop_syn()?;
        let f = self.pop_element()?;
        self.push(StackElement::Element(Rc::new(TypedElement::new_bind(
            f, x,
        )?)));
        Ok(())
    }

    fn arg(&mut self, n: NonZeroUsize) -> Result<()> {
        self.expect_syn()?;
        self.push(StackElement::Element(TypedElement::new_argument(
            n,
            vec_rev_get(&self.arg_stack, n.get())
                .ok_or_else(|| {
                    OperationError::new(format!("Argument index out of range: {}", n.get()))
                })?
                .clone(),
        )));
        Ok(())
    }

    fn uni(&mut self) -> Result<()> {
        self.push(StackElement::Types(Vec::new()));
        Ok(())
    }

    fn def(&mut self, s: String) -> Result<()> {
        self.expect_real()?;
        let el = self.pop_element()?;
        if el.max_ref != 0 {
            return Err(OperationError::new("Exporting an unbounded element"));
        }
        self.add_sym(s, true, el)?;
        Ok(())
    }

    fn hyp(&mut self, s: String) -> Result<()> {
        let el = self.pop_element()?;
        self.pop_syn()?;
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
        let (vec, reg) = self.peek_types()?;
        let q = s_pop(vec)?;
        let p = s_pop(vec)?;
        vec.push(reg.infer(p, q));
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

    fn req(&mut self, s: &str) -> Result<()> {
        let (is_real, el) = self
            .sym_table
            .get(s)
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
        self.expect_syn()?;
        let p = self.pop_element()?;
        let (p_ans, q) = self.pop_imply()?;
        if !TypedElement::check_equal(&p_ans, &p, &mut self.ty_reg) {
            return Err(OperationError::new("Using mp but condition not met"));
        }
        self.push(StackElement::Element(q));
        Ok(())
    }

    fn sat(&mut self) -> Result<()> {
        self.expect_real()?;
        let (_, q) = self.pop_imply()?;
        self.push(StackElement::Element(q));
        Ok(())
    }

    fn var(&mut self) -> Result<()> {
        let (vec, reg) = self.peek_types()?;
        vec.push(reg.symbol());
        Ok(())
    }
}
