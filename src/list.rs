use std::sync::{Arc, Mutex};

use crate::{try_lock, Node};

pub struct LinkedList<T> {
    head: Arc<Mutex<Option<Node<T>>>>,
    tail: Arc<Mutex<Option<Node<T>>>>,
}

impl<T> LinkedList<T> {
    pub fn new() -> Self {
        Self {
            head: Arc::new(Mutex::new(None)),
            tail: Arc::new(Mutex::new(None)),
        }
    }

    pub fn head(&self) -> Option<Node<T>> {
        self.head.lock().unwrap().clone()
    }

    pub fn tail(&self) -> Option<Node<T>> {
        self.tail.lock().unwrap().clone()
    }

    pub fn push_front(&self, value: T) -> Node<T> {
        loop {
            let mut head = self.head.lock().unwrap();

            if let Some(head) = head.as_mut() {
                *head = head.insert_left(value);

                break head.clone();
            } else {
                let mut tail = try_lock!(self.tail);

                let node = Node::new_insulated(value);

                *head = node.clone().into();
                *tail = node.clone().into();

                break node;
            }
        }
    }

    pub fn push_back(&self, value: T) -> Node<T> {
        let mut tail = self.tail.lock().unwrap();

        if let Some(tail) = tail.as_mut() {
            *tail = tail.insert_right(value);

            tail.clone()
        } else {
            let mut head = self.head.lock().unwrap();

            let node = Node::new_insulated(value);

            *head = node.clone().into();
            *tail = node.clone().into();

            node
        }
    }

    pub fn pop_front(&self) -> Option<Arc<T>> {
        let mut tail = self.tail.lock().unwrap();
        let mut head = self.head.lock().unwrap();

        if *tail == *head {
            if let Some(head) = head.take() {
                tail.take();

                head.value.into()
            } else {
                None
            }
        } else {
            drop(tail);

            let head = unsafe { head.as_mut().unwrap_unchecked() };

            let (value, right) = head.insulate_right_owned();

            *head = unsafe { right.unwrap_unchecked() };

            value.into()
        }
    }

    pub fn pop_back(&self) -> Option<Arc<T>> {
        let mut tail = self.tail.lock().unwrap();

        if let Some(tail) = tail.as_mut() {
            let (value, left) = tail.insulate_left_owned();

            if let Some(left) = left {
                *tail = left;
            } else {
                self.head.lock().unwrap().take();
            }

            value.into()
        } else {
            None
        }
    }
}

impl<T> Clone for LinkedList<T> {
    fn clone(&self) -> Self {
        Self {
            head: Arc::clone(&self.head),
            tail: Arc::clone(&self.tail),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_front() {
        let list = LinkedList::new();

        list.push_front(2);
        list.push_front(1);

        assert_eq!(
            list.head()
                .unwrap()
                .into_iter()
                .map(|a| *a)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn push_back() {
        let list = LinkedList::new();

        list.push_back(1);
        list.push_back(2);

        assert_eq!(
            list.head()
                .unwrap()
                .into_iter()
                .map(|a| *a)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn pop_front() {
        let list = LinkedList::new();

        list.push_back(1);
        list.push_back(2);

        assert_eq!(*list.pop_front().unwrap(), 1);

        assert_eq!(
            list.head()
                .unwrap()
                .into_iter()
                .map(|a| *a)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn pop_back() {
        let list = LinkedList::new();

        list.push_back(1);
        list.push_back(2);

        assert_eq!(*list.pop_back().unwrap(), 2);

        assert_eq!(
            list.head()
                .unwrap()
                .into_iter()
                .map(|a| *a)
                .collect::<Vec<_>>(),
            vec![1]
        );
    }
}
