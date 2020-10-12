use rust_bert::pipelines::conversation::{ConversationModel, ConversationManager, Conversation, ConversationConfig};
use rust_bert::resources::{LocalResource, Resource};
use tch::{Device};
use uuid::Uuid;

use std::fs;
use std::path::{PathBuf};

use std::sync::Mutex;

use log::*;

use crate::BOT_NAME;
use crate::Error;

pub struct Conv {
    model: ConversationModel,
    manager: Mutex<ConversationManager>,
    uuid: Uuid,
    past: Mutex<Vec<String>>,
}


impl Conv {
    pub fn new(model_name: &str) -> Self {
        let conversation_config = ConversationConfig {
            model_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/model.ot", model_name))}),
            config_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/config.json", model_name))}),
            vocab_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/vocab.json", model_name))}),
            merges_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/merges.txt", model_name))}),
            min_length: 0,
            max_length: 1000,
            min_length_for_response: 32,
            do_sample: true,
            early_stopping: false,
            num_beams: 1,
            temperature: 1.0,
            top_k: 50,
            top_p: 0.9,
            repetition_penalty: 1.0,
            length_penalty: 1.0,
            no_repeat_ngram_size: 0,
            num_return_sequences: 1,
            device: Device::cuda_if_available(),
        };
        let conversation_model = ConversationModel::new(conversation_config).expect("Unable to setup model");

        let conversation = Conversation::new_empty();

        let mut conversation_manager = ConversationManager::new();
        let conversation_uuid = conversation_manager.add(conversation);

        Self{
            model: conversation_model,
            manager: Mutex::new(conversation_manager),
            uuid: conversation_uuid,
            past: Mutex::new(vec!()),
        }
    }

    pub fn add_past(&self, file_path: &str) -> Result<(), Error> {
        let history_path: PathBuf = PathBuf::from(file_path);
        let user_past_str = fs::read_to_string(&history_path).unwrap_or_else(|_| {
            info!("{} does not know you yet", BOT_NAME);
            "".to_string()
        });

        let mut user_past = self.past.lock().unwrap();
        user_past.append(
            &mut user_past_str.lines().map(|i| i.to_string()).filter(|x| !x.is_empty()).collect::<Vec<String>>()
        );

        let mut conversation_manager = self.manager.lock().unwrap();
        if let Some(conversation) = conversation_manager.get(&self.uuid).as_mut() {
            for line in &(*user_past) {
                #[allow(clippy::collapsible_if)]
                if ! line.is_empty() {
                    if (*conversation).add_user_input(&line).is_err() {
                        error!("{} failed to remember", BOT_NAME);
                    }
                }
            }
            (*conversation).mark_processed();
            Ok(())
        } else {
            Err(Error::ConversationUnknown)
        }
    }

    pub fn say(&self, input: &str) -> Result<String, Error> {
        let mut conversation_manager = self.manager.lock().unwrap();
        if let Some(convo) = conversation_manager.get(&self.uuid).as_mut() {
            if convo.add_user_input(&input).is_err() {
                return Err(Error::UnableToSpeak);
            }
        } else {
            return Err(Error::ConversationUnknown);
        }
        let output = {
            let resp = self.model.generate_responses(&mut conversation_manager);
            let my_resp = resp.get(&self.uuid).unwrap_or_else(|| &"").to_string();
            if my_resp.is_empty() {
                Err(Error::UnableToSpeak)
            } else {
                Ok(my_resp)
            }
        }?;
        println!("{}: {}", BOT_NAME, output);
        self.past.lock().unwrap().push(input.to_owned());
        if let Some(convo) = conversation_manager.get(&self.uuid).as_mut() {
            (*convo).mark_processed();
        }
        Ok(output)
    }

    pub fn save_past(&self, file_path: &str) -> Result<(), Error> {
        if std::fs::write(&file_path, self.past.lock().unwrap().iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<String>>().join("\n").as_bytes()).is_err() {
            Err(Error::UnableToWriteJournel)
        } else {
            Ok(())
        }
    }
}