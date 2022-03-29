use std::collections::HashMap;
use crate::socket::Connection;

pub struct Player<'a, 'c> {
    connect: &'a Connection<'c>,
    id: String,
    name: String,
    score: u32,
    answers: HashMap<u8, u8>,
    answer_time: time
}