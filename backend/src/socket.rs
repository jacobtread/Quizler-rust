use std::io::Cursor;
use actix::*;
use actix_web_actors::ws;
use wsbps::{Readable, Writable};
use crate::game::{ClientAction, GameManager, NameTakenResult, ServerAction};
use crate::packets::{ClientPackets, GameState, ServerPackets};
use crate::tools::Identifier;
use log::{error, info, warn};
use fut::{ready, Ready};

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

    fn packet<W: Writable>(&self, ctx: &mut CContext, mut packet: W) {
        let mut out = Vec::new();
        match packet.write(&mut out) {
            Ok(_) => ctx.binary(out),
            Err(err) => error!("Failed to write packet {:?}", err),
        };
    }

    fn handle_action(res: Result<ClientAction, MailboxError>, act: &mut Connection, ctx: &mut CContext) -> Ready<()> {
        match res {
            Ok(res) => match res {
                ClientAction::CreatedGame { id, title } => {
                    act.hosted_id = Some(id.clone());
                    act.packet(ctx, ServerPackets::JoinedGame {
                        id: id.clone(),
                        owner: true,
                        title: title.clone(),
                    });
                    act.packet(ctx, ServerPackets::GameState { state: GameState::Waiting });
                    info!("Created new game {} ({})", title, id)
                }
                ClientAction::NameTakenResult(result) => {
                    match result {
                        NameTakenResult::GameNotFound => {
                            act.packet(ctx, ServerPackets::Error { cause: String::from("Game doesn't exist") })
                        }
                        NameTakenResult::Taken => act.packet(ctx, ServerPackets::NameTakenResult { result: true }),
                        NameTakenResult::Free => act.packet(ctx, ServerPackets::NameTakenResult { result: false }),
                    }
                }
                ClientAction::None => {}
            }
            Err(_) => ctx.stop()
        }
        ready(())
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
                        self.manager.send(ServerAction::Packet(p))
                            .into_actor(self)
                            .then(Connection::handle_action)
                            .wait(ctx);
                    }
                    Err(_) => {}
                };
            }
            _ => (),
        };
    }
}