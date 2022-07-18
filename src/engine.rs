use std::{rc::Rc, vec::Vec, iter, fmt};

use super::{ds, isa::{ISA, OperationError}};

type Env = ds::SkipList<usize, Term>;

pub struct Engine {
    stack: Vec<Term>,
    num_symbols: usize,
    num_concepts: usize,
    num_assum: usize,
}

pub enum TermEnum {
    Symbol(usize),
    SymbolRef(usize),
    Assumption(Term),
    Express,
    Forall {
        var: usize,
        expr: Term,
    },
    Imply(Term, Term),
    Concept{
        id: usize,
        vars: Vec<Term>,
        defs: Vec<Term>,
        loop_ptr: usize,
    },
    Closure(Term, Env),
}
use TermEnum::*;

#[derive(Clone)]
pub struct Term(Rc<TermEnum>);

fn vec2str<T: ToString>(v: &Vec<T>) -> String {
    v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", ")
}

impl Term {
    fn is_movable(&self) -> bool {
        match self.get_enum() {
            Symbol(_) | Assumption(_) | Express => false,
            _ => true,
        }
    }
    fn unwrap_closure(&self) -> Self {
        if let Closure(expr, env) = self.get_enum() {
            match expr.get_enum() {
                Symbol(_) | Assumption(_) | Express => panic!("Closure should not contain non-movable terms"),
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
                        new_env = new_env.add(k, Term::from(Closure(v, env.clone())));
                    }
                    Self::unwrap_closure(&Term::from(Closure(expr.clone(), new_env)))
                },
                Concept { id, vars, defs, loop_ptr} => {
                    let mut vars2 = Vec::with_capacity(vars.len());
                    let mut defs2 = Vec::with_capacity(defs.len());
                    for k in vars {
                        vars2.push(Term::from(Closure(k.clone(), env.clone())));
                    }
                    for t in defs {
                        defs2.push(Term::from(Closure(t.clone(), env.clone())));
                    }
                    Term::from(Concept { id: *id, vars: vars2, defs: defs2, loop_ptr: *loop_ptr })
                }
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
            (Express, Express) => true,
            _ => false,
        }
    }
    fn remove_predicates(self: &Term) -> Term {
        match self.unwrap_closure().get_enum() {
            Symbol(_) | Assumption(_) | Express => panic!("Closure should not contain non-movable terms"),
            Closure(..) => panic!("Closure should have been removed"),
            SymbolRef(_) | Concept {..} => self.clone(),
            Forall { var, expr } => Self::from(Forall {
                var: *var,
                expr: expr.remove_predicates(),
            }),
            Imply(_, q) => q.remove_predicates(),
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self,f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self.unwrap_closure().get_enum() {
            Symbol(t) | SymbolRef(t) => t.to_string(),
            Assumption(t) => format!("({t})=>"),
            Express => "σ".to_string(),
            Forall {var, expr} => format!("(∀{var})({expr})"),
            Imply(t1, t2) => format!("({t1})=>({t2})"),
            Concept {id, vars, ..} =>
                format!("Concept {id} [{}]", vec2str(vars)),
            Closure(..) => panic!(),
        };
        write!(f,"{s}")
    }
}

impl From<TermEnum> for Term {
    fn from(v: TermEnum) -> Term {
        Term(Rc::new(v))
    }
}

type Result<T> = std::result::Result<T, OperationError>;

impl fmt::Display for Engine {
    fn fmt(&self,f: &mut fmt::Formatter) -> fmt::Result {
        for x in self.stack.iter().rev() {
            write!(f,"{x}\n")?;
        }
        Ok(())
    }
}

impl ISA for Engine {
    type Term = Term;
    fn print(&self) -> Result<()> {
        if self.is_normal_mode() {
            print!("[Norm]");
        } else {
            print!("[Expr]");
        }
        match self.stack.last() {
            Some(v) => {
                println!("{}",v);
            }
            None => {
                println!("Empty stack");
            }
        }
        Ok(())
    }
    fn push(&mut self, n: isize) -> Result<()> {
        let idx = if n < 0 { self.stack.len() - ((-n) as usize) } else { n as usize };
        if idx > self.stack.len() {
            return Err(OperationError::new("Push index longer than stack"));
        }
        let el = self.stack[idx].clone();
        let new_el = match el.get_enum() {
            Symbol(d) => if self.is_normal_mode() {
                return Err(OperationError::new("symbols cannot be used in normal mode"));
            } else {
                Term::from(SymbolRef(d.clone()))
            },
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
        if let Express = el.get_enum() {
            assert!(self.num_assum > 0);
            self.num_assum -= 1;
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
            return Err(OperationError::new("Cannot use non-movable element as expression"));
        }
        let id = if let Symbol(d) = sym.get_enum() { d } else {
            return Err(OperationError::new("Cannot use movable element as variable"));
        };
        let func = make_forall(&mut self.num_symbols, *id, expr);
        self.stack.push(func);
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
            return Err(OperationError::new("Cannot use non-movable element as parameter"));
        }
        let el = match func.unwrap_closure().get_enum() {
            Forall { var, expr } => Term::from(Closure (
                // TODO: boost by testing whether underlying expr is a closure
                expr.clone(),
                Env::new().add(*var, param),
            )),
            Imply(p, q) => if self.deep_eq(&param, p) {
                q.clone()
            } else {
                return Err(OperationError::new("Not deep equal when applying antecedent"));
            },
            Express => {
                let func = if let Some(v) = self.stack.pop() { v } else {
                    return Err(OperationError::new("Function does not exist under express"));
                }.unwrap_closure();
                if let Forall { var, expr } = func.get_enum() {
                    self.num_assum -= 1;
                    Term::from(Closure(expr.clone(), Env::new().add(*var, param)))
                } else {
                    return Err(OperationError::new("The element under express is not function"));
                }
            }
            _ => {
                return Err(OperationError::new("Only implication or function is appliable"));
            }
        };
        self.stack.push(el);
        Ok(())
    }

    fn abs(&mut self) -> Result<()> {
        let l = self.stack.len();
        if l < 2 {
            return Err(OperationError::new("Stack needs to contain at least two elements"));
        }
        let q = self.stack.pop().unwrap();
        let p = self.stack.pop().unwrap();
        if !q.is_movable() {
            return Err(OperationError::new("Cannot use non-movable element as condition"));
        }
        if let Assumption(expr) = p.get_enum() {
            self.stack.push(Term::from(Imply(expr.clone(), q)));
        } else {
            return Err(OperationError::new("Only assumptions can be used as antecedent"));
        }
        Ok(())
    }

    fn express(&mut self) -> Result<()> {
        self.stack.push(Term::from(Express));
        self.num_assum += 1;
        Ok(())
    }

    fn assume(&mut self) -> Result<()> {
        let x = if let Some(x) = self.stack.pop() { x } else {
            return Err(OperationError::new("Nothing to assume"));
        };
        if !x.is_movable() {
            return Err(OperationError::new("Non-movable expression cannot be assumed"));
        }
        let e = if let Some(v) = self.stack.pop() { v } else {
            return Err(OperationError::new("Missing express"));
        };
        if let Express = e.get_enum() { } else {
            return Err(OperationError::new("Assumption should be made on an express"));
        }
        self.stack.push(Term::from(Assumption(x)));
        self.num_assum -= 1;
        Ok(())
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
            let x = x.clone();
            Ok((self.wrap_env(x), self.is_normal_mode()))
        } else {
            Err(OperationError::new("Nothing to export"))
        }
    }

    fn concept(&mut self) -> Result<(Self::Term, bool)> {
        self.num_concepts += 1;
        let id = self.num_concepts;
        let mut vars = Vec::new();
        let mut defs = Vec::new();
        for t in self.stack.iter() {
            match t.get_enum() {
                Assumption(t) => defs.push(t.clone()),
                Symbol(x) => vars.push(Term::from(SymbolRef(*x))),
                _ => (),
            }
        }
        Ok((
            self.wrap_env(Term::from(Concept { id, vars, defs, loop_ptr: 0 })),
            self.is_normal_mode(),
        ))
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
        if let Concept { id, vars, defs, loop_ptr } = x.get_enum() {
            let mut nxt = loop_ptr + 1;
            if nxt == defs.len() { nxt += 1 }
            self.stack.push(defs[*loop_ptr].clone());
            self.stack.push(Term::from(Concept {
                id: *id,
                vars: vars.clone(),
                defs: defs.clone(),
                loop_ptr: nxt,
            }));
            Ok(())
        } else {
            return Err(OperationError::new("Only concepts can be unbinded"));
        }
    }

    fn clear(&mut self) -> Result<()> {
        self.stack.clear();
        self.num_assum = 0;
        Ok(())
    }

    fn trust_all(&mut self) -> Result<()> {
        if let Some(x) = self.stack.pop() {
            if !x.is_movable() {
                return Err(OperationError::new("Non-movable expression cannot be assumed"));
            }
            if self.is_normal_mode() {
                return Err(OperationError::new("Cannot trust in normal mode"));
            }
            self.stack.push(x.remove_predicates());
            Ok(())
        } else {
            Err(OperationError::new("Nothing to trust"))
        }
    }
}

fn make_forall(num_symbols: &mut usize, old_id: usize, expr: Term) -> Term {
    // New id is required to make a pure function
    *num_symbols += 1;
    let new_id = *num_symbols;
    Term::from(Forall {
        var: new_id,
        expr: Term::from(Closure(
            expr,
            Env::new().add(old_id, Term::from(SymbolRef(new_id))),
        )),
    })
}

impl Engine {
    pub fn new() -> Engine {
        Engine { stack: Vec::new(), num_symbols: 0, num_concepts: 0, num_assum: 0 }
    }
    fn wrap_env(&mut self, mut ans: Term) -> Term {
        for t in self.stack.iter().rev() {
            match t.get_enum() {
                Symbol(var) => ans = make_forall(&mut self.num_symbols,*var, ans),
                Assumption(p) => ans = Term::from(Imply(p.clone(), ans)),
                _ => (),
            }
        }
        ans
    }
    fn is_normal_mode(&self) -> bool {
        self.num_assum == 0
    }

    fn deep_eq(&mut self, a: &Term, b: &Term) -> bool {
        if Term::shallow_eq(a, b) { return true }
        let a = a.unwrap_closure();
        let b = b.unwrap_closure();
        assert!(a.is_movable() && b.is_movable());
        match (a.get_enum(), b.get_enum()) {
            (SymbolRef(a), SymbolRef(b)) => a == b,
            (Forall{var: v1, expr: e1}, Forall{var: v2, expr: e2}) => {
                if v1 == v2 && Term::shallow_eq(e1, e2) {
                    true
                } else {
                    self.num_symbols += 1;
                    let sym = Term::from(SymbolRef(self.num_symbols));
                    let env1 = Env::new().add(*v1, sym.clone());
                    let env2 = Env::new().add(*v2, sym);
                    self.deep_eq(
                        &Term::from(Closure(e1.clone(), env1)),
                        &Term::from(Closure(e2.clone(), env2)),
                    )
                }
            },
            (Imply(p1, q1), Imply(p2, q2)) => {
                self.deep_eq(p1, p2) && self.deep_eq(q1, q2)
            },
            (
                Concept { id: i1, vars: v1, ..},
                Concept { id: i2, vars: v2, ..},
            ) => if i1 != i2 {
                false
            } else {
                iter::zip(v1.iter(), v2.iter()).all(|(a, b)| self.deep_eq(a, b))
            }
            _ => false,
        }
    }
}
