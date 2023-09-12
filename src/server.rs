use std::sync::atomic::{AtomicBool, Ordering};

use actix_web::{
    get, http::StatusCode, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use log::{error, info, warn};

const API_KEY: &'static str = env!("API_KEY");

fn is_api_key_valid(req: &HttpRequest) -> bool {
    match req
        .headers()
        .get("Api-Key")
        .map(|x| x.to_str().unwrap())
    {
        Some(API_KEY) => true,
        Some(api_key) => {
            info!("{api_key}");
            false
        },
        _ => false
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
        .inspect(|peer_addr| info!("Received GET to /open from {peer_addr}."));
    if !is_api_key_valid(&req) {
        warn!("Api key missing or invalid.");
        return HttpResponse::Unauthorized();
    }
    if let Ok(true) = data
        .open
        .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
    {
        return match data.tx.send(APIEvent::Close) {
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
    });
    HttpServer::new(move || {
        let state = state.clone();
        App::new()
            .service(open)
            .service(close)
            .service(index)
            .app_data(state)
    })
    .bind("[::]:1337")
    .expect("Failed to bind to address.")
    .run()
    .await
    .expect("Failed to initialize api server.");
}
