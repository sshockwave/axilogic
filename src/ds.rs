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
    fn get(&self, k: &K) -> Option<V> {
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
            }
        }
    }
    fn dfs_add(ptr: Rc<SkipListNode<K, V>>, k: K, v: V, h: usize) -> Rc<SkipListNode<K, V>>{
        let ans = SkipListNode {
            neighbor: ptr.neighbor,
            child: ptr.child,
            data: ptr.data,
        };
        if let Some((cur_k, cur_v)) = ptr.data {
            if k == cur_k {
                ans.data = Some((cur_k, v));
                return Rc::new(ans);
            }
        }
        let down = true;
        if let Some(neighbor) = ptr.neighbor {
            let next_k = &neighbor.data.unwrap().0;
            if &k >= next_k {
                down = false;
            }
        }
        if !down {
            ans.neighbor = Some(Self::dfs_add(ptr.neighbor.unwrap(), k, v, h));
        } else if h == 0 { // Insert on the right
            let x = SkipListNode {
                neighbor: ptr.neighbor,
                child: ptr.child,
                data: Some((k, v)),
            };
            loop {
                if let Some(c) = x.child {
                    let go_right = false;
                    if let Some((child_k, _)) = c.data {
                        if child_k <= k {
                            go_right = true;
                        }
                    } else {
                        go_right = true;
                    }
                    if go_right {
                        x.child = c.neighbor;
                        continue;
                    }
                }
                break;
            }
            ans.neighbor = Some(Rc::new(x));
        } else {
            ans.child = Some(Self::dfs_add(match ptr.child {
                Some(c) => c,
                None => Rc::new(SkipListNode { neighbor: None, child: None, data: ptr.data }),
            }, k, v, h - 1));
        }
        if let Some(c) = ans.child {
            if let Some((sub_k, _)) = c.data {
                if k == sub_k {
                    ans.child = c.neighbor;
                }
            }
        }
        Rc::new(ans)
    }
    fn add(&self, k: K, v: V) -> Self {
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
