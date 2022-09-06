use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant, SystemTime};
use actix::Addr;
use tokio::sync::mpsc;

use crate::Connection;
use crate::game::{ClientAction, Player};
use crate::packets::{ClientPackets, GameState, QuestionData, ServerPackets};
use crate::tools::{Identifier, random_identifier};

pub struct GameManager {
    pub games: HashMap<Identifier, Game>,
    receiver: Receiver<ACP>,
    sender: Sender<ACP>,
}

pub struct ACP(Addr<Connection>, ClientPackets);

impl GameManager {
    const ID_LENGTH: usize = 5;


    fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<ACP>(32);
        Self {
            games: HashMap::new(),
            receiver,
            sender,
        }
    }

    fn sender(&self) -> Sender<MCW> {
        return self.sender.clone();
    }

    fn is_game_id(&self, id: &Identifier) -> bool {
        self.games.contains_key(id)
    }

    fn insert_game(&mut self, id: &Identifier, game: Game) {
        self.games.insert(id.clone(), game);
    }

    fn get_game_mut(&mut self, id: &Identifier) -> Option<&mut Game> {
        self.games.get_mut(id)
    }

    fn get_game(&self, id: &Identifier) -> Option<&Game> {
        self.games.get(id)
    }
}

fn handle_managers(manager: &mut GameManager) {
    tokio::spawn(async move || {
        let mut receiver = &mut manager.receiver;
        while let Some(message) = receiver.recv().await {
            let conn = message.0;
            let packet = message.1;
            let action = match packet {
                ClientPackets::CreateGame { title, questions } => {
                    let mut id: Identifier;
                    loop {
                        id = random_identifier(Game::ID_LENGTH);
                        if !manager.is_game_id(&id) { break; };
                    };
                    let mut q = Vec::with_capacity(questions.len());
                    for que in questions {
                        q.push(Question {
                            data: que,
                            start_time: None,
                        })
                    }
                    let game = Game::new(&conn, &id, &title, q);
                    manager.insert_game(&id, game);
                    ClientAction::CreatedGame {
                        id,
                        title,
                    }
                }
                ClientPackets::CheckNameTaken { id, name } => {
                    let game = manager.games.get(&id);
                    match game {
                        None => ClientAction::Error("That game code doesn't exist"),
                        Some(game) => ClientAction::NameTakenResult(game.is_name_taken(&name))
                    }
                }
                ClientPackets::RequestGameState { id } => {
                    let game = manager.games.get(&id);
                    ClientAction::Packet(ServerPackets::GameState {
                        state: match game {
                            None => GameState::DoesNotExist,
                            Some(game) => game.state.clone()
                        }
                    })
                }
                ClientPackets::RequestJoin { id, name } => {
                    let game = manager.games.get_mut(&id);
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
                ClientPackets::Kick { id } => ClientAction::BeginKick(id),
                _ => ClientAction::None
            };

            let _ = conn.send(action);
        }
    });
}


struct Question {
    data: QuestionData,
    start_time: Option<SystemTime>,
}

struct Game {
    host: Addr<Connection>,
    id: Identifier,
    title: String,
    questions: Vec<Question>,
    players: HashMap<Identifier, Player>,
    state: GameState,
    timer: GameTimer,
}

impl Game {
    fn new(
        host: &Addr<Connection>,
        id: &Identifier,
        title: &String,
        questions: Vec<Question>,
    ) -> Self {
        Self {
            host: host.clone(),
            id: id.clone(),
            title: title.clone(),
            questions,
            players: HashMap::new(),
            state: GameState::Waiting,
            timer: GameTimer::new(),
        }
    }
}


#[derive(Debug)]
struct GameTime {
    pub last_sync: Instant,
    pub game_start: Instant,
    pub need_sync: bool,
}

#[derive(Debug)]
struct GameTimer {
    pub last_sync: Instant,
    pub start: Instant,
    pub duration: Duration,
    pub elapsed: Duration,
    pub need_sync: bool,
}

impl GameTimer {
    fn new() -> Self {
        Self {
            last_sync: Instant::now(),
            start: Instant::now(),
            duration: Duration::from_secs(0),
            elapsed: Duration::from_secs(0),
            need_sync: false,
        }
    }
}