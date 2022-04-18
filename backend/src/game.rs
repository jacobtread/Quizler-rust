use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use actix::*;
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
    pub games: Arc<RwLock<HashMap<Identifier, Game>>>,
}

const GAME_SLEEP_INTERVAL: Duration = Duration::from_secs(1);

impl Handler<CreateGame> for GameManager {
    type Result = MessageResult<CreateGame>;

    fn handle(&mut self, msg: CreateGame, ctx: &mut Self::Context) -> Self::Result {
        let mut id: Identifier;
        let mut games = self.games.write().unwrap();
        loop {
            id = random_identifier(Game::ID_LENGTH);
            if !games.contains_key(&id) { break; };
        };
        let game = Game {
            id: id.clone(),
            title: msg.title.clone(),
            questions: msg.questions,
            players: HashMap::new(),
            state: GameState::Waiting,
        };
        games.insert(id.clone(), game);
        MessageResult(CreatedGame {
            id,
            title: msg.title,
        })
    }
}


impl GameManager {
    pub fn new() -> GameManager {
        GameManager { games: Arc::new(RwLock::new(HashMap::new())) }
    }
}


impl Actor for GameManager {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(GAME_SLEEP_INTERVAL, |act, ctx| {
            let games = act.games.read().unwrap();
            if games.len() > 0 {
                let stopped: Vec<&Identifier> = games.par_iter()
                    .filter(|(id, game)| {
                        println!("running game loop for {} ({})", game.title, game.id);
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