use std::collections::BinaryHeap;
use std::cmp::Reverse;

pub struct MinHeap<T> {
    pub heap: BinaryHeap<Reverse<T>>,
}

impl<T: Ord> MinHeap<T> {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
        }
    }
    pub fn push(&mut self, item: T) {
        self.heap.push(Reverse(item));
    }
    pub fn pop(&mut self) -> Option<T> {
        self.heap.pop().map_or(None, |rev_t| { Some(rev_t.0) })
    }
    pub fn peek(&self) -> Option<&T> {
        self.heap.peek().map_or(None, |rev_t| { Some(&rev_t.0) })
    }
    pub fn len(&self) -> usize {
        self.heap.len()
    }
}

