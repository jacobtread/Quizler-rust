use std::collections::HashMap;
use crate::player::Player;

pub struct Game<'a> {
    players: HashMap<String, &'a Player<'a>>
}