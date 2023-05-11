use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use log::{error, info};

#[derive(Debug, Clone, Copy)]
pub enum APIEvent {
    Open,
    Close,
}

#[get("/open")]
async fn open(
    req: HttpRequest,
    data: web::Data<tokio::sync::broadcast::Sender<APIEvent>>,
) -> impl Responder {
    info!("Received GET to /open from {}.", req.peer_addr().unwrap());
    match data.send(APIEvent::Open) {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => {
            error!("Failed to send message to other jobs.");
            HttpResponse::InternalServerError()
        }
    }
}
#[get("/close")]
async fn close(
    req: HttpRequest,
    data: web::Data<tokio::sync::broadcast::Sender<APIEvent>>,
) -> impl Responder {
    info!("Received GET to /close from {}.", req.peer_addr().unwrap());
    match data.send(APIEvent::Close) {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => {
            error!("Failed to send message to other jobs.");
            HttpResponse::InternalServerError()
        }
    }
}
pub async fn run(tx: tokio::sync::broadcast::Sender<APIEvent>) {
    HttpServer::new(move || {
        let tx = tx.clone();
        App::new()
            .service(open)
            .service(close)
            .app_data(web::Data::new(tx))
    })
    .bind("[::]:1337")
    .unwrap()
    .run()
    .await
    .unwrap();
}
