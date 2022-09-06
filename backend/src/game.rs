use std::collections::HashMap;
use std::sync::{Arc, mpsc, RwLock, TryLockResult};
use std::time::{Duration, Instant};

use actix::*;
use actix_web::web::Data;
use log::{error, info};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use wsbps::VarInt;

use crate::Connection;
use crate::packets::{ClientPackets, GameState, PlayerDataMode, QuestionData, ServerPackets, StateChange};
use crate::packets::ServerPackets::{PlayerData};
use crate::socket::GameData;
use crate::tools::{Identifier, random_identifier};

pub type AnswerIndex = u8;
pub type QuestionIndex = u8;

pub struct GameManager {
    pub games: Arc<RwLock<HashMap<Identifier, Game>>>,
}


impl GameManager {
    const SLEEP_INTERVAL: Duration = Duration::from_secs(1);
    const START_DELAY: Duration = Duration::from_secs(5);
    const QUESTION_TIME: Duration = Duration::from_secs(10);
    const MARK_TIME: Duration = Duration::from_secs(3);
    const BONUS_TIME: Duration = Duration::from_secs(5);

    const POINTS: u32 = 100;
    const BONUS_POINTS: f32 = 200.0;

    pub fn new() -> Data<Addr<GameManager>> {
        Data::new(GameManager {
            games: Arc::new(RwLock::new(HashMap::new()))
        }.start())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
#[derive(Debug)]
enum GameChangeType {
    Remove,
    Started,
    SkipQuestion,
    NextQuestion,
    SyncTime { total: Duration, remaining: Duration },
    Continue,
}

impl Actor for GameManager {
    type Context = Context<Self>;


    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(GameManager::SLEEP_INTERVAL, |act, _ctx| {
            let arc = act.games.clone();
            let mut games = &mut arc.write().unwrap();

            let keys: Vec<&Identifier> = games.keys()
                .collect();



            let removal_keys: Vec<&Identifier> = keys.iter()
                .filter(|key| {
                    let key = **key;
                    let mut game = games.get_mut(key).unwrap();
                    game.update();

                    game.state == GameState::Stopped
                })
                .map(|key| key.clone())
                .collect();

            for key in removal_keys { // Iterate the stopped ids and remove
                if let Some(game) = games.remove(key) {
                    info!("Removed stopped game {} ({})", game.title, game.id);
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
    TryKick { id: Identifier, game_data: GameData },
    None,
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum ClientAction {
    CreatedGame { id: Identifier, title: String },
    NameTakenResult(bool),
    Packet(ServerPackets),
    Error(&'static str),
    JoinedGame { id: Identifier, player_id: Identifier, title: String },
    StateChange(StateChange),
    BeginKick(Identifier),
    Disconnect,
    Multiple(Vec<ClientAction>),
    None,
}


impl Handler<ServerAction> for GameManager {
    type Result = MessageResult<ServerAction>;

    fn handle(&mut self, msg: ServerAction, ctx: &mut Self::Context) -> Self::Result {
        MessageResult(match msg {
            ServerAction::Packet { packet, ret } => {
            }
            ServerAction::DoStateChange { state, game_data } => {
                match state {
                    StateChange::Start => {
                        if game_data.game_id.is_none() {
                            ClientAction::Error("You are not in a game.")
                        } else {
                            let mut games = self.games.write().unwrap();
                            let game_id = game_data.game_id.unwrap();
                            let game = games.get_mut(&game_id);
                            match game {
                                None => ClientAction::Multiple(vec![
                                    ClientAction::Error("You are not in a game."),
                                    ClientAction::Disconnect,
                                ]),
                                Some(game) => {
                                    game.state = GameState::Starting;
                                    game.timer.track(GameManager::START_DELAY);
                                    game.broadcast(ServerPackets::GameState { state: GameState::Starting });
                                    // let data = game_data.clone();
                                    // ctx.run_later(GameManager::START_DELAY, move |act,ctx| {
                                    //     info!("STARTING TIME! {:?}", data)
                                    // });
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
            ServerAction::TryKick { id, game_data } => {
                if !game_data.hosting {
                    ClientAction::Error("You are not the host.")
                } else {
                    if game_data.game_id.is_some() {
                        let mut games = self.games.write().unwrap();
                        let game = games.get_mut(&game_data.game_id.unwrap());
                        match game {
                            None => ClientAction::Error("You are not in a game."),
                            Some(game) => {
                                game.remove_player(id);
                                ClientAction::None
                            }
                        }
                    } else {
                        ClientAction::Error("You are not in a game.")
                    }
                }
            }
            ServerAction::None => ClientAction::None,
        })
    }
}

#[derive(Debug)]
pub struct Question {
    pub data: QuestionData,
    pub start_time: Instant,
}

#[derive(Debug)]
pub struct Game {
    pub host: Addr<Connection>,
    pub id: Identifier,
    pub title: String,
    pub questions: Vec<Question>,
    pub players: Arc<RwLock<HashMap<Identifier, Player>>>,
    pub state: GameState,
    pub timer: GameTimer,
}

impl Game {
    pub fn sync(&mut self) {
        if !self.timer.need_sync { return; }
        let now = Instant::now();
        self.timer.elapsed = now - self.timer.start;
        if self.timer.last_sync + GameTimer::SYNC_DELAY <= now {
            self.timer.last_sync = now;
            let remaining = self.timer.remaining();
            let packet = ServerPackets::TimeSync {
                total: VarInt(self.timer.duration.as_millis() as u32),
                remaining: VarInt(remaining as u32),
            };
            self.broadcast(packet);
            if remaining == 0 {
                self.timer.need_sync = false;
            }
        }
    }

    pub fn update(&mut self) {
        self.sync();
        match self.state {
            GameState::Starting => {
                if self.timer.elapsed >= GameManager::START_DELAY {
                    self.state = GameState::Started
                }
            }
            _ => {}
        };
    }
}

#[derive(Debug)]
pub struct GameTime {
    pub last_sync: Instant,
    pub game_start: Instant,
    pub need_sync: bool,
}

#[derive(Debug)]
pub struct GameTimer {
    pub last_sync: Instant,
    pub start: Instant,
    pub duration: Duration,
    pub elapsed: Duration,
    pub need_sync: bool,
}

impl GameTimer {
    const SYNC_DELAY: Duration = Duration::from_secs(2);

    pub fn new() -> GameTimer {
        GameTimer {
            last_sync: Instant::now(),
            start: Instant::now(),
            duration: Duration::from_secs(0),
            elapsed: Duration::from_secs(0),
            need_sync: false,
        }
    }

    pub fn track(&mut self, duration: Duration) {
        self.duration = duration;
        self.start = Instant::now();
        self.elapsed = Duration::from_secs(0);
        self.need_sync = true;
    }

    pub fn remaining(&self) -> u32 {
        if self.duration < self.elapsed {
            0
        } else {
            (self.duration - self.elapsed).as_millis() as u32
        }
    }
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
                player.ret.do_send(ClientAction::Multiple(vec![
                    ClientAction::Packet(ServerPackets::Disconnect { reason: String::from("Removed from game.") }),
                    ClientAction::Disconnect,
                ]))
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

    fn broadcast(&self, packet: ServerPackets) {
        let action = ClientAction::Packet(packet);
        let players = self.players.read().unwrap();
        players.values().for_each(|p| p.ret.do_send(action.clone()));
        self.host.do_send(action)
    }

    fn broadcast_excluding(&self, excluding: Identifier, packet: ServerPackets) {
        let players = self.players.read().unwrap();
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