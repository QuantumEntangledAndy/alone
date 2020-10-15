use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use std::collections::HashMap;

pub struct Ready {
    ready_tracking: Mutex<HashMap<&'static str, AtomicBool>>,
}

impl Ready {
    pub fn new() -> Self {
        Self{
            ready_tracking: Default::default(),
        }
    }

    pub fn not_ready(&self, tag: &'static str) {
        let mut ready_tracking = self.ready_tracking.lock().unwrap();
        let status = ready_tracking.entry(tag).or_insert_with(|| AtomicBool::new(false));
        status.store(false, Ordering::Relaxed);
    }

    pub fn ready(&self, tag: &'static str) {
        let mut ready_tracking = self.ready_tracking.lock().unwrap();
        let status = ready_tracking.entry(tag).or_insert_with(|| AtomicBool::new(true));
        status.store(true, Ordering::Relaxed);
    }

    pub fn all_ready(&self) -> bool {
        let ready_tracking = self.ready_tracking.lock().unwrap();
        for val in ready_tracking.values() {
            if ! val.load(Ordering::Relaxed) {
                return false;
            }
        }
        true
    }

    pub fn set_all(&self, value: bool) {
        let ready_tracking = self.ready_tracking.lock().unwrap();
        for val in ready_tracking.values() {
            val.store(value, Ordering::Relaxed);
        }
    }
}
