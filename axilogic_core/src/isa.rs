use std::num::NonZeroUsize;

use crate::err::Result;

pub trait InstructionSet {
    /// Universal quantification
    fn uni(&mut self) -> Result<()>;

    /// Types
    fn var(&mut self) -> Result<()>;
    fn hkt(&mut self) -> Result<()>; // [..., P, Q] => [..., P=>Q]

    // End of arguments or body
    fn qed(&mut self) -> Result<()>;

    /// Logic
    fn mp(&mut self) -> Result<()>; // [..., P=>Q, P] => [..., Q]
    fn app(&mut self) -> Result<()>; // [..., x->f(x), syn, y] => [..., f(y)]

    /// Import
    fn req(&mut self, s: String) -> Result<()>;
    fn def(&mut self, s: String) -> Result<()>; // [..., y] => [...]
    fn hyp(&mut self, s: String) -> Result<()>; // [..., syn, y] => [...]

    fn obj(&mut self, n: usize, s: String) -> Result<()>;

    fn syn(&mut self) -> Result<()>; // [...] => [..., syn]
    /// Synthetic mode only
    fn sat(&mut self) -> Result<()>;
    fn arg(&mut self, n: NonZeroUsize) -> Result<()>;
}
