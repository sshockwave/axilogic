use super::err::Result;

pub trait ISA {
    // stack: [0, 1, 2, ..., -2, -1]
    fn push(&mut self, i: isize) -> Result<()>;
    fn pop(&mut self) -> Result<()>;

    fn variable(&mut self) -> Result<()>;
    fn forall(&mut self) -> Result<()>; // [...,       x, F(x)] => [..., x->F(x)]
    fn apply(&mut self) -> Result<()>; // [..., x->F(x),    x] => [...,    F(x)]

    // Concepts are used to check() equality between two expressions.
    // Each invocation of concept() will push a new unique concept to the stack.
    // It has the form a -> (b -> (... -> (z -> instance)))
    fn concept(&mut self, n: usize) -> Result<()>;
    fn mp(&mut self) -> Result<()>; // [...,    P->Q,    P] => [...,       Q]

    // Enter expression mode.
    // In this mode, we only construct expressions
    // but do not verify its correctness,
    // i.e. you can assert() anything.
    // The expression mode exits if we apply()
    // and stack[-1] is the last element inside the expression mode.
    fn express(&mut self) -> Result<()>;
    fn assert(&mut self) -> Result<()>; // [...,          P->Q] => [...,       Q]

    // When exporting, the stack should not contain any unbound variables.
    // stack[-1] will be popped and saved to the symbol table.
    // If we are currently in expression mode,
    // we also exit the expression mode.
    fn export(&mut self, name: String) -> Result<()>;
    fn import(&mut self, name: String) -> Result<()>;
}
