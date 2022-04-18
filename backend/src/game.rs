use std::collections::HashMap;
use rand::Rng;
use tokio::time::Instant;

pub type Identifier = String;
pub type AnswerIndex = u8;
pub type QuestionIndex = u8;

#[derive(Debug)]
pub struct Game {
    pub id: Identifier,
    pub players: HashMap<Identifier, Player>,
}

const IDENTIFIER_CHARS: &'static [char; 16] = &['A', 'B', 'C', 'D', 'E', 'F', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

pub fn random_identifier(length: usize) -> Identifier {
    let mut out = String::with_capacity(length);
    let mut rand = rand::thread_rng();
    for _ in 0..length {
        let ran = rand.gen_range(0..16);
        out.push( IDENTIFIER_CHARS[ran])
    }
    out
}

impl Game {
    const ID_LENGTH: usize = 5;

    fn new(id: Identifier) -> Game {
        Game { id, players: HashMap::new() }
    }

    fn get_player_id(&self) -> Identifier {
        loop {
            let id = random_identifier(Player::ID_LENGTH);
            if !self.players.contains_key(&id) {
                return id
            }
        }
    }

    fn new_player(&self, name: String) -> Player {
        let id = self.get_player_id();
        Player::new(id, name)
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

    fn new(id: Identifier, name: String) -> Player {
        Player {
            id,
            name,
            score: 0,
            answers: HashMap::new(),
            answer_time: None,
        }
    }
}