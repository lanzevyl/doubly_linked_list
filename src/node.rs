use std::sync::{Arc, Mutex};

use crate::try_lock;

#[derive(Debug)]
pub(crate) struct Routes<T> {
    pub left: Option<Node<T>>,
    pub right: Option<Node<T>>,
}

impl<T> Routes<T> {
    pub fn new(left: Node<T>, right: Node<T>) -> Self {
        Self {
            left: left.into(),
            right: right.into(),
        }
    }

    pub fn new_insulated() -> Self {
        Self {
            left: None,
            right: None,
        }
    }

    pub fn from_left(left: Node<T>) -> Self {
        Self {
            left: left.into(),
            right: None,
        }
    }

    pub fn from_right(right: Node<T>) -> Self {
        Self {
            left: None,
            right: right.into(),
        }
    }

    pub fn is_insulate(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

#[derive(Debug)]
pub struct Node<T> {
    pub(crate) routes: Arc<Mutex<Routes<T>>>,
    pub(crate) value: Arc<T>,
}

impl<T> Node<T> {
    pub fn value(&self) -> &Arc<T> {
        &self.value
    }

    pub fn left(&self) -> Option<Node<T>> {
        self.routes.lock().unwrap().left.clone()
    }

    pub fn right(&self) -> Option<Node<T>> {
        self.routes.lock().unwrap().right.clone()
    }

    pub fn is_insulate(&self) -> bool {
        self.routes.lock().unwrap().is_insulate()
    }

    pub(crate) fn from_routes(value: T, routes: Routes<T>) -> Self {
        Self {
            routes: Arc::new(Mutex::new(routes)),
            value: Arc::new(value),
        }
    }

    pub(crate) fn new(value: T, left: Node<T>, right: Node<T>) -> Self {
        Self::from_routes(value, Routes::new(left, right))
    }

    pub(crate) fn new_insulated(value: T) -> Self {
        Self::from_routes(value, Routes::new_insulated())
    }

    pub(crate) fn from_right(value: T, right: Node<T>) -> Self {
        Self::from_routes(value, Routes::from_right(right))
    }

    pub(crate) fn from_left(value: T, left: Node<T>) -> Self {
        Self::from_routes(value, Routes::from_left(left))
    }

    pub(crate) fn insert_left(&self, value: T) -> Node<T> {
        loop {
            let mut self_routes = self.routes.lock().unwrap();

            if let Some(left) = self_routes.left.take() {
                match left.routes.try_lock() {
                    Ok(mut left_routes) => {
                        let mid = Node::new(value, left.clone(), self.clone());

                        self_routes.left = mid.clone().into();
                        left_routes.right = mid.clone().into();

                        break mid;
                    }
                    Err(_) => {
                        self_routes.left = left.clone().into();

                        continue;
                    }
                }
            } else {
                let mid = Node::from_right(value, self.clone());

                self_routes.left = mid.clone().into();

                break mid;
            }
        }
    }

    pub(crate) fn insert_right(&self, value: T) -> Node<T> {
        let mut self_routes = self.routes.lock().unwrap();

        if let Some(right) = self_routes.right.take() {
            let mut right_routes = right.routes.lock().unwrap();

            let mid = Node::new(value, self.clone(), right.clone());

            self_routes.right = mid.clone().into();
            right_routes.left = mid.clone().into();

            mid
        } else {
            let mid = Node::from_left(value, self.clone());

            self_routes.right = mid.clone().into();

            mid
        }
    }

    pub(crate) fn insulate_left(&self) -> (&Arc<T>, Option<Node<T>>) {
        loop {
            let mut self_routes = self.routes.lock().unwrap();

            if let Some(left) = self_routes.left.as_ref() {
                let mut left_routes = try_lock!(left.routes);

                left_routes.right = self_routes.right.clone();
            }

            break (&self.value, self_routes.left.take());
        }
    }

    pub(crate) fn insulate_right(&self) -> (&Arc<T>, Option<Node<T>>) {
        let mut self_routes = self.routes.lock().unwrap();

        if let Some(right) = self_routes.right.as_ref() {
            let mut right_routes = right.routes.lock().unwrap();

            right_routes.left = self_routes.left.clone();
        }

        (&self.value, self_routes.right.take())
    }

    pub(crate) fn insulate(&self) -> (&Arc<T>, Option<Node<T>>, Option<Node<T>>) {
        loop {
            let mut self_routes = self.routes.lock().unwrap();

            let left_guard = if let Some(left) = self_routes.left.as_ref() {
                let mut left_routes = try_lock!(left.routes);

                left_routes.right = self_routes.right.clone();

                Some(left_routes)
            } else {
                None
            };

            if let Some(right) = self_routes.right.as_ref() {
                let mut right_routes = right.routes.lock().unwrap();

                right_routes.left = self_routes.left.clone();
            }

            drop(left_guard);

            break (
                &self.value,
                self_routes.left.take(),
                self_routes.right.take(),
            );
        }
    }

    pub(crate) fn insulate_left_owned(&self) -> (Arc<T>, Option<Node<T>>) {
        let (v, l) = self.insulate_left();

        (Arc::clone(v), l)
    }

    pub(crate) fn insulate_right_owned(&self) -> (Arc<T>, Option<Node<T>>) {
        let (v, r) = self.insulate_right();

        (Arc::clone(v), r)
    }

    pub(crate) fn insulate_owned(&self) -> (Arc<T>, Option<Node<T>>, Option<Node<T>>) {
        let (v, l, r) = self.insulate();

        (Arc::clone(v), l, r)
    }
}

impl<T> Clone for Node<T> {
    fn clone(&self) -> Self {
        Self {
            routes: Arc::clone(&self.routes),
            value: Arc::clone(&self.value),
        }
    }
}

impl<T> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.routes, &other.routes)
    }
}

pub struct NodeIterator<T> {
    node: Option<Node<T>>,
}

impl<T> IntoIterator for Node<T> {
    type Item = Arc<T>;
    type IntoIter = NodeIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        NodeIterator { node: self.into() }
    }
}

impl<T> Iterator for NodeIterator<T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node.take() {
            self.node = node.right();

            Some(node.value)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_left() {
        let node = Node::new_insulated(2);
        let head = node.insert_left(1);

        assert_eq!(head.into_iter().map(|a| *a).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn insert_right() {
        let head = Node::new_insulated(1);
        head.insert_right(2);

        assert_eq!(head.into_iter().map(|a| *a).collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn insulate_left() {
        let tail = Node::new_insulated(2);
        let head = tail.insert_left(1);

        assert_eq!(tail.insulate_left().1.unwrap(), head);

        assert_eq!(head.into_iter().map(|a| *a).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn insulate_right() {
        let head = Node::new_insulated(1);
        let tail = head.insert_right(2);

        assert_eq!(head.insulate_right().1.unwrap(), tail);

        assert_eq!(head.into_iter().map(|a| *a).collect::<Vec<_>>(), vec![1]);
    }

    #[test]
    fn insulate() {
        let mid = Node::new_insulated(2);
        let head = mid.insert_left(1);
        mid.insert_right(3);

        assert_eq!(**mid.insulate().0, 2);
        assert!(mid.is_insulate());

        assert_eq!(head.into_iter().map(|a| *a).collect::<Vec<_>>(), vec![1, 3]);
    }

    #[test]
    fn insert_right_insert_left() {
        use std::thread;

        for _ in 0..200000 {
            let head = Node::new_insulated(1);
            let tail = head.insert_right(4);

            let r = thread::spawn(move || {
                tail.insert_left(3);
            });

            thread::spawn({
                let head = head.clone();

                move || {
                    head.insert_right(2);
                }
            })
            .join()
            .unwrap();

            r.join().unwrap();

            assert_eq!(
                head.into_iter().map(|a| *a).collect::<Vec<_>>(),
                vec![1, 2, 3, 4]
            );
        }
    }

    #[test]
    fn insulate_right_insulate_left() {
        use std::thread;

        for _ in 0..200000 {
            let one = Node::new_insulated(1);
            let thr = one.insert_right(3);
            let fiv = thr.insert_right(5);

            let r1 = thread::spawn({
                let one = one.clone();

                move || {
                    one.insulate_right();
                }
            });

            thread::spawn(move || {
                fiv.insulate_left();
            })
            .join()
            .unwrap();

            r1.join().unwrap();

            assert!(thr.is_insulate());
        }
    }

    #[test]
    fn insert_right_insulate_insert_left() {
        use std::thread;

        for _ in 0..200000 {
            let one = Node::new_insulated(1);
            let thr = one.insert_right(3);
            let fiv = thr.insert_right(5);

            let r1 = thread::spawn({
                let one = one.clone();

                move || {
                    one.insert_right(2);
                }
            });

            let r2 = thread::spawn(move || {
                fiv.insert_left(4);
            });

            thread::spawn(move || {
                thr.insulate();
            })
            .join()
            .unwrap();

            r2.join().unwrap();
            r1.join().unwrap();

            assert_eq!(
                one.into_iter().map(|a| *a).collect::<Vec<_>>(),
                vec![1, 2, 4, 5]
            );
        }
    }

    #[test]
    fn insulate_insulate() {
        use std::thread;

        for _ in 0..200000 {
            let one = Node::new_insulated(1);
            let two = one.insert_right(2);
            let thr = two.insert_right(3);
            thr.insert_right(4);

            let r = thread::spawn(move || {
                two.insulate();
            });

            thread::spawn(move || {
                thr.insulate();
            })
            .join()
            .unwrap();

            r.join().unwrap();

            assert_eq!(one.into_iter().map(|a| *a).collect::<Vec<_>>(), vec![1, 4]);
        }
    }
}
