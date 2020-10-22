use crate::status::Status;
use crate::ready::Ready;

use futures::StreamExt;
use futures::future::{Abortable, AbortHandle};

use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;

use scopeguard::defer_on_unwind;
use bus::{Bus, BusReader};
use telegram_bot::{Api, UpdateKind, UserId, Integer, MessageChat, MessageKind, InputFileUpload, CanReplySendMessage, CanReplySendPhoto, Error as TeleError, reply_markup, ReplyKeyboardMarkup};

use log::*;

use crate::RX_TIMEOUT;


pub async fn start_telegram(
    token: &str,
    id: i64,
    mut send_to_bot: Bus<String>,
    mut get_from_bot: BusReader<String>,
    mut get_picture_from_bot: Option<BusReader<Option<PathBuf>>>,
    status: &Status,
    ready_count: &Ready,
    bot_name: &str,
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

    fn get_reply_keyboard(status: &Status) -> ReplyKeyboardMarkup {
        if status.images_enabled() {
            return reply_markup!(reply_keyboard, selective,
                 ["/stop"],
                 ["/noimages"]
            );
        } else {
            return reply_markup!(reply_keyboard, selective,
                 ["/stop"],
                 ["/yesimages"]
            );
        }
    }

    // Fetch new updates via long poll method
    let mut stream = api.stream();

    while let Ok(Some(update)) = {
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
                if user.id == UserId::new(id as Integer) && message.reply_to_message.is_none() {
                    if let MessageKind::Text { ref data, .. } = message.kind {
                        // Print received text message to stdout.
                        let mut reply_message = None;
                        let mut reply_pic = None;
                        match data.trim() {
                            "/noimages" => {
                                status.enable_images(false);
                            },
                            "/yesimages" => {
                                status.enable_images(true);
                            },
                            "/stop" => {
                                status.stop();
                                break;
                            },
                            "/start" => {
                                reply_message = Some("Waiting for you to say something".to_string());
                            },
                            n if n.starts_with('/') => {
                                debug!("Got unknown command from telegram {}", n)
                            }
                            n => {
                                debug!("You: {}", n.to_string());
                                {
                                    send_to_bot.broadcast(n.to_string());
                                }
                                while status.is_alive() {
                                    match get_from_bot.recv_timeout(RX_TIMEOUT) {
                                        Ok(reply) => {
                                            debug!("{}: {}", bot_name, reply);
                                            reply_message = Some(reply);
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
                                                    if let Ok(image_path_str) = image_path.into_os_string().into_string() {
                                                        reply_pic = Some(image_path_str);
                                                    }
                                                }
                                                break;
                                            },
                                            Err(RecvTimeoutError::Disconnected) => {
                                                status.stop();
                                                error!("Bot picture communication channel dropped.");
                                                break;
                                            }
                                            Err(RecvTimeoutError::Timeout) => {
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if status.is_alive() {
                            match (reply_message, reply_pic) {
                                (Some(reply), Some(pic)) => {
                                    let mut send_this = message.photo_reply(InputFileUpload::with_path(pic));
                                    send_this.reply_markup(get_reply_keyboard(&status));
                                    send_this.caption(reply);
                                    api.send(send_this).await?;
                                },
                                (Some(reply), None) => {
                                    let mut send_this = message.text_reply(reply);
                                    send_this.reply_markup(get_reply_keyboard(&status));
                                    api.send(send_this).await?;
                                },
                                (None, Some(pic)) => {
                                    let mut send_this = message.photo_reply(InputFileUpload::with_path(pic));
                                    send_this.reply_markup(get_reply_keyboard(&status));
                                    api.send(send_this).await?;
                                },
                                (None, None) => {
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    debug!("Telegram: Shutting down");
    status.stop();
    Ok(())
}
