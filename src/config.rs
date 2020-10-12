use serde::Deserialize;
use validator::{ValidationError};
use validator_derive::Validate;

use std::path::PathBuf;

#[derive(Debug, Deserialize, Validate, Clone)]
pub struct Config {
    #[validate(custom = "ensure_model_files") ]
    pub model_name: String,
}

fn ensure_model_files(model_name: &str) -> Result<(), ValidationError> {
    if ! PathBuf::from(format!("./{}.model/model.ot", model_name)).exists() {
        return Err(ValidationError::new("Rust model missing"));
    }
    if ! PathBuf::from(format!("./{}.model/config.json", model_name)).exists() {
        return Err(ValidationError::new("Config model missing"));
    }
    if ! PathBuf::from(format!("./{}.model/vocab.json", model_name)).exists() {
        return Err(ValidationError::new("Vocab model missing"));
    }
    if ! PathBuf::from(format!("./{}.model/merges.txt", model_name)).exists() {
        return Err(ValidationError::new("Merges model missing"));
    }
    Ok(())
}
