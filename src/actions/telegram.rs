use log::info;
use teloxide::{requests::Requester, types::ChatId, Bot};

use crate::{server::APIEvent, Config};

async fn send_message_to_channel(bot: &Bot, message: &str, public: bool, config: &Config) {
    let _ = bot
        .send_message(
            ChatId(*if public {
                &config.telegram_chat_id_public
            } else {
                &config.telegram_chat_id_private
            }),
            message,
        )
        .await
        .unwrap();
}
pub async fn run_telegram_bot(mut rx: tokio::sync::broadcast::Receiver<APIEvent>, config: Config) {
    let bot = Bot::new(&config.telegram_api_key);
    info!("Started teloxide bot.");

    let mut last_event = None;

    while let Ok(event) = rx.recv().await {
        info!("Sending message to telegram chat.");
        match (last_event, event) {
            (None, APIEvent::Close) | (Some(APIEvent::Open), APIEvent::Close) => {
                send_message_to_channel(&bot, &config.general_close, true, &config).await;
            }
            (_, APIEvent::OpenIntern) => {
                send_message_to_channel(&bot, &config.member_open, false, &config).await;
            }
            (_, APIEvent::Open) => {
                send_message_to_channel(&bot, &config.general_open, true, &config).await
            }
            (Some(APIEvent::OpenIntern), APIEvent::Close) => {
                send_message_to_channel(&bot, &config.member_close, false, &config).await
            }
            _ => todo!("How did we get here?"),
        };
        last_event = Some(event);
    }
}
