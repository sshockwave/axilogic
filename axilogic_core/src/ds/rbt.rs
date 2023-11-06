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
    left: Tree<K, I>,
    right: Tree<K, I>,
}

/// Requirements:
/// 1. A red node does not have a red child
/// 2. Every path from root to leaf has the same number of black nodes
pub struct Tree<K: Clone, I: SearchInfo<K>> {
    root: Option<Rc<Node<K, I>>>,
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

impl<K: Clone, I: SearchInfo<K>> Clone for Tree<K, I> {
    fn clone(&self) -> Self {
        Tree {
            root: self.root.clone(),
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> From<Node<K, I>> for Tree<K, I> {
    fn from(value: Node<K, I>) -> Self {
        Tree {
            root: Some(Rc::new(value)),
        }
    }
}

impl<K: Clone, I: SearchInfo<K>> Tree<K, I> {
    pub fn new() -> Self {
        Tree { root: None }
    }
    fn root_color(&self) -> &Color {
        self.root.as_ref().map_or(&Color::Black, |x| &x.color)
    }
    fn rotate_left(node: &mut Node<K, I>, right_child: Node<K, I>) {
        let mut left_child = std::mem::replace(node, right_child);
        std::mem::swap(&mut left_child.right, &mut node.left);
        left_child.update();
        node.left = left_child.into();
    }
    fn rotate_right(node: &mut Node<K, I>, left_child: Node<K, I>) {
        let mut right_child = std::mem::replace(node, left_child);
        std::mem::swap(&mut node.right, &mut right_child.left);
        right_child.update();
        node.right = right_child.into();
    }
    fn insert_fixup(
        state: InsertState,
        mut node: Node<K, I>,
        child_side: Side,
        child_node: Node<K, I>,
        parent_data: Option<(&Side, &Color)>,
    ) -> (InsertState, Node<K, I>) {
        use InsertState::*;
        let (child, other_child) = match child_side {
            Side::Left => (&mut node.left, &mut node.right),
            Side::Right => (&mut node.right, &mut node.left),
        };
        let state = match (state, &node.color, parent_data) {
            // (child state, this color, (this side, sibiling color))
            (Resolved, _, _) | (SingleRed, Color::Black, _) => {
                *child = child_node.into();
                Resolved
            }
            (SingleRed, Color::Red, None) => {
                // This is the root
                *child = child_node.into();
                node.color = Color::Black;
                Resolved
            }
            (SingleRed, Color::Red, Some((_, Color::Red))) => {
                *child = child_node.into();
                node.color = Color::Black;
                NewBlack
            }
            (SingleRed, Color::Red, Some((self_side, Color::Black))) => {
                match (self_side, child_side) {
                    (Side::Left, Side::Right) => Self::rotate_left(&mut node, child_node),
                    (Side::Right, Side::Left) => Self::rotate_right(&mut node, child_node),
                    _ => {}
                }
                TwoRed
            }
            (NewBlack, Color::Black, _) => {
                let mut other_child_node = other_child.root.as_ref().unwrap().as_ref().clone();
                assert!(matches!(other_child_node.color, Color::Red));
                other_child_node.color = Color::Black;
                *child = child_node.into();
                *other_child = other_child_node.into();
                node.color = Color::Red;
                SingleRed
            }
            (TwoRed, Color::Black, _) => {
                node.color = Color::Red;
                match child_side {
                    Side::Left => Self::rotate_right(&mut node, child_node),
                    Side::Right => Self::rotate_left(&mut node, child_node),
                }
                node.color = Color::Black;
                Resolved
            }
            (NewBlack | TwoRed, Color::Red, _) => {
                unreachable!("Red node cannot have red child")
            }
        };
        node.update();
        (state, node)
    }
    fn insert_node(
        &mut self,
        mut inserter: impl Searcher<K, I> + Into<K>,
        parent_data: Option<(&Side, &Color)>,
    ) -> (InsertState, Node<K, I>) {
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
                    left: Tree::new(),
                    right: Tree::new(),
                },
            );
        };
        let child_side = match inserter.cmp(&node.key, &node.info) {
            Equal => {
                node.key = inserter.into();
                return (InsertState::Resolved, node);
            }
            Less => Side::Left,
            Greater => Side::Right,
        };
        let (child, other_child) = match child_side {
            Side::Left => (&mut node.left, &mut node.right),
            Side::Right => (&mut node.right, &mut node.left),
        };
        let (mut state, child_node) =
            child.insert_node(inserter, Some((&child_side, other_child.root_color())));
        Self::insert_fixup(state, node, child_side, child_node, parent_data)
    }
    pub fn set(&mut self, key: impl Searcher<K, I> + Into<K>) {
        let (state, node) = self.insert_node(key, None);
        assert!(matches!(
            state,
            InsertState::Resolved | InsertState::SingleRed
        ));
        *self = node.into();
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
        (left, left_bh): (Self, usize),
        mid: K,
        (right, right_bh): (Self, usize),
        parent_data: Option<(&Side, &Color)>,
    ) -> (InsertState, Node<K, I>) {
        use InsertState::*;
        let child_side = match left_bh.cmp(&right_bh) {
            Equal => match (left.root_color(), right.root_color()) {
                (Color::Black, Color::Black) => {
                    let info = I::new(
                        left.root.as_ref().map(|x| &x.info),
                        &mid,
                        right.root.as_ref().map(|x| &x.info),
                    );
                    return (
                        SingleRed,
                        Node {
                            key: mid,
                            info,
                            color: Color::Red,
                            left,
                            right,
                        },
                    );
                }
                (Color::Red, _) => Side::Right,
                (Color::Black, Color::Red) => Side::Left,
            },
            Less => Side::Left,
            Greater => Side::Right,
        };
        let (node, node_bh) = match child_side {
            Side::Left => (&right, right_bh),
            Side::Right => (&left, left_bh),
        };
        let mut node = node.root.as_ref().unwrap().as_ref().clone();
        let child_bh = match node.color {
            Color::Black => node_bh - 1,
            Color::Red => node_bh,
        };
        let (mut state, child_node) = match child_side {
            Side::Left => Self::join_nodes(
                (left, left_bh),
                mid,
                (node.left.clone(), child_bh),
                Some((&child_side, node.right.root_color())),
            ),
            Side::Right => Self::join_nodes(
                (node.right.clone(), child_bh),
                mid,
                (right, right_bh),
                Some((&child_side, node.left.root_color())),
            ),
        };
        Self::insert_fixup(state, node, child_side, child_node, parent_data)
    }
}
