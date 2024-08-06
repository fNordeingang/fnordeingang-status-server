use log::info;
use megalodon::{entities::StatusVisibility, mastodon::Mastodon, megalodon::PostStatusInputOptions, Megalodon};

use crate::{server::APIEvent, Config};
async fn post_status(client: &Mastodon, status: String) {
    let _ = client.post_status(status, Some(&PostStatusInputOptions {
        visibility: Some(StatusVisibility::Public),
        ..Default::default()
    })).await;
    info!("Posting status to mastodon.");
}
pub async fn run_mastodon_bot(mut rx: tokio::sync::broadcast::Receiver<APIEvent>, config: Config) {
    let client = Mastodon::new(config.mastodon_instance, Some(config.mastodon_access_token), None);
    let mut last_event = None;
    while let Ok(event) = rx.recv().await {
        match (last_event, event) {
            (None, APIEvent::Close) | (Some(APIEvent::Open), APIEvent::Close) => {
                post_status(&client, config.general_close.clone()).await;
            },
            (_, APIEvent::Open) => {
                post_status(&client, config.general_open.clone()).await;
            },
            _ => {}
        }
        last_event = Some(event);
    }
}
