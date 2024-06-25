use log::info;
use teloxide::{requests::Requester, types::ChatId, Bot};

use crate::{
    server::APIEvent, TELEGRAM_API_KEY, TELEGRAM_CHAT_ID_PRIVATE, TELEGRAM_CHAT_ID_PUBLIC,
};

async fn send_message_to_channel(bot: &Bot, message: &str, public: bool) {
    let _ = bot
        .send_message(
            ChatId(
                *if public {
                    &TELEGRAM_CHAT_ID_PUBLIC
                } else {
                    &TELEGRAM_CHAT_ID_PRIVATE
                }
                .get()
                .unwrap(),
            ),
            message,
        )
        .await
        .unwrap();
}
pub async fn run_telegram_bot(mut rx: tokio::sync::broadcast::Receiver<APIEvent>) {
    let bot = Bot::new(TELEGRAM_API_KEY.get().unwrap());
    info!("Started teloxide bot.");

    let mut last_event = None;

    while let Ok(event) = rx.recv().await {
        info!("Sending message to telegram chat.");
        match (last_event, event) {
            (None, APIEvent::Close) | (Some(APIEvent::Open), APIEvent::Close) => {
                send_message_to_channel(&bot, "Der fNordeingang ist jetzt geschlossen.", true).await;
            }
            (_, APIEvent::OpenIntern) => {
                send_message_to_channel(&bot, "Der fNordeingang ist jetzt für Member geöffnet.", false).await;
            }
            (_, APIEvent::Open) => {
                send_message_to_channel(&bot, "Der fNordeingang ist jetzt geöffnet.", true).await
            }
            (Some(APIEvent::OpenIntern), APIEvent::Close) => {
                send_message_to_channel(&bot, "Der fNordeingang ist jetzt auch für Member geschlossen.", false).await
            }
            _ => todo!("How did we get here?")
        };
        last_event = Some(event);
    }
}
