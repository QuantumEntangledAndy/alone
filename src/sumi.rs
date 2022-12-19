#![allow(dead_code)]
use rust_bert::pegasus::{PegasusConfigResources, PegasusModelResources, PegasusVocabResources};
use rust_bert::pipelines::common::ModelType;
use rust_bert::pipelines::summarization::{SummarizationConfig, SummarizationModel};
use rust_bert::resources::RemoteResource;

pub struct Sumi {
    model: SummarizationModel,
}

impl Sumi {
    pub fn new() -> Self {
        let sumi_model = SummarizationModel::new(SummarizationConfig {
            model_type: ModelType::Pegasus,
            model_resource: Box::new(RemoteResource::from_pretrained(
                PegasusModelResources::CNN_DAILYMAIL,
            )),
            config_resource: Box::new(RemoteResource::from_pretrained(
                PegasusConfigResources::CNN_DAILYMAIL,
            )),
            vocab_resource: Box::new(RemoteResource::from_pretrained(
                PegasusVocabResources::CNN_DAILYMAIL,
            )),
            // merges_resource: Box::new(RemoteResource::from_pretrained(
            //     PegasusModelResources::CNN_DAILYMAIL,
            // )),
            ..Default::default()
        })
        .unwrap();

        Self { model: sumi_model }
    }

    pub fn summary(&self, input: &str) -> Option<String> {
        self.model.summarize(&[input]).pop()
    }
}
