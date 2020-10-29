use std::sync::{Mutex, atomic::{AtomicBool, Ordering}};
use futures::future::AbortHandle;
use std::collections::HashMap;
use bus::{Bus, BusReader};
use std::path::PathBuf;

pub struct AppCtl{
    alive: AtomicBool,
    abort_handles: Mutex<HashMap<&'static str, AbortHandle>>,
    images: AtomicBool,
    me_channel: Mutex<Bus<String>>,
    bot_channel: Mutex<Bus<String>>,
    bot_pic_channel: Mutex<Bus<Option<PathBuf>>>,
}

impl AppCtl {
    pub fn new() -> Self {
        Self {
            alive: AtomicBool::new(true),
            abort_handles: Mutex::new(Default::default()),
            images: AtomicBool::new(false),
            me_channel: Mutex::new(Bus::new(1000)),
            bot_channel: Mutex::new(Bus::new(1000)),
            bot_pic_channel: Mutex::new(Bus::new(1000)),
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
        abort_handles.remove(tag);
        abort_handles.insert(tag, handle);
    }

    pub fn enable_images(&self, enabled: bool) {
        self.images.store(enabled, Ordering::Relaxed);
    }
    pub fn images_enabled(&self) -> bool {
        self.images.load(Ordering::Relaxed)
    }

    pub fn broadcast_me_channel(&self, message: &str) {
        let mut me_channel = self.me_channel.lock().unwrap();
        me_channel.broadcast(message.to_string());
    }

    pub fn listen_me_channel(&self) -> BusReader<String> {
        let mut me_channel = self.me_channel.lock().unwrap();
        me_channel.add_rx()
    }

    pub fn broadcast_bot_channel(&self, message: &str) {
        let mut bot_channel = self.bot_channel.lock().unwrap();
        bot_channel.broadcast(message.to_string());
    }

    pub fn listen_bot_channel(&self) -> BusReader<String> {
        let mut bot_channel = self.bot_channel.lock().unwrap();
        bot_channel.add_rx()
    }

    pub fn broadcast_bot_pic_channel(&self, message: Option<PathBuf>) {
        let mut bot_pic_channel = self.bot_pic_channel.lock().unwrap();
        bot_pic_channel.broadcast(message);
    }

    pub fn listen_bot_pic_channel(&self) -> BusReader<Option<PathBuf>> {
        let mut bot_pic_channel = self.bot_pic_channel.lock().unwrap();
        bot_pic_channel.add_rx()
    }
}
