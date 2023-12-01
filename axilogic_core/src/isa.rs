use super::err::Result;

pub trait InstructionSet {
    fn uni(&mut self, n: usize) -> Result<()>;
    fn def(&mut self, n: usize, s: String) -> Result<()>;
    fn hyp(&mut self, n: usize, s: String) -> Result<()>;
    fn qed(&mut self) -> Result<()>;

    fn mp(&mut self) -> Result<()>; // [..., P=>Q, P] => [..., Q]
    fn sat(&mut self) -> Result<()>;
    fn req(&mut self, s: String) -> Result<()>;
    fn spec(&mut self) -> Result<()>; // stack top must be a universal quantification
    fn app(&mut self) -> Result<()>;
}
