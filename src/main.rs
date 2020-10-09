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
        std::env::set_var("RUST_LOG", "alone=debug,cached_path::cache=info");
    }
    pretty_env_logger::init();

    info!("Finding {}", BOT_NAME);
    let mut conversation = Conversation::new_empty();

    let history_path: PathBuf = PathBuf::from(r"./past.history");
    let user_past_str = fs::read_to_string(&history_path).unwrap_or_else(|_| {
        info!("{} does not know you yet", BOT_NAME);
        "".to_string()
    });

    let mut user_past: Vec<String> = user_past_str.lines().map(|i| i.to_string()).filter(|x| !x.is_empty()).collect();

    for line in &user_past {
        #[allow(clippy::collapsible_if)]
        if ! line.is_empty() {
            if conversation.add_user_input(&line).is_err() {
                error!("{} failed to remember", BOT_NAME);
            }
        }
    }

    conversation.mark_processed();

    debug!("Loading conv manager");
    let mut conversation_manager = ConversationManager::new();
    let conversation_uuid = conversation_manager.add(conversation);

    debug!("Loading conv model");
    let conversation_config = ConversationConfig::default();
    let conversation_model = ConversationModel::new(conversation_config).expect("Unable to setup model");

    let keep_running = Arc::new(AtomicBool::new(true));

    debug!("Setting up stop signals");
    let keep_running_signal = keep_running.clone();
    let mut signal_count = 0;
    ctrlc::set_handler(move || {
        if signal_count > 0 {
            std::process::exit(1);
        } else {
            (*keep_running_signal).store(true, Ordering::Release);
            signal_count += 1;
        }
    })
    .expect("Error setting Ctrl-C handler");

    let mut input = String::new();
    debug!("Starting conv");
    while (*keep_running).load(Ordering::Acquire) {
        print!("You: ");
        io::stdout().flush().expect("Could not flush stdout");
        if io::stdin().read_line(&mut input).is_err() {
            error!("You lost your voice");
            continue;
        }
        if let Some(convo) = conversation_manager.get(&conversation_uuid) {
            if convo.add_user_input(&input).is_err() {
                error!("{} couldn't hear you", BOT_NAME);
                continue;
            }
            let output = conversation_model.generate_responses(&mut conversation_manager);
            println!("{}: {}", BOT_NAME, output[&conversation_uuid]);
            user_past.push(input.clone());
        }
    }

    info!("Leaving town");
    if std::fs::write(&history_path, user_past.iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<String>>().join("\n").as_bytes()).is_err() {
        error!("{} just lost their memory", BOT_NAME)
    }
}
