use super::err::Result;

pub trait InstructionSet {
    /// Universal quantification
    fn uni(&mut self) -> Result<()>;
    fn def(&mut self, s: String) -> Result<()>;
    fn hyp(&mut self, s: String) -> Result<()>; // To imaginary mode
    fn obj(&mut self, s: String) -> Result<()>; // No act

    /// Types
    fn var(&mut self) -> Result<()>;
    fn hkt(&mut self) -> Result<()>; // [..., P, Q] => [..., P=>Q]

    /// Predicate
    fn qed(&mut self) -> Result<()>;

    /// Logic
    fn mp(&mut self) -> Result<()>; // [..., P=>Q, P] => [..., Q]
    fn im(&mut self) -> Result<()>; // To imaginary mode
    fn app(&mut self) -> Result<()>; // [..., x->f(x), y] => [..., f(y)]

    /// Import
    fn req(&mut self, s: String) -> Result<()>;

    /// Imaginary mode only
    fn sat(&mut self) -> Result<()>;
    fn arg(&mut self, n: usize) -> Result<()>;
}
