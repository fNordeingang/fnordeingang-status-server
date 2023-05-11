use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::UNIX_EPOCH,
};

use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use log::{error, info};
use spaceapi::{Contact, Location, State, StatusBuilder};

use crate::server::APIEvent;

struct SpaceAPIState {
    open: AtomicBool,
    last_changed: AtomicU64,
}

async fn update_spaceapi_state(
    mut rx: tokio::sync::broadcast::Receiver<APIEvent>,
    state: web::Data<SpaceAPIState>,
) {
    info!("Starting spaceapi updater.");
    while let Ok(event) = rx.recv().await {
        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        match event {
            APIEvent::Open => {
                state.open.store(true, Ordering::Relaxed);
                state.last_changed.store(time, Ordering::Relaxed);
                info!("Changed spaceapi state to open.");
            }
            APIEvent::Close => {
                state.open.store(false, Ordering::Relaxed);
                state.last_changed.store(time, Ordering::Relaxed);
                info!("Changed spaceapi state to closed.");
            }
        }
    }
}
#[get("/spaceapi.json")]
async fn spaceapi_json(req: HttpRequest, data: web::Data<SpaceAPIState>) -> impl Responder {
    info!(
        "spaceapi.json was requested by {}",
        req.peer_addr().unwrap()
    );
    let status = StatusBuilder::mixed("fNordeingang")
        .logo("https://fnordeingang.de/wp-content/uploads/2013/06/logo_final21.png")
        .url("https://fnordeingang.de")
        .state(State {
            open: Some(data.open.load(Ordering::Relaxed)),
            lastchange: Some(data.last_changed.load(Ordering::Relaxed)),
            ..Default::default()
        })
        .location(Location {
            address: Some("Körnerstr. 72, 41464 Neuss, Germany".to_string()),
            lat: 6.692624,
            lon: 51.186234,
            ..Default::default()
        })
        .contact(Contact {
            email: Some("verein@fnordeingang.de".to_string()),
            mastodon: Some("@fnordeingang@telefant.net".to_string()),
            issue_mail: Some("vorstand@fnordeingang.de".to_string()),
            ..Default::default()
        })
        .add_project("http://github.com/fnordeingang")
        .add_issue_report_channel(spaceapi::IssueReportChannel::IssueMail)
        .build();
    match status {
        Ok(status) => {
            return HttpResponse::Ok()
                .content_type("application/json")
                .body(serde_json::to_string(&status).unwrap());
        }
        Err(e) => {
            error!("Failed to serialize spaceapi status. Error: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
pub async fn run_spaceapi_server(rx: tokio::sync::broadcast::Receiver<APIEvent>) {
    let state = web::Data::new(SpaceAPIState {
        open: AtomicBool::new(false),
        last_changed: AtomicU64::new(0),
    });
    let update_state_future = update_spaceapi_state(rx, state.clone());
    let spaceapi_server_future = HttpServer::new(move || {
        let state = state.clone();
        App::new().app_data(state).service(spaceapi_json)
    })
    .bind("[::]:8080")
    .unwrap()
    .run();
    let _ = tokio::join!(update_state_future, spaceapi_server_future);
}
