use std::path::PathBuf;

use crate::server::APIEvent;
use clap::Parser;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::{fs::OpenOptions, io::AsyncReadExt};

mod actions;
mod server;

#[derive(Parser)]
#[command(author = "Frostie314159", version)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

#[derive(Deserialize, Serialize, Clone)]
struct Config {
    // Telegram API
    telegram_api_key: String,
    telegram_chat_id_public: i64,
    telegram_chat_id_private: i64,

    // Mastodon API
    mastodon_instance: String,
    mastodon_access_token: String,

    // State
    last_state: Option<usize>,
    last_state_change: Option<u64>,

    // Messages
    general_close: String,
    general_open: String,
    member_close: String,
    member_open: String,

    // Server
    api_key: String,
    api_port: Option<u16>,
    api_address: Option<String>,
    rate_limiter_timeout: Option<usize>,
    rate_limiter_tokens: Option<usize>,

    // Spaceapi
    space_name: String,
    logo: String,
    url: String,
    address: String,
    latitude: f64,
    longitude: f64,
    email: String,
    mastodon: String,
    issue_mail: String,
}

#[actix_web::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();

    info!("Starting fnord-status server.");

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .append(false)
        .open(cli.config)
        .await
        .unwrap();

    let mut buf = String::new();
    file.read_to_string(&mut buf).await.unwrap();

    let config = toml::from_str::<Config>(&buf).expect("Failed to parse config file.");

    let (tx, _rx) = tokio::sync::broadcast::channel::<APIEvent>(1);

    futures::join!(
        actions::telegram::run_telegram_bot(tx.subscribe(), config.clone()),
        actions::mastodon::run_mastodon_bot(tx.subscribe(), config.clone()),
        server::run(tx, config, file)
    );
}
