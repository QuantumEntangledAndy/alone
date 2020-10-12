use crossbeam_channel::unbounded;
use crossbeam::scope;

use std::io;
use std::io::prelude::*;

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use log::*;
use err_derive::Error;
use validator::Validate;

mod conv;
mod classy;
mod senti;
mod enti;
mod config;

use self::conv::Conv;
use self::classy::Classy;
use self::senti::Senti;
use self::enti::Enti;
use self::config::Config;

#[derive(Debug, Error)]
pub enum Error {
    #[error(display = "Unknown Speaker")]
    ConversationUnknown,
    #[error(display = "Can't Hear")]
    UnableToHear,
    #[error(display = "Can't Speak")]
    UnableToSpeak,
    #[error(display = "Can't remember what happened")]
    UnableToWriteJournel,
    #[error(display = "Config file invalid")]
    ValidationError(#[error(source)] validator::ValidationErrors),
    #[error(display = "Config syntax invalid")]
    ConfigError(#[error(source)] toml::de::Error),
    #[error(display = "Cannot read config")]
    IoError(#[error(source)] std::io::Error),
}


const BOT_NAME: &str = "Holly";


fn main() -> Result<(), Error> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "alone=debug,cached_path::cache=info");
    }
    pretty_env_logger::init();

    let config: Config = toml::from_str(&std::fs::read_to_string("alone.toml")?)?;
    config.validate()?;

    info!("Finding {}", BOT_NAME);

    let keep_running_arc = Arc::new(AtomicBool::new(true));
    let (send_input, get_input) = unbounded::<String>();

    debug!("Setting up stop signals");
    let keep_running_signal = keep_running_arc.clone();
    let mut signal_count = 0;
    ctrlc::set_handler(move || {
        if signal_count > 0 {
            std::process::exit(1);
        } else {
            (*keep_running_signal).store(false, Ordering::Release);
            signal_count += 1;
        }
    })
    .expect("Error setting Ctrl-C handler");

    scope(|s| {
        let input_recv = get_input.clone();
        let keep_running = keep_running_arc.clone();
        let model_name = config.model_name.clone();
        s.spawn(move |_| {
            debug!("Loading conversation model");
            let conv = Arc::new(Conv::new(&model_name));
            if conv.add_past("./past.history").is_err() {
                error!("{} couldn't remember the past.", BOT_NAME);
            }

            while (*keep_running).load(Ordering::Acquire) {
                debug!("Dialogue");
                let input = input_recv.recv().expect("We should get data");
                if let Err(e) = conv.say(&input) {
                    match e {
                        Error::UnableToHear => error!("{} couldn't hear you", BOT_NAME),
                        Error::UnableToSpeak => error!("{} couldn't speak to you", BOT_NAME),
                        Error::ConversationUnknown => error!("{} doesn't know you", BOT_NAME),
                        _ => {}
                    }
                }
            }
            info!("Leaving town");
            if conv.save_past("./past.history").is_err() {
                error!("{} failed to remember todays session.", BOT_NAME);
            }
            (*keep_running).store(false, Ordering::Release);
        });

        let input_recv = get_input.clone();
        let keep_running = keep_running_arc.clone();
        s.spawn(move |_| {
            debug!("Loading Classification model");
            let classy = Classy::new();

            while (*keep_running).load(Ordering::Acquire) {
                let input = input_recv.recv().expect("We should get data");
                debug!("Classification");
                let output = classy.classify(&input);
                debug!("{:?}", output);
            }
            (*keep_running).store(false, Ordering::Release);
        });

        let input_recv = get_input.clone();
        let keep_running = keep_running_arc.clone();
        s.spawn(move |_| {
            debug!("Loading sentiment model");
            let senti = Senti::new();

            while (*keep_running).load(Ordering::Acquire) {
                let input = input_recv.recv().expect("We should get data");
                debug!("Sentiment");
                let output = senti.sentimentice(&input);
                debug!("{:?}", output);
            }
            (*keep_running).store(false, Ordering::Release);
        });

        let input_recv = get_input.clone();
        let keep_running = keep_running_arc.clone();
        s.spawn(move |_| {
            debug!("Loading entity model");
            let enti =  Enti::new();

            while (*keep_running).load(Ordering::Acquire) {
                let input = input_recv.recv().expect("We should get data");
                debug!("Entities");
                let output = enti.entities(&input);
                debug!("{:?}", output);
            }
            (*keep_running).store(false, Ordering::Release);
        });

        let keep_running = keep_running_arc.clone();
        s.spawn(move |_| {
            debug!("Starting conv");
            while (*keep_running).load(Ordering::Acquire) {
                print!("You: ");
                let mut input = String::new();
                io::stdout().flush().expect("Could not flush stdout");
                if io::stdin().read_line(&mut input).is_err() {
                    error!("You lost your voice");
                    continue;
                }
                if ! (*keep_running).load(Ordering::Acquire) {
                    break;
                }
                if input.len() > 1 {
                    if send_input.send(input.to_string()).is_err() {
                        error!("You lost your voice")
                    }
                } else {
                    (*keep_running).store(false, Ordering::Release);
                    break;
                }
            }
        });
    }).unwrap();

    Ok(())

}
