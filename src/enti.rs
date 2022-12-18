#![allow(dead_code)]
use rust_bert::pipelines::ner::{Entity, NERModel};

pub struct Enti {
    model: NERModel,
}

impl Enti {
    pub fn new() -> Self {
        let entity_model = NERModel::new(Default::default()).expect("Unable to setup model");

        Self {
            model: entity_model,
        }
    }

    pub fn entities(&self, input: &str) -> Option<Entity> {
        self.model.predict(&[input]).pop().and_then(|mut p| p.pop())
    }
}
