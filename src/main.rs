use std::{path::PathBuf, sync::OnceLock};

use crate::server::APIEvent;
use clap::Parser;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::{fs::OpenOptions, io::AsyncReadExt};

mod actions;
mod server;

pub(crate) static API_KEY: OnceLock<String> = OnceLock::new();
pub(crate) static TELEGRAM_API_KEY: OnceLock<String> = OnceLock::new();
pub(crate) static TELEGRAM_CHAT_ID: OnceLock<String> = OnceLock::new();

#[derive(Parser)]
#[command(author = "Frostie314159", version)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: PathBuf,
}

#[derive(Deserialize, Serialize, Clone)]
struct Config {
    api_key: String,
    telegram_api_key: String,
    telegram_chat_id: String,
    rate_limiter_timeout: Option<usize>,
    rate_limiter_tokens: Option<usize>,
    last_state_open: Option<bool>,
    last_state_change: Option<u64>,
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

    API_KEY.set(config.api_key.clone()).unwrap();
    TELEGRAM_API_KEY
        .set(config.telegram_api_key.clone())
        .unwrap();
    TELEGRAM_CHAT_ID
        .set(config.telegram_chat_id.clone())
        .unwrap();

    let (tx, _rx) = tokio::sync::broadcast::channel::<APIEvent>(1);

    futures::join!(
        actions::telegram::run_telegram_bot(tx.subscribe()),
        server::run(tx, config, file)
    );
}
