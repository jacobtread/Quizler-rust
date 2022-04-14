use std::collections::HashMap;
use std::time;
use std::time::Duration;
use actix::prelude::*;

use crate::socket::Connection;

pub struct Player {
    id: String,
    name: String,
    score: u32,
    answers: HashMap<u8, u8>,
    answer_time: Option<time::SystemTime>,
}

impl Player {
    pub fn new() -> Player {
        return Player {
            id: String::from(""),
            name: String::from("Test"),
            score: 0,
            answers: HashMap::new(),
            answer_time: None
        };
    }
}