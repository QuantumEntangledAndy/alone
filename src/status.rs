use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use futures::future::AbortHandle;
use std::collections::HashMap;

pub struct Status{
    alive: AtomicBool,
    abort_handles: Mutex<HashMap<&'static str, AbortHandle>>,
}

impl Status {
    pub fn new() -> Self {
        Self {
            alive: AtomicBool::new(true),
            abort_handles: Mutex::new(Default::default()),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.alive.store(false, Ordering::Relaxed);
        let abort_handles = &(*self.abort_handles.lock().unwrap());
        for abort_handle in abort_handles.values() {
            abort_handle.abort();
        }
    }

    pub fn add_abortable(&self, tag: &'static str, handle: AbortHandle) {
        let mut abort_handles = self.abort_handles.lock().unwrap();
        abort_handles.insert(tag, handle);
    }
}
