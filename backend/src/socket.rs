use std::io::Cursor;
use actix::{Actor, StreamHandler, Addr, WrapFuture, ActorFutureExt, ContextFutureSpawner, fut, MailboxError, ActorContext};
use actix_web_actors::ws;
use wsbps::{Readable, Writable};
use crate::game::{CreatedGame, CreateGame, GameManager};
use crate::packets::{CCreateGame, ClientPackets, GameState, SGameState, SJoinedGame};
use crate::tools::Identifier;
use log::{error, info, warn};

pub struct Connection {
    pub player_id: Option<Identifier>,
    pub game_id: Option<Identifier>,
    pub hosted_id: Option<Identifier>,
    pub manager: Addr<GameManager>,
}


impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;
}

type CContext = <Connection as Actor>::Context;

impl Connection {
    pub fn new(manager: Addr<GameManager>) -> Connection {
        Connection {
            player_id: None,
            game_id: None,
            hosted_id: None,
            manager,
        }
    }

    fn send_packet<W: Writable>(&self, ctx: &mut CContext, mut packet: W) {
        let mut out = Vec::new();
        match packet.write(&mut out) {
            Ok(_) => ctx.binary(out),
            Err(err) => error!("Failed to write packet {}", err),

        };
    }

    fn on_created_game(res: Result<CreatedGame, MailboxError>, act: &mut Connection, ctx: &mut CContext) -> actix::fut::Ready<()> {
        match res {
            Ok(res) => {
                act.hosted_id = Some(res.id.clone());
                act.send_packet(ctx, SJoinedGame {
                    id: res.id.clone(),
                    owner: true,
                    title: res.title.clone(),
                });
                act.send_packet(ctx, SGameState { state: GameState::Waiting });
                info!("Created new game {} ({})", res.title, res.id)
            }
            // something died
            _ => ctx.stop(),
        }
        fut::ready(())
    }

    fn on_create_game(&self, packet: &CCreateGame, ctx: &mut CContext) {
        self.manager.send(CreateGame {
            title: packet.title.clone(),
            questions: packet.questions.clone(),
        })
            .into_actor(self)
            .then(Connection::on_created_game)
            .wait(ctx);
    }
}


impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Connection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => { ctx.pong(&msg); }
            Ok(ws::Message::Text(text)) => { warn!("Received text message \"{}\".. dont know what to do with it.",text); }
            Ok(ws::Message::Binary(bin)) => {
                let mut cursor = Cursor::new(bin.to_vec());
                match ClientPackets::read(&mut cursor) {
                    Ok(p) => {
                        println!("{:?}", p);
                        match p {
                            ClientPackets::CCreateGame(c) => self.on_create_game(&c, ctx),
                            ClientPackets::CCheckNameTaken(_) => {}
                            ClientPackets::CRequestGameState(_) => {}
                            ClientPackets::CRequestJoin(_) => {}
                            ClientPackets::CStateChange(_) => {}
                            ClientPackets::CAnswer(_) => {}
                            ClientPackets::CKick(_) => {}
                        }
                    }
                    Err(_) => {}
                };
            }
            _ => (),
        };
    }
}