use crossbeam::scope;

use std::io;
use std::io::prelude::*;

use std::sync::{Arc};

use log::*;
use err_derive::Error;
use validator::Validate;
use bus::{Bus};

#[macro_use(defer_on_unwind)] extern crate scopeguard;

mod conv;
mod classy;
mod senti;
mod enti;
mod config;
mod wordimage;
mod ready;
mod status;

use self::conv::{start_conv};
use self::config::{Config};
use self::wordimage::{start_wordimages};
use self::ready::Ready;
use self::status::Status;

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

    let status_arc = Arc::new(Status::new());

    let ready = Arc::new(Ready::new());

    let mut send_input = Bus::new(1000);

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
        let input_recv = send_input.add_rx();
        let status = status_arc.clone();
        let model_name = config.model_name.clone();
        let max_context = config.max_context;
        let ready_count = ready.clone();
        s.spawn(move |_| {
            start_conv(&*status, &model_name, max_context, &*ready_count, input_recv);
        });

        let input_recv = send_input.add_rx();
        let status = status_arc.clone();
        let ready_count = ready.clone();
        if let Some(word_images) = config.word_images {
            s.spawn(move |_| {
                start_wordimages(&*status, &*ready_count, &word_images, input_recv);
            });
        }

        let status = status_arc.clone();
        let ready_count = ready.clone();
        s.spawn(move |_| {
            console_input(&*status, &*ready_count, send_input);
        });
    }).unwrap();

    Ok(())

}

fn console_input(status: &Status, ready_count: &Ready, mut send_input: Bus<String>) {
    defer_on_unwind!{ status.stop(); }
    while ! ready_count.all_ready() && status.is_alive()  {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    debug!("Starting conv");
    while status.is_alive() {
        if ! status.is_alive() {
            break;
        }
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
            break;
        }
        if input.len() > 1 {
            ready_count.set_all(false);
            let input = match input.chars().last().unwrap() {
                '!'|'.'|'?' => input.trim().to_string(),
                _ => format!("{}.", input.trim().to_string()),
            };

            send_input.broadcast(input);
        } else {
            status.stop();
            break;
        }
        while ! ready_count.all_ready() && status.is_alive()  {
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}
