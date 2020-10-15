use crate::status::Status;
use crate::ready::Ready;

use futures::StreamExt;
use std::path::PathBuf;

use scopeguard::defer_on_unwind;
use bus::{Bus, BusReader};
use telegram_bot::{Api, UpdateKind, UserId, Integer, MessageChat, MessageKind, InputFileUpload, CanReplySendMessage, CanReplySendPhoto, Error as TeleError};

use crate::RX_TIMEOUT;

pub async fn start_telegram(
    token: &str,
    id: i64,
    mut send_to_bot: Bus<String>,
    mut get_from_bot: BusReader<String>,
    mut get_picture_from_bot: BusReader<Option<PathBuf>>,
    status: &Status,
    ready_count: &Ready,
) -> Result<(), TeleError> {
    defer_on_unwind!{ status.stop(); }
    while ! ready_count.all_ready() && status.is_alive()  {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let api = Api::new(token);

    // Fetch new updates via long poll method
    let mut stream = api.stream();
    'stream: while let Some(update) = stream.next().await {
        // If the received update contains a new message...
        let update = update?;
        if let UpdateKind::Message(message) = update.kind {
            if let MessageChat::Private(user) = &message.chat  {
                if user.id == UserId::new(id as Integer) {
                    if let MessageKind::Text { ref data, .. } = message.kind {
                        // Print received text message to stdout.
                        println!("<{}>: {}", &message.from.first_name, data);
                        {
                            send_to_bot.broadcast(data.to_string());
                        }

                        while status.is_alive() {
                            if let Ok(reply) = get_from_bot.recv_timeout(RX_TIMEOUT) {
                                api.send(message.text_reply(reply)).await?;
                            }
                        }
                        while status.is_alive() {
                            if let Ok(image_path) = get_picture_from_bot.recv_timeout(RX_TIMEOUT) {
                                if let Some(image_path) = image_path {
                                    if let Ok(image_path_str) = image_path.into_os_string().into_string() {
                                        api.send(message.photo_reply(InputFileUpload::with_path(image_path_str))).await?;
                                    }
                                }
                            }
                        }

                        if ! status.is_alive() {
                            break 'stream;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
