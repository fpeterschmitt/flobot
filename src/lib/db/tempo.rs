use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Tempo<T> {
    store: Arc<Mutex<HashMap<T, Instant>>>,
}

/// Tempo provides a simple interface to store keys and check for there expiration. No self-cleaning,
/// meaning it's not suitable for large quantities of data: once a key is added, it will be removed
/// only on lookup.
///
/// Internaly, it uses a standard Arc container so it's safe to use with threads.
///
/// # Example
///
/// ```rust
/// # fn main() {
/// # use flobot::db::tempo::Tempo;
/// # use std::thread::sleep;
/// use std::time::Duration;
/// # let mut tempo = Tempo::new();
/// assert_eq!(false, tempo.exists("try"));
///
/// tempo.set("try", Duration::from_secs(1));
/// assert_eq!(true, tempo.exists("try"));
///
/// tempo.set("expire", Duration::from_millis(100));
/// assert_eq!(true, tempo.exists("expire"));
///
/// sleep(Duration::from_millis(101));
/// assert_eq!(false, tempo.exists("expire"));
///
/// // Share your tempo to other threads.
/// let mut ctempo = tempo.clone(); // move this one to your new thread
/// tempo.set("cloned", Duration::from_secs(1)); // "main" thread sets a key
/// assert_eq!(true, ctempo.exists("cloned")); // this key is available in the other thread
/// # }
/// ```
impl<T: Hash + Eq> Tempo<T> {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set(&mut self, key: T, ttl: Duration) {
        let expire_in = Instant::now().add(ttl);
        let mut store = self.store.lock().unwrap();
        store.insert(key, expire_in);
    }

    pub fn exists(&mut self, key: T) -> bool {
        let mut store = self.store.lock().unwrap();
        let res = store.get(&key);
        match res {
            Some(expire_in) => {
                let now = Instant::now();
                if expire_in.le(&now) {
                    store.remove(&key);
                    return false;
                }
                return true;
            }
            None => return false,
        };
    }
}