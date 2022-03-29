mod socket;
mod player;
mod game;
mod packets;

use std::collections::HashMap;
use std::io::{Cursor, Read};

use packets::ClientPackets;

use actix::{Actor, StreamHandler};
use actix_web::{App, Error, get, HttpRequest, HttpResponse, HttpServer, Responder, web};
use actix_web_actors::ws;
use wsbps::io::VarInt;
use wsbps::Readable;

const APP_INDEX: &str = include_str!("../public/index.html");

/// Define HTTP actor
struct MyWs;

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;
}


/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => {
                let mut cursor = Cursor::new(bin.to_vec());
                match ClientPackets::read(&mut cursor) {
                    Ok(p) => {
                        println!("{:?}",p);
                        match p {
                            ClientPackets::CCreateGame(_) => {}
                            ClientPackets::CCheckNameTaken(_) => {}
                            ClientPackets::CRequestGameState(_) => {}
                            ClientPackets::CRequestJoin(x) => {}
                            ClientPackets::CStateChange(_) => {}
                            ClientPackets::CAnswer(_) => {}
                            ClientPackets::CKick(_) => {}
                        }
                    }
                    Err(_) => {}
                }
                // ctx.binary(bin)
            }
            _ => (),
        }
    }
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body(APP_INDEX)
}

#[get("/ws")]
async fn ws_route(req: HttpRequest, stream: web::Payload) -> impl Responder {
    return match ws::start(MyWs {}, &req, stream) {
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