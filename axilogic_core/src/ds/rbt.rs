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

enum InsertState {
    Resolved,
    SingleRed,
    NewBlack,
    TwoRed,
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
    // Deletion: https://medium.com/analytics-vidhya/deletion-in-red-black-rb-tree-92301e1474ea
    fn del_case3(node: &mut Self, child_side: Side, mut other_child: Self) -> DeleteState {
        // child is double black
        assert!(matches!(match child_side {
            Side::Left => &node.left,
            Side::Right => &node.right,
        }.root_color(), Color::Black));
        assert!(matches!(other_child.color, Color::Black));
        assert!(matches!(other_child.left.root_color(), Color::Black));
        assert!(matches!(other_child.right.root_color(), Color::Black));
        other_child.color = Color::Red;
        *match child_side {
            Side::Left => &mut node.right,
            Side::Right => &mut node.left,
        } = other_child.into();
        let result = match node.color {
            Color::Black => {
                DeleteState::DoubleBlack
            }
            Color::Red => {
                node.color = Color::Black;
                DeleteState::Resolved
            }
        };
        node.update();
        result
    }
    fn del_case6(&mut self, child_side: Side, mut other_child: Self, mut other_child_far: Self) -> DeleteState {
        // child is double black
        assert!(matches!(other_child.color, Color::Black));
        assert!(matches!(other_child_far.color, Color::Red));
        match &child_side {
            Side::Left => {
                assert!(matches!(self.left.root_color(), Color::Black));
                assert!(matches!(other_child.left.root_color(), Color::Black));
            }
            Side::Right => {
                assert!(matches!(self.right.root_color(), Color::Black));
                assert!(matches!(other_child.right.root_color(), Color::Black));
            }
        }
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
    fn del_case5(&mut self, child_side: Side, mut other_child: Self, mut other_child_near: Self) -> DeleteState {
        // child is double black
        assert!(matches!(other_child.color, Color::Black));
        assert!(matches!(other_child_near.color, Color::Red));
        match &child_side {
            Side::Left => {
                assert!(matches!(self.left.root_color(), Color::Black));
                assert!(matches!(other_child.right.root_color(), Color::Black));
            }
            Side::Right => {
                assert!(matches!(self.right.root_color(), Color::Black));
                assert!(matches!(other_child.left.root_color(), Color::Black));
            }
        }
        std::mem::swap(&mut other_child.color, &mut other_child_near.color);
        let other_child_far = match child_side {
            Side::Left => other_child.rotate_right_half(other_child_near),
            Side::Right => other_child.rotate_left_half(other_child_near),
        };
        self.del_case6(child_side, other_child, other_child_far)
    }
    fn del_black_sibling(&mut self, child_side: Side) -> DeleteState {
        // child is double black
        match &child_side {
            Side::Left => {
                assert!(matches!(self.left.root_color(), Color::Black));
            }
            Side::Right => {
                assert!(matches!(self.right.root_color(), Color::Black));
            }
        }
        todo!()
    }
    fn del_case4(&mut self, child_side: Side, mut other_child: Self) -> DeleteState {
        assert!(matches!(other_child.color, Color::Red));
        std::mem::swap(&mut self.color, &mut other_child.color);
        let new_child = match child_side {
            Side::Left => self.rotate_left(other_child),
            Side::Right => self.rotate_right(other_child),
        };
        todo!("")
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
        let t = self.root?;
        if let Color::Red = t.color {
            Some(t.as_ref())
        } else {
            None
        }
    }
    // Returns whether the resulting tree is higher than the original child (in terms of black height)
    fn insert_fixup(
        state: InsertState,
        mut node: Node<K, I>,
        child_side: Side,
        child_node: Node<K, I>,
        child_higher: bool,
        parent_data: Option<(&Side, &Color)>,
    ) -> (InsertState, Node<K, I>, bool) {
        use InsertState::*;
        let (child, other_child) = match child_side {
            Side::Left => (&mut node.left, &mut node.right),
            Side::Right => (&mut node.right, &mut node.left),
        };
        assert!(!child_higher || matches!(state, InsertState::Resolved | InsertState::NewBlack));
        let (state, higher) = match (state, &node.color, parent_data) {
            // (child state, this color, (this side, sibiling color))
            (Resolved, _, _) | (SingleRed, Color::Black, _) => {
                *child = child_node.into();
                (Resolved, matches!(&node.color, Color::Black))
            }
            (SingleRed, Color::Red, None) => {
                // This is the root
                *child = child_node.into();
                node.color = Color::Black;
                (Resolved, true)
            }
            (SingleRed, Color::Red, Some((_, Color::Red))) => {
                *child = child_node.into();
                node.color = Color::Black;
                (NewBlack, true)
            }
            (SingleRed, Color::Red, Some((self_side, Color::Black))) => {
                match (self_side, child_side) {
                    (Side::Left, Side::Right) => node.rotate_left(child_node),
                    (Side::Right, Side::Left) => node.rotate_right(child_node),
                    _ => {}
                }
                (TwoRed, false)
            }
            (NewBlack, Color::Black, _) => {
                let mut other_child_node = other_child.root.as_ref().unwrap().as_ref().clone();
                assert!(matches!(other_child_node.color, Color::Red));
                other_child_node.color = Color::Black;
                *child = child_node.into();
                *other_child = other_child_node.into();
                node.color = Color::Red;
                (SingleRed, false)
            }
            (TwoRed, Color::Black, _) => {
                node.color = Color::Red;
                match child_side {
                    Side::Left => node.rotate_right(child_node),
                    Side::Right => node.rotate_left(child_node),
                }
                node.color = Color::Black;
                (Resolved, true)
            }
            (NewBlack | TwoRed, Color::Red, _) => {
                unreachable!("Red node cannot have red child")
            }
        };
        assert!(!child_higher || !higher);
        (state, node, child_higher || higher)
    }
    fn insert_node(
        &mut self,
        mut inserter: impl Searcher<K, I> + Into<K>,
        parent_data: Option<(&Side, &Color)>,
    ) -> (InsertState, Node<K, I>, bool) {
        let mut node = if let Some(x) = std::mem::replace(&mut self.root, None).as_ref() {
            x.as_ref().clone()
        } else {
            let key = inserter.into();
            let info = I::new(None, &key, None);
            return (
                InsertState::SingleRed,
                Node {
                    key,
                    info,
                    color: Color::Red,
                    left: SubTree::new(),
                    right: SubTree::new(),
                },
                false,
            );
        };
        let child_side = match inserter.cmp(&node.key, &node.info) {
            Equal => {
                node.key = inserter.into();
                return (InsertState::Resolved, node, false);
            }
            Less => Side::Left,
            Greater => Side::Right,
        };
        let (child, other_child) = match child_side {
            Side::Left => (&mut node.left, &mut node.right),
            Side::Right => (&mut node.right, &mut node.left),
        };
        let (state, child_node, child_higher) =
            child.insert_node(inserter, Some((&child_side, other_child.root_color())));
        Self::insert_fixup(
            state,
            node,
            child_side,
            child_node,
            child_higher,
            parent_data,
        )
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
    fn join_nodes(
        left: Tree<K, I>,
        mid: K,
        right: Tree<K, I>,
        parent_data: Option<(&Side, &Color)>,
    ) -> (InsertState, Node<K, I>, usize) {
        use InsertState::*;
        let child_side = match left.height.cmp(&right.height) {
            Equal => match (left.tree.root_color(), right.tree.root_color()) {
                (Color::Black, Color::Black) => {
                    let info = I::new(
                        left.tree.root.as_ref().map(|x| &x.info),
                        &mid,
                        right.tree.root.as_ref().map(|x| &x.info),
                    );
                    return (
                        SingleRed,
                        Node {
                            key: mid,
                            info,
                            color: Color::Red,
                            left: left.tree,
                            right: right.tree,
                        },
                        left.height,
                    );
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
        let (state, child_node, child_bh) = match child_side {
            Side::Left => Self::join_nodes(
                left,
                mid,
                Tree {
                    tree: node.left.clone(),
                    height: child_bh,
                },
                Some((&child_side, node.right.root_color())),
            ),
            Side::Right => Self::join_nodes(
                Tree {
                    tree: node.right.clone(),
                    height: child_bh,
                },
                mid,
                right,
                Some((&child_side, node.left.root_color())),
            ),
        };
        let (state, node, higher) =
            Self::insert_fixup(state, node, child_side, child_node, false, parent_data);
        (state, node, higher as usize + child_bh)
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
        let (state, node, higher) = self.tree.insert_node(key, None);
        assert!(matches!(
            state,
            InsertState::Resolved | InsertState::SingleRed
        ));
        self.tree = node.into();
        if higher {
            self.height += 1;
        }
    }
    pub fn get(&self, key: impl Searcher<K, I>) -> Option<&K> {
        self.tree.get(key)
    }
    fn join(self, mid: K, right: Self) -> Self {
        let (state, node, height) = SubTree::join_nodes(self, mid, right, None);
        assert!(matches!(
            state,
            InsertState::Resolved | InsertState::SingleRed
        ));
        Tree {
            tree: node.into(),
            height,
        }
    }
    pub fn cat(self, rhs: Self) -> Self {
        todo!("delete in rbt");
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
        let mut iter = Iter {
            stack: Vec::new(),
        };
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
