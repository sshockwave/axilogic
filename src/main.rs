use std::{io::{BufRead, stdin, BufReader}, fmt};

mod isa;
mod engine;
mod ds;
mod pkg;
mod scan;

struct Runner<E: isa::ISA + fmt::Display> {
    eng: E,
    pkgdir: pkg::PkgDir<(E::Term, bool)>,
}

impl<E: isa::ISA + fmt::Display> Runner<E> {
    fn new(eng: E) -> Self {
        Self { eng, pkgdir: pkg::PkgDir::new() }
    }
    fn run_one_command<B: BufRead>(&mut self, cmd: String, input: &mut TokenScanner<B>) -> Result<(), OperationError>{
        let eng = &mut self.eng;
        match cmd.as_str() {
            "push" => {
                let num_s = if let Some(v) = input.next() { v } else {
                    panic!("Expected parameter for push on line {}", input.get_line_no());
                };
                let num_s = match num_s {
                    Ok(v) => v,
                    Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
                };
                let n = if let Ok(v) = num_s.parse() { v } else {
                    panic!("Integer '{}' parse failure on line {}", num_s, input.get_line_no());
                };
                eng.push(n)
            }
            "swap" => eng.swap(),
            "pop" => eng.pop(),
            "symbol" => eng.symbol(),
            "forall" => eng.forall(),
            "apply" => eng.apply(),
            "express" => eng.express(),
            "assume" => eng.assume(),
            "abstract" => eng.abs(),
            "trust" => eng.trust(),
            "trustall" => eng.trust_all(),
            "unbind" => eng.unbind(),
            "print" => eng.print(),
            "clear" => eng.clear(),
            "export" => {
                let path = if let Some(v) = input.next() { v } else {
                    panic!("Expected parameter for export on line {}", input.get_line_no());
                };
                let path = match path {
                    Ok(v) => v,
                    Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
                };
                // TODO: check name validity
                eng.export().map(
                    |x| self.pkgdir.set(path.to_string(), x)
                )
            }
            "concept" => {
                let path = if let Some(v) = input.next() { v } else {
                    panic!("Expected parameter for export on line {}", input.get_line_no());
                };
                let path = match path {
                    Ok(v) => v,
                    Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
                };
                // TODO: check name validity
                eng.concept().map(
                    |x| self.pkgdir.set(path.to_string(), x)
                )
            }
            "refer" => {
                let path = if let Some(v) = input.next() { v } else {
                    panic!("Expected parameter for export on line {}", input.get_line_no());
                };
                let path = match path {
                    Ok(v) => v,
                    Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
                };
                let (a, b) = if let Some(v) = self.pkgdir.get(&path) { v } else {
                    panic!("Name {} does not exist on line {}", path, input.get_line_no());
                };
                eng.refer(a.clone(), *b)
            }
            "echo" => {
                let str = if let Some(v) = input.next() { v } else {
                    panic!("Expected parameter for export on line {}", input.get_line_no());
                };
                let msg = match str {
                    Ok(v) => v,
                    Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
                };
                println!("{}", msg);
                Ok(())
            }
            s => {
                panic!("Undefined command: {}", s);
            },
        }
    }

    fn run<B: BufRead>(&mut self, input: B) {
        let mut input = scan::TokenScanner::new(input);
        loop {
            let cmd = if let Some(v) = input.next() { v } else { break };
            let cmd = match cmd {
                Ok(v) => v,
                Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
            };
            let result = self.run_one_command(cmd, &mut input);
            if let Err(v) = result {
                panic!("Error occurred on line {}: {:?}", input.get_line_no(), v);
            }
        }
        println!("Examination succeeded.");
    }
}

fn main() {
    Runner::new(engine::Engine::new()).run(stdin().lock());
}
