use std::io::{BufRead, stdin};

mod isa;
mod engine;
mod ds;
mod pkg;
mod scan;

fn run<E: isa::ISA, B: BufRead>(eng: &mut E, input: B) {
    let mut input = scan::TokenScanner::new(input);
    let mut pkgdir = pkg::PkgDir::new();
    loop {
        let cmd = if let Some(v) = input.next() { v } else { break };
        let cmd = match cmd {
            Ok(v) => v,
            Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
        };
        let result = match cmd.as_str() {
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
            "unbind" => eng.unbind(),
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
                    |x| pkgdir.set(path.to_string(), x)
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
                    |x| pkgdir.set(path.to_string(), x)
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
                let (a, b) = if let Some(v) = pkgdir.get(&path) { v } else {
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
        };
        if let Err(v) = result {
            panic!("Error occurred on line {}: {:?}", input.get_line_no(), v);
        }
    }
    println!("Examination succeeded.");
}

fn main() {
    run(&mut engine::Engine::new(), stdin().lock());
}
