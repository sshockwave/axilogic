use std::{vec::Vec, str::Split};

pub const PARENT_DIR: &'static str = "super";
pub fn to_iter<'a>(path: &'a str) -> Split<'a, char> {
    path.split(':')
}

pub fn to_normal_parts<'a, S: Iterator<Item=&'a str>>(iter: S) -> Vec<&'a str> {
    let mut ans = Vec::new();
    for s in iter {
        if s == PARENT_DIR {
            if let Some(top) = ans.last() {
                if top != &PARENT_DIR {
                    ans.pop();
                    continue;
                }
            }
        }
        if s != "" {
            ans.push(s);
        }
    }
    ans
}
pub fn collect<'a>(iter: &Vec<&'a str>) -> String {
    iter.join(":")
}
pub fn start_with(a: String, b: String) -> bool {
    let a = to_normal_parts(to_iter(&a));
    let b = to_normal_parts(to_iter(&b));
    if a.len() < b.len() { return false }
    for i in 0..b.len() {
        if a[i] != b[i] { return false }
    }
    true
}
pub fn normalize(path: String) -> String {
    collect(&to_normal_parts(to_iter(&path)))
}
pub fn join(a: String, b: String) -> String {
    collect(&to_normal_parts(to_iter(&a).chain(to_iter(&b))))
}
