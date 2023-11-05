use super::err::Result;

pub trait ISA {
    // stack: [0, 1, 2, ..., -2, -1]
    fn push(&mut self, i: isize) -> Result<()>;
    fn pop(&mut self) -> Result<()>;

    fn symbol(&mut self) -> Result<()>;
    fn forall(&mut self) -> Result<()>;  // [...,       x, F(x)] => [..., x->F(x)]
    fn apply(&mut self) -> Result<()>;   // [..., x->F(x),    x] => [...,    F(x)]

    fn check(&mut self) -> Result<()>;   // [...,    P->Q,    P] => [...,       Q]

    // When exporting, the stack will be popped
    // and the popped element will be saved to the symbol table.
    fn export(&mut self, name: String) -> Result<()>;
    fn import(&mut self, name: String) -> Result<()>;

    fn concept(&mut self) -> Result<()>; // [...,       P,    Q] => [..., sym(P, Q)]
    fn unwrap(&mut self) -> Result<()>;  // [...,     sym(P, Q)] => [...,   P,  Q]
}
