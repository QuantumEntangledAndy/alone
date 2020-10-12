use rust_bert::pipelines::zero_shot_classification::ZeroShotClassificationModel;
use rust_bert::pipelines::sequence_classification::Label;

pub struct Classy {
    model: ZeroShotClassificationModel,
}

impl Classy {
    pub fn new() -> Self {
        let sequence_classification_model = ZeroShotClassificationModel::new(Default::default()).expect("Unable to setup model");

        Self{
            model: sequence_classification_model,
        }
    }

    pub fn classify(&self, input: &str) -> Option<Vec<Label>> {
        let candidate_labels = &["love", "hello", "location", "time", "sex"];
        self.model.predict_multilabel(
            &[&input],
            candidate_labels,
            None,
            128,
        ).pop()
    }
}
