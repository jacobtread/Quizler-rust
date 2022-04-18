use std::io::Cursor;
use actix::{Handler, Actor, StreamHandler, AsyncContext};
use actix_web_actors::ws;
use wsbps::{Readable, Writable};
use crate::packets::{ClientPackets, ServerPackets};

pub struct Connection {
    pub player: Option<Player>,
    pub manager: Addr<GameManager>,
}

impl Connection {}

impl Handler<ServerPackets> for Connection {
    type Result = ();

    fn handle(&mut self, mut msg: ServerPackets, ctx: &mut Self::Context) -> Self::Result {
        let mut out = Vec::new();
        msg.write(&mut out);
    }
}

impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = ctx.address();
    }
}


impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Connection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => {
                let mut cursor = Cursor::new(bin.to_vec());
                match ClientPackets::read(&mut cursor) {
                    Ok(p) => {
                        println!("{:?}", p);
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