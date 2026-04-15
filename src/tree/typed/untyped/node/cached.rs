use std::sync::Mutex;

#[derive(Debug)]
pub struct Cached<T>(Mutex<Option<T>>);

impl<T: Clone> Cached<T> {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub fn invalidate(&mut self) {
        *self.0.lock().unwrap() = None;
    }

    pub fn get<F>(&self, compute: F) -> T
    where
        F: FnOnce() -> T,
    {
        let stored = &mut *self.0.lock().unwrap();
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

impl<T: Clone> Default for Cached<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for Cached<T> {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().unwrap().clone()))
    }
}
