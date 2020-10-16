use crate::status::Status;
use crate::ready::Ready;

use futures::StreamExt;
use futures::future::{Abortable, AbortHandle};
use std::path::PathBuf;

use scopeguard::defer_on_unwind;
use bus::{Bus, BusReader};
use telegram_bot::{Api, UpdateKind, UserId, Integer, MessageChat, MessageKind, InputFileUpload, CanReplySendMessage, CanReplySendPhoto, Error as TeleError};

use log::*;

use crate::RX_TIMEOUT;
use crate::BOT_NAME;

pub async fn start_telegram(
    token: &str,
    id: i64,
    mut send_to_bot: Bus<String>,
    mut get_from_bot: BusReader<String>,
    mut get_picture_from_bot: Option<BusReader<Option<PathBuf>>>,
    status: &Status,
    ready_count: &Ready,
) -> Result<(), TeleError> {
    defer_on_unwind!{ status.stop(); }
    while ! ready_count.all_ready() && status.is_alive()  {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    if ! status.is_alive() {
        return Ok(()); // Early exit
    }
    info!("Telegram Starting");

    let api = Api::new(token);

    info!("Telegram Started");

    // Fetch new updates via long poll method
    let mut stream = api.stream();

    'stream: while let Ok(Some(update)) = {
        debug!("Waiting for new message.");
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        status.add_abortable("telegram", abort_handle);
        let future = Abortable::new(stream.next(), abort_registration);
        future.await
    } {
        // If the received update contains a new message...
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            if let MessageChat::Private(user) = &message.chat  {
                if user.id == UserId::new(id as Integer) {
                    if let MessageKind::Text { ref data, .. } = message.kind {
                        // Print received text message to stdout.
                        if ! data.trim().starts_with('/') {
                            println!("You: {}", data.to_string());
                            {
                                send_to_bot.broadcast(data.to_string());
                            }

                            while status.is_alive() {
                                if let Ok(reply) = get_from_bot.recv_timeout(RX_TIMEOUT) {
                                    println!("{}: {}", BOT_NAME, reply);
                                    api.send(message.text_reply(reply)).await?;
                                    break;
                                }
                            }
                            if let Some(get_picture_from_bot) = get_picture_from_bot.as_mut() {
                                while status.is_alive() {
                                    if let Ok(image_path) = get_picture_from_bot.recv_timeout(RX_TIMEOUT) {
                                        if let Some(image_path) = image_path {
                                            if let Ok(image_path_str) = image_path.into_os_string().into_string() {
                                                api.send(message.photo_reply(InputFileUpload::with_path(image_path_str))).await?;
                                            }
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    status.stop();
    Ok(())
}
