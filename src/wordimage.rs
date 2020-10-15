use crate::classy::Classy;
use crate::config::{WordImagesConfig, WordImageData};
use crate::ready::Ready;

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::collections::HashSet;

use rand::seq::SliceRandom;
use validator::Validate;
use bus::BusReader;

use log::*;

pub struct WordImage {
    classy: Classy,
    word_images: Vec<WordImageData>,
}

impl WordImage {
    pub fn new(config: &WordImagesConfig) -> Self {
        Self {
            classy: Classy::new(),
            word_images: config.word_images.to_vec(),
        }
    }

    fn all_words(&self) -> Vec<String> {
        let temp_vec: Vec<String> = self.word_images.iter().map(|i| i.words.to_vec()).flatten().collect();
        let temp_hash: HashSet<String> = temp_vec.into_iter().collect();
        temp_hash.into_iter().collect()
    }

    pub fn show_images(&self, input: &str) {
        let words_owd = self.all_words();
        let words: Vec<_> = words_owd.iter().map(String::as_str).collect();
        if let Some(labels) = self.classy.classify_with_lables(input, &words) {
            let strong_labels: Vec<_> = labels.iter().filter(|i| i.score > 0.96).collect();
            let target_label = strong_labels.choose(&mut rand::thread_rng());
            if let Some(target_label) = target_label {
                let valid_word_images: Vec<_> = self.word_images.iter().filter(|i| i.words.contains(&target_label.text)).collect();
                let target_word_image = valid_word_images.choose(&mut rand::thread_rng());
                if let Some(target_word_image) = target_word_image {
                    let path = &target_word_image.path;
                    if path.exists() {
                        if let Ok(output) = std::process::Command::new("imgcat").args(&[path]).output() {
                            println!("{}", String::from_utf8_lossy(&output.stdout).into_owned());
                        } else {
                            error!("Failed to show imgcat for {:?}", path);
                        }
                    }
                }
            }
        }
    }
}


pub fn start_wordimages(keep_running: Arc<AtomicBool>, ready_count: &Ready, config_path: &str, mut input_recv: BusReader<String>) {
    defer_on_unwind!{ keep_running.store(false, Ordering::Relaxed); }
    debug!("Wordimages: Loading");
    if let Ok(config_str) = std::fs::read_to_string(config_path) {
        if let Ok(word_config) = toml::from_str::<WordImagesConfig>(&config_str) {
            if word_config.validate().is_ok() {
                let wordy = WordImage::new(&word_config);
                ready_count.not_ready("wordimage");
                debug!("Wordimages: Ready");

                while (*keep_running).load(Ordering::Relaxed) {
                    if let Ok(input) = input_recv.try_recv() {
                        wordy.show_images(&input);
                        ready_count.ready("wordimage");
                        while ! ready_count.all_ready() && (*keep_running).load(Ordering::Relaxed)  {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
            } else {
                ready_count.ready("wordimage");
                debug!("Wordimages: Error not valid WordImagesConfig");
            }
        } else {
            ready_count.ready("wordimage");
            debug!("Wordimages: Error not valid toml");
        }
    } else {
        ready_count.ready("wordimage");
        debug!("Wordimages: Error valid file");
    }
    (*keep_running).store(false, Ordering::Relaxed);
}
