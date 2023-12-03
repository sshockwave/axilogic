use rpds::HashTrieSet;
use std::hash::Hash;

fn set_union_<T: Hash + Eq>(a: &mut HashTrieSet<T>, b: &HashTrieSet<T>) {
    for x in b.iter() {
        a.insert_mut(x.clone());
    }
}

pub fn set_union_own<T: Hash + Eq>(mut a: HashTrieSet<T>, mut b: HashTrieSet<T>) -> HashTrieSet<T> {
    let (mut a, b) = if a.size() >= b.size() {
        (a, &b)
    } else {
        (b, &a)
    };
    set_union_(&mut a, b);
    a
}

pub fn set_union_mut<T: Hash + Eq>(a: &mut HashTrieSet<T>, b: &HashTrieSet<T>) {
    if a.size() >= b.size() {
        set_union_(a, b)
    } else {
        let mut b = b.clone();
        std::mem::swap(a, &mut b);
        set_union_(a, &b);
    }
}

pub fn set_union<T: Hash + Eq>(a: &HashTrieSet<T>, b: &HashTrieSet<T>) -> HashTrieSet<T> {
    let (mut a, b) = if a.size() > b.size() {
        (a.clone(), b)
    } else {
        (b.clone(), a)
    };
    set_union_(&mut a, b);
    a
}

fn set_diff1<T: Hash + Eq>(a: &mut HashTrieSet<T>, b: &HashTrieSet<T>) {
    for x in b.iter() {
        a.remove_mut(x);
    }
}

fn set_diff2<T: Clone + Hash + Eq>(a: &HashTrieSet<T>, b: &HashTrieSet<T>) -> HashTrieSet<T> {
    a.iter().filter(|&x| !b.contains(x)).collect()
}

pub fn set_diff_mut<T: Hash + Eq>(a: &mut HashTrieSet<T>, b: &HashTrieSet<T>) {
    if a.size() < b.size() {
        *a = set_diff2(a, b);
    } else {
        set_diff1(a, b);
    }
}

pub fn set_diff<T: Hash + Eq>(a: &HashTrieSet<T>, b: &HashTrieSet<T>) -> HashTrieSet<T> {
    if a.size() < b.size() {
        set_diff2(a, b)
    } else {
        let mut a = a.clone();
        set_diff1(&mut a, b);
        a
    }
}
