use std::{
    io::SeekFrom,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
    time::{Duration, UNIX_EPOCH},
};

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    get, http::StatusCode, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use log::{error, info, warn};
use spaceapi::{Contact, Location, StatusBuilder};
use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt},
    sync::{Mutex, MutexGuard},
};

const STATE_CLOSED: usize = 0;
const STATE_OPEN_INTERN: usize = 1;
const STATE_OPEN: usize = 2;

use crate::{Config, API_KEY};

fn is_api_key_valid(req: &HttpRequest) -> bool {
    req.headers()
        .get("Api-Key")
        .map(|x| x.to_str().unwrap().to_string())
        == API_KEY.get().cloned()
}

#[derive(Debug, Clone, Copy)]
pub enum APIEvent {
    Open,
    OpenIntern,
    Close,
}
struct State {
    tx: tokio::sync::broadcast::Sender<APIEvent>,
    state: AtomicUsize,
    last_changed: AtomicU64,
    config: Mutex<Config>,
    config_file: Mutex<File>,
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
async fn write_config_file(data: web::Data<State>, config_lock_guard: MutexGuard<'_, Config>) {
    let mut config_file = data.config_file.lock().await;
    config_file.seek(SeekFrom::Start(0)).await.unwrap();
    config_file.set_len(0).await.unwrap();
    config_file
        .write(toml::to_string(&*config_lock_guard).unwrap().as_bytes())
        .await
        .unwrap();
}
#[get("/open")]
async fn open(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    req.peer_addr()
        .inspect(|peer_addr| info!("Received GET to /open from {peer_addr}."));
    if !is_api_key_valid(&req) {
        warn!("Api key missing or invalid.");
        return HttpResponse::Unauthorized();
    }
    if let Ok(STATE_CLOSED) | Ok(STATE_OPEN_INTERN) = data.state.compare_exchange(
        STATE_CLOSED,
        STATE_OPEN,
        Ordering::Acquire,
        Ordering::Relaxed,
    ) {
        data.reset_last_change();
        let mut config_lock_guard = data.config.lock().await;
        config_lock_guard.last_state_change = Some(data.last_changed.load(Ordering::Relaxed));
        config_lock_guard.last_state = Some(STATE_OPEN);

        write_config_file(data.clone(), config_lock_guard).await;
        data.state.store(STATE_OPEN, Ordering::Relaxed);

        match data.tx.send(APIEvent::Open) {
            Ok(_) => HttpResponse::Ok(),
            Err(_) => {
                error!("Failed to send message to other jobs.");
                HttpResponse::InternalServerError()
            }
        }
    } else {
        info!("State wasn't changed since state is already open.");
        HttpResponse::AlreadyReported()
    }
}
#[get("/open_intern")]
async fn open_intern(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    req.peer_addr()
        .inspect(|peer_addr| info!("Received GET to /open_intern from {peer_addr}."));
    if !is_api_key_valid(&req) {
        warn!("Api key missing or invalid.");
        return HttpResponse::Unauthorized();
    }
    if data.state.load(Ordering::Relaxed) == STATE_OPEN_INTERN {
        info!("State wasn't changed since state is already open internally.");
        return HttpResponse::AlreadyReported();
    }
    // Open has the state number two, so anything above that is invalid.
    if data.state.load(Ordering::Relaxed) > STATE_OPEN {
        error!("Last state is invalid. Forcing requested state.");
    }
    data.reset_last_change();
    let mut config_lock_guard = data.config.lock().await;
    config_lock_guard.last_state_change = Some(data.last_changed.load(Ordering::Relaxed));
    config_lock_guard.last_state = Some(STATE_OPEN_INTERN);

    write_config_file(data.clone(), config_lock_guard).await;
    data.state.store(STATE_OPEN_INTERN, Ordering::Relaxed);

    match data.tx.send(APIEvent::OpenIntern) {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => {
            error!("Failed to send message to other jobs.");
            HttpResponse::InternalServerError()
        }
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
    if data.state.load(Ordering::Relaxed) == STATE_CLOSED {
        info!("State wasn't changed since state is already closed.");
        return HttpResponse::AlreadyReported();
    }
    // Open has the state number two, so anything above that is invalid.
    if data.state.load(Ordering::Relaxed) > STATE_OPEN {
        error!("Last state is invalid. Forcing requested state.");
    }
    data.reset_last_change();
    let mut config_lock_guard = data.config.lock().await;
    config_lock_guard.last_state_change = Some(data.last_changed.load(Ordering::Relaxed));
    config_lock_guard.last_state = Some(STATE_CLOSED);

    write_config_file(data.clone(), config_lock_guard).await;
    data.state.store(STATE_CLOSED, Ordering::Relaxed);

    match data.tx.send(APIEvent::Close) {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => {
            error!("Failed to send message to other jobs.");
            HttpResponse::InternalServerError()
        }
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
            open: Some(data.state.load(Ordering::Relaxed) == STATE_OPEN),
            lastchange: Some(data.last_changed.load(Ordering::Relaxed)),
            ..Default::default()
        })
        .location(Location {
            address: Some("Körnerstr. 72, 41464 Neuss, Germany".to_string()),
            lat: 51.186234,
            lon: 6.692624,
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
        .add_extension("ccc", "chaostreff")
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
pub async fn run(tx: tokio::sync::broadcast::Sender<APIEvent>, config: Config, config_file: File) {
    let state = web::Data::new(State {
        tx,
        state: AtomicUsize::new(config.last_state.unwrap_or_default()),
        last_changed: AtomicU64::new(config.last_state_change.unwrap_or_default()),
        config: Mutex::new(config.clone()),
        config_file: Mutex::new(config_file),
    });
    let governor_config = GovernorConfigBuilder::default()
        .period(Duration::from_secs(
            config.rate_limiter_timeout.unwrap_or(300) as u64,
        ))
        .burst_size(config.rate_limiter_tokens.unwrap_or(3) as u32)
        .finish()
        .unwrap();

    HttpServer::new(move || {
        let state = state.clone();
        App::new()
            .wrap(Cors::default().allow_any_origin())
            .service(
                web::scope("/api")
                    .wrap(Governor::new(&governor_config))
                    .service(open)
                    .service(close)
                    .service(open_intern),
            )
            .service(index)
            .service(spaceapi_json)
            .app_data(state)
    })
    .bind("[::]:13337")
    .expect("Failed to bind to address.")
    .run()
    .await
    .expect("Failed to initialize api server.");
}
