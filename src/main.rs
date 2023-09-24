#![feature(result_option_inspect)]

use std::{path::PathBuf, sync::OnceLock};

use crate::server::APIEvent;
use clap::Parser;
use log::info;
use serde::Deserialize;

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

#[derive(Deserialize)]
struct Config {
    api_key: String,
    telegram_api_key: String,
    telegram_chat_id: String,
    rate_limiter_timeout: Option<usize>,
    rate_limiter_tokens: Option<usize>,
}

#[actix_web::main]
async fn main() {
    env_logger::init();
    let cli = Cli::parse();

    info!("Starting fnord-status server.");

    let config = toml::from_str::<Config>(
        &tokio::fs::read_to_string(cli.config)
            .await
            .expect("Failed to read config file."),
    )
    .expect("Failed to parse config file.");

    API_KEY.set(config.api_key.clone()).unwrap();
    TELEGRAM_API_KEY.set(config.telegram_api_key.clone()).unwrap();
    TELEGRAM_CHAT_ID.set(config.telegram_chat_id.clone()).unwrap();

    let (tx, _rx) = tokio::sync::broadcast::channel::<APIEvent>(1);

    futures::join!(
        actions::telegram::run_telegram_bot(tx.subscribe()),
        server::run(tx, config)
    );
}
