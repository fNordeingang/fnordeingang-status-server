#![feature(result_option_inspect)]

use crate::server::APIEvent;
use log::info;

mod actions;
mod server;

#[actix_web::main]
async fn main() {
    env_logger::init();
    info!("Starting fnord-status server.");

    let (tx, _rx) = tokio::sync::broadcast::channel::<APIEvent>(1);

    futures::join!(
        actions::telegram::run_telegram_bot(tx.subscribe()),
        server::run(tx)
    );
}
