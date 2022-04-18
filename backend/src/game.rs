use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use actix::{Actor, Addr, ArbiterHandle, AsyncContext, Context, Handler, MailboxError, Message, MessageResult, Running, WrapFuture};
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio::runtime::Handle;
use tokio::sync::MutexGuard;
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
    pub games: HashMap<Identifier, Arc<Mutex<Game>>>,
}

const GAME_SLEEP_INTERVAL: Duration = Duration::from_secs(1);

impl Handler<CreateGame> for GameManager {
    type Result = MessageResult<CreateGame>;

    fn handle(&mut self, msg: CreateGame, ctx: &mut Self::Context) -> Self::Result {
        let mut id: Identifier;
        loop {
            id = random_identifier(Game::ID_LENGTH);
            if !self.games.contains_key(&id) { break; };
        };
        let game = Arc::new(Mutex::new(Game {
            id: id.clone(),
            title: msg.title.clone(),
            questions: msg.questions,
            players: HashMap::new(),
            state: GameState::Waiting,
        }));
        self.games.insert(id.clone(), game.clone());
        MessageResult(CreatedGame {
            id,
            title: msg.title,
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

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(GAME_SLEEP_INTERVAL, |act, ctx| {
           act.games.par_iter()
               .for_each(|(id, game)| {
                   let game = game.lock().unwrap();
                   println!("running game loop for {} ({})", game.title, game.id);
               });
        });
    }
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