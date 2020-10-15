use rust_bert::pipelines::conversation::{ConversationModel, ConversationManager, Conversation, ConversationConfig};
use rust_bert::resources::{LocalResource, Resource};
use tch::{Device};
use uuid::Uuid;

use std::fs;
use std::path::{PathBuf};
use std::sync::{Mutex, Arc};
use scopeguard::defer_on_unwind;

use bus::BusReader;

use log::*;

use crate::BOT_NAME;
use crate::Error;
use crate::ready::Ready;
use crate::status::Status;

pub struct Conv {
    model: ConversationModel,
    manager: Mutex<ConversationManager>,
    uuid: Uuid,
    past: Mutex<Vec<String>>,
    max_context: usize,
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
                    (*conversation).mark_processed();
                }
            }
            Ok(())
        } else {
            Err(Error::ConversationUnknown)
        }
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
                if convo.add_user_input(&input).is_err() {
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

    pub fn save_past(&self, file_path: &str) -> Result<(), Error> {
        if std::fs::write(&file_path, self.past.lock().unwrap().iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<String>>().join("\n").as_bytes()).is_err() {
            Err(Error::UnableToWriteJournel)
        } else {
            Ok(())
        }
    }
}


pub fn start_conv(status: &Status, model_name: &str, max_context: usize, ready_count: &Ready, mut input_recv: BusReader<String>) {
    defer_on_unwind! { status.stop() }
    debug!("Conversation model: Loading");
    let conv = Arc::new(Conv::new(&model_name, max_context));
    if conv.add_past("./past.history").is_err() {
        error!("{} couldn't remember the past.", BOT_NAME);
    }
    ready_count.not_ready("conv");
    debug!("Conversation model: Ready");

    while status.is_alive() {
        if let Ok(input) = input_recv.try_recv() {
            match conv.say(&input) {
                Err(Error::UnableToHear) => error!("{} couldn't hear you", BOT_NAME),
                Err(Error::UnableToSpeak) => error!("{} couldn't speak to you", BOT_NAME),
                Err(Error::ConversationUnknown) => error!("{} doesn't know you", BOT_NAME),
                Err(_) => {}
                Ok(output) => {
                    debug!("Dialogue");
                    println!("{}: {}", BOT_NAME, output);
                }
            }
            ready_count.ready("conv");
            while ! ready_count.all_ready() && status.is_alive()  {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
    info!("Leaving town");
    if conv.save_past("./past.history").is_err() {
        error!("{} failed to remember todays session.", BOT_NAME);
    }
    status.stop();
}
