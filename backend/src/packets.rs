use std::collections::HashMap;
use wsbps::{packet_data, packets, VarInt};
use crate::tools::Identifier;

packet_data! {

    enum GameState (->) (u8) {
        Waiting: 0,
        Starting: 1,
        Started: 2,
        Stopped: 3,
        DoesNotExist: 4
    }

    enum PlayerDataMode (->) (u8) {
        Add: 0,
        Remove: 1,
        Me: 2
    }

    enum StateChange (<-) (u8) {
        Disconnect: 0,
        Start: 1,
        Skip: 2
    }

    struct QuestionData (<-) {
        image_type: String,
        image: Vec<u8>,
        question: String,
        values: Vec<String>,
        answers: Vec<u8>
    }
}

pub type ScoresMap = HashMap<String, u32>;

packets! {
    ServerPackets (->) {
        Disconnect (0x00) { reason: String }
        Error (0x01) { cause: String }
        JoinedGame (0x02) { id: Identifier, owner: bool, title: String}
        NameTakenResult (0x03) { result: bool }
        GameState (0x04) { state: GameState }
        PlayerData (0x05) { id: Identifier, name: String, mode: PlayerDataMode }
        TimeSync (0x06) { total: VarInt, remaining: VarInt}
        Question (0x07) { image: Vec<u8>, question: String, answers: Vec<String> }
        AnswerResult (0x08) { result: bool }
        Scores (0x09) { scores: ScoresMap }
    }

    ClientPackets (<-) {
        CreateGame (0x00) { title: String, questions: Vec<QuestionData> }
        CheckNameTaken (0x01) { id: Identifier, name: String}
        RequestGameState (0x02) { id: Identifier }
        RequestJoin (0x03) { id: Identifier, name: String }
        StateChange (0x04) { state: StateChange }
        Answer (0x05) { id: u8 }
        Kick (0x06) { id: Identifier }
    }
}
