use std::{rc::Rc, vec::Vec};

#[derive(PartialEq, Eq)]
struct StackElement<T> {
    prev: Vec<Rc<Self>>,
    val: T,
}

#[derive(PartialEq, Eq, Clone)]
pub struct PersistentStack<T> {
    ptr: Rc<StackElement<T>>,
}

impl<T> PersistentStack<T> {
    pub fn new(val: T) -> Self { Self { ptr: Rc::new(StackElement { prev: Vec::new(), val }) } }
    pub fn top(&self, n: usize) -> T {
        let ptr = self.ptr;
        let cnt: usize = 0;
        while n > 0 {
            if n % 2 == 1 {
                ptr = ptr.prev[cnt];
            }
            n /= 2;
            cnt += 1;
        }
        return ptr.val;
    }
    pub fn push(&self, val: T) -> Self {
        let prev = Vec::new();
        let ptr = self.ptr;
        let cnt = 0;
        loop {
            prev.push(ptr);
            if cnt >= ptr.prev.len() {
                break;
            }
            ptr = ptr.prev[cnt];
            cnt += 1;
        }
        Self { ptr: Rc::new(StackElement { prev, val }) }
    }
    pub fn len(&self) -> usize {
        let n = 1;
        let ptr = self.ptr;
        loop {
            let l = ptr.prev.len();
            if l == 0 {
                break n
            }
            n += (1 as usize) << (l - 1);
        }
    }
    pub fn pop(&self) -> (Option<Self>, T) {
        (self.ptr.prev.first().map(|s| Self { ptr: s.clone() }), self.ptr.val)
    }
}

struct SkipListNode<K, V> {
    neighbor: Option<Rc<Self>>,
    child: Option<Rc<Self>>,
    data: Option<(K, V)>,
}

#[derive(Clone)]
struct SkipList<K: PartialOrd<K> + Eq, V> {
    root: Rc<SkipListNode<K, V>>,
    height: usize,
}

impl<K: PartialOrd<K> + Eq, V> SkipList<K, V> {
    fn gen_height() -> usize {
        return 0;
        /*
        use rand::Rng;
        let mut rng = rand::task_rng();
        let n = 0;
        while rng.gen() {
            n += 1;
        }
        n*/
    }
    pub fn get(&self, k: &K) -> Option<V> {
        let ptr = self.root;
        loop {
            let down = true;
            if let Some(neighbor) = ptr.neighbor {
                let next_k = &neighbor.data.unwrap().0;
                if k >= next_k {
                    down = false;
                }
            }
            match if down { ptr.child } else { ptr.neighbor } {
                Some(v) => ptr = v,
                None => break None,
            }
            if let Some((cur_k, cur_v)) = ptr.data {
                if k == &cur_k {
                    break Some(cur_v);
                }
                if k < &cur_k {
                    break None;
                }
            }
        }
    }
    /**
     * ptr is the last element <= k. It is one level lower than k.
     */
    fn dfs_find_child(ptr: Rc<SkipListNode<K, V>>, k: K, v: V) -> Option<Rc<SkipListNode<K, V>>> {
        if let Some((p_k, _)) = ptr.data {
            if p_k >= k {
                return Some(ptr);
            }
        }
        if Self::is_go_right(ptr.neighbor, k) {
            return Self::dfs_find_child(ptr.neighbor.unwrap(), k, v);
        }
        if let Some(child) = ptr.child {
            if let Some(child2) = Self::dfs_find_child(child, k, v) {
                if let Some(neighbor) = ptr.neighbor {
                    if neighbor.data.unwrap().0 < child2.data.unwrap().0 {
                        return Some(neighbor);
                    }
                }
                return Some(Rc::new(SkipListNode {
                    neighbor: ptr.neighbor,
                    child: Some(child2),
                    data: Some((k, v))
                }));
            }
        }
        return ptr.neighbor;
    }
    fn equal_key(data1: Option<(K, V)>, data2: Option<(K, V)>) -> bool {
        data1.map(|x| x.0) == data2.map(|x| x.0)
    }
    fn is_go_right(ptr: Option<Rc<SkipListNode<K, V>>>, k: K) -> bool {
        if let Some(x) = ptr {
            k >= x.data.unwrap().0
        } else { false }
    }
    fn dfs_add(ptr: Rc<SkipListNode<K, V>>, k: K, v: V, h: usize) -> Rc<SkipListNode<K, V>>{
        let ans = SkipListNode {
            neighbor: ptr.neighbor,
            child: ptr.child,
            data: ptr.data,
        };
        if let Some((cur_k, _)) = ptr.data {
            if k == cur_k {
                ans.data = Some((cur_k, v));
                return Rc::new(ans);
            }
        }
        if Self::is_go_right(ptr.neighbor, k) { // Search on the right
            ans.neighbor = Some(Self::dfs_add(ptr.neighbor.unwrap(), k, v, h));
        } else if h == 0 { // Insert on the right
            ans.neighbor = Some(Rc::new(SkipListNode {
                neighbor: ptr.neighbor,
                child: ptr.child.map(|x| Self::dfs_find_child(x, k, v)).flatten(),
                data: Some((k, v)),
            }));
        } else if Self::is_go_right(ptr.child, k) {
            ans.child = Some(Self::dfs_add(ptr.child.unwrap(), k, v, h - 1));
        } else if h == 1 {
            ans.child = Some(Rc::new(SkipListNode {
                neighbor: ptr.child,
                child: None,
                data: Some((k, v)),
            }));
        } else {
            ans.child = Some(Self::dfs_add(Rc::new(SkipListNode {
                neighbor: None,
                child: None,
                data: ptr.data,
            }), k, v, h - 1));
        }
        Rc::new(ans)
    }
    pub fn add(&self, k: K, v: V) -> Self {
        let h = Self::gen_height();
        let ptr = self.root;
        let new_h = self.height;
        while h > new_h {
            ptr = Rc::new(SkipListNode { neighbor: None, child: Some(ptr), data: None });
            new_h += 1;
        }
        Self { root: Self::dfs_add(ptr, k, v, new_h - h), height: new_h }
    }
}
