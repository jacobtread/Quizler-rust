extern crate core;

pub mod socket;
pub mod game;
pub mod packets;
mod tools;

use actix::{Actor, Addr};


use actix_web::{App, get, HttpRequest, HttpResponse, HttpServer, Responder, web};
use actix_web_actors::ws;
use crate::game::GameManager;
use crate::socket::Connection;

const APP_INDEX: &str = include_str!("../public/index.html");


#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(APP_INDEX)
}

#[get("/ws")]
async fn ws_route(req: HttpRequest, stream: web::Payload, manager: web::Data<Addr<GameManager>>) -> impl Responder {
    return match ws::start(Connection::new(manager.get_ref().clone()), &req, stream) {
        Ok(resp) => resp,
        Err(_) => HttpResponse::InternalServerError().body("Failed to connect")
    };
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let manager = GameManager::new().start();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(manager.clone()))
            .service(index)
            .service(ws_route)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}