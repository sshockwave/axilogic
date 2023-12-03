use std::{
    cell::RefCell,
    collections::HashSet,
    hash::{BuildHasher, Hash, Hasher},
    rc::{Rc, Weak},
};

pub trait Dedup {
    type Ptr: Eq + Clone + Hash + Ord;
    type Key: Eq;
    fn get(&mut self, key: Self::Key) -> Self::Ptr;
}

struct HashDedupNode<K: Eq + Hash> {
    key: K,
    hash: u64,
    registry: Weak<RefCell<HashSet<HashDedupEntry<K>>>>,
}

// Compare via address
pub struct HashDedupPtr<K: Eq + Hash> {
    data: Rc<HashDedupNode<K>>,
}
impl<K: Eq + Hash> Clone for HashDedupPtr<K> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}
impl<K: Eq + Hash> PartialEq for HashDedupPtr<K> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }
}
impl<K: Eq + Hash> Eq for HashDedupPtr<K> {}
impl<K: Eq + Hash> Hash for HashDedupPtr<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash.hash(state);
    }
}
impl<K: Eq + Hash> Drop for HashDedupPtr<K> {
    fn drop(&mut self) {
        if let Some(registry) = self.data.registry.upgrade() {
            registry.borrow_mut().remove(&HashDedupEntry {
                data: self.data.clone(),
            });
        }
    }
}
impl<K: Eq + Hash> PartialOrd for HashDedupPtr<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(Rc::as_ptr(&self.data).cmp(&Rc::as_ptr(&other.data)))
    }
}
impl<K: Eq + Hash> Ord for HashDedupPtr<K> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Rc::as_ptr(&self.data).cmp(&Rc::as_ptr(&other.data))
    }
}

// Compare via key
struct HashDedupEntry<K: Eq + Hash> {
    data: Rc<HashDedupNode<K>>,
}
impl<K: Eq + Hash> Clone for HashDedupEntry<K> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}
impl<K: Eq + Hash> PartialEq for HashDedupEntry<K> {
    fn eq(&self, other: &Self) -> bool {
        self.data.key == other.data.key
    }
}
impl<K: Eq + Hash> Eq for HashDedupEntry<K> {}
impl<K: Eq + Hash> Hash for HashDedupEntry<K> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.key.hash(state);
    }
}

pub struct HashDedup<K: Eq + Hash> {
    map: Rc<RefCell<HashSet<HashDedupEntry<K>>>>,
}
impl<K: Eq + Hash> HashDedup<K> {
    pub fn new() -> Self {
        Self {
            map: Rc::new(RefCell::new(HashSet::new())),
        }
    }
}
impl<K: Eq + Hash> Dedup for HashDedup<K> {
    type Ptr = HashDedupPtr<K>;
    type Key = K;
    fn get(&mut self, key: Self::Key) -> Self::Ptr {
        let mut map = self.map.borrow_mut();
        let hash = {
            let mut hasher = map.hasher().build_hasher();
            key.hash(&mut hasher);
            hasher.finish()
        };
        let entry = HashDedupEntry {
            data: Rc::new(HashDedupNode {
                key,
                hash,
                registry: Rc::downgrade(&self.map),
            }),
        };
        HashDedupPtr {
            data: if map.insert(entry.clone()) {
                entry.data
            } else {
                map.get(&entry).unwrap().data.clone()
            },
        }
    }
}
