/// A persistent Red-Black Tree
/// https://sites.google.com/view/comparison-dynamic-bst/tango-trees/red-black-trees
use std::{
    cmp::Ordering::{self, *},
    rc::Rc,
};

use crate::util::rc_move_or_clone;

#[derive(Clone)]
enum Color {
    Red,
    Black,
}

enum Side {
    Left,
    Right,
}

enum InsertState<K: Clone, I: SearchInfo<K>> {
    Resolved(SubTree<K, I>),
    SingleRed(Node<K, I>),
    DoubleRed(Node<K, I>, bool, Node<K, I>), // node, side of child, child
}

enum DeleteState {
    DoubleBlack,
    Resolved,
}

pub trait SearchInfo<K>: Clone {
    fn new(left: Option<&Self>, key: &K, right: Option<&Self>) -> Self;
}

pub trait Searcher<K, I: SearchInfo<K>> {
    fn cmp(&mut self, key: &K, info: &I) -> Ordering;
}

struct Node<K: Clone, I: SearchInfo<K>> {
    key: K,
    info: I,
    color: Color,
    left: SubTree<K, I>,
    right: SubTree<K, I>,
}

/// Requirements:
/// 1. A red node does not have a red child
/// 2. Every path from root to leaf has the same number of black nodes
pub struct SubTree<K: Clone, I: SearchInfo<K>> {
    root: Option<Rc<Node<K, I>>>,
}

#[derive(Clone)]
pub struct Tree<K: Clone, I: SearchInfo<K>> {
    tree: SubTree<K, I>,
    height: usize, // the black height of the tree
}

pub struct Iter<'a, K: Clone, I: SearchInfo<K>> {
    stack: Vec<&'a Node<K, I>>,
}

impl<K: Clone, I: SearchInfo<K>> Node<K, I> {
    fn update(&mut self) {
        self.info = I::new(
            self.left.root.as_ref().map(|x| &x.info),
            &self.key,
            self.right.root.as_ref().map(|x| &x.info),
        );
    }
}

impl<K: Clone, I: SearchInfo<K>> Clone for Node<K, I> {
    fn clone(&self) -> Self {
        Node {
            key: self.key.clone(),
            color: self.color.clone(),
            info: self.info.clone(),
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> Clone for SubTree<K, I> {
    fn clone(&self) -> Self {
        SubTree {
            root: self.root.clone(),
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> From<Node<K, I>> for SubTree<K, I> {
    fn from(mut value: Node<K, I>) -> Self {
        value.update();
        SubTree {
            root: Some(Rc::new(value)),
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> InsertState<K, I> {
    fn higher(&self) -> bool {
        match self {
            Self::DoubleRed(_, _, _) => true,
            Self::Resolved(_) | Self::SingleRed(_) => false,
        }
    }
}

const LEFT: bool = false;
const RIGHT: bool = true;

impl<K: Clone, I: SearchInfo<K>> Node<K, I> {
    fn child_mut<const SIDE: bool>(&mut self) -> (&mut SubTree<K, I>, &mut SubTree<K, I>) {
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
    // Deletion: https://medium.com/analytics-vidhya/deletion-in-red-black-rb-tree-92301e1474ea
    fn del_case6<const SIDE: bool>(
        &mut self,
        mut other_child: Self,
        mut other_child_far: Self,
    ) -> DeleteState {
        other_child_far.color = Color::Black;
        *other_child.child_mut::<SIDE>().1 = other_child_far.into();
        std::mem::swap(&mut self.color, &mut other_child.color);
        self.rot_embed::<SIDE>(other_child);
        DeleteState::Resolved
    }
    fn del_black_sibling<const SIDE: bool>(&mut self) -> DeleteState {
        // case 3 || case 5 || case 6
        assert!(matches!(self.left.root_color(), Color::Black));
        assert!(matches!(self.right.root_color(), Color::Black));
        // Because child is double black, other_child must be of height >= 2 and at least contain a real node
        let mut other_child = self.child_mut::<SIDE>().1
        .root
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
        let (other_child_near, other_child_far) = other_child.child_mut::<SIDE>();
        if let Some(x) = other_child_far.is_red() {
            let x = x.clone();
            self.del_case6::<SIDE>(other_child, x)
        } else if let Some(other_child_near) = other_child_near.is_red() {
            // case 5
            let mut other_child_near = other_child_near.clone();
            std::mem::swap(&mut other_child.color, &mut other_child_near.color);
            let other_child_far = match SIDE {
                LEFT => other_child.rot::<RIGHT>(other_child_near),
                RIGHT => other_child.rot::<LEFT>(other_child_near),
            };
            self.del_case6::<SIDE>(other_child, other_child_far)
        } else {
            // case 3
            other_child.color = Color::Red;
            *self.child_mut::<SIDE>().1 = other_child.into();
            match self.color {
                Color::Black => DeleteState::DoubleBlack,
                Color::Red => {
                    self.color = Color::Black;
                    DeleteState::Resolved
                }
            }
        }
    }
    fn del_fixup<const SIDE: bool>(&mut self, state: DeleteState) -> DeleteState {
        if let DeleteState::Resolved = state {
            return DeleteState::Resolved;
        }
        // child is double black
        assert!(matches!(self.child_mut::<SIDE>().0.root_color(), Color::Black));
        let other_child = self.child_mut::<SIDE>().1;
        if let Some(other_child) = other_child.is_red() {
            // case 4
            let mut other_child = other_child.clone();
            std::mem::swap(&mut self.color, &mut other_child.color);
            let mut new_child = self.rot::<SIDE>(other_child);
            let state = new_child.del_black_sibling::<SIDE>();
            *self.child_mut::<SIDE>().0 = new_child.into();
            if let DeleteState::Resolved = state {
                return DeleteState::Resolved;
            }
        }
        self.del_black_sibling::<SIDE>()
    }
}

impl<K: Clone, I: SearchInfo<K>> From<InsertState<K, I>> for SubTree<K, I> {
    fn from(value: InsertState<K, I>) -> Self {
        use InsertState::*;
        match value {
            Resolved(x) => x,
            SingleRed(x) => x.into(),
            DoubleRed(mut x, side, son) => {
                // black height becomes greater
                x.color = Color::Black;
                match side {
                    Side::Left => x.left = son.into(),
                    Side::Right => x.right = son.into(),
                }
                x.into()
            }
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> SubTree<K, I> {
    pub fn new() -> Self {
        SubTree { root: None }
    }
    fn root_color(&self) -> &Color {
        self.root.as_ref().map_or(&Color::Black, |x| &x.color)
    }
    fn is_red(&self) -> Option<&Node<K, I>> {
        let t = self.root.as_ref()?;
        if let Color::Red = t.color {
            Some(t.as_ref())
        } else {
            None
        }
    }
    fn set_fixup<const SIDE: bool>(
        mut node: Node<K, I>,
        state: InsertState<K, I>,
    ) -> InsertState<K, I> {
        use InsertState::*;
        match state {
            Resolved(child) => {
                *node.child_mut::<SIDE>().0 = child;
                Resolved(node.into())
            }
            SingleRed(child) => DoubleRed(node, SIDE, child),
            DoubleRed(mut child, grandson_side, grandson) => {
                assert!(matches!(node.color, Color::Black));
                let other_child_ref = node.child_mut::<SIDE>().1;
                if let Some(other_child) = other_child_ref.is_red() {
                    let mut other_child = other_child.clone();
                    other_child.color = Color::Black;
                    child.color = Color::Black;
                    node.color = Color::Red;
                    *other_child_ref = other_child.into();
                    *node.child_mut::<SIDE>().0 = child;
                    SingleRed(node)
                } else {
                    match (SIDE, grandson_side) {
                        (LEFT, Side::Right) => child.rot_embed::<LEFT>(grandson),
                        (RIGHT, Side::Left) => child.rot_embed::<RIGHT>(grandson),
                        (LEFT, Side::Left) => child.left = grandson.into(),
                        (RIGHT, Side::Right) => child.right = grandson.into(),
                    }
                    child.color = Color::Black;
                    node.color = Color::Red;
                    match SIDE {
                        LEFT => node.rot_embed::<RIGHT>(child),
                        RIGHT => node.rot_embed::<LEFT>(child),
                    }
                    Resolved(node.into())
                }
            }
        }
    }
    fn insert_node(
        &mut self,
        mut inserter: impl Searcher<K, I> + Into<K>,
    ) -> InsertState<K, I> {
        let mut node = if let Some(x) = self.root.take() {
            rc_move_or_clone(x)
        } else {
            let key = inserter.into();
            let info = I::new(None, &key, None);
            return InsertState::SingleRed(Node {
                key,
                info,
                color: Color::Red,
                left: SubTree::new(),
                right: SubTree::new(),
            });
        };
        let (child, child_side) = match inserter.cmp(&node.key, &node.info) {
            Equal => {
                node.key = inserter.into();
                return InsertState::Resolved(node.into());
            }
            Less => (&mut node.left, Side::Left),
            Greater => (&mut node.right, Side::Right),
        };
        let state = child.insert_node(inserter);
        match child_side {
            Side::Left => Self::set_fixup::<LEFT>(node, state),
            Side::Right => Self::set_fixup::<RIGHT>(node, state),
        }
    }
    pub fn get(&self, mut key: impl Searcher<K, I>) -> Option<&K> {
        let node = if let Some(x) = self.root.as_ref() {
            x.as_ref()
        } else {
            return None;
        };
        match key.cmp(&node.key, &node.info) {
            Equal => Some(&node.key),
            Less => node.left.get(key),
            Greater => node.right.get(key),
        }
    }
    fn join_nodes(mut left: Tree<K, I>, mid: K, mut right: Tree<K, I>) -> InsertState<K, I> {
        use InsertState::*;
        let child_side = match left.height.cmp(&right.height) {
            Equal => match (left.tree.root_color(), right.tree.root_color()) {
                (Color::Black, Color::Black) => {
                    let info = I::new(
                        left.tree.root.as_ref().map(|x| &x.info),
                        &mid,
                        right.tree.root.as_ref().map(|x| &x.info),
                    );
                    return SingleRed(Node {
                        key: mid,
                        info,
                        color: Color::Red,
                        left: left.tree,
                        right: right.tree,
                    });
                }
                (Color::Red, _) => Side::Right,
                (Color::Black, Color::Red) => Side::Left,
            },
            Less => Side::Left,
            Greater => Side::Right,
        };
        let node = match child_side {
            Side::Left => &right,
            Side::Right => &left,
        };
        let node_bh = node.height;
        let node = node.tree.root.as_ref().unwrap().as_ref().clone();
        let child_bh = match node.color {
            Color::Black => node_bh - 1,
            Color::Red => node_bh,
        };
        match child_side {
            Side::Left => right = Tree {
                tree: node.left.clone(),
                height: child_bh,
            },
            Side::Right => left = Tree {
                tree: node.right.clone(),
                height: child_bh,
            },
        };
        let state = Self::join_nodes(left, mid, right);
        match child_side {
            Side::Left => Self::set_fixup::<LEFT>(node, state),
            Side::Right => Self::set_fixup::<RIGHT>(node, state),
        }
    }
    fn pop_front(&mut self) -> Option<(DeleteState, K)> {
        let mut node = self.root.as_ref()?.as_ref().clone();
        Some(match node.left.pop_front() {
            Some((state, key)) => {
                let state = node.del_fixup::<false>(state);
                *self = node.into();
                (state, key)
            }
            None => (match node.color {
                // delete self
                Color::Red => {
                    assert!(matches!(node.left.root, None));
                    assert!(matches!(node.right.root, None));
                    self.root = None;
                    DeleteState::Resolved
                }
                Color::Black => {
                    *self = node.right;
                    if let Some(node) = self.root.as_ref() {
                        let mut node = node.as_ref().clone();
                        assert!(matches!(node.color, Color::Red));
                        node.color = Color::Black;
                        *self = node.into();
                        DeleteState::Resolved
                    } else {
                        DeleteState::DoubleBlack
                    }
                }
            }, node.key),
        })
    }
    fn del(&mut self, mut key: impl Searcher<K, I>) -> DeleteState {
        use DeleteState::*;
        let mut node = if let Some(x) = self.root.as_mut() {
            x.as_ref().clone()
        } else {
            return Resolved;
        };
        let (child, child_side) = match key.cmp(&node.key, &node.info) {
            Equal => {
                let pop_state = node.right.pop_front();
                return match pop_state {
                    Some((state, key)) => {
                        node.key = key;
                        let state = node.del_fixup::<true>(state);
                        *self = node.into();
                        state
                    }
                    None => match node.color {
                        Color::Red => {
                            assert!(matches!(node.left.root, None));
                            assert!(matches!(node.right.root, None));
                            self.root = None;
                            Resolved
                        }
                        Color::Black => {
                            *self = node.left;
                            if let Some(node) = self.root.as_ref() {
                                let mut node = node.as_ref().clone();
                                assert!(matches!(node.color, Color::Red));
                                assert!(matches!(node.left.root, None));
                                assert!(matches!(node.right.root, None));
                                node.color = Color::Black;
                                *self = node.into();
                                Resolved
                            } else {
                                DoubleBlack
                            }
                        }
                    },
                };
            }
            Less => (&mut node.left, Side::Left),
            Greater => (&mut node.right, Side::Right),
        };
        let state = child.del(key);
        let state = match &child_side {
            Side::Left => node.del_fixup::<false>(state),
            Side::Right => node.del_fixup::<true>(state),
        };
        *self = node.into();
        state
    }
}

impl<K: Clone, I: SearchInfo<K>> Tree<K, I> {
    pub fn new() -> Self {
        Tree {
            tree: SubTree::new(),
            height: 0,
        }
    }
    pub fn set(&mut self, key: impl Searcher<K, I> + Into<K>) {
        let state = self.tree.insert_node(key);
        if state.higher() {
            self.height += 1;
        }
        self.tree = state.into();
    }
    pub fn get(&self, key: impl Searcher<K, I>) -> Option<&K> {
        self.tree.get(key)
    }
    pub fn del(&mut self, key: impl Searcher<K, I>) {
        let state = self.tree.del(key);
        match state {
            DeleteState::DoubleBlack => self.height -= 1,
            DeleteState::Resolved => {}
        }
    }
    fn join(self, mid: K, right: Self) -> Self {
        let height = std::cmp::max(self.height, right.height);
        let state = SubTree::join_nodes(self, mid, right);
        let height = if state.higher() { height + 1 } else { height };
        Tree {
            tree: state.into(),
            height,
        }
    }
    pub fn cat(self, mut rhs: Self) -> Self {
        if let Some((_, key)) = rhs.tree.pop_front() {
            self.join(key, rhs)
        } else {
            self
        }
    }
    pub fn cut(&self, mut key: impl Searcher<K, I>) -> (Tree<K, I>, Tree<K, I>) {
        let node = if let Some(x) = self.tree.root.as_ref() {
            x
        } else {
            return (Tree::new(), Tree::new());
        };
        let child_side = match key.cmp(&node.key, &node.info) {
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
    pub fn iter(&self) -> Iter<'_, K, I> {
        let mut iter = Iter { stack: Vec::new() };
        iter.push_all_left(&self.tree);
        iter
    }
}

impl<'a, K: Clone + 'a, I: SearchInfo<K>> Iter<'a, K, I> {
    fn push_all_left(&mut self, mut x: &'a SubTree<K, I>) {
        while let Some(v) = x.root.as_ref() {
            self.stack.push(v.as_ref());
            x = &v.left;
        }
    }
}

impl<'a, K: Clone + 'a, I: SearchInfo<K>> Iterator for Iter<'a, K, I> {
    type Item = &'a K;
    fn next(&mut self) -> Option<Self::Item> {
        let cur = self.stack.pop()?;
        self.push_all_left(&cur.right);
        Some(&cur.key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type K = usize;
    type I = IntInfo;
    struct IntSearch {
        key: K,
    }
    #[derive(Clone)]
    struct IntInfo();
    impl Searcher<K, I> for IntSearch {
        fn cmp(&mut self, key: &K, _: &I) -> Ordering {
            self.key.cmp(key)
        }
    }
    impl Into<K> for IntSearch {
        fn into(self) -> K {
            self.key
        }
    }
    impl SearchInfo<K> for IntInfo {
        fn new(_: Option<&Self>, _: &K, _: Option<&Self>) -> Self {
            Self()
        }
    }
    fn sanity_check(x: &SubTree<K, I>, bh: usize) {
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
