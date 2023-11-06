use std::{
    cmp::{Ord, Ordering::*},
    rc::Rc,
};

#[derive(Clone)]
enum Color {
    Red,
    Black,
}

#[derive(Clone)]
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

struct Node<K: Ord, V> {
    kv: Rc<(K, V)>,
    color: Color,
    size: usize,
    left: Tree<K, V>,
    right: Tree<K, V>,
}

/// Requirements:
/// 1. A red node does not have a red child
/// 2. Every path from root to leaf has the same number of black nodes
pub struct Tree<K: Ord, V> {
    root: Option<Rc<Node<K, V>>>,
}

impl<K: Ord, V> Node<K, V> {
    fn update(&mut self) {
        self.size = self.left.size() + self.right.size() + 1;
    }
}

impl<K: Ord, V> Clone for Node<K, V> {
    fn clone(&self) -> Self {
        Node {
            kv: self.kv.clone(),
            color: self.color.clone(),
            size: self.size,
            left: self.left.clone(),
            right: self.right.clone(),
        }
    }
}

impl<K: Ord, V> Clone for Tree<K, V> {
    fn clone(&self) -> Self {
        Tree {
            root: self.root.clone(),
        }
    }
}

impl<K: Ord, V> Tree<K, V> {
    pub fn new() -> Self {
        Tree { root: None }
    }
    pub fn size(&self) -> usize {
        self.root.as_ref().map_or(0, |x| x.size)
    }
    fn root_color(&self) -> Color {
        self.root.as_ref().map_or(Color::Black, |x| x.color.clone())
    }
    fn rotate_left(node: &mut Node<K, V>, right_child: Node<K, V>) {}
    fn rotate_right(node: &mut Node<K, V>, left_child: Node<K, V>) {}
    fn insert_node(
        &self,
        key: K,
        value: V,
        data: Option<(Side, Color)>,
    ) -> (InsertState, Node<K, V>, Rc<(K, V)>) {
        let node_rc = if let Some(x) = self.root.as_ref() {
            x
        } else {
            let rc = Rc::new((key, value));
            return (
                SingleRed,
                Node {
                    kv: rc.clone(),
                    color: Color::Red,
                    size: 1,
                    left: Tree::new(),
                    right: Tree::new(),
                },
                rc,
            );
        };
        use InsertState::*;
        let mut node = node_rc.as_ref().clone();
        let (child, other_child, child_side) = match key.cmp(&node_rc.kv.0) {
            Equal => {
                let rc = Rc::new((key, value));
                node.kv = rc.clone();
                return (Resolved, node, rc);
            }
            Less => (&mut node.left, &mut node.right, Side::Left),
            Greater => (&mut node.right, &mut node.left, Side::Right),
        };
        let (state, child_node, rc) = child.insert_node(
            key,
            value,
            Some((child_side.clone(), other_child.root_color())),
        );
        let state = match (state, node.color.clone(), data) {
            // (child state, this color, (this side, sibiling color))
            (Resolved, _, _) | (SingleRed, Color::Black, _) => {
                child.root = Some(Rc::new(child_node));
                node.update();
                Resolved
            }
            (SingleRed, Color::Red, None) => {
                // This is the root
                child.root = Some(Rc::new(child_node));
                node.color = Color::Black;
                node.update();
                Resolved
            }
            (SingleRed, Color::Red, Some((_, Color::Red))) => {
                child.root = Some(Rc::new(child_node));
                node.color = Color::Black;
                node.update();
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
                child.root = Some(Rc::new(child_node));
                assert!(matches!(other_child.root_color(), Color::Red));
                let mut other_child_node = other_child.root.as_ref().unwrap().as_ref().clone();
                other_child_node.color = Color::Black;
                other_child.root = Some(Rc::new(other_child_node));
                node.color = Color::Red;
                node.update();
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
        (state, node, rc)
    }
    pub fn insert(&mut self, key: K, value: V) -> Rc<(K, V)> {
        let (state, node, rc) = self.insert_node(key, value, None);
        assert!(matches!(state, InsertState::Resolved));
        self.root = Some(Rc::new(node));
        rc
    }
}
