use crate::classy::Classy;
use crate::config::{WordImagesConfig, WordImageData};
use crate::ready::Ready;
use crate::status::Status;
use crate::RX_TIMEOUT;

use std::path::PathBuf;
use std::collections::HashSet;
use std::sync::mpsc::RecvTimeoutError;

use rand::seq::SliceRandom;
use validator::Validate;
use bus::{BusReader, Bus};
use scopeguard::defer_on_unwind;

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

    fn all_words(&self) -> Vec<String> {
        let temp_vec: Vec<String> = self.word_images.iter().map(|i| i.words.to_vec()).flatten().collect();
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
                let valid_word_images: Vec<_> = self.word_images.iter().filter(|i| i.words.contains(&target_label.text)).collect();
                let target_word_image = valid_word_images.choose(&mut rand::thread_rng());
                if let Some(target_word_image) = target_word_image {
                    return Some(target_word_image.path.clone());
                }
            }
        }
        None
    }
}


pub fn start_wordimages(
    status: &Status,
    ready_count: &Ready,
    model_name: &str,
    config_path: &str,
    mut get_from_bot: BusReader<String>,
    mut send_picture_to_me: Bus<Option<PathBuf>>
) {
    defer_on_unwind!{ status.stop() }
    debug!("Wordimages: Loading");
    ready_count.not_ready("wordimage");
    match std::fs::read_to_string(config_path) {
        Ok(config_str) => {
            match toml::from_str::<WordImagesConfig>(&config_str) {
                Ok(word_config) => {
                    match word_config.validate() {
                        Ok(_) => {
                            let wordy = WordImage::new(model_name, &word_config);
                            debug!("Wordimages: Ready");
                            ready_count.ready("wordimage");

                            while status.is_alive() {
                                match get_from_bot.recv_timeout(RX_TIMEOUT) {
                                    Ok(input) => {
                                        if status.images_enabled() {
                                            send_picture_to_me.broadcast(wordy.get_image_path(&input));
                                        } else {
                                            send_picture_to_me.broadcast(None);
                                        }
                                    },
                                    Err(RecvTimeoutError::Disconnected) => {
                                        status.stop();
                                        error!("Bot communication channel dropped.");
                                        break;
                                    }
                                    Err(RecvTimeoutError::Timeout) => {
                                        continue;
                                    }
                                }
                            }

                            debug!("Wordimages: Shutting down");
                        }
                        Err(e) => {
                            error!("Wordimages: Error not valid WordImagesConfig: {}", e);
                        }
                    }
                },
                Err(e) => {
                    error!("Wordimages: Error not valid toml: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Wordimages: Error file not readable: {}", e);
        }
    }

    status.stop();
}
