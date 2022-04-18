use std::collections::HashMap;
use std::thread::sleep;
use std::time::Duration;
use actix::{Actor, Context, Handler, Message, MessageResult};
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

impl Handler<CreateGame> for GameManager {
    type Result = MessageResult<CreateGame>;

    fn handle(&mut self, msg: CreateGame, _: &mut Self::Context) -> Self::Result {
        let id = self.new_game(msg.title, msg.questions);
        let game = self.games.get(&id)
            .expect("expected game to be created");
        MessageResult(CreatedGame {
            id,
            title: game.title.clone(),
        })
    }
}

impl GameManager {
    pub fn new() -> GameManager {
        GameManager { games: HashMap::new() }
    }

    fn new_game(&mut self, title: String, questions: Vec<QuestionData>) -> Identifier {
        let mut id: Identifier;
        loop {
            id = random_identifier(Game::ID_LENGTH);
            if !self.games.contains_key(&id) { break; };
        };
        let game = Game {
            id: id.clone(),
            title,
            questions,
            players: HashMap::new(),
            state: GameState::Waiting,
        };
        self.games.insert(id.clone(), game);
        id
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

impl Actor for Game {
    type Context = Context<Self>;
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