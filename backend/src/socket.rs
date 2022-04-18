use std::io::Cursor;
use std::net::IpAddr::V4;
use std::ops::Add;
use actix::{Handler, Actor, StreamHandler, AsyncContext, Addr, Running, Context, ArbiterHandle, WrapFuture, ActorFutureExt, ContextFutureSpawner, fut, MailboxError, ActorContext};
use actix::dev::Mailbox;
use actix_web_actors::ws;
use wsbps::{Readable, Writable};
use crate::game::{CreatedGame, CreateGame, Game, GameManager, Player};
use crate::packets::{ClientPackets, GameState, ServerPackets, SGameState, SJoinedGame};
use crate::tools::Identifier;
use log::{error, info, warn};

pub struct Connection {
    pub player_id: Option<Identifier>,
    pub game_id: Option<Identifier>,
    pub hosted_id: Option<Identifier>,
    pub manager: Addr<GameManager>,
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

    fn send_packet<W: Writable>(&self, ctx: &mut Self::Context, mut packet: W) {
        let mut out = Vec::new();
        packet.write(&mut out);
        ctx.binary(out)
    }
}

impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;
}


impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Connection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let addr = ctx.address();
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                warn!("Received text message \"{}\".. dont know what to do with it.", String::from(text))
            }
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
                                                self.hosted_id = Some(msg.id.clone());
                                                self.send_packet(ctx, SJoinedGame {
                                                    id: msg.id,
                                                    owner: true,
                                                    title: msg.title,
                                                });
                                                self.send_packet(ctx, SGameState { state: GameState::WAITING });
                                                info!("Created new game {} ({})", msg.title, msg.id)
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
                }
            }
            _ => (),
        }
    }
}