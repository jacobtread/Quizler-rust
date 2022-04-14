use std::collections::HashMap;
use std::sync::mpsc;
use actix::{Actor};
use actix::prelude::*;
use tokio::sync::broadcast;
use tokio::sync::mpsc::{Sender, Receiver, channel};
use crate::Connection;
use crate::player::Player;


#[derive(Debug, Clone, Copy)]
pub enum GameCommand {
    Create()
}

pub enum GameCommandResult {
    Create()
}

struct Game {
    players: HashMap<usize, Connection>
}

struct GameServer {

}

impl Actor for GameServer {
    type Context = Context<Self>;
}

impl Handler<GameCommand> for GameServer {
    type Result = GameCommandResult;

    fn handle(&mut self, msg: GameCommand, ctx: &mut Self::Context) -> Self::Result {
        GameCommandResult::Create()
    }
}

impl GameServer {

    fn new() -> GameServer {
        GameServer {
        }
    }

    fn init(&mut self) {

    }

}
