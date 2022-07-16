use std::{iter::*, io::{Lines, BufRead, Result}};

pub struct TokenScanner <B: BufRead> {
    lines: Lines<B>,
    line_num: usize,
    tokens: Vec<String>,
    ended: bool,
}

impl<B: BufRead> TokenScanner<B> {
    pub fn new(buf: B) -> Self {
        TokenScanner {
            lines: buf.lines(),
            line_num: 0,
            tokens: Vec::new(),
            ended: false,
        }
    }
    pub fn get_line_no(&self) -> usize {
        self.line_num
    }
}

impl<B: BufRead> Iterator for TokenScanner<B> {
    type Item = Result<String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.ended { return None }
        loop {
            let s = self.tokens.pop();
            if let Some(v) = s { return Some(Ok(v)) }
            let s = self.lines.next();
            let s = if let Some (v) = s { v } else {
                self.ended = true;
                return None;
            };
            let s = if let Ok(s) = s { s } else {
                return Some(s);
            };
            self.line_num += 1;
            self.tokens = Vec::new();
            for t in s.split_whitespace() {
                if t.contains('#') {
                    let s = t.split('#').next().unwrap();
                    if !s.is_empty() {
                        self.tokens.push(s.to_owned());
                    }
                    break
                }
                self.tokens.push(t.to_owned());
            }
            self.tokens.reverse();
        }
    }
}
