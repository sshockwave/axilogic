use std::{io::{BufRead, stdin}, vec::Vec};

mod isa;
mod engine;
mod ds;
mod pkg;

fn run<E: isa::ISA, B: BufRead>(eng: &mut E, input: B) {
    let mut line_count: usize = 0;
    use regex::Regex;
    let word_gap = Regex::new(r"\s+").unwrap();
    let empty_string = Regex::new(r"^\s*$").unwrap();
    let mut pkgdir = pkg::PkgDir::new();
    for raw_line in input.lines() {
        line_count += 1;
        let line = if let Ok(v) = raw_line { v } else {
            panic!("Error occurred while reading line {}", line_count);
        };
        let tokens: Vec<_> = word_gap
            .split(&line)
            .filter(|x| !empty_string.is_match(x))
            .take_while(|x| !x.starts_with('#'))
            .collect();
        let cmd = if let Some(v) = tokens.first() { v } else { continue };
        let mut result = Ok(());
        match *cmd {
            "push" => {
                assert_eq!(tokens.len(), 2, "Number of parameter should be exactly 1 on line {}", line_count);
                let num_s = tokens[1];
                let n = if let Ok(v) = num_s.parse() { v } else {
                    panic!("Integer parse failure on line {}", line_count);
                };
                result = eng.push(n);
            }
            "swap" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.swap();
            }
            "pop" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.pop();
            }
            "symbol" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.symbol();
            }
            "forall" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.forall();
            }
            "apply" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.apply();
            }
            "express" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.express();
            }
            "assume" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.assume();
            }
            "abstract" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.abs();
            }
            "trust" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.trust();
            }
            "unbind" => {
                assert_eq!(tokens.len(), 1, "No params should be on line {}", line_count);
                result = eng.unbind();
            }
            "export" => {
                assert_eq!(tokens.len(), 2, "No params should be on line {}", line_count);
                let path = tokens[1];
                result = eng.export().map(
                    |x| pkgdir.set(path.to_string(), x)
                );
            }
            "concept" => {
                assert_eq!(tokens.len(), 2, "No params should be on line {}", line_count);
                let path = tokens[1];
                result = eng.concept().map(
                    |x| pkgdir.set(path.to_string(), x)
                );
            }
            "refer" => {
                let path = tokens[1..].join(":");
                let (a, b) = if let Some(v) = pkgdir.get(&path) { v } else {
                    panic!("Name {} does not exist on line {}", path, line_count);
                };
                result = eng.refer(a.clone(), *b);
            }
            s => {
                println!("{:?}", tokens);
                panic!("Undefined command: {}", s);
            },
        }
        if let Err(v) = result {
            panic!("Error occurred on line {}: {:?}", line_count, v);
        }
    }
    println!("Examination succeeded.");
}

fn main() {
    run(&mut engine::Engine::new(), stdin().lock());
}
