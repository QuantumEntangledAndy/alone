use serde::Deserialize;
use validator::{ValidationError};
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
}

fn deault_debug() -> bool {
    false
}

fn default_max_context() -> usize {
    0
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
