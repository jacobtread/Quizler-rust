use std::io::Cursor;
use actix::*;
use actix_web_actors::ws;
use wsbps::{Readable, Writable};
use crate::game::{ClientAction, GameManager, ServerAction};
use crate::packets::{ClientPackets, GameState, ServerPackets, StateChange};
use crate::tools::Identifier;
use log::{error, info, warn, debug};
use fut::{ready, Ready};

pub struct Connection {
    pub game_data: GameData,
    pub manager: Addr<GameManager>,

}

#[derive(Debug, Clone)]
pub struct GameData {
    pub hosting: bool,
    pub player_id: Option<Identifier>,
    pub game_id: Option<Identifier>,
}

impl GameData {
    fn reset(&mut self) {
        self.hosting = false;
        self.player_id = None;
        self.game_id = None;
    }
}


impl Actor for Connection {
    type Context = ws::WebsocketContext<Self>;
}

type CContext = <Connection as Actor>::Context;

impl Connection {
    pub fn new(manager: Addr<GameManager>) -> Connection {
        Connection {
            game_data: GameData {
                player_id: None,
                game_id: None,
                hosting: false,
            },
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
                    act.game_data.hosting= true;
                    act.game_data.game_id = Some(id.clone());
                    act.packet(ctx, ServerPackets::JoinedGame {
                        owner: true,
                        id: id.clone(),
                        title: title.clone(),
                    });
                    act.packet(ctx, ServerPackets::GameState { state: GameState::Waiting });
                    info!("Created new game {} ({})", title, id)
                }
                ClientAction::NameTakenResult(result) => act.packet(ctx, ServerPackets::NameTakenResult { result }),
                ClientAction::Packet(packet) => {
                    debug!("-> {:?}", packet);
                    act.packet(ctx, packet);
                }
                ClientAction::Error(msg) => act.packet(ctx, ServerPackets::Error { cause: String::from(msg) }),
                ClientAction::JoinedGame { id, player_id, title } => {
                    act.game_data.player_id = Some(player_id);
                    act.game_data.game_id = Some(id.clone());
                    act.packet(ctx, ServerPackets::JoinedGame {
                        owner: false,
                        id,
                        title,
                    })
                }
                ClientAction::StateChange(state) => {
                    act.manager.send(ServerAction::DoStateChange {
                        state,
                        game_data: act.game_data.clone(),
                    })
                        .into_actor(act)
                        .then(Connection::handle_action)
                        .wait(ctx);
                }
                ClientAction::Disconnect => {
                    act.game_data.reset();
                }
                ClientAction::Multiple(actions) => {
                    for action in actions {
                        Connection::handle_action(Ok(action), act, ctx);
                    }
                }
                ClientAction::None => {}
                ClientAction::BeginKick(id) => {
                    act.manager.send(ServerAction::TryKick {
                        game_data: act.game_data.clone(),
                        id
                    })
                        .into_actor(act)
                        .then(Connection::handle_action)
                        .wait(ctx);
                }
            }
            Err(err) => {
                error!("err {:?}", err);
                ctx.stop()
            }
        }
        ready(())
    }
}

impl Handler<ClientAction> for Connection {
    type Result = ();

    fn handle(&mut self, msg: ClientAction, ctx: &mut Self::Context) -> Self::Result {
        Connection::handle_action(Ok(msg), self, ctx);
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
                    Ok(packet) => {
                        debug!("<- {:?}", packet);
                        let ret = ctx.address().clone();
                        self.manager.send(ServerAction::Packet {
                            packet,
                            ret,
                        })
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