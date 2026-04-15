use std::sync::Mutex;

pub struct CachedHash(Mutex<Option<blake3::Hash>>);

impl CachedHash {
    pub fn new() -> Self {
        CachedHash(Mutex::new(None))
    }

    pub fn reset(&mut self) {
        *self.0.lock().unwrap() = None;
    }

    pub fn get<F>(&self, compute: F) -> blake3::Hash
    where
        F: FnOnce() -> blake3::Hash,
    {
        let hash = &mut *self.0.lock().unwrap();
        match *hash {
            Some(hash) => hash,
            None => {
                let h = compute();
                *hash = Some(h);
                h
            }
        }
    }
}

impl Default for CachedHash {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CachedHash {
    fn clone(&self) -> Self {
        Self(Mutex::new(self.0.lock().unwrap().clone()))
    }
}
