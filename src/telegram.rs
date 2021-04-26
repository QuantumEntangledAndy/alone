use crate::appctl::AppCtl;

use futures::StreamExt;
use futures::future::{Abortable, AbortHandle};

use std::sync::mpsc::RecvTimeoutError;

use scopeguard::defer_on_unwind;
use telegram_bot::{Api, UpdateKind, UserId, Integer, MessageChat, MessageKind, InputFileUpload, CanReplySendMessage, CanReplySendPhoto, Error as TeleError, reply_markup, ReplyKeyboardMarkup};

use log::*;

use crate::RX_TIMEOUT;

#[allow(clippy::too_many_arguments)]
pub async fn start_telegram(
    appctl: &AppCtl,
    token: &str,
    id: i64,
    bot_name: &str,
) -> Result<(), TeleError> {
    defer_on_unwind!{ appctl.stop(); }
    let mut get_from_bot = appctl.listen_bot_channel();
    let mut get_picture_from_bot = appctl.listen_bot_pic_channel();

    while appctl.is_alive() {
        info!("Telegram Starting");

        let api = Api::new(token);

        info!("Telegram Started");

        fn get_reply_keyboard(status: &AppCtl) -> ReplyKeyboardMarkup {
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
            appctl.add_abortable("telegram", abort_handle);
            let future = Abortable::new(stream.next(), abort_registration);
            let result = future.await;
            if let Err(e) = result {
                error!("Telegram error: {:?}", e);
            }
            result
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
                                    appctl.enable_images(false);
                                },
                                "/yesimages" => {
                                    appctl.enable_images(true);
                                },
                                "/stop" => {
                                    appctl.stop();
                                    break;
                                },
                                "/start" => {
                                    reply_message = Some("Waiting for you to say something".to_string());
                                },
                                n if n.starts_with('/') => {
                                    debug!("Got unknown command from telegram {}", n);
                                }
                                n => {
                                    debug!("You: {}", n.to_string());
                                    {
                                        appctl.broadcast_me_channel(&n);
                                    }
                                    while appctl.is_alive() {
                                        match get_from_bot.recv_timeout(RX_TIMEOUT) {
                                            Ok(reply) => {
                                                debug!("{}: {}", bot_name, reply);
                                                reply_message = Some(reply);
                                                break;
                                            },
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
                                                    if let Ok(image_path_str) = image_path.into_os_string().into_string() {
                                                        reply_pic = Some(image_path_str);
                                                    }
                                                }
                                                break;
                                            },
                                            Err(RecvTimeoutError::Disconnected) => {
                                                appctl.stop();
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
                            if appctl.is_alive() {
                                match (reply_message, reply_pic) {
                                    (Some(reply), Some(pic)) => {
                                        let mut send_this = message.photo_reply(InputFileUpload::with_path(pic));
                                        send_this.reply_markup(get_reply_keyboard(&appctl));
                                        send_this.caption(reply);
                                        api.send(send_this).await?;
                                    },
                                    (Some(reply), None) => {
                                        let mut send_this = message.text_reply(reply);
                                        send_this.reply_markup(get_reply_keyboard(&appctl));
                                        api.send(send_this).await?;
                                    },
                                    (None, Some(pic)) => {
                                        let mut send_this = message.photo_reply(InputFileUpload::with_path(pic));
                                        send_this.reply_markup(get_reply_keyboard(&appctl));
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
    }
    debug!("Telegram: Shutting down");
    appctl.stop();
    Ok(())
}
