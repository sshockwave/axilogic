use std::{rc::Rc, collections::HashMap};

use crate::err::{Result, OperationError};

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum Element {
    Symbol,
    SymbolRef(usize), // Reference to a symbol, stack top is 0
    Universal(Rc<Element>),
    Concept{
        id: usize,
        len: usize,
        el: Vec<Rc<Element>>,
    },
}

pub struct Verifier {
    concept_cnt: usize,
    expression_line: Option<usize>,
    stack: Vec<Rc<Element>>,
    symbol_rc: Rc<Element>,
    symbol_table: HashMap<String, (bool, Rc<Element>)>,
}

fn count_symbol<'a, T: IntoIterator<Item = &'a Rc<Element>>>(iter: T) -> usize {
    let mut cnt = 0;
    for el in iter {
        if Element::Symbol == **el {
            cnt += 1;
        }
    }
    cnt
}

// If nothing is changed, return None.
fn dfs_patch(f: Rc<Element>, v: Rc<Element>, level: usize) -> Rc<Element> {
    use Element::*;
    match f.as_ref() {
        Symbol => panic!("Symbol in forall statement"),
        SymbolRef(i) => {
            if *i == level {
                v
            } else if *i > level {
                Rc::new(SymbolRef(*i - 1))
            } else {
                f
            }
        },
        Universal(body) => {
            let new_body = dfs_patch(body.clone(), v, level + 1);
            if Rc::ptr_eq(&f, &new_body) {
                f
            } else {
                Rc::new(Universal(new_body))
            }
        },
        Concept { id, len, el } => {
            let vec: Vec<_> = el.iter().map(|x| dfs_patch(x.clone(), v.clone(), level)).collect();
            if vec.iter().zip(el.iter()).all(|(x, y)| Rc::ptr_eq(x, y)) {
                f
            } else {
                Rc::new(Concept {
                    id: *id,
                    len: *len,
                    el: vec,
                })
            }
        },
    }
}

impl Verifier {
    pub fn new() -> Self {
        // TODO: create core::make_imply
        Self {
            concept_cnt: 1,
            expression_line: None,
            stack: Vec::new(),
            symbol_rc: Rc::new(Element::Symbol),
            symbol_table: HashMap::new(),
        }
    }

    fn maybe_exit_expr(&mut self) {
        if let Some(i) = self.expression_line {
            if i == self.stack.len() {
                self.expression_line = None;
            }
        }
    }

    fn pop_one(&mut self) -> Result<Rc<Element>> {
        match self.stack.pop() {
            Some(el) => Ok(el),
            None => Err(OperationError::new("Stack underflow")),
        }
    }
}

impl super::isa::ISA for Verifier {
    fn push(&mut self, i: isize) -> crate::err::Result<()> {
        let i = if i < 0 {
            let d = -i as usize;
            if d > self.stack.len() {
                return Err(OperationError::new("Stack underflow"));
            }
            self.stack.len() - d
        } else {
            let d = i as usize;
            if d > self.stack.len() {
                return Err(OperationError::new("Stack overflow"));
            }
            d
        };
        use Element::*;
        let el = self.stack.get(i).unwrap();
        let el = match self.stack[i].as_ref() {
            Symbol => {
                if let None = self.expression_line {
                    return Err(OperationError::new("Cannot duplicate symbol outside expression mode"));
                }
                Rc::new(SymbolRef(count_symbol(&self.stack[(i+1)..])))
            },
            SymbolRef(i2) => Rc::new(SymbolRef(i2 + count_symbol(&self.stack[(i+1)..]))),
            _ => el.clone(),
        };
        self.stack.push(el);
        Ok(())
    }

    fn pop(&mut self) -> Result<()> {
        self.pop_one();
        self.maybe_exit_expr();
        Ok(())
    }

    fn variable(&mut self) -> Result<()> {
        self.stack.push(self.symbol_rc.clone());
        Ok(())
    }

    fn forall(&mut self) -> Result<()> {
        let pred = self.pop_one()?;
        if let Element::Symbol = pred.as_ref() {
            return Err(OperationError::new("Variable cannot be a predicate for the forall qualifier"));
        }
        match self.pop_one()?.as_ref() {
            Element::Symbol => (),
            _ => return Err(OperationError::new("Expected symbol when binding a forall qualifier")),
        }
        self.stack.push(Rc::new(Element::Universal(pred)));
        if let Some(i) = self.expression_line {
            if i == self.stack.len() {
                return Err(OperationError::new("The quantifier is not in expression mode but the predicate is"));
            }
        }
        Ok(())
    }

    fn apply(&mut self) -> Result<()> {
        use Element::*;
        let v = self.pop_one()?;
        if let Symbol = v.as_ref() {
            return Err(OperationError::new("Cannot apply an unbounded variable"));
        }
        self.maybe_exit_expr();
        match self.pop_one()?.as_ref() {
            Universal(pred) => self.stack.push(dfs_patch(pred.clone(), v, 0)),
            Concept { id, len, el } => {
                if *len == el.len() {
                    return Err(OperationError::new("The concept has already been fully applied"));
                }
                let mut el = el.clone();
                el.push(v);
                self.stack.push(Rc::new(Concept { id: *id, len: *len, el }));
            },
            _ => return Err(OperationError::new("Expected forall statement when applying a variable")),
        }
        Ok(())
    }

    fn concept(&mut self, n: usize) -> Result<()> {
        self.stack.push(Rc::new(Element::Concept { id: self.concept_cnt, len: n, el: Vec::new() }));
        self.concept_cnt += 1;
        Ok(())
    }
    fn mp(&mut self) -> Result<()> {
        let p = self.pop_one()?;
        let pq = self.pop_one()?;
        if let Element::Concept { id, len, el } = pq.as_ref() {
            if *id != 0 {
                return Err(OperationError::new("Expected imply statement"));
            }
            assert_eq!(*len, 2);
            if el.len() != 2 {
                return Err(OperationError::new("Imply statement incomplete"));
            }
            if el[0] != p {
                return Err(OperationError::new("Condition mismatch"));
            }
            self.stack.push(el[1].clone());
            if let Some(i) = self.expression_line {
                if i == self.stack.len() {
                    return Err(OperationError::new("The predicate is not in expression mode but the premise is"));
                }
            }
            Ok(())
        } else {
            Err(OperationError::new("Expected imply statement"))
        }
    }
    fn express(&mut self) -> Result<()> {
        if let Some(_) = self.expression_line {
            return Err(OperationError::new("Already in expression mode"));
        }
        self.expression_line = Some(self.stack.len());
        Ok(())
    }
    fn assert(&mut self) -> Result<()> {
        let pq = self.pop_one()?;
        self.maybe_exit_expr();
        if let None = self.expression_line {
            return Err(OperationError::new("Not in expression mode"));
        }
        if let Element::Concept { id, len, el } = pq.as_ref() {
            if *id != 0 {
                return Err(OperationError::new("Expected imply statement"));
            }
            assert_eq!(*len, 2);
            if el.len() != 2 {
                return Err(OperationError::new("Imply statement incomplete"));
            }
            self.stack.push(el[1].clone());
            Ok(())
        } else {
            Err(OperationError::new("Expected imply statement"))
        }
    }
    fn export(&mut self, name: String) -> Result<()> {
        if count_symbol(self.stack.as_slice()) != 0 {
            return Err(OperationError::new("Cannot export a statement with unbounded variables"));
        }
        let el = self.pop_one()?;
        self.symbol_table.insert(name, (matches!(self.expression_line, None), el));
        self.maybe_exit_expr();
        Ok(())
    }
    fn import(&mut self, name: String) -> Result<()> {
        let (expr, el) = self.symbol_table.get(&name).ok_or_else(|| OperationError::new(&format!("Symbol {} not found", name)))?;
        if !expr && !matches!(self.expression_line, None) {
            return Err(OperationError::new("The imported target is a hypothesis but the current context is not in expression mode"));
        }
        self.stack.push(el.clone());
        Ok(())
    }
}
