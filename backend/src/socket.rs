use std::io::Cursor;
use std::sync::Mutex;
use actix::{Addr, ArbiterHandle, AsyncContext, Context, Running};
use actix_web_actors::ws;
use packets::ClientPackets;
use actix::{Actor, StreamHandler};
use wsbps::io::VarInt;
use wsbps::Readable;
use crate::packets::ClientPackets;
use crate::Player;

pub struct Connection {
    player: Option<Player>,
}

impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.address()
            .send()
    }
}


impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Connection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        println!("HERE");
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