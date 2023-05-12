use std::sync::atomic::{AtomicBool, Ordering};

use actix_web::{
    get, http::StatusCode, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use hmac::{Hmac, Mac};
use log::{error, info};
use rand::RngCore;
use sha3::Sha3_512;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy)]
pub enum APIEvent {
    Open,
    Close,
}
struct State {
    tx: tokio::sync::broadcast::Sender<APIEvent>,
    open: AtomicBool,
    current_auth_token: Mutex<Option<Vec<u8>>>,
    shared_secret: Vec<u8>,
}
#[derive(Debug)]
enum AuthTokenError {
    Invalid,
    NonePresent,
}
type HmacSHA512 = Hmac<Sha3_512>;
impl State {
    /// Returns a challenge and generates the auth token.
    pub async fn generate_auth_token(&self) -> Vec<u8> {
        let mut challenge = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut challenge);
        let challenge = challenge.to_vec();

        let mut mac = HmacSHA512::new_from_slice(self.shared_secret.as_slice())
            .expect("Failed to initalize hmac");
        mac.update(challenge.as_slice());
        *self.current_auth_token.lock().await = Some(mac.finalize().into_bytes().to_vec());
        challenge
    }
    pub async fn validate_auth_token(
        &self,
        provided_auth_token: Vec<u8>,
    ) -> Result<(), AuthTokenError> {
        let mut current_auth_token = self.current_auth_token.lock().await;
        if let Some(auth_token) = current_auth_token.clone() {
            if auth_token == provided_auth_token {
                *current_auth_token = None;
                Ok(())
            } else {
                *current_auth_token = None;
                Err(AuthTokenError::Invalid)
            }
        } else {
            Err(AuthTokenError::NonePresent)
        }
    }
}
#[get("/auth_challenge")]
async fn auth_challenge(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    info!(
        "Received GET to /auth-challenge from {}.",
        req.peer_addr().unwrap()
    );

    let challenge = data.generate_auth_token().await;
    info!(
        "Generated new challenge {}.",
        hex::encode(challenge.clone())
    );

    HttpResponse::Ok()
        .append_header(("Auth-Challenge", hex::encode(challenge)))
        .finish()
}
#[get("/open")]
async fn open(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    info!("Received GET to /open from {}.", req.peer_addr().unwrap());
    if let Some(provided_auth_token) = req.headers().get("auth-challenge") {
        if let Err(e) = data
            .validate_auth_token(hex::decode(provided_auth_token).unwrap())
            .await
        {
            error!("Failed to validate auth token. Error {e:?}");
            return HttpResponse::Unauthorized();
        }
        if let Ok(false) =
            data.open
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
    } else {
        error!("No auth token was provided.");
        HttpResponse::Unauthorized()
    }
}
#[get("/close")]
async fn close(req: HttpRequest, data: web::Data<State>) -> impl Responder {
    info!("Received GET to /close from {}.", req.peer_addr().unwrap());
    if let Some(provided_auth_token) = req.headers().get("auth-challenge") {
        if let Err(e) = data
            .validate_auth_token(hex::decode(provided_auth_token).unwrap())
            .await
        {
            error!("Failed to validate auth token. Error {e:?}");
            return HttpResponse::Unauthorized();
        }
        if let Ok(true) =
            data.open
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
    } else {
        error!("No auth token was provided.");
        HttpResponse::Unauthorized()
    }
}
#[get("/")]
async fn index(req: HttpRequest) -> impl Responder {
    info!(
        "Received GET to /index.html from {}.",
        req.peer_addr().unwrap()
    );
    ("I'm an API not a webserver.", StatusCode::IM_A_TEAPOT)
}
pub async fn run(tx: tokio::sync::broadcast::Sender<APIEvent>) {
    let state = web::Data::new(State {
        tx,
        open: AtomicBool::new(false),
        current_auth_token: Mutex::new(None),
        shared_secret: hex::decode(env!("SHARED_SECRET")).unwrap(),
    });
    HttpServer::new(move || {
        let state = state.clone();
        App::new()
            .service(open)
            .service(close)
            .service(index)
            .service(auth_challenge)
            .app_data(state)
    })
    .bind("[::]:1337")
    .unwrap()
    .run()
    .await
    .unwrap();
}
