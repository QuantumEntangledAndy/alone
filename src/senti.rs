#![allow(dead_code)]
use rust_bert::pipelines::sentiment::{SentimentModel, Sentiment};

pub struct Senti {
    model: SentimentModel,
}

impl Senti {
    pub fn new() -> Self {
        let sentiment_model = SentimentModel::new(Default::default()).expect("Unable to setup model");

        Self{
            model: sentiment_model,
        }
    }

    pub fn sentimentice(&self, input: &str) -> Option<Sentiment> {
        self.model.predict(&[input]).pop()
    }
}
