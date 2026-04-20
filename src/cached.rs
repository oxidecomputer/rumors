use std::sync::RwLock;

#[derive(Debug)]
pub struct Cached<T>(RwLock<Option<T>>);

impl<T> Cached<T> {
    pub fn new() -> Self {
        Self(RwLock::new(None))
    }

    pub fn invalidate(&mut self) {
        *self.0.write().unwrap() = None;
    }

    pub fn clone_or_compute<F>(&self, compute: F) -> T
    where
        T: Clone,
        F: FnOnce() -> T,
    {
        let stored = &mut *self.0.write().unwrap();
        match stored {
            Some(stored) => stored.clone(),
            None => {
                let computed = compute();
                *stored = Some(computed.clone());
                computed
            }
        }
    }
}

impl<T> From<T> for Cached<T> {
    fn from(value: T) -> Self {
        Self(RwLock::new(Some(value)))
    }
}

impl<T> Default for Cached<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for Cached<T> {
    fn clone(&self) -> Self {
        Self(RwLock::new(self.0.read().unwrap().clone()))
    }
}
