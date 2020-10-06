use rust_bert::pipelines::conversation::{ConversationModel, ConversationManager, Conversation, ConversationConfig};

use std::fs;
use std::path::{PathBuf};

use std::io;
use std::io::prelude::*;

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use log::*;


const BOT_NAME: &str = "Holly";


fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "alone=info");
    }
    pretty_env_logger::init();

    info!("Finding {}", BOT_NAME);
    let mut conversation = Conversation::new_empty();

    let history_path: PathBuf = PathBuf::from(r"./past.history");
    let user_past = fs::read_to_string(history_path).unwrap_or_else(|_| {
        info!("{} does not know you yet", BOT_NAME);
        "".to_string()
    });

    for line in user_past.lines() {
        #[allow(clippy::collapsible_if)]
        if ! line.is_empty() {
            if conversation.add_user_input(line).is_err() {
                error!("{} failed to remember", BOT_NAME);
            }
        }
    }

    conversation.mark_processed();

    let mut conversation_manager = ConversationManager::new();
    let conversation_uuid = conversation_manager.add(conversation);

    let conversation_config = ConversationConfig::default();
    let conversation_model = ConversationModel::new(conversation_config).expect("Unable to setup model");

    let stop_var = Arc::new(AtomicBool::new(false));

    let stop_var_signal = stop_var.clone();
    ctrlc::set_handler(move || {
        (*stop_var_signal).store(true, Ordering::Relaxed);
    })
    .expect("Error setting Ctrl-C handler");

    let stop_var_loop = stop_var;
    while ! (*stop_var_loop).load(Ordering::Relaxed) {
        print!("You: ");
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if let Some(convo) = conversation_manager.get(&conversation_uuid) {
                let input = line.unwrap();
                if ! input.is_empty() {
                    if convo.add_user_input(&input).is_err() {
                        error!("{} couldn't hear you", BOT_NAME)
                    } else {
                        let output = conversation_model.generate_responses(&mut conversation_manager);
                        println!("{}: {}", BOT_NAME, output[&conversation_uuid]);
                    }
                }
                print!("You: ");
            }
        }
    }

    info!("Leaving town");
}
