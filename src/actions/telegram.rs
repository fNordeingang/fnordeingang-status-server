use log::info;
use teloxide::{requests::Requester, types::ChatId, Bot};

use crate::server::APIEvent;

pub async fn run_telegram_bot(mut rx: tokio::sync::broadcast::Receiver<APIEvent>) {
    let bot = Bot::new(env!("TELEGRAM_API_TOKEN"));
    info!("Started teloxide bot.");

    while let Ok(event) = rx.recv().await {
        match event {
            APIEvent::Open => {
                info!("Sending message to telegram chat.");
                let _ = bot
                    .send_message(
                        ChatId(env!("TELEGRAM_CHAT_ID").parse().unwrap()),
                        "Der fNordeingang ist jetzt geöffnet.",
                    )
                    .await
                    .unwrap();
            }
            APIEvent::Close => {
                info!("Sending message to telegram chat.");
                let _ = bot
                    .send_message(
                        ChatId(env!("TELEGRAM_CHAT_ID").parse().unwrap()),
                        "Der fNordeingang ist jetzt geschlossen.",
                    )
                    .await
                    .unwrap();
            }
        };
    }
}
