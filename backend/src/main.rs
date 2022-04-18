pub  mod socket;
pub mod player;
pub mod game;
pub mod packets;
mod tools;

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::Arc;


use actix_web::{App, Error, get, HttpRequest, HttpResponse, HttpServer, Responder, web};
use actix_web_actors::ws;
use crate::player::Player;
use crate::socket::Connection;

const APP_INDEX: &str = include_str!("../public/index.html");

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(APP_INDEX)
}

#[get("/ws")]
async fn ws_route(req: HttpRequest, stream: web::Payload) -> impl Responder {
    return match ws::start(Connection {}, &req, stream) {
        Ok(resp) => resp,
        Err(_) => HttpResponse::InternalServerError().body("Failed to connect")
    };
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(ws_route)
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}