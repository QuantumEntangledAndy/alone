use rust_bert::pipelines::conversation::{ConversationModel, ConversationManager, Conversation, ConversationConfig};
use rust_bert::resources::{LocalResource, Resource};
use tch::{Device};
use uuid::Uuid;

use std::fs;
use std::path::{PathBuf};
use std::sync::{Mutex, Arc};
use scopeguard::defer_on_unwind;

use bus::{Bus, BusReader};
use serde::{Deserialize, Serialize};
use inflector::cases::{sentencecase::{is_sentence_case, to_sentence_case}, snakecase::to_snake_case};

use log::*;

use crate::BOT_NAME;
use crate::Error;
use crate::ready::Ready;
use crate::status::Status;
use crate::RX_TIMEOUT;

pub struct Conv {
    model: ConversationModel,
    manager: Mutex<ConversationManager>,
    uuid: Uuid,
    past: Mutex<Vec<String>>,
    max_context: usize,
    history: Mutex<Vec<Past>>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum Speaker {
    Me,
    Bot,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Past {
    speaker: Speaker,
    id: u64,
    message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct History {
    history: Vec<Past>,
}


impl Conv {
    pub fn new(model_name: &str, max_context: usize) -> Self {
        let mut conversation_config;
        if model_name == "default" {
            conversation_config = ConversationConfig::default();
            conversation_config.min_length = 2;
        } else {
            conversation_config = ConversationConfig {
                model_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/model.ot", model_name))}),
                config_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/config.json", model_name))}),
                vocab_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/vocab.json", model_name))}),
                merges_resource: Resource::Local(LocalResource{local_path: PathBuf::from(format!("./{}.model/merges.txt", model_name))}),
                min_length: 2,
                max_length: 100,
                min_length_for_response: 32,
                do_sample: true,
                early_stopping: false,
                num_beams: 5,
                temperature: 1.3,
                top_k: 50,
                top_p: 0.95,
                repetition_penalty: 1.5,
                length_penalty: 1.0,
                no_repeat_ngram_size: 0,
                num_return_sequences: 1,
                device: Device::cuda_if_available(),
            };
        }

        let conversation_model = ConversationModel::new(conversation_config).expect("Unable to setup model");

        let conversation = Conversation::new_empty();

        let mut conversation_manager = ConversationManager::new();
        let conversation_uuid = conversation_manager.add(conversation);

        Self{
            model: conversation_model,
            manager: Mutex::new(conversation_manager),
            uuid: conversation_uuid,
            past: Mutex::new(vec!()),
            max_context,
            history: Mutex::new(Default::default()),
        }
    }

    pub fn remember_past(&self, file_path: &str) -> Result<(), Error> {
        let history_path: PathBuf = PathBuf::from(file_path);
        let user_past_str = fs::read_to_string(&history_path).unwrap_or_else(|_| {
            info!("{} does not know you yet", BOT_NAME);
            "".to_string()
        });

        let mut history_file: History = toml::from_str(&user_past_str).expect("Couldn't load history.");

        let mut conversation_manager = self.manager.lock().unwrap();
        if let Some(conversation) = conversation_manager.get(&self.uuid).as_mut() {
            history_file.history.sort_unstable_by_key(|k| k.id);
            let mut my_history = self.history.lock().unwrap();
            for past in history_file.history {
                match past.speaker {
                    Speaker::Me => {
                        conversation.past_user_inputs.push(past.message.clone());
                    },
                    Speaker::Bot => {
                        conversation.generated_responses.push(past.message.clone());
                    }
                }
                if conversation.add_user_input(&Self::swap_persons(&past.message)).is_err() {
                    error!("Failed to read journal entry");
                }
                conversation.mark_processed();
                (*my_history).push(past.clone());
            }
            (*my_history).sort_unstable_by_key(|k| k.id);
            Ok(())
        } else {
            Err(Error::ConversationUnknown)
        }
    }

    pub fn add_to_journel(&self, speaker: Speaker, message: &str) {
        let mut my_history = self.history.lock().unwrap();
        let new_id;
        if let Some(last_item) = my_history.last() {
            new_id = last_item.id + 1;
        } else {
            new_id = 0
        }
        (*my_history).push(
            Past{
                speaker,
                id: new_id,
                message: message.to_string()
            }
        )
    }

    pub fn say(&self, input: &str) -> Result<String, Error> {
        trace!("  Conv recieved: {}", input);
        let mut conversation_manager = self.manager.lock().unwrap();
        if let Some(convo) = conversation_manager.get(&self.uuid).as_mut() {
            if self.max_context > 0 {
                if convo.past_user_inputs.len() > self.max_context {
                    trace!("Old len: {:?}", convo.past_user_inputs.len());
                    let drain_amount = convo.past_user_inputs.len() - self.max_context;
                    convo.past_user_inputs.drain(0..drain_amount);
                    trace!("New len: {:?}", convo.past_user_inputs.len());
                }
                if convo.generated_responses.len() > self.max_context {
                    trace!("Old len: {:?}", convo.generated_responses.len());
                    let drain_amount = convo.generated_responses.len() - self.max_context;
                    convo.generated_responses.drain(0..drain_amount);
                    trace!("New len: {:?}", convo.generated_responses.len());
                }
                if convo.add_user_input(&Self::swap_persons(input)).is_err() {
                    return Err(Error::UnableToSpeak);
                }
            }
        } else {
            return Err(Error::ConversationUnknown);
        }
        let output = {
            trace!("  Generating responses");
            let resp = self.model.generate_responses(&mut conversation_manager);
            trace!("  Got responses: {:?}", resp);
            if let Some(my_resp) = resp.get(&self.uuid) {
                Ok(my_resp.to_string())
            } else {
                Err(Error::UnableToSpeak)
            }
        }?;
        self.past.lock().unwrap().push(input.to_owned());
        Ok(output)
    }

    pub fn save_journal(&self, file_path: &str) -> Result<(), Error> {
        let my_history = self.history.lock().unwrap();
        if std::fs::write(
            &file_path,
            toml::to_vec(&History{
                history: (*my_history).clone(),
            }).unwrap(),
        ).is_err() {
            Err(Error::UnableToWriteJournel)
        } else {
            Ok(())
        }
    }

    fn swap_persons(input: &str) -> String {
        let mut words = vec![];
        for word in input.split(' ').filter(|i| !i.is_empty()) {
            let mut new_word = match to_snake_case(&word).as_str() {
                "you're" => "I'm",
                "youre" => "I'm",
                "you" => "I",
                "your" => "my",
                "yours" => "mine",
                "yourself" => "myself",
                "i'm" => "you're",
                "i" => "you",
                "my" => "your",
                "me" => "you",
                "mine" => "yours",
                "myself" => "yourself",
                n => n,
            }.to_string();
            if is_sentence_case(&word) {
                new_word = to_sentence_case(&new_word);
            }
            words.push(new_word);
        }

        words.join(" ")
    }
}


pub fn start_conv(
    status: &Status,
    model_name: &str,
    max_context: usize,
    ready_count: &Ready,
    mut get_from_me: BusReader<String>,
    mut send_to_me: Bus<String>
) {
    defer_on_unwind! { status.stop() }
    debug!("Conversation model: Loading");
    ready_count.not_ready("conv");
    let conv = Arc::new(Conv::new(&model_name, max_context));
    if conv.remember_past("./journal.toml").is_err() {
        error!("{} couldn't remember the past.", BOT_NAME);
    }

    debug!("Conversation model: Ready");
    ready_count.ready("conv");

    while status.is_alive() {
        if let Ok(input) = get_from_me.recv_timeout(RX_TIMEOUT) {
            conv.add_to_journel(Speaker::Me, &input);

            match conv.say(&input) {
                Err(Error::UnableToHear) => error!("{} couldn't hear you", BOT_NAME),
                Err(Error::UnableToSpeak) => error!("{} couldn't speak to you", BOT_NAME),
                Err(Error::ConversationUnknown) => error!("{} doesn't know you", BOT_NAME),
                Err(_) => {}
                Ok(output) => {
                    send_to_me.broadcast(output.to_string());
                }
            }
        }
    }
    info!("Leaving town");
    if conv.save_journal("./journal.toml").is_err() {
        error!("Failed to write journal.");
    }
    status.stop();
}
