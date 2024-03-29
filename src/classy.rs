use rust_bert::pipelines::common::ModelType;
use rust_bert::pipelines::sequence_classification::Label;
use rust_bert::pipelines::zero_shot_classification::{
    ZeroShotClassificationConfig, ZeroShotClassificationModel,
};
use rust_bert::resources::{LocalResource, ResourceProvider};

use std::path::PathBuf;

pub struct Classy {
    model: ZeroShotClassificationModel,
}

impl Classy {
    pub fn new(model_name: &str) -> Self {
        let sequence_classification_model = if model_name == "default" {
            ZeroShotClassificationModel::new(Default::default()).expect("Unable to setup model")
        } else {
            let merges_path = PathBuf::from(format!("./{}.model/merges.txt", model_name));
            let merges: Option<Box<(dyn ResourceProvider + Send + 'static)>> =
                if merges_path.exists() {
                    Some(Box::new(LocalResource {
                        local_path: merges_path,
                    }))
                } else {
                    None
                };
            let config = ZeroShotClassificationConfig {
                model_type: ModelType::Bart,
                model_resource: Box::new(LocalResource {
                    local_path: PathBuf::from(format!("./{}.model/model.ot", model_name)),
                }),
                config_resource: Box::new(LocalResource {
                    local_path: PathBuf::from(format!("./{}.model/config.json", model_name)),
                }),
                vocab_resource: Box::new(LocalResource {
                    local_path: PathBuf::from(format!("./{}.model/vocab.json", model_name)),
                }),
                merges_resource: merges,
                lower_case: false,
                strip_accents: None,
                add_prefix_space: None,
                ..Default::default()
            };
            ZeroShotClassificationModel::new(config).expect("Unable to setup model")
        };

        Self {
            model: sequence_classification_model,
        }
    }

    #[allow(dead_code)]
    pub fn classify(&self, input: &str) -> Option<Vec<Label>> {
        let candidate_labels = &["love", "hello", "location", "time", "sex"];
        self.model
            .predict_multilabel([input], candidate_labels, None, 128)
            .pop()
    }

    pub fn classify_with_lables(
        &self,
        input: &str,
        candidate_labels: &[&str],
    ) -> Option<Vec<Label>> {
        self.model
            .predict_multilabel([input], candidate_labels, None, 128)
            .pop()
    }
}
