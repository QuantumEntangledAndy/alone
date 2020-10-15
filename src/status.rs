use std::sync::atomic::{AtomicBool, Ordering};

pub struct Status{
    alive: AtomicBool,
}

impl Status {
    pub fn new() -> Self {
        Self {
            alive: AtomicBool::new(true),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    pub fn stop(&self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}
