use std::{rc::Rc, vec::Vec};

use super::{ds, isa::{ISA, OperationError}};

type Env = ds::SkipList<usize, Term>;

pub struct Engine {
    stack: Vec<Term>,
    num_symbols: usize,
    num_concepts: usize,
    assume_height: Option<usize>, // stack[assume_height:] is falsy
}

pub enum TermEnum {
    Symbol(usize),
    SymbolRef(usize),
    Assumption(Term),
    Forall {
        var: usize,
        expr: Term,
    },
    Imply(Term, Term),
    Concept {
        id: usize,
        val: Option<(Term, Term)>, // (cur_val, rest)
    },
    Closure(Term, Env),
}
use TermEnum::*;

#[derive(Clone)]
pub struct Term(Rc<TermEnum>);

impl Term {
    fn is_movable(&self) -> bool {
        match self.0.as_ref() {
            Symbol(_) | Assumption(_) => false,
            _ => true,
        }
    }
    fn unwrap_closure(&self) -> Self {
        if let Closure(expr, env) = self.0.as_ref() {
            match expr.0.as_ref() {
                Symbol(_) | Assumption(_) => panic!("Closure should not contain non-movable terms"),
                SymbolRef(id) => env.get(id).map(Self::unwrap_closure).unwrap_or_else(|| expr.clone()),
                Forall { var, expr } => Self::from(Forall {
                    var: var.clone(),
                    expr: Term::from(Closure(expr.clone(), env.del(var))),
                }),
                Imply(p, q) => Term::from(Imply(
                    Term::from(Closure(p.clone(), env.clone())),
                    Term::from(Closure(q.clone(), env.clone())),
                )),
                Closure(expr, inner_env) => {
                    let mut new_env = env.clone();
                    // TODO: boost by merging the smaller one to the larger
                    for (k, v) in inner_env {
                        new_env = new_env.add(k, v);
                    }
                    Self::unwrap_closure(&Term::from(Closure(expr.clone(), new_env)))
                },
                Concept { id, val} => Term::from(Concept {
                    id: *id,
                    val: val.as_ref().map(|x| (
                        Term::from(Closure(x.0.clone(), env.clone())),
                        Term::from(Closure(x.1.clone(), env.clone())),
                    )),
                }),
            }
        } else {
            self.clone()
        }
    }
    fn get_enum(&self) -> &TermEnum {
        self.0.as_ref()
    }
    fn shallow_eq(a: &Term, b: &Term) -> bool {
        if Rc::ptr_eq(&a.0, &b.0) {
            return true;
        }
        match (a.get_enum(), b.get_enum()) {
            (Symbol(a), Symbol(b)) => a == b,
            (SymbolRef(a), SymbolRef(b)) => a == b,
            _ => false,
        }
    }
    fn deep_eq(a: &Term, b: &Term) -> bool {
        // TODO: implement strong deep equal
        Self::shallow_eq(a, b)
    }
}

impl From<TermEnum> for Term {
    fn from(v: TermEnum) -> Term {
        Term(Rc::new(v))
    }
}

type Result<T> = std::result::Result<T, OperationError>;


impl ISA for Engine {
    type Term = Term;
    fn push(&mut self, n: isize) -> Result<()> {
        let idx = if n < 0 { self.stack.len() - ((-n) as usize) } else { n as usize };
        if idx > self.stack.len() {
            return Err(OperationError::new("Push index longer than stack"));
        }
        let el = self.stack[idx].clone();
        let new_el = match el.get_enum() {
            Symbol(d) => Term::from(SymbolRef(d.clone())),
            Assumption(v) => v.clone(),
            _ => el.clone(),
        };
        self.stack.push(if Term::shallow_eq(&new_el, &el) { el.clone() } else { new_el });
        Ok(())
    }

    fn pop(&mut self) -> Result<()> {
        let el = if let Some(v) = self.stack.pop() { v } else {
            return Err(OperationError::new("Cannot pop on empty stack"));
        };
        if !el.is_movable() {
            return Err(OperationError::new("Stack top is not a movable element"));
        }
        if let Some(v) = self.assume_height {
            if self.stack.len() < v {
                self.assume_height = None;
            }
        }
        Ok(())
    }

    fn swap(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(OperationError::new("Cannot swap stack with less than two elements"))
        }
        let a = self.stack.pop().unwrap();
        let b = self.stack.pop().unwrap();
        if !(a.is_movable() && b.is_movable()) {
            return Err(OperationError::new("Cannot swap unmovable elements"))
        }
        self.stack.push(a);
        self.stack.push(b);
        Ok(())
    }

    fn symbol(&mut self) -> Result<()> {
        self.num_symbols += 1;
        self.stack.push(Term::from(Symbol(self.num_symbols)));
        Ok(())
    }

    fn forall(&mut self) -> Result<()> {
        let l = self.stack.len();
        if l < 2 {
            return Err(OperationError::new("Stack needs to contain at least two elements"));
        }
        let expr = self.stack.pop().unwrap();
        let sym = self.stack.pop().unwrap();
        if !expr.is_movable() {
            return Err(OperationError::new("Cannot use movable element as expression"));
        }
        self.stack.push(match sym.get_enum() {
            Symbol(d) => Term::from(Forall { var: *d, expr }),
            _ => {
                return Err(OperationError::new("Cannot use movable element as expression"));
            }
        });
        Ok(())
    }

    fn apply(&mut self) -> Result<()> {
        let l = self.stack.len();
        if l < 2 {
            return Err(OperationError::new("Stack needs to contain at least two elements"));
        }
        let param = self.stack.pop().unwrap();
        let func = self.stack.pop().unwrap();
        if !param.is_movable() {
            return Err(OperationError::new("Cannot use movable element as parameter"));
        }
        self.stack.push(match func.unwrap_closure().get_enum() {
            Forall { var, expr } => Term::from(Closure (
                // TODO: boost by testing whether underlying expr is a closure
                expr.clone(),
                Env::new().add(*var, param),
            )),
            Imply(p, q) => if Term::deep_eq(&param, p) {
                q.clone()
            } else {
                return Err(OperationError::new("Not deep equal when applying antecedent"));
            },
            _ => {
                return Err(OperationError::new("Only implication or function is appliable"));
            }
        });
        Ok(())
    }

    fn abs(&mut self) -> Result<()> {
        let l = self.stack.len();
        if l < 2 {
            return Err(OperationError::new("Stack needs to contain at least two elements"));
        }
        let p = self.stack.pop().unwrap();
        let q = self.stack.pop().unwrap();
        if !q.is_movable() {
            return Err(OperationError::new("Cannot use movable element as condition"));
        }
        if let Assumption(expr) = p.get_enum() {
            self.stack.push(Term::from(Imply(expr.clone(), q)));
        } else {
            return Err(OperationError::new("Only assumptions can be used as antecedent"));
        }
        Ok(())
    }

    fn express(&mut self) -> Result<()> {
        if let Some(_) = self.assume_height {
            return Err(OperationError::new("Already in assumption mode"));
        }
        self.assume_height = Some(self.stack.len());
        Ok(())
    }

    fn assume(&mut self) -> Result<()> {
        if let Some(x) = self.stack.pop() {
            if !x.is_movable() {
                return Err(OperationError::new("Non-movable expression cannot be assumed"));
            }
            if let Some(h) = self.assume_height {
                if self.stack.len() == h {
                    self.assume_height = None;
                }
                self.stack.push(Term::from(Assumption(x)));
                Ok(())
            } else {
                Err(OperationError::new("Cannot assume in normal mode"))
            }
        } else {
            Err(OperationError::new("Nothing to assume"))
        }
    }

    fn trust(&mut self) -> Result<()> {
        if let Some(x) = self.stack.pop() {
            if !x.is_movable() {
                return Err(OperationError::new("Non-movable expression cannot be assumed"));
            }
            if self.is_normal_mode() {
                return Err(OperationError::new("Cannot trust in normal mode"));
            }
            if let Imply(_, q) = x.unwrap_closure().get_enum() {
                self.stack.push(q.clone());
                Ok(())
            } else {
                Err(OperationError::new("Only implications can be trusted"))
            }
        } else {
            Err(OperationError::new("Nothing to trust"))
        }
    }

    fn export(&mut self) -> Result<(Self::Term, bool)> {
        if let Some(x) = self.stack.last() {
            if !x.is_movable() {
                return Err(OperationError::new("Only movable items can be exported"))
            }
            Ok((self.wrap_env(x.clone()), self.is_normal_mode()))
        } else {
            Err(OperationError::new("Nothing to export"))
        }
    }

    fn concept(&mut self) -> Result<(Self::Term, bool)> {
        self.num_concepts += 1;
        let id = self.num_concepts;
        let mut val = None;
        for t in self.stack.iter() {
            if let Assumption(p) =  t.get_enum() {
                val = Some((p.clone(), Term::from(Concept { id, val })));
            }
        }
        Ok((self.wrap_env(Term::from(Concept{ id, val })), self.is_normal_mode()))
    }

    fn refer(&mut self, term: Self::Term, truthy: bool) -> Result<()> {
        if self.is_normal_mode() && !truthy {
            return Err(OperationError::new("Falsy values cannot be used in normal mode"));
        }
        self.stack.push(term);
        Ok(())
    }

    fn unbind(&mut self) -> Result<()> {
        let x = if let Some(x) = self.stack.pop() { x } else {
            return Err(OperationError::new("Nothing to unbind"));
        }.unwrap_closure();
        let val = if let Concept { val, .. } = x.get_enum() { val } else {
            return Err(OperationError::new("Only concepts can be unbinded"));
        };
        let (cur, rest) = if let Some(v) = val { v } else {
            return Err(OperationError::new("Concept is already empty"));
        };
        self.stack.push(rest.clone());
        self.stack.push(cur.clone());
        Ok(())
    }
}

impl Engine {
    pub fn new() -> Engine {
        Engine { stack: Vec::new(), num_symbols: 0, num_concepts: 0, assume_height: None }
    }
    fn wrap_env(&self, mut ans: Term) -> Term {
        for t in self.stack.iter().rev() {
            match t.get_enum() {
                Symbol(var) => ans = Term::from(Forall { var: *var, expr: ans }),
                Assumption(p) => ans = Term::from(Imply(p.clone(), ans)),
                _ => (),
            }
        }
        ans
    }
    fn is_normal_mode(&self) -> bool {
        matches!(self.assume_height, None)
    }
}
