/// A persistent Red-Black Tree
/// https://sites.google.com/view/comparison-dynamic-bst/tango-trees/red-black-trees
use std::{
    cmp::Ordering::{self, *},
    marker::PhantomData,
    rc::Rc,
};

use crate::util::rc_take;

#[derive(Clone)]
enum Color {
    Red,
    Black,
}

enum Side {
    Left,
    Right,
}

enum InsertState<I: SearchInfo> {
    Resolved(SubTree<I>),
    SingleRed(Node<I>),
    DoubleRed(Node<I>, bool, Node<I>), // node, side of child, child
}

enum DeleteState<K> {
    DoubleBlack(K),
    Resolved(K),
    NotFound,
}

pub trait SearchInfo: Clone {
    type Key: Clone;
    fn new(left: Option<&Self>, key: &Self::Key, right: Option<&Self>) -> Self;
}

pub trait Searcher {
    type Info: SearchInfo;
    fn cmp(
        &mut self,
        left: Option<&Self::Info>,
        key: &<Self::Info as SearchInfo>::Key,
        right: Option<&Self::Info>,
    ) -> Ordering;
}

#[derive(Clone)]
struct Node<I: SearchInfo> {
    key: I::Key,
    info: I,
    color: Color,
    left: SubTree<I>,
    right: SubTree<I>,
}

/// Requirements:
/// 1. A red node does not have a red child
/// 2. Every path from root to leaf has the same number of black nodes
#[derive(Clone)]
pub struct SubTree<I: SearchInfo> {
    root: Option<Rc<Node<I>>>,
}

#[derive(Clone)]
pub struct Tree<I: SearchInfo> {
    tree: SubTree<I>,
    height: usize, // the black height of the tree
}

pub struct Iter<'a, I: SearchInfo> {
    stack: Vec<&'a Node<I>>,
}

impl<I: SearchInfo> Node<I> {
    fn update(&mut self) {
        self.info = I::new(self.left.info(), &self.key, self.right.info());
    }
}

impl<I: SearchInfo> From<Node<I>> for SubTree<I> {
    fn from(mut value: Node<I>) -> Self {
        value.update();
        SubTree {
            root: Some(Rc::new(value)),
        }
    }
}

impl<I: SearchInfo> InsertState<I> {
    fn higher(&self) -> bool {
        match self {
            Self::DoubleRed(_, _, _) => true,
            Self::Resolved(_) | Self::SingleRed(_) => false,
        }
    }
}

const LEFT: bool = false;
const RIGHT: bool = true;

impl<I: SearchInfo> Node<I> {
    fn child_mut<const SIDE: bool>(&mut self) -> (&mut SubTree<I>, &mut SubTree<I>) {
        match SIDE {
            LEFT => (&mut self.left, &mut self.right),
            RIGHT => (&mut self.right, &mut self.left),
        }
    }
    fn rot<const SIDE: bool>(&mut self, other_child: Self) -> Self {
        let mut new_child = std::mem::replace(self, other_child);
        std::mem::swap(new_child.child_mut::<SIDE>().1, self.child_mut::<SIDE>().0);
        new_child
    }
    fn rot_embed<const SIDE: bool>(&mut self, other_child: Self) {
        *self.child_mut::<SIDE>().0 = self.rot::<SIDE>(other_child).into();
    }
    fn cmp(&self, searcher: &mut impl Searcher<Info = I>) -> Ordering {
        searcher.cmp(self.left.info(), &self.key, self.right.info())
    }
    fn set_fixup<const SIDE: bool>(mut self, state: InsertState<I>) -> InsertState<I> {
        use InsertState::*;
        match state {
            Resolved(child) => {
                *self.child_mut::<SIDE>().0 = child;
                Resolved(self.into())
            }
            SingleRed(child) => DoubleRed(self, SIDE, child),
            DoubleRed(mut child, grandson_side, grandson) => {
                assert!(matches!(self.color, Color::Black));
                let other_child_ref = self.child_mut::<SIDE>().1;
                if let Some(other_child) = other_child_ref.is_red() {
                    let mut other_child = other_child.clone();
                    other_child.color = Color::Black;
                    child.color = Color::Black;
                    *other_child_ref = other_child.into();
                    self.color = Color::Red;
                    *self.child_mut::<SIDE>().0 = child.into();
                    SingleRed(self)
                } else {
                    match (SIDE, grandson_side) {
                        (LEFT, RIGHT) => child.rot_embed::<LEFT>(grandson),
                        (RIGHT, LEFT) => child.rot_embed::<RIGHT>(grandson),
                        (LEFT, LEFT) => child.left = grandson.into(),
                        (RIGHT, RIGHT) => child.right = grandson.into(),
                    }
                    child.color = Color::Black;
                    self.color = Color::Red;
                    match SIDE {
                        LEFT => self.rot_embed::<RIGHT>(child),
                        RIGHT => self.rot_embed::<LEFT>(child),
                    }
                    Resolved(self.into())
                }
            }
        }
    }
    fn insert_node<const SIDE: bool>(
        mut self,
        key: impl Searcher<Info = I> + Into<I::Key>,
    ) -> InsertState<I> {
        let state = self.child_mut::<SIDE>().0.insert_node(key);
        self.set_fixup::<SIDE>(state)
    }
    // Deletion: https://medium.com/analytics-vidhya/deletion-in-red-black-rb-tree-92301e1474ea
    fn del_case6<const SIDE: bool>(
        &mut self,
        mut other_child: Self,
        mut other_child_far: Self,
        key: I::Key,
    ) -> DeleteState<I::Key> {
        other_child_far.color = Color::Black;
        *other_child.child_mut::<SIDE>().1 = other_child_far.into();
        std::mem::swap(&mut self.color, &mut other_child.color);
        self.rot_embed::<SIDE>(other_child);
        DeleteState::Resolved(key)
    }
    fn del_black_sibling<const SIDE: bool>(&mut self, key: I::Key) -> DeleteState<I::Key> {
        // case 3 || case 5 || case 6
        assert!(matches!(self.left.root_color(), Color::Black));
        assert!(matches!(self.right.root_color(), Color::Black));
        // Because child is double black, other_child must be of height >= 2 and at least contain a real node
        let mut other_child = self
            .child_mut::<SIDE>()
            .1
            .root
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        let (other_child_near, other_child_far) = other_child.child_mut::<SIDE>();
        if let Some(x) = other_child_far.is_red() {
            let x = x.clone();
            self.del_case6::<SIDE>(other_child, x, key)
        } else if let Some(other_child_near) = other_child_near.is_red() {
            // case 5
            let mut other_child_near = other_child_near.clone();
            std::mem::swap(&mut other_child.color, &mut other_child_near.color);
            let other_child_far = match SIDE {
                LEFT => other_child.rot::<RIGHT>(other_child_near),
                RIGHT => other_child.rot::<LEFT>(other_child_near),
            };
            self.del_case6::<SIDE>(other_child, other_child_far, key)
        } else {
            // case 3
            other_child.color = Color::Red;
            *self.child_mut::<SIDE>().1 = other_child.into();
            match self.color {
                Color::Black => DeleteState::DoubleBlack(key),
                Color::Red => {
                    self.color = Color::Black;
                    DeleteState::Resolved(key)
                }
            }
        }
    }
    fn del_fixup<const SIDE: bool>(&mut self, state: DeleteState<I::Key>) -> DeleteState<I::Key> {
        use DeleteState::*;
        let mut key = match state {
            Resolved(k) => return Resolved(k),
            NotFound => return NotFound,
            DoubleBlack(k) => k,
        };
        // child is double black
        assert!(matches!(
            self.child_mut::<SIDE>().0.root_color(),
            Color::Black
        ));
        let other_child = self.child_mut::<SIDE>().1;
        if let Some(other_child) = other_child.is_red() {
            // case 4
            let mut other_child = other_child.clone();
            std::mem::swap(&mut self.color, &mut other_child.color);
            let mut new_child = self.rot::<SIDE>(other_child);
            let state = new_child.del_black_sibling::<SIDE>(key);
            *self.child_mut::<SIDE>().0 = new_child.into();
            key = match state {
                Resolved(k) => return Resolved(k),
                DoubleBlack(k) => k,
                NotFound => unreachable!(),
            };
        }
        self.del_black_sibling::<SIDE>(key)
    }
    fn del_side<const SIDE: bool>(&mut self, key: impl Searcher<Info = I>) -> DeleteState<I::Key> {
        let state = self.child_mut::<SIDE>().0.del(key);
        self.del_fixup::<SIDE>(state)
    }
}

impl<I: SearchInfo> From<InsertState<I>> for SubTree<I> {
    fn from(value: InsertState<I>) -> Self {
        use InsertState::*;
        match value {
            Resolved(x) => x,
            SingleRed(x) => x.into(),
            DoubleRed(mut x, side, son) => {
                // black height becomes greater
                x.color = Color::Black;
                match side {
                    LEFT => x.left = son.into(),
                    RIGHT => x.right = son.into(),
                }
                x.into()
            }
        }
    }
}

impl<I: SearchInfo> SubTree<I> {
    pub fn new() -> Self {
        SubTree { root: None }
    }
    fn info(&self) -> Option<&I> {
        self.root.as_ref().map(|x| &x.info)
    }
    fn root_color(&self) -> &Color {
        self.root.as_ref().map_or(&Color::Black, |x| &x.color)
    }
    fn is_red(&self) -> Option<&Node<I>> {
        let t = self.root.as_ref()?;
        if let Color::Red = t.color {
            Some(t.as_ref())
        } else {
            None
        }
    }
    fn insert_node(&mut self, mut key: impl Searcher<Info = I> + Into<I::Key>) -> InsertState<I> {
        let mut node = if let Some(x) = self.root.take() {
            rc_take(x)
        } else {
            let key = key.into();
            let info = I::new(None, &key, None);
            return InsertState::SingleRed(Node {
                key,
                info,
                color: Color::Red,
                left: SubTree::new(),
                right: SubTree::new(),
            });
        };
        match node.cmp(&mut key) {
            Equal => {
                node.key = key.into();
                InsertState::Resolved(node.into())
            }
            Less => node.insert_node::<LEFT>(key),
            Greater => node.insert_node::<RIGHT>(key),
        }
    }
    pub fn get(&self, mut key: impl Searcher<Info = I>) -> Option<&I::Key> {
        let node = if let Some(x) = self.root.as_ref() {
            x.as_ref()
        } else {
            return None;
        };
        match node.cmp(&mut key) {
            Equal => Some(&node.key),
            Less => node.left.get(key),
            Greater => node.right.get(key),
        }
    }
    fn del(&mut self, mut key: impl Searcher<Info = I>) -> DeleteState<I::Key> {
        use DeleteState::*;
        let mut node = if let Some(x) = self.root.take() {
            x
        } else {
            return NotFound;
        };
        let state = match node.cmp(&mut key) {
            Equal => {
                let mut right = node.right.clone();
                let mut state = right.del(LeftmostSearcher(PhantomData));
                match &mut state {
                    DoubleBlack(state_key) | Resolved(state_key) => {
                        let node = Rc::make_mut(&mut node);
                        std::mem::swap(state_key, &mut node.key);
                        node.right = right;
                        node.del_fixup::<RIGHT>(state)
                    }
                    NotFound => match node.color {
                        Color::Red => {
                            assert!(matches!(node.left.root, None));
                            assert!(matches!(node.right.root, None));
                            return Resolved(node.key.clone());
                        }
                        Color::Black => {
                            let (mut left, key) = Rc::try_unwrap(node).map_or_else(
                                |e| (e.left.clone(), e.key.clone()),
                                |v| (v.left, v.key),
                            );
                            if let Some(left_node) = left.root.take() {
                                node = left_node.clone();
                                let node = Rc::make_mut(&mut node);
                                assert!(matches!(node.color, Color::Red));
                                assert!(matches!(node.left.root, None));
                                assert!(matches!(node.right.root, None));
                                node.color = Color::Black;
                                Resolved(key)
                            } else {
                                return DoubleBlack(key);
                            }
                        }
                    },
                }
            }
            Less => Rc::make_mut(&mut node).del_side::<LEFT>(key),
            Greater => Rc::make_mut(&mut node).del_side::<RIGHT>(key),
        };
        self.root.replace(node);
        return state;
    }
}

impl<I: SearchInfo> Tree<I> {
    pub fn new() -> Self {
        Tree {
            tree: SubTree::new(),
            height: 0,
        }
    }
    pub fn set(&mut self, key: impl Searcher<Info = I> + Into<I::Key>) {
        let state = self.tree.insert_node(key);
        if state.higher() {
            self.height += 1;
        }
        self.tree = state.into();
    }
    pub fn get(&self, key: impl Searcher<Info = I>) -> Option<&I::Key> {
        self.tree.get(key)
    }
    pub fn del(&mut self, key: impl Searcher<Info = I>) -> Option<I::Key> {
        let state = self.tree.del(key);
        match state {
            DeleteState::DoubleBlack(k) => {
                self.height -= 1;
                Some(k)
            }
            DeleteState::Resolved(k) => Some(k),
            DeleteState::NotFound => None,
        }
    }
    fn join(self, mid: I::Key, right: Self) -> Self {
        let height = std::cmp::max(self.height, right.height);
        let state = Self::join_nodes(self, mid, right);
        let height = if state.higher() { height + 1 } else { height };
        Tree {
            tree: state.into(),
            height,
        }
    }
    fn join_nodes(mut left: Self, mid: I::Key, mut right: Self) -> InsertState<I> {
        use InsertState::*;
        let child_side = match left.height.cmp(&right.height) {
            Equal => match (left.tree.root_color(), right.tree.root_color()) {
                (Color::Black, Color::Black) => {
                    let info = I::new(left.tree.info(), &mid, right.tree.info());
                    return SingleRed(Node {
                        key: mid,
                        info,
                        color: Color::Red,
                        left: left.tree,
                        right: right.tree,
                    });
                }
                (Color::Red, _) => RIGHT,
                (Color::Black, Color::Red) => LEFT,
            },
            Less => LEFT,
            Greater => RIGHT,
        };
        match child_side {
            LEFT => Self::join_side::<LEFT>(left, mid, right),
            RIGHT => Self::join_side::<RIGHT>(left, mid, right),
        }
    }
    fn join_side<const SIDE: bool>(mut left: Self, mid: I::Key, mut right: Self) -> InsertState<I> {
        let node = match SIDE {
            LEFT => &mut right,
            RIGHT => &mut left,
        };
        let node_bh = node.height;
        let node = rc_take(node.tree.root.take().unwrap());
        let child_bh = match node.color {
            Color::Black => node_bh - 1,
            Color::Red => node_bh,
        };
        match SIDE {
            LEFT => {
                right = Tree {
                    tree: node.left.clone(),
                    height: child_bh,
                }
            }
            RIGHT => {
                left = Tree {
                    tree: node.right.clone(),
                    height: child_bh,
                }
            }
        };
        let state = Self::join_nodes(left, mid, right);
        match SIDE {
            LEFT => node.set_fixup::<LEFT>(state),
            RIGHT => node.set_fixup::<RIGHT>(state),
        }
    }
    pub fn cat(self, mut rhs: Self) -> Self {
        if let Some(key) = rhs.del(LeftmostSearcher(PhantomData)) {
            self.join(key, rhs)
        } else {
            self
        }
    }
    pub fn cut(mut self, mut key: impl Searcher<Info = I>) -> (Tree<I>, Tree<I>) {
        let node = if let Some(x) = self.tree.root.take() {
            x
        } else {
            return (Tree::new(), Tree::new());
        };
        let child_side = match node.cmp(&mut key) {
            Equal => unreachable!("Cannot split at existing node"),
            Less => Side::Left,
            Greater => Side::Right,
        };
        let child = match child_side {
            Side::Left => &node.left,
            Side::Right => &node.right,
        };
        let child_bh = match child.root_color() {
            Color::Black => self.height - 1,
            Color::Red => self.height,
        };
        let (left, right) = Tree {
            tree: child.clone(),
            height: child_bh,
        }
        .cut(key);
        match child_side {
            Side::Left => (
                left,
                Self::join(
                    right,
                    node.key.clone(),
                    Tree {
                        tree: node.right.clone(),
                        height: child_bh,
                    },
                ),
            ),
            Side::Right => (
                Self::join(
                    Tree {
                        tree: node.left.clone(),
                        height: child_bh,
                    },
                    node.key.clone(),
                    left,
                ),
                right,
            ),
        }
    }
    pub fn iter(&self) -> Iter<'_, I> {
        let mut iter = Iter { stack: Vec::new() };
        iter.push_all_left(&self.tree);
        iter
    }
}

impl<'a, I: SearchInfo> Iter<'a, I> {
    fn push_all_left(&mut self, mut x: &'a SubTree<I>) {
        while let Some(v) = x.root.as_ref() {
            self.stack.push(v.as_ref());
            x = &v.left;
        }
    }
}

impl<'a, I: SearchInfo> Iterator for Iter<'a, I> {
    type Item = &'a I::Key;
    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.stack.pop()?;
        self.push_all_left(&cur.right);
        Some(&cur.key)
    }
}

struct LeftmostSearcher<I: SearchInfo>(PhantomData<I>);

impl<I: SearchInfo> Searcher for LeftmostSearcher<I> {
    type Info = I;
    fn cmp(&mut self, left: Option<&I>, _: &I::Key, _: Option<&I>) -> Ordering {
        if let Some(_) = left {
            Less
        } else {
            Equal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type I = IntInfo;
    type K = usize;
    struct IntSearch {
        key: K,
    }
    #[derive(Clone)]
    struct IntInfo();
    impl Searcher for IntSearch {
        type Info = I;
        fn cmp(&mut self, _: Option<&I>, key: &K, _: Option<&I>) -> Ordering {
            self.key.cmp(key)
        }
    }
    impl Into<K> for IntSearch {
        fn into(self) -> K {
            self.key
        }
    }
    impl SearchInfo for IntInfo {
        type Key = K;
        fn new(_: Option<&Self>, _: &K, _: Option<&Self>) -> Self {
            Self()
        }
    }
    fn sanity_check(x: &SubTree<I>, bh: usize) {
        let x = if let Some(x) = x.root.as_ref() {
            x.as_ref()
        } else {
            assert_eq!(bh, 0);
            return;
        };
        let child_bh = match x.color {
            Color::Black => {
                assert!(bh > 0);
                bh - 1
            }
            Color::Red => bh,
        };
        assert!(matches!(x.color, Color::Black) || matches!(x.left.root_color(), Color::Black));
        sanity_check(&x.left, child_bh);
        assert!(matches!(x.color, Color::Black) || matches!(x.right.root_color(), Color::Black));
        sanity_check(&x.right, child_bh);
    }
    #[test]
    fn test_ordered_insert() {
        const N: usize = 1000;
        let mut tree = Tree::new();
        for i in 0..N {
            tree.set(IntSearch { key: i });
            sanity_check(&tree.tree, tree.height);
        }
        for (x, i) in tree.iter().zip(0..N) {
            assert_eq!(*x, i);
        }
        tree = Tree::new();
        for i in (0..N).rev() {
            tree.set(IntSearch { key: i });
            sanity_check(&tree.tree, tree.height);
        }
        for (x, i) in tree.iter().zip(0..N) {
            assert_eq!(*x, i);
        }
    }
}
