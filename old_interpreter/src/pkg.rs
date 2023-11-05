use std::collections::HashMap;
use super::path as pathlib;

pub struct PkgDir<T> {
    val: Option<T>,
    root: HashMap<String, PkgDir<T>>,
}

impl<T> PkgDir<T> {
    pub fn new() -> Self {
        PkgDir { val: None, root: HashMap::new() }
    }
    pub fn set(&mut self, path: String, item: T) {
        let mut ptr = self;
        for s in pathlib::to_normal_parts(pathlib::to_iter(&path)) {
            if s != "" {
                ptr = ptr.root.entry(s.to_string()).or_insert_with(Self::new);
            }
        }
        ptr.val = Some(item);
    }
    pub fn get<'a>(&'a self, path: &str) -> Option<&'a T> {
        let mut ptr = self;
        for s in pathlib::to_normal_parts(pathlib::to_iter(path)) {
            if s != "" {
                ptr = if let Some(v) = ptr.root.get(s) { v } else {
                    return None
                }
            }
        }
        ptr.val.as_ref()
    }
}
