/// A persistent Red-Black Tree
/// https://sites.google.com/view/comparison-dynamic-bst/tango-trees/red-black-trees
use std::{
    cmp::Ordering::{self, *},
    rc::Rc,
};

#[derive(Clone)]
enum Color {
    Red,
    Black,
}

enum Side {
    Left,
    Right,
}

enum InsertState<T> {
    Resolved(T),
    SingleRed(T),
    DoubleRed(T, Side, T), // node, side of child, child
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

impl<T> InsertState<T> {
    fn higher(&self) -> bool {
        match self {
            Self::DoubleRed(_, _, _) => true,
            Self::Resolved(_) | Self::SingleRed(_) => false,
        }
    }
}

impl Side {
    fn other(&self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> Node<K, I> {
    fn rotate_left_half(&mut self, right_child: Self) -> Self {
        let mut left_child = std::mem::replace(self, right_child);
        std::mem::swap(&mut left_child.right, &mut self.left);
        left_child
    }
    fn rotate_right_half(&mut self, left_child: Self) -> Self {
        let mut right_child = std::mem::replace(self, left_child);
        std::mem::swap(&mut self.right, &mut right_child.left);
        right_child
    }
    fn rotate_left(&mut self, right_child: Node<K, I>) {
        let mut left_child = self.rotate_left_half(right_child);
        left_child.update();
        self.left = left_child.into();
    }
    fn rotate_right(&mut self, left_child: Node<K, I>) {
        let mut right_child = self.rotate_right_half(left_child);
        right_child.update();
        self.right = right_child.into();
    }
    fn set_child(&mut self, child_side: &Side, child: Self) {
        match child_side {
            Side::Left => self.right = child.into(),
            Side::Right => self.left = child.into(),
        }
    }
    // Deletion: https://medium.com/analytics-vidhya/deletion-in-red-black-rb-tree-92301e1474ea
    fn del_case3(&mut self, child_side: &Side, mut other_child: Self) -> DeleteState {
        other_child.color = Color::Red;
        self.set_child(&child_side.other(), other_child);
        match self.color {
            Color::Black => DeleteState::DoubleBlack,
            Color::Red => {
                self.color = Color::Black;
                DeleteState::Resolved
            }
        }
    }
    fn del_case6(
        &mut self,
        child_side: &Side,
        mut other_child: Self,
        mut other_child_far: Self,
    ) -> DeleteState {
        std::mem::swap(&mut self.color, &mut other_child.color);
        other_child_far.color = Color::Black;
        match child_side {
            Side::Left => {
                self.rotate_left(other_child);
                self.right = other_child_far.into();
            }
            Side::Right => {
                self.rotate_right(other_child);
                self.left = other_child_far.into();
            }
        }
        DeleteState::Resolved
    }
    fn del_case5(
        &mut self,
        child_side: &Side,
        mut other_child: Self,
        mut other_child_near: Self,
    ) -> DeleteState {
        std::mem::swap(&mut other_child.color, &mut other_child_near.color);
        let other_child_far = match child_side {
            Side::Left => other_child.rotate_right_half(other_child_near),
            Side::Right => other_child.rotate_left_half(other_child_near),
        };
        self.del_case6(child_side, other_child, other_child_far)
    }
    fn del_black_sibling(&mut self, child_side: &Side) -> DeleteState {
        // case 3 || case 5 || case 6
        assert!(matches!(self.left.root_color(), Color::Black));
        assert!(matches!(self.right.root_color(), Color::Black));
        // Because child is double black, other_child must be of height >= 2 and at least contain a real node
        let other_child = match child_side {
            Side::Left => &mut self.right,
            Side::Right => &mut self.left,
        }
        .root
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
        let (other_child_near, other_child_far) = match &child_side {
            Side::Left => (&other_child.left, &other_child.right),
            Side::Right => (&other_child.right, &other_child.left),
        };
        if let Some(x) = other_child_far.is_red() {
            let x = x.clone();
            self.del_case6(child_side, other_child, x)
        } else if let Some(x) = other_child_near.is_red() {
            let x = x.clone();
            self.del_case5(child_side, other_child, x)
        } else {
            self.del_case3(child_side, other_child)
        }
    }
    fn del_fixup(&mut self, state: DeleteState, child_side: &Side) -> DeleteState {
        if let DeleteState::Resolved = state {
            return DeleteState::Resolved;
        }
        // child is double black
        match &child_side {
            Side::Left => assert!(matches!(self.left.root_color(), Color::Black)),
            Side::Right => assert!(matches!(self.right.root_color(), Color::Black)),
        }
        let other_child = match child_side {
            Side::Left => &self.right,
            Side::Right => &self.left,
        };
        if let Some(other_child) = other_child.is_red() {
            // case 4
            let mut other_child = other_child.clone();
            std::mem::swap(&mut self.color, &mut other_child.color);
            let mut new_child = match &child_side {
                Side::Left => self.rotate_left_half(other_child),
                Side::Right => self.rotate_right_half(other_child),
            };
            let state = new_child.del_black_sibling(child_side);
            match child_side {
                Side::Left => self.left = new_child.into(),
                Side::Right => self.right = new_child.into(),
            }
            if let DeleteState::Resolved = state {
                return DeleteState::Resolved;
            }
        }
        self.del_black_sibling(child_side)
    }
}

impl<K: Clone, I: SearchInfo<K>> From<InsertState<Node<K, I>>> for SubTree<K, I> {
    fn from(value: InsertState<Node<K, I>>) -> Self {
        use InsertState::*;
        match value {
            Resolved(x) | SingleRed(x) => x,
            DoubleRed(mut x, side, son) => {
                // black height becomes greater
                x.color = Color::Black;
                x.set_child(&side, son);
                x
            }
        }
        .into()
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
    fn set_fixup(
        mut node: Node<K, I>,
        state: InsertState<Node<K, I>>,
        child_side: Side,
    ) -> InsertState<Node<K, I>> {
        use InsertState::*;
        match state {
            Resolved(child) => {
                node.set_child(&child_side, child);
                Resolved(node)
            }
            SingleRed(child) => DoubleRed(node, child_side, child),
            DoubleRed(mut child, grandson_side, grandson) => {
                assert!(matches!(node.color, Color::Black));
                let other_child_ref = match &child_side {
                    Side::Left => &mut node.right,
                    Side::Right => &mut node.left,
                };
                if let Some(other_child) = other_child_ref.is_red() {
                    let mut other_child = other_child.clone();
                    other_child.color = Color::Black;
                    child.color = Color::Black;
                    node.color = Color::Red;
                    *other_child_ref = other_child.into();
                    node.set_child(&child_side, child);
                    SingleRed(node)
                } else {
                    match (&child_side, grandson_side) {
                        (Side::Left, Side::Right) => child.rotate_left(grandson),
                        (Side::Right, Side::Left) => child.rotate_right(grandson),
                        (Side::Left, Side::Left) => child.left = grandson.into(),
                        (Side::Right, Side::Right) => child.right = grandson.into(),
                    }
                    child.color = Color::Black;
                    node.color = Color::Red;
                    match child_side {
                        Side::Left => node.rotate_right(child),
                        Side::Right => node.rotate_left(child),
                    }
                    Resolved(node)
                }
            }
        }
    }
    fn insert_node(
        &mut self,
        mut inserter: impl Searcher<K, I> + Into<K>,
    ) -> InsertState<Node<K, I>> {
        let mut node = if let Some(x) = std::mem::replace(&mut self.root, None).as_ref() {
            x.as_ref().clone()
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
                return InsertState::Resolved(node);
            }
            Less => (&mut node.left, Side::Left),
            Greater => (&mut node.right, Side::Right),
        };
        let state = child.insert_node(inserter);
        Self::set_fixup(node, state, child_side)
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
    fn join_nodes(left: Tree<K, I>, mid: K, right: Tree<K, I>) -> InsertState<Node<K, I>> {
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
        let (node, node_bh) = match child_side {
            Side::Left => (&right, right.height),
            Side::Right => (&left, left.height),
        };
        let node = node.tree.root.as_ref().unwrap().as_ref().clone();
        let child_bh = match node.color {
            Color::Black => node_bh - 1,
            Color::Red => node_bh,
        };
        let state = match child_side {
            Side::Left => Self::join_nodes(
                left,
                mid,
                Tree {
                    tree: node.left.clone(),
                    height: child_bh,
                },
            ),
            Side::Right => Self::join_nodes(
                Tree {
                    tree: node.right.clone(),
                    height: child_bh,
                },
                mid,
                right,
            ),
        };
        Self::set_fixup(node, state, child_side)
    }
    fn pop_front(&mut self) -> Option<(DeleteState, K)> {
        let mut node = self.root.as_ref()?.as_ref().clone();
        match node.left.pop_front() {
            Some((state, key)) => {
                let state = node.del_fixup(state, &Side::Left);
                *self = node.into();
                Some((state, key))
            }
            None => match node.color {
                // delete self
                Color::Red => {
                    assert!(matches!(node.left.root, None));
                    assert!(matches!(node.right.root, None));
                    self.root = None;
                    Some((DeleteState::Resolved, node.key))
                }
                Color::Black => {
                    *self = node.right;
                    let key = node.key;
                    if let Some(node) = self.root.as_ref() {
                        let mut node = node.as_ref().clone();
                        assert!(matches!(node.color, Color::Red));
                        node.color = Color::Black;
                        *self = node.into();
                        Some((DeleteState::Resolved, key))
                    } else {
                        Some((DeleteState::DoubleBlack, key))
                    }
                }
            },
        }
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
                        let state = node.del_fixup(state, &Side::Right);
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
                                node.color = Color::Black;
                                *self = node.into();
                                Resolved
                            } else {
                                DoubleBlack
                            }
                        }
                    }
                };
            }
            Less => (&mut node.left, Side::Left),
            Greater => (&mut node.right, Side::Right),
        };
        let state = child.del(key);
        let state = node.del_fixup(state, &child_side);
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
