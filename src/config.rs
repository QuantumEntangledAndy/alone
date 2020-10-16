use serde::Deserialize;
#[allow(unused_imports)] // Validate is required in the arm build but not the amd.
use validator::{ValidationError, Validate};
use validator_derive::Validate;

use std::path::PathBuf;

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct Config {
    #[validate(custom = "ensure_model_files")]
    pub model_name: String,

    #[serde(default = "default_max_context")]
    pub max_context: usize,

    #[serde(default = "deault_debug")]
    pub debug: bool,

    #[serde(default)]
    #[validate(custom = "ensure_word_images")]
    pub word_images: Option<String>,

    #[serde(default)]
    pub telegram_token: Option<String>,

    #[serde(default)]
    pub telegram_id: Option<i64>,
}

fn deault_debug() -> bool {
    false
}

fn default_max_context() -> usize {
    0
}

fn ensure_word_images(word_images: &str) -> Result<(), ValidationError> {
    if PathBuf::from(&word_images).exists() {
        Ok(())
    } else {
        Err(ValidationError::new("Word image config file missind"))
    }
}

fn ensure_model_files(model_name: &str) -> Result<(), ValidationError> {
    if model_name == "default" {
        Ok(())
    }
    else if ! PathBuf::from(format!("./{}.model/model.ot", model_name)).exists() {
        Err(ValidationError::new("Rust model missing"))
    }
    else if ! PathBuf::from(format!("./{}.model/config.json", model_name)).exists() {
        Err(ValidationError::new("Config model missing"))
    }
    else if ! PathBuf::from(format!("./{}.model/vocab.json", model_name)).exists() {
        Err(ValidationError::new("Vocab model missing"))
    }
    else if ! PathBuf::from(format!("./{}.model/merges.txt", model_name)).exists() {
        Err(ValidationError::new("Merges model missing"))
    } else {
        Ok(())
    }
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct WordImagesConfig {
    #[serde(default)]
    #[validate]
    pub word_images: Vec<WordImageData>,
}

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct WordImageData {
    pub path: PathBuf,

    #[validate(length(min = 1))]
    pub words: Vec<String>,
}
