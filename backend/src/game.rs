use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::thread::sleep;
use std::time::Duration;
use actix::{Actor, Addr, ArbiterHandle, AsyncContext, Context, Handler, MailboxError, Message, MessageResult, Running, WrapFuture};
use log::info;
use tokio::time::Instant;
use crate::packets::{GameState, QuestionData};
use crate::tools::{Identifier, random_identifier};

pub type AnswerIndex = u8;
pub type QuestionIndex = u8;

#[derive(Message)]
#[rtype(result = "()")]
pub struct CreatedGame {
    pub id: Identifier,
    pub title: String,
}

#[derive(Message)]
#[rtype(result = "CreatedGame")]
pub struct CreateGame {
    pub title: String,
    pub questions: Vec<QuestionData>,
}

pub struct GameManager {
    pub games: HashMap<Identifier, Game>,
}

#[derive(Message)]
#[rtype(result = "GameResponse")]
enum GameRequest {
    Details(Identifier)
}

enum GameResponseError {
    UnknownGame
}

impl Display for GameResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            GameResponseError::UnknownGame => "Unknown Game"
        })
    }
}

#[derive(Message)]
#[rtype(result = "()")]
enum GameResponse {
    Details {
        name: String,
        state: GameState,
    },
    Error(GameResponseError),
}

impl Handler<CreateGame> for GameManager {
    type Result = MessageResult<CreateGame>;

    fn handle(&mut self, msg: CreateGame, ctx: &mut Self::Context) -> Self::Result {
        let mut id: Identifier;
        loop {
            id = random_identifier(Game::ID_LENGTH);
            if !self.games.contains_key(&id) { break; };
        };
        let game = Game {
            id: id.clone(),
            title: msg.title.clone(),
            questions: msg.questions,
            players: HashMap::new(),
            state: GameState::Waiting,
        };
        self.games.insert(id.clone(), game);
        let tmp_id = id.clone();
        let addr = ctx.address().clone();
        tokio::spawn(async move {
            let id = tmp_id;
            println!("pre loop");
            'game: loop {
                let result: Result<GameResponse, MailboxError> = addr.send(GameRequest::Details(id.clone())).await;
                match result {
                    Ok(value) => {
                        match value {
                            GameResponse::Details { name, state } => {
                                println!("Game {} ({}) is {:?}", name, id, state);
                                sleep(Duration::from_secs(1));
                            }
                            GameResponse::Error(err) => {
                                info!("Error when retrieving game {}", err);
                                break 'game;
                            }
                        }
                    }
                    Err(_) => {
                        break 'game;
                    }
                }
            }
        });
        println!("End spawn");
        MessageResult(CreatedGame {
            id,
            title: msg.title,
        })
    }
}

impl Handler<GameRequest> for GameManager {
    type Result = MessageResult<GameRequest>;

    fn handle(&mut self, msg: GameRequest, _: &mut Self::Context) -> Self::Result {
        MessageResult(match msg {
            GameRequest::Details(id) => {
                let game = self.games.get(&id);
                match game {
                    None => GameResponse::Error(GameResponseError::UnknownGame),
                    Some(value) => {
                        GameResponse::Details {
                            state: value.state.clone(),
                            name: value.title.clone(),
                        }
                    }
                }
            }
        })
    }
}

impl GameManager {
    pub fn new() -> GameManager {
        GameManager { games: HashMap::new() }
    }
}


impl Actor for GameManager {
    type Context = Context<Self>;
}

#[derive(Debug)]
pub struct Game {
    pub id: Identifier,
    pub title: String,
    pub questions: Vec<QuestionData>,
    pub players: HashMap<Identifier, Player>,
    pub state: GameState,
}


impl Game {
    const ID_LENGTH: usize = 5;

    fn new_player(&mut self, name: String) -> Identifier {
        let mut id: Identifier;
        loop {
            id = random_identifier(Player::ID_LENGTH);
            if !self.players.contains_key(&id) { break; };
        };
        self.players.insert(id.clone(), Player {
            id: id.clone(),
            name: name.clone(),
            score: 0,
            answers: HashMap::new(),
            answer_time: None,
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
}

impl Player {
    const ID_LENGTH: usize = 5;
}