use crossbeam_channel::unbounded;
use crossbeam::scope;

use std::io;
use std::io::prelude::*;

use std::sync::{Arc, atomic::{AtomicBool, AtomicUsize, Ordering}};

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
    let config: Config = toml::from_str(&std::fs::read_to_string("alone.toml")?)?;
    config.validate()?;

    if config.debug {
        std::env::set_var("RUST_LOG", "alone=debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "alone=info");
    }
    pretty_env_logger::init();

    info!("Finding {}", BOT_NAME);

    let keep_running_arc = Arc::new(AtomicBool::new(true));
    let num_channels: usize;
    if config.debug {
        num_channels = 4;
    } else {
        num_channels = 1;
    }
    let ready_count_arc = Arc::new(AtomicUsize::new(num_channels));

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
        let ready_count = ready_count_arc.clone();
        s.spawn(move |_| {
            debug!("Conversation model: Loading");
            let conv = Arc::new(Conv::new(&model_name));
            if conv.add_past("./past.history").is_err() {
                error!("{} couldn't remember the past.", BOT_NAME);
            }
            (*ready_count).fetch_sub(1, Ordering::Release);
            debug!("Conversation model: Ready");

            while (*keep_running).load(Ordering::Acquire) {
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
                    (*ready_count).fetch_sub(1, Ordering::Release);
                    while (*ready_count).load(Ordering::Acquire) > 0 && (*keep_running).load(Ordering::Acquire)  {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
            info!("Leaving town");
            if conv.save_past("./past.history").is_err() {
                error!("{} failed to remember todays session.", BOT_NAME);
            }
            (*keep_running).store(false, Ordering::Release);
        });

        if config.debug {
            let input_recv = get_input.clone();
            let keep_running = keep_running_arc.clone();
            let ready_count = ready_count_arc.clone();
            s.spawn(move |_| {
                debug!("Classification model: Loading");
                let classy = Classy::new();
                (*ready_count).fetch_sub(1, Ordering::Release);
                debug!("Classification model: Ready");

                while (*keep_running).load(Ordering::Acquire) {
                    if let Ok(input) = input_recv.try_recv() {
                        let output = classy.classify(&input);
                        debug!("Classification");
                        debug!("{:?}", output);
                        (*ready_count).fetch_sub(1, Ordering::Release);
                        while (*ready_count).load(Ordering::Acquire) > 0 && (*keep_running).load(Ordering::Acquire)  {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
                (*keep_running).store(false, Ordering::Release);
            });
        }

        if config.debug {
            let input_recv = get_input.clone();
            let keep_running = keep_running_arc.clone();
            let ready_count = ready_count_arc.clone();
            s.spawn(move |_| {
                debug!("Sentiment model: Loading");
                let senti = Senti::new();
                (*ready_count).fetch_sub(1, Ordering::Release);
                debug!("Sentiment model: Ready");
                while (*keep_running).load(Ordering::Acquire) {
                    if let Ok(input) = input_recv.try_recv() {
                        let output = senti.sentimentice(&input);
                        debug!("Sentiment");
                        debug!("{:?}", output);
                        (*ready_count).fetch_sub(1, Ordering::Release);
                        while (*ready_count).load(Ordering::Acquire) > 0 && (*keep_running).load(Ordering::Acquire)  {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
                (*keep_running).store(false, Ordering::Release);
            });
        }

        if config.debug {
            let input_recv = get_input.clone();
            let keep_running = keep_running_arc.clone();
            let ready_count = ready_count_arc.clone();
            s.spawn(move |_| {
                debug!("Entity model: Loading");
                let enti =  Enti::new();
                (*ready_count).fetch_sub(1, Ordering::Release);
                debug!("Entity model: Ready");
                while (*keep_running).load(Ordering::Acquire) {
                    if let Ok(input) = input_recv.try_recv() {
                        let output = enti.entities(&input);
                        debug!("Entities");
                        debug!("{:?}", output);
                        (*ready_count).fetch_sub(1, Ordering::Release);
                        while (*ready_count).load(Ordering::Acquire) > 0 && (*keep_running).load(Ordering::Acquire)  {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
                (*keep_running).store(false, Ordering::Release);
            });
        }

        let keep_running = keep_running_arc.clone();
        let ready_count = ready_count_arc.clone();
        s.spawn(move |_| {
            while (*ready_count).load(Ordering::Acquire) > 0 && (*keep_running).load(Ordering::Acquire)  {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
            debug!("Starting conv");
            while (*keep_running).load(Ordering::Acquire) {
                if ! (*keep_running).load(Ordering::Acquire) {
                    break;
                }
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
                    (*ready_count).store(num_channels, Ordering::Release);
                    for _ in 0..num_channels {
                        if send_input.send(input.to_string()).is_err() {
                            error!("You lost your voice")
                        }
                    }
                } else {
                    (*keep_running).store(false, Ordering::Release);
                    break;
                }
                while (*ready_count).load(Ordering::Acquire) > 0 && (*keep_running).load(Ordering::Acquire)  {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        });
    }).unwrap();

    Ok(())

}
