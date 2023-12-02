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

enum InsertState<I: SearchInfo> {
    Black(BlackNode<I>), // resolved
    Red(RedNode<I>),
    DoubleRed(
        BlackSubTree<I>,
        I::Key,
        BlackSubTree<I>,
        I::Key,
        BlackSubTree<I>,
    ),
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

type InfoOf<T: Node> = <T::Ptr as NodePtr>::Info;
type KeyOf<T: Node> = <InfoOf<T> as SearchInfo>::Key;

trait NodeStore {
    type Ptr: NodePtr;
    fn new<const SIDE: bool>(child: Self::Ptr, key: KeyOf<Self>, other_child: Self::Ptr) -> Self;
    fn unpack<const SIDE: bool>(self) -> (Self::Ptr, KeyOf<Self>, Self::Ptr);
    fn cmp(&self, searcher: &mut impl Searcher<Info = InfoOf<Self>>) -> Ordering;
    fn child_mut<const SIDE: bool>(&mut self) -> (&mut Self::Ptr, &mut Self::Ptr);
    fn info(&self) -> InfoOf<Self>;
}

trait Node: NodeStore
where
    Self: Sized + Clone,
    SubTree<InfoOf<Self>>: From<Self>,
{
    const COLOR: Color;
    fn add_fixup<const SIDE: bool>(
        state: InsertState<InfoOf<Self>>,
        key: KeyOf<Self>,
        other_child: Self::Ptr,
    ) -> InsertState<InfoOf<Self>>;
    fn add_side<const SIDE: bool>(
        mut self,
        key: impl Inserter<InfoOf<Self>>,
    ) -> InsertState<InfoOf<Self>> {
        let (child, self_key, other_child) = self.unpack::<SIDE>();
        let state = child.add(key);
        Self::add_fixup::<SIDE>(state, self_key, other_child)
    }
    fn replace_key(self, key: KeyOf<Self>) -> InsertState<InfoOf<Self>>;
    fn add(self, key: impl Inserter<InfoOf<Self>>) -> InsertState<InfoOf<Self>> {
        match self.cmp(&mut key) {
            Equal => self.replace_key(key.into()),
            Less => self.add_side::<LEFT>(key),
            Greater => self.add_side::<RIGHT>(key),
        }
    }

    fn del_case6<const SIDE: bool>(
        a: BlackSubTree<InfoOf<Self>>,            // child
        b: KeyOf<Self>,                           // self
        c: SubTree<InfoOf<Self>>,                 // other_child_near
        d: KeyOf<Self>,                           // other_child
        e: (RedNode<InfoOf<Self>>, InfoOf<Self>), // other_child_far
    ) -> Self {
        let e: (BlackNode<_>, _) = (e.0.into(), e.1);
        let abc: BlackSubTree<_> = BlackNode::new::<SIDE>(a.into(), b, c).into();
        Self::new::<SIDE>(abc.into(), d, Some(Rc::new(e)).into())
    }
    fn del_black_sibling<const SIDE: bool>(
        a: BlackSubTree<InfoOf<Self>>,
        b: KeyOf<Self>,
        d: BlackSubTree<InfoOf<Self>>,
    ) -> Result<SubTree<InfoOf<Self>>, BlackSubTree<InfoOf<Self>>> {
        // Because a is double black, other_child must contain at least one real node
        let (d, d_info) = rc_take(d.unwrap());
        let (c, d, e) = d.unpack::<SIDE>();
        let e = match e.take_if_red() {
            Ok(e) => return Ok(Self::del_case6::<SIDE>(a, b, c, d, rc_take(e)).into()),
            Err(e) => e,
        };
        let c = match c.take_if_red() {
            Ok(c) => {
                // case 5
                let (c_left, c, c_right) = rc_take(c).0.unpack::<SIDE>();
                let cde = RedNode::new::<SIDE>(c_right, d, e);
                return Ok(
                    Self::del_case6::<SIDE>(a, b, c_left.into(), c, (cde, cde.info())).into(),
                );
            }
            Err(c) => c,
        };
        // resolved if is red
        let root = BlackNode::new::<SIDE>(
            a.into(),
            b,
            SubTree::Red(Rc::new((RedNode::new::<SIDE>(c, d, e), d_info))),
        );
        match Self::COLOR {
            Color::Red => Ok(root.into()),
            Color::Black => Err(root.into()),
        }
    }
    fn del_fix_double_black<const SIDE: bool>(
        a: BlackSubTree<InfoOf<Self>>,
        b: KeyOf<Self>,
        d: Self::Ptr,
    ) -> Result<SubTree<InfoOf<Self>>, BlackSubTree<InfoOf<Self>>>;
    fn del_self(self) -> DeleteState<KeyOf<Self>>;
    fn del_side<const SIDE: bool>(
        &mut self,
        key: impl Searcher<Info = InfoOf<Self>>,
    ) -> DeleteState<KeyOf<Self>> {
        let state = self.child_mut::<SIDE>().0.del(key);
        self.del_fixup::<SIDE>(state)
    }
    // Deletion: https://medium.com/analytics-vidhya/deletion-in-red-black-rb-tree-92301e1474ea
    fn del(
        rc: &mut Rc<(Self, InfoOf<Self>)>,
        key: impl Searcher<Info = InfoOf<Self>>,
    ) -> (bool, DeleteState<KeyOf<Self>>) {
        let node = &rc.0;
        (true, match node.cmp(&mut key) {
            Equal => {
                let mut right = rc.0.child_mut::<LEFT>().0.clone();
                let mut state = right.del(LeftmostSearcher(PhantomData));
                use DeleteState::*;
                match state {
                    DoubleBlack(state_key) | Resolved(state_key) => {
                        let (left, key, _) = Rc::make_mut(rc).0.unpack::<LEFT>();
                        std::mem::swap(&mut state_key, &mut node.key);
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
        })
    }
}

trait Inserter<I: SearchInfo>: Searcher<Info = I> + Into<I::Key> {}
impl<I: SearchInfo, T: Searcher<Info = I> + Into<I::Key>> Inserter<I> for T {}

fn new_node<I: SearchInfo>(key: impl Inserter<I>) -> InsertState<I> {
    InsertState::Red(RedNode {
        key: key.into(),
        left: None,
        right: None,
    })
}

trait NodePtr: From<BlackSubTree<Self::Info>> + Into<SubTree<Self::Info>> + Clone {
    type Info: SearchInfo;
    fn info(&self) -> Option<&Self::Info>;
    fn add(self, key: impl Inserter<Self::Info>) -> InsertState<Self::Info>;
    fn as_ref(&self) -> Option<NodeRef<'_, Self::Info>>;
    fn get(
        &self,
        key: impl Searcher<Info = Self::Info>,
    ) -> Option<&<Self::Info as SearchInfo>::Key>;
    fn del(
        &mut self,
        key: impl Searcher<Info = Self::Info>,
    ) -> DeleteState<<Self::Info as SearchInfo>::Key>;
}

#[derive(Clone)]
struct NodeImpl<P: NodePtr> {
    key: <P::Info as SearchInfo>::Key,
    left: P,
    right: P,
}

type RedNode<I: SearchInfo> = NodeImpl<BlackSubTree<I>>;
type BlackNode<I: SearchInfo> = NodeImpl<SubTree<I>>;

/// Requirements:
/// 1. A red node does not have a red child
/// 2. Every path from root to leaf has the same number of black nodes
enum SubTree<I: SearchInfo> {
    Red(Rc<(RedNode<I>, I)>),
    Black(Rc<(BlackNode<I>, I)>),
    None,
}
type BlackSubTree<I: SearchInfo> = Option<Rc<(BlackNode<I>, I)>>;

impl<I: SearchInfo> Clone for SubTree<I> {
    fn clone(&self) -> Self {
        match self {
            Self::Red(x) => Self::Red(x.clone()),
            Self::Black(x) => Self::Black(x.clone()),
            Self::None => Self::None,
        }
    }
}
impl<I: SearchInfo> SubTree<I> {
    fn take_if_red(self) -> Result<Rc<(RedNode<I>, I)>, BlackSubTree<I>> {
        match self {
            Self::Red(x) => Ok(x),
            Self::Black(x) => Err(Some(x)),
            Self::None => Err(None),
        }
    }
}

#[derive(Clone)]
pub struct Tree<I: SearchInfo> {
    tree: BlackSubTree<I>,
    height: usize, // the black height of the tree
}

pub enum NodeRef<'a, I: SearchInfo> {
    Red(&'a RedNode<I>),
    Black(&'a BlackNode<I>),
}

pub struct Iter<'a, I: SearchInfo> {
    stack: Vec<NodeRef<'a, I>>,
}

impl<I: SearchInfo> InsertState<I> {
    fn higher(&self) -> bool {
        match self {
            Self::DoubleRed(..) => true,
            Self::Black(_) | Self::Red(_) => false,
        }
    }
}

const LEFT: bool = false;
const RIGHT: bool = true;

impl<P: NodePtr> NodeStore for NodeImpl<P> {
    type Ptr = P;
    fn new<const SIDE: bool>(child: P, key: KeyOf<Self>, other_child: P) -> Self {
        match SIDE {
            LEFT => Self {
                key,
                left: child,
                right: other_child,
            },
            RIGHT => Self {
                key,
                left: other_child,
                right: child,
            },
        }
    }
    fn info(&self) -> P::Info {
        P::Info::new(self.left.info(), &self.key, self.right.info())
    }
    fn cmp(&self, searcher: &mut impl Searcher<Info = P::Info>) -> Ordering {
        searcher.cmp(self.left.info(), &self.key, self.right.info())
    }
    fn unpack<const SIDE: bool>(self) -> (P, <P::Info as SearchInfo>::Key, P) {
        match SIDE {
            LEFT => (self.left, self.key, self.right),
            RIGHT => (self.right, self.key, self.left),
        }
    }
    fn child_mut<const SIDE: bool>(&mut self) -> (&mut P, &mut P) {
        match SIDE {
            LEFT => (&mut self.left, &mut self.right),
            RIGHT => (&mut self.right, &mut self.left),
        }
    }
}
impl<P: NodePtr> NodeImpl<P> {
    fn info(&self) -> P::Info {
        P::Info::new(self.left.info(), &self.key, self.right.info())
    }
}

impl<I: SearchInfo> From<BlackSubTree<I>> for SubTree<I> {
    fn from(value: BlackSubTree<I>) -> Self {
        match value {
            Some(x) => Self::Black(x),
            None => Self::None,
        }
    }
}
impl<I: SearchInfo> From<RedNode<I>> for BlackNode<I> {
    fn from(value: RedNode<I>) -> Self {
        BlackNode {
            key: value.key,
            left: value.left.into(),
            right: value.right.into(),
        }
    }
}
impl<I: SearchInfo> From<RedNode<I>> for SubTree<I> {
    fn from(value: RedNode<I>) -> Self {
        SubTree::Red(Rc::new((value, value.info())))
    }
}
impl<I: SearchInfo> From<BlackNode<I>> for SubTree<I> {
    fn from(value: BlackNode<I>) -> Self {
        SubTree::Black(Rc::new((value, value.info())))
    }
}
impl<I: SearchInfo> From<BlackNode<I>> for BlackSubTree<I> {
    fn from(value: BlackNode<I>) -> Self {
        Some(Rc::new((value, value.info())))
    }
}

impl<I: SearchInfo> Node for BlackNode<I> {
    const COLOR: Color = Color::Black;
    fn replace_key(self, key: I::Key) -> InsertState<I> {
        InsertState::Black(Self {
            key,
            left: self.left,
            right: self.right,
        })
    }
    fn add_fixup<const SIDE: bool>(
        state: InsertState<I>,
        key: I::Key,
        other_child: Self::Ptr,
    ) -> InsertState<I> {
        use InsertState::*;
        match state {
            Black(son) => Black(BlackNode::new::<SIDE>(son.into(), key, other_child)),
            Red(son) => Black(BlackNode::new::<SIDE>(son.into(), key, other_child)),
            DoubleRed(a, b, c, d, e) => {
                let (a, b, c, d, e) = match SIDE {
                    LEFT => (a, b, c, d, e),
                    RIGHT => (e, d, c, b, a),
                };
                match other_child.take_if_red() {
                    Ok(other_child) => {
                        let (other_child, info) = rc_take(other_child);
                        Red(RedNode::new::<SIDE>(
                            BlackNode::new::<SIDE>(
                                RedNode::new::<SIDE>(a, b, c).into(),
                                d,
                                e.into(),
                            )
                            .into(),
                            key,
                            Some(Rc::new((other_child.into(), info))),
                        ))
                    }
                    Err(other_child) => Black(BlackNode::new::<SIDE>(
                        RedNode::new::<SIDE>(a, b, c).into(),
                        d,
                        RedNode::new::<SIDE>(e, key, other_child).into(),
                    )),
                }
            }
        }
    }
    fn del_fix_double_black<const SIDE: bool>(
        a: BlackSubTree<InfoOf<Self>>,
        b: KeyOf<Self>,
        d: Self::Ptr,
    ) -> Result<SubTree<InfoOf<Self>>, BlackSubTree<InfoOf<Self>>> {
        let (a, b, d) = match d.take_if_red() {
            Ok(d) => {
                // case 4
                let d = rc_take(d).0;
                let (c, d, e) = d.unpack::<SIDE>();
                match RedNode::del_black_sibling(a, b, c) {
                    Ok(x) => return Ok(Self::new::<SIDE>(x, d, e.into()).into()),
                    Err(x) => (x, d, e),
                }
            }
            Err(d) => (a, b, d),
        };
        Self::del_black_sibling::<SIDE>(a, b, d)
    }
}

impl<I: SearchInfo> Node for RedNode<I> {
    const COLOR: Color = Color::Red;
    fn replace_key(self, key: I::Key) -> InsertState<I> {
        InsertState::Red(Self {
            key,
            left: self.left,
            right: self.right,
        })
    }
    fn add_fixup<const SIDE: bool>(
        state: InsertState<I>,
        key: I::Key,
        other_child: Self::Ptr,
    ) -> InsertState<I> {
        use InsertState::*;
        match state {
            Black(child) => Red(RedNode::new::<SIDE>(child.into(), key, other_child.into())),
            Red(child) => match SIDE {
                LEFT => DoubleRed(child.left, child.key, child.right, key, other_child),
                RIGHT => DoubleRed(other_child, key, child.left, child.key, child.right),
            },
            DoubleRed(..) => unreachable!(),
        }
    }
    fn del_fix_double_black<const SIDE: bool>(
        a: BlackSubTree<InfoOf<Self>>,
        b: KeyOf<Self>,
        d: Self::Ptr,
    ) -> Result<SubTree<InfoOf<Self>>, BlackSubTree<InfoOf<Self>>> {
        Self::del_black_sibling::<SIDE>(a, b, d)
    }
}

impl<I: SearchInfo> NodePtr for SubTree<I> {
    type Info = I;
    fn as_ref(&self) -> Option<NodeRef<'_, I>> {
        match self {
            Self::Red(x) => Some(NodeRef::Red(&x.0)),
            Self::Black(x) => Some(NodeRef::Black(&x.0)),
            Self::None => None,
        }
    }
    fn info(&self) -> Option<&I> {
        match self {
            Self::Red(x) => Some(&x.1),
            Self::Black(x) => Some(&x.1),
            Self::None => None,
        }
    }
    fn add(self, key: impl Inserter<Self::Info>) -> InsertState<Self::Info> {
        match self {
            Self::Red(x) => rc_take(x).0.add(key),
            Self::Black(x) => rc_take(x).0.add(key),
            Self::None => new_node(key),
        }
    }
    fn get(&self, key: impl Searcher<Info = I>) -> Option<&I::Key> {
        match self {
            Self::Red(x) => x.0.get(key),
            Self::Black(x) => x.0.get(key),
            Self::None => None,
        }
    }
}

impl<I: SearchInfo> NodePtr for BlackSubTree<I> {
    type Info = I;
    fn as_ref(&self) -> Option<NodeRef<'_, Self::Info>> {
        Some(NodeRef::Black(&self.as_ref()?.0))
    }
    fn info(&self) -> Option<&Self::Info> {
        self.as_ref().map(|x| &x.1)
    }
    fn add(self, key: impl Inserter<Self::Info>) -> InsertState<Self::Info> {
        match self {
            Some(x) => rc_take(x).0.add(key),
            None => new_node(key),
        }
    }
    fn get(&self, key: impl Searcher<Info = I>) -> Option<&I::Key> {
        self.as_ref()?.0.get(key)
    }
}

impl<I: SearchInfo> SubTree<I> {
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
    pub fn add(&mut self, key: impl Searcher<Info = I> + Into<I::Key>) {
        let state = std::mem::replace(&mut self.tree, SubTree::new()).add(key);
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
                    return Red(RedNode {
                        key: mid,
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
            LEFT => node.add_fixup::<LEFT>(state),
            RIGHT => node.add_fixup::<RIGHT>(state),
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
            Less => LEFT,
            Greater => RIGHT,
        };
        let child = match child_side {
            LEFT => &node.left,
            RIGHT => &node.right,
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
            LEFT => (
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
            RIGHT => (
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
    fn push_all_left<P: NodePtr<Info = I>>(&mut self, mut x: &'a SubTree<I>) {
        while let Some(v) = x.root.as_ref() {
            self.stack.push(v.as_ref().0);
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
            tree.add(IntSearch { key: i });
            sanity_check(&tree.tree, tree.height);
        }
        for (x, i) in tree.iter().zip(0..N) {
            assert_eq!(*x, i);
        }
        tree = Tree::new();
        for i in (0..N).rev() {
            tree.add(IntSearch { key: i });
            sanity_check(&tree.tree, tree.height);
        }
        for (x, i) in tree.iter().zip(0..N) {
            assert_eq!(*x, i);
        }
    }
}
