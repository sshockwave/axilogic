use std::{io::{BufRead, BufReader}, fmt, fs::File, path::{Path, PathBuf}};

mod isa;
mod engine;
mod ds;
mod pkg;
mod scan;
mod path;

use isa::OperationError;
use scan::TokenScanner;


struct Runner<E: isa::ISA + fmt::Display> {
    eng: E,
    pkgdir: pkg::PkgDir<(E::Term, bool)>,
    fs_root: Option<PathBuf>,
}

impl<E: isa::ISA + fmt::Display> Runner<E> {
    fn new<P: AsRef<Path>>(eng: E, cwd: Option<P>) -> Self {
        Self {
            eng,
            pkgdir: pkg::PkgDir::new(),
            fs_root: cwd.map(|x|PathBuf::from(x.as_ref())),
        }
    }
    fn find_ref(&mut self, p: String) -> (E::Term, bool) {
        if let Some((a, b)) = self.pkgdir.get(&p) {
            return (a.clone(), *b);
        }
        let mut cur_path = if let Some(v) = &self.fs_root { v.clone() } else {
            panic!("No FS root provided and refer not found");
        };
        let mut cwd = Vec::new();
        for s in path::to_iter(&p) {
            if s == path::PARENT_DIR {
                let ok = cur_path.pop();
                assert!(ok, "Cannot go up any more");
            } else {
                cur_path.push(s);
            }
            cwd.push(s);
        }
        cwd.pop();
        cur_path.pop();
        cur_path.set_extension("thm");
        self.run(BufReader::new(File::open(cur_path).unwrap()), &path::collect(&cwd));
        if let Some((a, b)) = self.pkgdir.get(&p) {
            (a.clone(), *b)
        } else {
            panic!("Reference not found in corresponding file");
        }
    }
    fn run_one_command<B: BufRead>(&mut self, cmd: String, input: &mut TokenScanner<B>, cwd: &str) -> Result<(), OperationError>{
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
                let path = path::join(cwd.to_string(), path);
                assert!(path::start_with(path.clone(), cwd.to_string()), "Cannot export to super packages");
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
                let path = path::join(cwd.to_string(), path);
                assert!(path::start_with(path.clone(), cwd.to_string()), "Cannot make concept to super packages");
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
                let path = path::join(cwd.to_string(), path);
                let (a, b) = self.find_ref(path);
                self.eng.refer(a, b)
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

    fn run<B: BufRead>(&mut self, input: B, cwd: &str) {
        let mut input = scan::TokenScanner::new(input);
        loop {
            let cmd = if let Some(v) = input.next() { v } else { break };
            let cmd = match cmd {
                Ok(v) => v,
                Err(e) => panic!("Error occurred on parsing line {}: {:?}", input.get_line_no(), e),
            };
            let result = self.run_one_command(cmd, &mut input, cwd);
            if let Err(v) = result {
                panic!("Error occurred on {}:{} :: {:?}", cwd, input.get_line_no(), v);
            }
        }
        println!("Examination succeeded.");
    }
}

fn main() {
    let mut file_path = PathBuf::from("./content/main.thm");
    let file = File::open(file_path.as_os_str()).unwrap();
    file_path.pop();
    Runner::new(engine::Engine::new(), Some(file_path))
        .run(BufReader::new(file), "main");
}
