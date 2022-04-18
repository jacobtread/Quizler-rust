use std::io::Cursor;
use actix::{Actor, StreamHandler, AsyncContext, Addr, WrapFuture, ActorFutureExt, ContextFutureSpawner, fut, MailboxError, ActorContext, MessageResult};
use actix_web_actors::ws;
use wsbps::{Readable, Writable};
use crate::game::{CreatedGame, CreateGame, GameManager};
use crate::packets::{ClientPackets, GameState, SGameState, SJoinedGame};
use crate::tools::Identifier;
use log::{info, warn};

pub struct Connection {
    pub player_id: Option<Identifier>,
    pub game_id: Option<Identifier>,
    pub hosted_id: Option<Identifier>,
    pub manager: Addr<GameManager>,
}


impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;
}

impl Connection {
    pub fn new(manager: Addr<GameManager>) -> Connection {
        Connection {
            player_id: None,
            game_id: None,
            hosted_id: None,
            manager,
        }
    }

    fn send_packet<W: Writable>(&self, ctx: &mut ws::WebsocketContext<Connection>, mut packet: W) {
        let mut out = Vec::new();
        packet.write(&mut out);
        ctx.binary(out)
    }
}


impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Connection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let addr = ctx.address();
        match msg {
            Ok(ws::Message::Ping(msg)) => { ctx.pong(&msg); }
            Ok(ws::Message::Text(text)) => { warn!("Received text message \"{}\".. dont know what to do with it.",text); }
            Ok(ws::Message::Binary(bin)) => {
                let mut cursor = Cursor::new(bin.to_vec());
                match ClientPackets::read(&mut cursor) {
                    Ok(p) => {
                        println!("{:?}", p);
                        match p {
                            ClientPackets::CCreateGame(c) => {
                                self.manager.send(CreateGame {
                                    title: c.title,
                                    questions: c.questions,
                                })
                                    .into_actor(self)
                                    .then(|res: Result<CreatedGame, MailboxError>, act, ctx| {
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
                                    })
                                    .wait(ctx);
                            }
                            ClientPackets::CCheckNameTaken(_) => {}
                            ClientPackets::CRequestGameState(_) => {}
                            ClientPackets::CRequestJoin(x) => {}
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