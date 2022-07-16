use std::{vec::Vec, collections::HashMap, str::Split};

pub struct PkgDir<T> {
    val: Option<T>,
    root: HashMap<String, PkgDir<T>>,
}

impl<T> PkgDir<T> {
    pub const PARENT_DIR: &'static str = "super";
    pub fn new() -> Self {
        PkgDir { val: None, root: HashMap::new() }
    }
    fn to_iter<'a>(path: &'a str) -> Split<'a, char> {
        path.split(':')
    }
    fn to_normal_parts<'a, S: Iterator<Item=&'a str>>(iter: S) -> Vec<&'a str> {
        let mut ans = Vec::new();
        for s in iter {
            if s == Self::PARENT_DIR {
                if let Some(top) = ans.last() {
                    if top != &Self::PARENT_DIR {
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
    fn collect<'a>(iter: &Vec<&'a str>) -> String {
        iter.join(":")
    }
    pub fn normalize(path: String) -> String {
        Self::collect(&Self::to_normal_parts(Self::to_iter(&path)))
    }
    pub fn join(a: String, b: String) -> String {
        Self::collect(&Self::to_normal_parts(Self::to_iter(&a).chain(Self::to_iter(&b))))
    }
    pub fn set(&mut self, path: String, item: T) {
        let mut ptr = self;
        for s in Self::to_normal_parts(Self::to_iter(&path)) {
            if s != "" {
                ptr = ptr.root.entry(s.to_string()).or_insert_with(Self::new);
            }
        }
        ptr.val = Some(item);
    }
    pub fn get<'a>(&'a self, path: &str) -> Option<&'a T> {
        let mut ptr = self;
        for s in Self::to_normal_parts(Self::to_iter(path)) {
            if s != "" {
                ptr = if let Some(v) = ptr.root.get(s) { v } else {
                    return None
                }
            }
        }
        ptr.val.as_ref()
    }
}
