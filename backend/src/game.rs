use std::collections::HashMap;
use std::ops::Add;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use actix::*;
use actix_web::web::Data;
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use crate::Connection;
use crate::packets::{ClientPackets, GameState, PlayerDataMode, QuestionData, ServerPackets, StateChange};
use crate::packets::ServerPackets::PlayerData;
use crate::socket::GameData;
use crate::tools::{Identifier, random_identifier};

pub type AnswerIndex = u8;
pub type QuestionIndex = u8;

pub struct GameManager {
    pub games: Arc<RwLock<HashMap<Identifier, Game>>>,
}

const GAME_SLEEP_INTERVAL: Duration = Duration::from_secs(1);


impl GameManager {
    pub fn new() -> Data<Addr<GameManager>> {
        Data::new(GameManager {
            games: Arc::new(RwLock::new(HashMap::new()))
        }.start())
    }
}

impl Actor for GameManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(GAME_SLEEP_INTERVAL, |act, _ctx| {
            let games = act.games.read().unwrap();
            if games.len() > 0 {
                let stopped: Vec<&Identifier> = games.par_iter()
                    .filter(|(id, game)| {
                        game.state == GameState::Stopped
                    })
                    .map(|(id, _)| id)
                    .collect();
                if stopped.len() > 0 {
                    let mut games = act.games.write().unwrap();
                    for x in stopped {
                        games.remove(x)
                            .expect("failed to remove stopped game");
                    }
                }
            }
        });
    }
}

#[derive(Message)]
#[rtype(result = "ClientAction")]
pub enum ServerAction {
    Packet {
        packet: ClientPackets,
        ret: Addr<Connection>,
    },
    DoStateChange {
        state: StateChange,
        game_data: GameData,
    },
    None,
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum ClientAction {
    CreatedGame { id: Identifier, title: String },
    NameTakenResult(bool),
    Packet(ServerPackets),
    Error(&'static str),
    JoinedGame { id: Identifier, player_id: Identifier, title: String },
    StateChange(StateChange),
    Disconnect,
    Multiple(Vec<ClientAction>),
    None,
}


impl Handler<ServerAction> for GameManager {
    type Result = MessageResult<ServerAction>;

    fn handle(&mut self, msg: ServerAction, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(match msg {
            ServerAction::Packet { packet, ret } => match packet {
                ClientPackets::CreateGame { title, questions } => {
                    let mut id: Identifier;
                    let mut games = self.games.write().unwrap();
                    loop {
                        id = random_identifier(Game::ID_LENGTH);
                        if !games.contains_key(&id) { break; };
                    };
                    let game = Game {
                        host: ret,
                        id: id.clone(),
                        title: title.clone(),
                        questions,
                        players: Arc::new(RwLock::new(HashMap::new())),
                        state: GameState::Waiting,
                        last_time_sync: Instant::now(),
                    };
                    games.insert(id.clone(), game);
                    ClientAction::CreatedGame {
                        id,
                        title,
                    }
                }
                ClientPackets::CheckNameTaken { id, name } => {
                    let games = self.games.read().unwrap();
                    let game = games.get(&id);
                    match game {
                        None => ClientAction::Error("That game code doesn't exist"),
                        Some(game) => ClientAction::NameTakenResult(game.is_name_taken(&name))
                    }
                }
                ClientPackets::RequestGameState { id } => {
                    let games = self.games.read().unwrap();
                    let game = games.get(&id);
                    ClientAction::Packet(ServerPackets::GameState {
                        state: match game {
                            None => GameState::DoesNotExist,
                            Some(game) => game.state.clone()
                        }
                    })
                }
                ClientPackets::RequestJoin { id, name } => {
                    let mut games = self.games.write().unwrap();
                    let game = games.get_mut(&id);
                    match game {
                        None => ClientAction::Error("That game code doesn't exist"),
                        Some(game) => {
                            if game.is_name_taken(&name) {
                                ClientAction::Error("That name is already in use")
                            } else {
                                let player_id = game.new_player(name, ret);
                                ClientAction::JoinedGame { id, player_id, title: game.title.clone() }
                            }
                        }
                    }
                }
                ClientPackets::StateChange { state } => ClientAction::StateChange(state),
                _ => ClientAction::None
            }
            ServerAction::DoStateChange { state, game_data } => {
                match state {
                    StateChange::Start => {
                        if game_data.game_id.is_none() {
                            ClientAction::Error("You are not in a game.")
                        } else {
                            let mut games = self.games.write().unwrap();
                            let game = games.get_mut(&game_data.game_id.unwrap());
                            match game {
                                None => ClientAction::Multiple(vec![
                                    ClientAction::Error("You are not in a game."),
                                    ClientAction::Disconnect,
                                ]),
                                Some(game) => {
                                    game.state = GameState::Starting;
                                    game.broadcast(ServerPackets::GameState { state: GameState::Starting });
                                    ClientAction::None
                                }
                            }
                        }
                    }
                    StateChange::Skip => ClientAction::None,
                    StateChange::Disconnect => {
                        if game_data.game_id.is_some() {
                            let mut games = self.games.write().unwrap();
                            let game_id = game_data.game_id.unwrap();
                            let game = games.remove(&game_id);

                            if game.is_some() {
                                if game_data.hosting {
                                    let mut game = game.unwrap();
                                    info!("Shutting down game {} ({}) because host left", game.title, game.id);
                                    game.broadcast(ServerPackets::Disconnect { reason: String::from("Game ended.") });
                                } else if game_data.player_id.is_some() {
                                    game.unwrap().remove_player(game_data.player_id.unwrap())
                                }
                            }
                        }
                        ClientAction::Disconnect
                    }
                }
            }
            _ => ClientAction::None
        })
    }
}


#[derive(Debug)]
pub struct Game {
    pub host: Addr<Connection>,
    pub id: Identifier,
    pub title: String,
    pub questions: Vec<QuestionData>,
    pub players: Arc<RwLock<HashMap<Identifier, Player>>>,
    pub state: GameState,
    pub last_time_sync: Instant,
}


impl Game {
    const ID_LENGTH: usize = 5;

    fn is_name_taken(&self, name: &String) -> bool {
        let players = self.players.read().unwrap();
        players.values().any(|v| v.name.eq_ignore_ascii_case(name))
    }

    fn remove_player(&mut self, id: Identifier) {
        let mut players = self.players.write().unwrap();
        let player = players.remove(&id);
        match player {
            None => {}
            Some(player) => {
                players.values().for_each(|p| p.ret.do_send(ClientAction::Packet(player.as_data(PlayerDataMode::Remove))));
            }
        }
    }

    fn new_player(&mut self, name: String, ret: Addr<Connection>) -> Identifier {
        let mut players = self.players.write().unwrap();
        let mut id: Identifier;
        loop {
            id = random_identifier(Player::ID_LENGTH);
            if !players.contains_key(&id) { break; };
        };
        let player = Player {
            id: id.clone(),
            name: name.clone(),
            score: 0,
            answers: HashMap::new(),
            answer_time: None,
            ret: ret.clone(),
        };
        for v in players.values() {
            v.ret.do_send(ClientAction::Packet(player.as_data(PlayerDataMode::Add)));
            ret.do_send(ClientAction::Packet(v.as_data(PlayerDataMode::Add)));
        }
        ret.do_send(ClientAction::Packet(player.as_data(PlayerDataMode::Me)));
        self.host.do_send(ClientAction::Packet(player.as_data(PlayerDataMode::Add)));
        players.insert(id.clone(), player);
        id
    }

    fn broadcast(&mut self, packet: ServerPackets) {
        let mut players = self.players.read().unwrap();
        players.values().for_each(|p| p.ret.do_send(ClientAction::Packet(packet.clone())))
    }

    fn broadcast_excluding(&mut self, excluding: Identifier, packet: ServerPackets) {
        let mut players = self.players.read().unwrap();
        players.values().filter(|p| p.id != excluding).for_each(|p| p.ret.do_send(ClientAction::Packet(packet.clone())))
    }
}


#[derive(Debug)]
pub struct Player {
    pub id: Identifier,
    pub name: String,
    pub score: u32,
    pub answers: HashMap<QuestionIndex, AnswerIndex>,
    pub answer_time: Option<Instant>,
    pub ret: Addr<Connection>,
}

impl Player {
    const ID_LENGTH: usize = 5;

    pub fn as_data(&self, mode: PlayerDataMode) -> ServerPackets {
        ServerPackets::PlayerData {
            id: self.id.clone(),
            name: self.name.clone(),
            mode,
        }
    }
}