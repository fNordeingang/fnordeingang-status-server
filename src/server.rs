use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::UNIX_EPOCH,
};

use actix_cors::Cors;
use actix_web::{
    get, http::StatusCode, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use log::{error, info, warn};
use spaceapi::{Contact, Location, StatusBuilder};

const API_KEY: &'static str = env!("API_KEY");

fn is_api_key_valid(req: &HttpRequest) -> bool {
    match req.headers().get("Api-Key").map(|x| x.to_str().unwrap()) {
        Some(API_KEY) => true,
        Some(api_key) => {
            info!("Api key: {api_key} is invalid.");
            false
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum APIEvent {
    Open,
    Close,
}
struct State {
    tx: tokio::sync::broadcast::Sender<APIEvent>,
    open: AtomicBool,
    last_changed: AtomicU64,
}
impl State {
    pub fn reset_last_change(&self) {
        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_changed.store(time, Ordering::Relaxed);
    }
}
#[get("/open")]
async fn open(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    req.peer_addr()
        .inspect(|peer_addr| info!("Received GET to /open from {peer_addr}."));
    if !is_api_key_valid(&req) {
        warn!("Api key missing or invalid.");
        return HttpResponse::Unauthorized();
    }
    if let Ok(false) = data
        .open
        .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
    {
        data.reset_last_change();
        return match data.tx.send(APIEvent::Open) {
            Ok(_) => HttpResponse::Ok(),
            Err(_) => {
                error!("Failed to send message to other jobs.");
                HttpResponse::InternalServerError()
            }
        };
    } else {
        info!("State wasn't changed since state is already open.");
        HttpResponse::AlreadyReported()
    }
}
#[get("/close")]
async fn close(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    req.peer_addr()
        .inspect(|peer_addr| info!("Received GET to /close from {peer_addr}."));
    if !is_api_key_valid(&req) {
        warn!("Api key missing or invalid.");
        return HttpResponse::Unauthorized();
    }
    if let Ok(true) = data
        .open
        .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
    {
        data.reset_last_change();
        return match data.tx.send(APIEvent::Close) {
            Ok(_) => HttpResponse::Ok(),
            Err(_) => {
                error!("Failed to send message to other jobs.");
                HttpResponse::InternalServerError()
            }
        };
    } else {
        info!("State wasn't changed since state is already closed.");
        HttpResponse::AlreadyReported()
    }
}
#[get("/spaceapi.json")]
async fn spaceapi_json(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    req.peer_addr()
        .inspect(|peer_addr| info!("Received GET to /spaceapi.json from {peer_addr}."));
    let status = StatusBuilder::mixed("fNordeingang")
        .logo("https://fnordeingang.de/wp-content/uploads/2013/06/logo_final21.png")
        .url("https://fnordeingang.de")
        .state(spaceapi::State {
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
#[get("/")]
async fn index(req: HttpRequest) -> impl Responder {
    req.peer_addr()
        .inspect(|peer_addr| info!("Received GET to /open from {peer_addr}."));
    ("I'm an API not a webserver.", StatusCode::IM_A_TEAPOT)
}
pub async fn run(tx: tokio::sync::broadcast::Sender<APIEvent>) {
    let state = web::Data::new(State {
        tx,
        open: AtomicBool::new(false),
        last_changed: AtomicU64::new(0),
    });
    HttpServer::new(move || {
        let state = state.clone();
        App::new()
            .wrap(Cors::default().allow_any_origin())
            .service(open)
            .service(close)
            .service(index)
            .service(spaceapi_json)
            .app_data(state)
    })
    .bind("[::]:1337")
    .expect("Failed to bind to address.")
    .run()
    .await
    .expect("Failed to initialize api server.");
}
