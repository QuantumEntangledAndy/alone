#![deny(missing_docs)]
//! Alone
//!
//! This is a program that creates a small
//! chat bot using rust-bert.
//! It uses telegram to send and recieve
//! input but terminal input is ok too.

use crossbeam::scope;

use std::io;
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc};
use std::time::Duration;
use std::sync::mpsc::RecvTimeoutError;

use log::*;
use err_derive::Error;
use validator::Validate;
use bus::{Bus, BusReader};
use scopeguard::defer_on_unwind;
use tokio::runtime::Runtime;
use clap::Clap;

mod conv;
mod classy;
mod senti;
mod enti;
mod config;
mod wordimage;
mod ready;
mod status;
mod telegram;

use self::conv::{start_conv};
use self::config::{Config};
use self::wordimage::{start_wordimages};
use self::ready::Ready;
use self::status::Status;
use self::telegram::start_telegram;

const RX_TIMEOUT: Duration = Duration::from_millis(500);

/// Enum of applicable errors
#[derive(Debug, Error)]
pub enum Error {
    /// Occurs when the conversation is not found
    /// in the conversation manager
    #[error(display = "Unknown Speaker")]
    ConversationUnknown,
    /// Occurs when conversation model errors
    /// during receiveing a users message
    #[error(display = "Can't Hear")]
    UnableToHear,
    /// Occurs when the model errors when generating
    /// reply
    #[error(display = "Can't Speak")]
    UnableToSpeak,
    /// Occurs if the model fails to load the history
    /// file
    #[error(display = "Can't remember what happened")]
    UnableToWriteJournel,
    /// Occurs if the config file fails to validate
    #[error(display = "Config file invalid")]
    ValidationError(#[error(source)] validator::ValidationErrors),
    /// Occurs if the config file fails to deseralise
    #[error(display = "Config syntax invalid")]
    ConfigError(#[error(source)] toml::de::Error),
    /// Occurs if there is an io error during reading of
    /// the config
    #[error(display = "Cannot read config")]
    IoError(#[error(source)] std::io::Error),
}

#[derive(Clap)]
#[clap(author, about, version)]
struct Opts {
    /// Set the location of the config file
    ///
    /// If the config file is not given then
    /// it defaults to alone.toml in the cwd
    #[clap(short = 'c', long = "config", default_value = "alone.toml")]
    config: String,
}


fn main() -> Result<(), Error> {
    let opts: Opts = Opts::parse();

    let config: Config = toml::from_str(&std::fs::read_to_string(opts.config)?)?;
    config.validate()?;

    if config.debug {
        std::env::set_var("RUST_LOG", "alone=debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "alone=info");
    }
    pretty_env_logger::init();

    info!("Finding {}", config.bot_name);

    let status_arc = Arc::new(Status::new());

    let ready = Arc::new(Ready::new());

    let mut send_to_bot = Bus::new(1000);
    let mut send_to_me = Bus::new(1000);

    debug!("Setting up stop signals");
    let status = status_arc.clone();
    ctrlc::set_handler(move || {
        if ! status.is_alive() {
            std::process::exit(1);
        } else {
            status.stop();
        }
    })
    .expect("Error setting Ctrl-C handler");

    scope(|s| {
        let get_from_me = send_to_bot.add_rx();
        let get_from_bot = send_to_me.add_rx();
        let get_from_bot_to_wordy = send_to_me.add_rx();

        let status = status_arc.clone();
        let model_name = config.model_name.clone();
        let max_context = config.max_context;
        let ready_count = ready.clone();
        s.spawn(move |_| {
            start_conv(&*status, &model_name, max_context, &*ready_count, get_from_me, send_to_me);
        });

        let status = status_arc.clone();
        let ready_count = ready.clone();
        let model_name = config.classify_model_name.clone();
        let get_picture_from_bot = if let Some(word_images) = config.word_images {
            let mut send_picture_to_me = Bus::new(1000);
            let get_picture_from_bot = Some(send_picture_to_me.add_rx());
            s.spawn(move |_| {
                start_wordimages(&*status, &*ready_count, &model_name, &word_images, get_from_bot_to_wordy, send_picture_to_me);
            });
            get_picture_from_bot
        } else {
            None
        };

        let status = status_arc.clone();
        let ready_count = ready.clone();
        let telegram_token = config.telegram_token.clone();
        let telegram_id = config.telegram_id;
        let bot_name = config.bot_name.clone();
        s.spawn(move |_| {
            if let (Some(token), Some(id)) = (telegram_token, telegram_id) {
                // Create the runtime
                let mut rt = Runtime::new().unwrap();
                let _ = rt.block_on(
                    start_telegram(
                        &token,
                        id,
                        send_to_bot,
                        get_from_bot,
                        get_picture_from_bot,
                        &*status,
                        &*ready_count,
                    )
                );
            } else {
                console_input(&*status, &*ready_count, send_to_bot, get_from_bot, get_picture_from_bot, &bot_name);
            }
        });
    }).unwrap();

    Ok(())

}

fn console_input(
    status: &Status,
    ready_count: &Ready,
    mut send_input: Bus<String>,
    mut get_from_bot: BusReader<String>,
    mut get_picture_from_bot: Option<BusReader<Option<PathBuf>>>,
    bot_name: &str,
) {
    defer_on_unwind!{ status.stop(); }
    while ! ready_count.all_ready() && status.is_alive()  {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    debug!("Starting conv");
    while status.is_alive() {
        print!("You: ");
        let mut input = String::new();
        io::stdout().flush().expect("Could not flush stdout");
        let stdin = io::stdin();
        let mut write_stdin = stdin.lock();
        'outer: while status.is_alive() {
            let buffer = write_stdin.fill_buf().unwrap();
            for (i, byte) in buffer.iter().enumerate() {
                if byte == &(0x0A as u8) {
                    input = String::from_utf8_lossy(&buffer[0..i]).into_owned();
                    write_stdin.consume(i+1);
                    break 'outer;
                }
            }
        }
        if ! status.is_alive() {
            break; // Early exit
        }
        if input.len() > 1 {
            ready_count.set_all(false);
            let input = match input.chars().last().unwrap() {
                '!'|'.'|'?' => input.trim().to_string(),
                _ => format!("{}.", input.trim().to_string()),
            };

            send_input.broadcast(input);
            while status.is_alive() {
                match get_from_bot.recv_timeout(RX_TIMEOUT) {
                    Ok(reply) => {
                        println!("{}: {}", bot_name, reply);
                        break;
                    },
                    Err(RecvTimeoutError::Disconnected) => {
                        status.stop();
                        error!("Bot communication channel dropped.");
                        break;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        continue;
                    }
                }
            }
            if let Some(get_picture_from_bot) = get_picture_from_bot.as_mut() {
                while status.is_alive() {
                    match get_picture_from_bot.recv_timeout(RX_TIMEOUT) {
                        Ok(image_path) => {
                            if let Some(image_path) = image_path {
                                if let Ok(output) = std::process::Command::new("imgcat").args(&[&image_path]).output() {
                                    println!("{}", String::from_utf8_lossy(&output.stdout).into_owned());
                                } else {
                                    error!("Failed to show imgcat for {:?}", image_path);
                                }
                            }
                            break;
                        },
                        Err(RecvTimeoutError::Disconnected) => {
                            status.stop();
                            error!("Bot pic communication channel dropped.");
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => {
                            continue;
                        }
                    }
                }
            }
        } else {
            status.stop();
            break;
        }
    }
}
