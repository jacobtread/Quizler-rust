use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use actix::*;
use actix_web::web::Data;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use crate::Connection;
use crate::packets::{ClientPackets, GameState, QuestionData, ServerPackets};
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

pub enum NameTakenResult {
    GameNotFound,
    Taken,
    Free,
}

#[derive(Message)]
#[rtype(result = "ClientAction")]
pub enum ServerAction {
    Packet(ClientPackets),
    None,
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum ClientAction {
    CreatedGame { id: Identifier, title: String },
    NameTakenResult(NameTakenResult),
    Packet(ServerPackets),
    None,
}


impl Handler<ServerAction> for GameManager {
    type Result = MessageResult<ServerAction>;

    fn handle(&mut self, msg: ServerAction, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(match msg {
            ServerAction::Packet(packet) => match packet {
                ClientPackets::CreateGame { title, questions } => {
                    let mut id: Identifier;
                    let mut games = self.games.write().unwrap();
                    loop {
                        id = random_identifier(Game::ID_LENGTH);
                        if !games.contains_key(&id) { break; };
                    };
                    let game = Game {
                        id: id.clone(),
                        title: title.clone(),
                        questions,
                        players: Arc::new(RwLock::new(HashMap::new())),
                        state: GameState::Waiting,
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
                    ClientAction::NameTakenResult(match game {
                        None => NameTakenResult::GameNotFound,
                        Some(game) => if game.is_name_taken(name) {
                            NameTakenResult::Taken
                        } else {
                            NameTakenResult::Free
                        }
                    })
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
                ClientPackets::RequestJoin {id, name} => {
                    let games = self.games.read().unwrap();
                    let game = games.get(&id);
                     match game {
                        None => ClientAction::Packet(ServerPackets::Error {cause: String::from("That game code doesn't exist")}),
                        Some(game) => game.state.clone()
                    }
                }
                _ => ClientAction::None
            }
            _ => ClientAction::None
        })
    }
}


#[derive(Debug)]
pub struct Game {
    pub id: Identifier,
    pub title: String,
    pub questions: Vec<QuestionData>,
    pub players: Arc<RwLock<HashMap<Identifier, Player>>>,
    pub state: GameState,
}

impl Game {
    const ID_LENGTH: usize = 5;

    fn is_name_taken(&self, name: String) -> bool {
        let players = self.players.read().unwrap();
        players.values().any(|v| v.name.eq_ignore_ascii_case(name.as_str()))
    }

    fn new_player(&mut self, name: String, ret: Addr<Connection>) -> Identifier {
        let mut players = self.players.write().unwrap();
        let mut id: Identifier;
        loop {
            id = random_identifier(Player::ID_LENGTH);
            if !players.contains_key(&id) { break; };
        };
        players.insert(id.clone(), Player {
            id: id.clone(),
            name: name.clone(),
            score: 0,
            answers: HashMap::new(),
            answer_time: None,
            ret,
        });
        id
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
}