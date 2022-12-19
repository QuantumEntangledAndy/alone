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
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use err_derive::Error;
use log::*;
use scopeguard::defer_on_unwind;
use tokio::runtime::Runtime;
use validator::Validate;

mod appctl;
mod classy;
mod config;
mod conv;
mod enti;
mod senti;
mod sumi;
mod telegram;
mod wordimage;

use self::appctl::AppCtl;
use self::config::Config;
use self::conv::start_conv;
use self::telegram::start_telegram;
use self::wordimage::start_wordimages;

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

#[derive(Parser)]
#[command(author, about, version)]
struct Opts {
    /// Set the location of the config file
    ///
    /// If the config file is not given then
    /// it defaults to alone.toml in the cwd
    #[clap(short = 'c', long = "config", default_value = "alone.toml")]
    config: String,
    /// Force the usage of the terminal
    ///
    /// By default the app will use telegram if
    /// the IDs are given. This forces the use of
    /// the terminal even if the IDs are given
    #[clap(short = 't', long = "terminal")]
    force_terminal: bool,
}

fn main() -> Result<(), Error> {
    let opts: Opts = Opts::parse();

    let config: Config = toml::from_str(&std::fs::read_to_string(&opts.config)?)?;
    config.validate()?;

    if config.debug {
        std::env::set_var("RUST_LOG", "alone=debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "alone=info");
    }
    pretty_env_logger::init();

    info!("Finding {}", config.bot_name);

    let appctl = Arc::new(AppCtl::new());

    debug!("Setting up stop signals");
    let appctl_arc = appctl.clone();
    ctrlc::set_handler(move || {
        if !appctl_arc.is_alive() {
            std::process::exit(1);
        } else {
            appctl_arc.stop();
        }
    })
    .expect("Error setting Ctrl-C handler");

    scope(|s| {
        let appctl_arc = appctl.clone();
        let model_name = config.model_name.clone();
        let max_context = config.max_context;
        s.spawn(move |_| {
            start_conv(&appctl_arc, &model_name, max_context);
        });

        let appctl_arc = appctl.clone();
        let model_name = config.classify_model_name.clone();
        let word_images = config.word_images.clone();
        s.spawn(move |_| {
            start_wordimages(&appctl_arc, &model_name, word_images);
        });

        let appctl_arc = appctl.clone();
        let telegram_token = config.telegram_token.clone();
        let telegram_id = config.telegram_id;
        let bot_name = config.bot_name.clone();
        let force_terminal = opts.force_terminal;
        s.spawn(move |_| {
            if force_terminal || telegram_token.is_none() || telegram_id.is_none() {
                console_input(&appctl_arc, &bot_name);
            } else if let (Some(token), Some(id)) = (telegram_token, telegram_id) {
                // Create the runtime
                let mut rt = Runtime::new().unwrap();
                let _ = rt.block_on(start_telegram(&appctl_arc, &token, id, &bot_name));
            }
        });
    })
    .unwrap();

    Ok(())
}

fn console_input(appctl: &AppCtl, bot_name: &str) {
    defer_on_unwind! { appctl.stop(); }
    let mut get_from_bot = appctl.listen_bot_channel();
    let mut get_picture_from_bot = appctl.listen_bot_pic_channel();

    debug!("Starting conv");
    while appctl.is_alive() {
        print!("You: ");
        let mut input = String::new();
        io::stdout().flush().expect("Could not flush stdout");
        let stdin = io::stdin();
        let mut write_stdin = stdin.lock();
        'outer: while appctl.is_alive() {
            let buffer = write_stdin.fill_buf().unwrap();
            for (i, byte) in buffer.iter().enumerate() {
                if byte == &0x0A {
                    input = String::from_utf8_lossy(&buffer[0..i]).into_owned();
                    write_stdin.consume(i + 1);
                    break 'outer;
                }
            }
        }
        if !appctl.is_alive() {
            break; // Early exit
        }
        if input.len() > 1 {
            let input = match input.chars().last().unwrap() {
                '!' | '.' | '?' => input.trim().to_string(),
                _ => format!("{}.", input.trim()),
            };

            appctl.broadcast_me_channel(&input);
            while appctl.is_alive() {
                match get_from_bot.recv_timeout(RX_TIMEOUT) {
                    Ok(reply) => {
                        println!("{}: {}", bot_name, reply);
                        break;
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        appctl.stop();
                        error!("Bot communication channel dropped.");
                        break;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        continue;
                    }
                }
            }
            while appctl.is_alive() {
                match get_picture_from_bot.recv_timeout(RX_TIMEOUT) {
                    Ok(image_path) => {
                        if let Some(image_path) = image_path {
                            if let Ok(output) = std::process::Command::new("imgcat")
                                .args([&image_path])
                                .output()
                            {
                                println!(
                                    "{}",
                                    String::from_utf8_lossy(&output.stdout).into_owned()
                                );
                            } else {
                                error!("Failed to show imgcat for {:?}", image_path);
                            }
                        }
                        break;
                    }
                    Err(RecvTimeoutError::Disconnected) => {
                        appctl.stop();
                        error!("Bot pic communication channel dropped.");
                        break;
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        continue;
                    }
                }
            }
        } else {
            appctl.stop();
            break;
        }
    }
}
