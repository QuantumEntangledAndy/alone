use crate::appctl::AppCtl;
use crate::classy::Classy;
use crate::config::{WordImageData, WordImagesConfig};
use crate::RX_TIMEOUT;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;

use rand::seq::SliceRandom;
use scopeguard::defer_on_unwind;
use validator::Validate;

use log::*;

pub struct WordImage {
    classy: Classy,
    word_images: Vec<WordImageData>,
}

impl WordImage {
    pub fn new(model_name: &str, config: &WordImagesConfig) -> Self {
        Self {
            classy: Classy::new(model_name),
            word_images: config.word_images.to_vec(),
        }
    }

    pub fn new_from_path(model_name: &str, config_path: &str) -> Result<Self, String> {
        match std::fs::read_to_string(config_path) {
            Ok(config_str) => match toml::from_str::<WordImagesConfig>(&config_str) {
                Ok(word_config) => match word_config.validate() {
                    Ok(_) => Ok(WordImage::new(model_name, &word_config)),
                    Err(e) => Err(format!(
                        "Wordimages: Error not valid WordImagesConfig: {}",
                        e
                    )),
                },
                Err(e) => Err(format!("Wordimages: Error not valid toml: {}", e)),
            },
            Err(e) => Err(format!("Wordimages: Error file not readable: {}", e)),
        }
    }

    fn all_words(&self) -> Vec<String> {
        let temp_vec: Vec<String> = self
            .word_images
            .iter()
            .flat_map(|i| i.words.to_vec())
            .collect();
        let temp_hash: HashSet<String> = temp_vec.into_iter().collect();
        temp_hash.into_iter().collect()
    }

    pub fn get_image_path(&self, input: &str) -> Option<PathBuf> {
        let words_owd = self.all_words();
        let words: Vec<_> = words_owd.iter().map(String::as_str).collect();
        if let Some(labels) = self.classy.classify_with_lables(input, &words) {
            let strong_labels: Vec<_> = labels.iter().filter(|i| i.score > 0.96).collect();
            let target_label = strong_labels.choose(&mut rand::thread_rng());
            if let Some(target_label) = target_label {
                let valid_word_images: Vec<_> = self
                    .word_images
                    .iter()
                    .filter(|i| i.words.contains(&target_label.text))
                    .collect();
                let target_word_image = valid_word_images.choose(&mut rand::thread_rng());
                if let Some(target_word_image) = target_word_image {
                    return Some(target_word_image.path.clone());
                }
            }
        }
        None
    }
}

pub fn start_wordimages(appctl: &AppCtl, model_name: &str, config_path: Option<String>) {
    defer_on_unwind! { appctl.stop() }
    let mut get_from_bot = appctl.listen_bot_channel();
    debug!("Wordimages: Loading");

    let mut wordy: Option<WordImage> = None;

    while appctl.is_alive() {
        if appctl.images_enabled() && wordy.is_none() {
            // Only bother loading if enabled
            if let Some(config_path) = &config_path {
                match WordImage::new_from_path(model_name, config_path) {
                    Ok(new_wordy) => {
                        wordy = Some(new_wordy);
                    }
                    Err(error) => {
                        error!("{}", error);
                    }
                }
            }
        }

        if let Some(wordy) = &wordy {
            // Its been loaded
            debug!("Wordimages: Ready");

            match get_from_bot.recv_timeout(RX_TIMEOUT) {
                // Picture asked for
                Ok(input) => {
                    if appctl.images_enabled() {
                        // Find and send it
                        appctl.broadcast_bot_pic_channel(wordy.get_image_path(&input));
                    } else {
                        // But we have been turned off
                        appctl.broadcast_bot_pic_channel(None);
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    appctl.stop();
                    error!("Bot communication channel dropped.");
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    continue;
                }
            }

            debug!("Wordimages: Shutting down");
        } else {
            // Never loaded
            match get_from_bot.recv_timeout(RX_TIMEOUT) {
                // Picture asked for
                Ok(_) => {
                    appctl.broadcast_bot_pic_channel(None); // But we are not loaded
                }
                Err(RecvTimeoutError::Disconnected) => {
                    appctl.stop();
                    error!("Bot communication channel dropped.");
                    break;
                }
                Err(RecvTimeoutError::Timeout) => {
                    continue;
                }
            }
        }
    }

    appctl.stop();
}
