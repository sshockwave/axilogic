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
        el: Vec<Element>,
    },
}

pub struct Verifier {
    concept_cnt: usize,
    expression_line: Option<usize>,
    stack: Vec<Element>,
    symbol_table: HashMap<String, (bool, Element)>,
}

fn nth_symbol<'a, T: IntoIterator<Item = &'a Element>>(iter: T, mut n: usize) -> usize {
    let mut cnt = 0;
    for el in iter {
        if Element::Symbol == *el {
            if n == 0 {
                break;
            }
            n -= 1;
        }
        cnt += 1;
    }
    cnt
}
fn count_symbol<'a, T: IntoIterator<Item = &'a Element>>(iter: T) -> usize {
    let mut cnt = 0;
    for el in iter {
        if Element::Symbol == *el {
            cnt += 1;
        }
    }
    cnt
}

// If nothing is changed, return None.
fn dfs_patch(f: &Element, v: Element, level: usize) -> Option<Element> {
    use Element::*;
    match f {
        Symbol => panic!("Symbol in forall statement"),
        SymbolRef(i) => {
            if *i == level {
                if v == *f {
                    None
                } else {
                    Some(v)
                }
            } else if *i > level {
                Some(SymbolRef(*i - 1))
            } else {
                None
            }
        },
        Universal(f) => Some(Universal(Rc::new(dfs_patch(f.as_ref(), v, level + 1)?))),
        Concept { id, len, el } => {
            let vec: Vec<_> = el.iter().map(|x| dfs_patch(x, v.clone(), level)).collect();
            if vec.iter().all(|x| x.is_none()) {
                None
            } else {
                Some(Concept {
                    id: *id,
                    len: *len,
                    el: vec.into_iter().zip(el.iter()).map(|(x, default)| x.unwrap_or_else(|| default.clone())).collect(),
                })
            }
        },
    }
}

impl Verifier {
    fn new() -> Self {
        // TODO: create core::make_imply
        Self {
            concept_cnt: 1,
            expression_line: None,
            stack: Vec::new(),
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

    fn pop_one(&mut self) -> Result<Element> {
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
        let el = match self.stack[i] {
            Symbol => {
                if let None = self.expression_line {
                    return Err(OperationError::new("Cannot duplicate symbol outside expression mode"));
                }
                SymbolRef(count_symbol(&self.stack[(i+1)..]))
            },
            SymbolRef(i2) => SymbolRef(i2 + count_symbol(&self.stack[(i+1)..])),
            Universal(ref el) => Universal(el.clone()),
            Concept{ref id, ref len, ref el} => Concept{
                id: *id,
                len: *len,
                el: el.iter().map(|el| el.clone()).collect(),
            },
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
        self.stack.push(Element::Symbol);
        Ok(())
    }

    fn forall(&mut self) -> Result<()> {
        let pred = self.pop_one()?;
        if let Element::Symbol = pred {
            return Err(OperationError::new("Variable cannot be a predicate for the forall qualifier"));
        }
        match self.pop_one()? {
            Element::Symbol => (),
            _ => return Err(OperationError::new("Expected symbol when binding a forall qualifier")),
        }
        self.stack.push(Element::Universal(Rc::new(pred)));
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
        if let Symbol = v {
            return Err(OperationError::new("Cannot apply an unbounded variable"));
        }
        self.maybe_exit_expr();
        match self.pop_one()? {
            Universal(pred) => self.stack.push(dfs_patch(pred.as_ref(), v, 0).unwrap_or(pred.as_ref().clone())),
            Concept { id, len, mut el } => {
                if len == el.len() {
                    return Err(OperationError::new("The concept has already been fully applied"));
                }
                el.push(v);
                self.stack.push(Concept { id, len, el });
            },
            _ => return Err(OperationError::new("Expected forall statement when applying a variable")),
        }
        Ok(())
    }

    fn concept(&mut self, n: usize) -> Result<()> {
        self.stack.push(Element::Concept { id: self.concept_cnt, len: n, el: Vec::new() });
        self.concept_cnt += 1;
        Ok(())
    }
    fn mp(&mut self) -> Result<()> {
        let p = self.pop_one()?;
        let pq = self.pop_one()?;
        if let Element::Concept { id, len, mut el } = pq {
            if id != 0 {
                return Err(OperationError::new("Expected imply statement"));
            }
            assert_eq!(len, 2);
            if el.len() != 2 {
                return Err(OperationError::new("Imply statement incomplete"));
            }
            if el[0] != p {
                return Err(OperationError::new("Condition mismatch"));
            }
            self.stack.push(el.pop().unwrap());
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
        if let Element::Concept { id, len, mut el } = pq {
            if id != 0 {
                return Err(OperationError::new("Expected imply statement"));
            }
            assert_eq!(len, 2);
            if el.len() != 2 {
                return Err(OperationError::new("Imply statement incomplete"));
            }
            self.stack.push(el.pop().unwrap());
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
