use std::collections::HashMap;
use std::io::{Cursor, Read};

use actix::{Actor, StreamHandler};
use actix_web::{App, Error, get, HttpRequest, HttpResponse, HttpServer, Responder, web};
use actix_web_actors::ws;
use wsbps::{packet_data, packets, Readable};
use wsbps::io::VarInt;

const APP_INDEX: &str = include_str!("../public/index.html");

/// Define HTTP actor
struct MyWs;

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;
}

packet_data! {

    enum GameState (->) (u8) {
        WAITING: 0,
        STARTING: 1,
        STARTED: 2,
        STOPPED: 3,
        DOES_NOT_EXIST: 4
    }

    enum PlayerDataMode (->) (u8) {
        ADD: 0,
        REMOVE: 1,
        SELF: 2
    }

    enum StateChange (<-) (u8) {
        DISCONNECT: 0,
        START: 1,
        SKIP: 2
    }

    struct CreateQuestionData (<-) {
        image_type: String,
        image: Vec<u8>,
        question: String,
        answers: Vec<String>,
        values: Vec<u8>
    }
}

type ScoresMap = HashMap<String, u32>;

packets! {
    ServerPackets (->) {
        SDisconnect (0x00) { reason: String }
        SError (0x01) { cause: String }
        SJoinedGame (0x02) { id: String, owner: bool, title: String}
        SNameTakenResult (0x03) { result: bool }
        SGameState (0x04) { state: GameState }
        SPlayerData (0x05) { id: String, name: String, mode: PlayerDataMode }
        STimeSync (0x06) { total: VarInt, remaining: VarInt}
        SQuestion (0x07) { image: Vec<u8>, question: String, answers: Vec<String> }
        SAnswerResult (0x08) { result: bool }
        SScores (0x09) { scores: ScoresMap }
    }

    ClientPackets (<-) {
        CCreateGame (0x00) { title: String, questions: Vec<CreateQuestionData> }
        CCheckNameTaken (0x01) { id: String, name: String}
        CRequestGameState (0x02) { id: String }
        CRequestJoin (0x03) { id: String, name: String }
        CStateChange (0x04) { state: StateChange }
        CAnswer (0x05) { id: u8 }
        CKick (0x06) { id: String }
    }
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