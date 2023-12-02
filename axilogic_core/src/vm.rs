mod ty;

use std::{collections::HashMap, rc::Rc};

use crate::err::{OperationError, Result};

#[derive(Clone, PartialEq, Eq)]
pub enum ElementType {
    Symbol,
    Function(Rc<ElementType>, Rc<ElementType>),
}

#[derive(PartialEq, Eq, Clone)]
enum ElementContent {
    Argument(usize),
    Object {
        id: usize,
        params: Vec<Element>,
    },
    Universal {
        param_cnt: usize,
        param_len: usize,
        act: Element,
    },
    Implication(Element, Element),
    Application(Element, Element),
}

#[derive(Clone, Eq)]
struct Element {
    content: Rc<ElementContent>,
    ty: ElementType,
}

impl ElementContent {
    fn calc_type(&self) -> ElementType {
        todo!("")
    }
}

impl Element {
    fn dfs_patch(&mut self, id: usize, value: Element) {
        todo!("")
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.content == other.content
    }
}

enum StackData {
    Hypothesis {
        export_name: String,
        stack: Vec<Element>,
    },
    Universal {
        imag_line: Option<usize>,
        stack: Vec<Element>,
    },
    Definition {
        export_name: String,
        imag_line: Option<usize>,
        stack: Vec<Element>,
    },
    Object {
        export_name: String,
    },
}

struct Stack {
    arguments: Vec<ElementType>,
    data: StackData,
}

pub struct Verifier {
    object_def: Vec<Vec<ElementType>>,
    stack: Vec<Stack>,
    symbol_table: HashMap<String, (bool, Element)>, // is_real, element
}

impl Stack {
    fn push(&mut self, el: Element) {
        use StackData::*;
        match &mut self.data {
            Hypothesis { stack, .. } | Universal { stack, .. } | Definition { stack, .. } => {
                stack.push(el)
            }
            StackData::Object { .. } => panic!("Object has no stack"),
        }
    }
    fn pop(&mut self) -> Result<Element> {
        use StackData::*;
        match &mut self.data {
            Hypothesis { stack, .. } | Universal { stack, .. } | Definition { stack, .. } => {
                s_pop(stack)
            }
            StackData::Object { .. } => Err(OperationError::new("Object has no stack")),
        }
    }
    fn is_imag(&self) -> bool {
        use StackData::*;
        match &self.data {
            Hypothesis { .. }
            | Universal {
                imag_line: Some(_), ..
            }
            | Definition {
                imag_line: Some(_), ..
            } => true,
            _ => false,
        }
    }
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

impl Verifier {
    pub fn init_sys(&mut self) {
        todo!("proposition logic")
    }
    pub fn new() -> Self {
        let mut vm = Self {
            object_def: Vec::new(),
            stack: Vec::new(),
            symbol_table: HashMap::new(),
        };
        vm.init_sys();
        vm
    }
    pub fn has(&self, s: String) -> bool {
        self.symbol_table.contains_key(&s)
    }
}

impl super::isa::InstructionSet for Verifier {
    fn app(&mut self) -> Result<()> {
        let frame = s_top(&mut self.stack)?;
        let x = frame.pop()?;
        let mut f = frame.pop()?;
        use ElementContent::*;
        let ty = match &f.ty {
            ElementType::Function(x_ty, ty) => {
                if x_ty.as_ref() != &x.ty {
                    return Err(OperationError::new(
                        "Applying function with an argument of a wrong type",
                    ));
                }
                ty.clone()
            }
            _ => return Err(OperationError::new("Applying non-function")),
        };
        let content = match f.content.as_ref() {
            Argument(_) | Implication(_, _) | Application(_, _) => Rc::new(Application(f, x)),
            Universal {
                param_cnt,
                param_len,
                act,
            } => {
                let mut act = act.clone();
                act.dfs_patch(*param_cnt, x);
                let param_cnt = param_cnt + 1;
                if param_cnt == *param_len {
                    assert!(act.ty == *ty);
                    act.content
                } else {
                    Rc::new(Universal {
                        param_cnt,
                        param_len: *param_len,
                        act,
                    })
                }
            }
            Object { .. } => {
                if let Object { id, params } = Rc::<ElementContent>::make_mut(&mut f.content) {
                    assert!(params.len() < self.object_def[*id].len());
                    params.push(x);
                } else {
                    unreachable!()
                }
                f.content
            }
        };
        frame.push(Element {
            content,
            ty: ty.as_ref().clone(),
        });
        Ok(())
    }
    fn arg(&mut self, n: usize) -> Result<()> {
        let frame = s_top(&mut self.stack)?;
        if !frame.is_imag() {
            return Err(OperationError::new(
                "Using argument of function in non-imaginary mode",
            ));
        }
        frame.push(Element {
            content: Rc::new(ElementContent::Argument(n)),
            ty: frame
                .arguments
                .get(n)
                .ok_or_else(|| OperationError::new("Argument index out of range"))?
                .clone(),
        });
        Ok(())
    }

    fn uni(&mut self) -> Result<()> {
        self.stack.push(Stack {
            arguments: Vec::new(),
            data: StackData::Universal {
                imag_line: None,
                stack: Vec::new(),
            },
        });
        Ok(())
    }
    fn def(&mut self, s: String) -> Result<()> {
        self.stack.push(Stack {
            arguments: Vec::new(),
            data: StackData::Definition {
                export_name: s,
                imag_line: None,
                stack: Vec::new(),
            },
        });
        Ok(())
    }
    fn hyp(&mut self, s: String) -> Result<()> {
        self.stack.push(Stack {
            arguments: Vec::new(),
            data: StackData::Hypothesis {
                export_name: s,
                stack: Vec::new(),
            },
        });
        Ok(())
    }
    fn obj(&mut self, s: String) -> Result<()> {
        self.stack.push(Stack {
            arguments: Vec::new(),
            data: StackData::Object { export_name: s },
        });
        Ok(())
    }

    fn hkt(&mut self) -> Result<()> {
        unimplemented!()
    }
    fn im(&mut self) -> Result<()> {
        let frame = s_top(&mut self.stack)?;
        use StackData::*;
        match &mut frame.data {
            Universal { imag_line, stack }
            | Definition {
                imag_line, stack, ..
            } => {
                if let Some(_) = imag_line {
                    return Err(OperationError::new("Imaginary mode already set"));
                }
                let quant = s_pop(stack)?;
                if let ElementType::Symbol = quant.ty {
                    return Err(OperationError::new(
                        "Entering imaginary mode requires a universal quantifier",
                    ));
                }
                stack.push(quant);
                imag_line.replace(stack.len());
                Ok(())
            }
            _ => Err(OperationError::new(
                "Using imaginary mode in non-universal mode",
            )),
        }
    }
    fn qed(&mut self) -> Result<()> {
        let frame = s_pop(&mut self.stack)?;
        use StackData::*;
        let (body_ty, body_el) = match frame.data {
            Object { .. } => {
                let id = self.object_def.len();
                self.object_def.push(frame.arguments.clone());
                (
                    ElementType::Symbol,
                    ElementContent::Object {
                        id,
                        params: Vec::new(),
                    },
                )
            }
            Universal { mut stack, .. }
            | Definition { mut stack, .. }
            | Hypothesis { mut stack, .. } => {
                if stack.len() != 1 {
                    return Err(OperationError::new("QED with non-singleton stack"));
                }
                let el = stack.pop().unwrap();
                (el.ty, el.content.as_ref().clone())
            }
        };
        let el = Element {
            content: Rc::new(body_el)/* Rc::new(ElementContent::Universal {
                param_cnt: 0,
                param_len: 0,
                act: Rc::new(body_el),
            })*/,
            ty: ElementType::Function(Rc::new(ElementType::Symbol), Rc::new(body_ty)),
        }; // TODO
        match frame.data {
            Definition { export_name, .. } => {
                self.symbol_table.insert(export_name, (true, el));
            }
            Object { export_name } | Hypothesis { export_name, .. } => {
                self.symbol_table.insert(export_name, (false, el));
            }
            Universal { .. } => {
                let frame = s_top(&mut self.stack)?;
                frame.push(el);
            }
        }
        Ok(())
    }
    fn req(&mut self, s: String) -> Result<()> {
        let (is_real, el) = self
            .symbol_table
            .get(&s)
            .ok_or_else(|| OperationError::new(format!("Symbol not found: {}", s)))?;
        let frame = s_top(&mut self.stack)?;
        if !is_real && frame.is_imag() {
            return Err(OperationError::new(format!(
                "Using imaginary symbol {} in imaginary mode",
                s
            )));
        }
        frame.push(el.clone());
        Ok(())
    }
    fn mp(&mut self) -> Result<()> {
        let frame = s_top(&mut self.stack)?;
        if frame.is_imag() {
            return Err(OperationError::new(
                "Use sat instead of mp in imaginary mode",
            ));
        }
        let p_pred = frame.pop()?;
        let pq = frame.pop()?;
        if let ElementContent::Implication(p_ans, q) = pq.content.as_ref() {
            if p_pred != *p_ans {
                return Err(OperationError::new("MP but condition not satisfied"));
            }
            frame.push(q.clone());
            Ok(())
        } else {
            Err(OperationError::new("Using mp with non-implication"))
        }
    }
    fn sat(&mut self) -> Result<()> {
        let frame = s_top(&mut self.stack)?;
        if !frame.is_imag() {
            return Err(OperationError::new("Using sat in non-imaginary mode"));
        }
        let pq = frame.pop()?;
        if let ElementContent::Implication(_, q) = pq.content.as_ref() {
            frame.push(q.clone());
            Ok(())
        } else {
            Err(OperationError::new("Using sat with non-implication"))
        }
    }
    fn var(&mut self) -> Result<()> {
        let frame = s_top(&mut self.stack)?;
        frame.arguments.push(ElementType::Symbol);
        Ok(())
    }
}
