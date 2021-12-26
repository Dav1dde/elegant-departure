use std::sync::{Arc, RwLock};

type List<T> = Arc<RwLock<Vec<T>>>;

pub struct DepList<T: Clone> {
    inner: List<T>,
}

impl<T: Clone> DepList<T> {
    pub fn new() -> Self {
        Self {
            inner: List::default(),
        }
    }

    pub fn push(&self, item: T) {
        self.inner.write().unwrap().push(item);
    }

    pub fn iter(&self) -> DepListIter<T> {
        DepListIter::new(self.inner.clone())
    }
}

pub struct DepListIter<T: Clone> {
    inner: List<T>,
    current: usize,
}

impl<T: Clone> DepListIter<T> {
    fn new(inner: List<T>) -> Self {
        Self { inner, current: 0 }
    }
}

impl<T: Clone> Iterator for DepListIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.inner.read().unwrap().get(self.current).cloned();
        self.current += 1;
        result
    }
}
