use std::collections::HashMap;
use wsbps::{packet_data, packets, VarInt};

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
        answers: Vec<String>,
        values: Vec<u8>
    }
}

pub type ScoresMap = HashMap<String, u32>;

packets! {
    ServerPackets (->) {
        SDisconnect (0x00) { reason: String }
        SError (0x01) { cause: String }
        SJoinedGame (0x02) { id: String, owner: bool, title: String}
        SNameTakenResult (0x03) { result: bool }
        SGameState (0x04) { state: GameState }
        SPlayerData (0x05) { id: String, name: String, mode: PlayerDataMode }
        STimeSync (0x06) { total: VarInt, remaining: VarInt}
        SQuestion (0x07) { image: Vec<u8>, question: String, answers: Vec<String> }
        SAnswerResult (0x08) { result: bool }
        SScores (0x09) { scores: ScoresMap }
    }

    ClientPackets (<-) {
        CCreateGame (0x00) { title: String, questions: Vec<QuestionData> }
        CCheckNameTaken (0x01) { id: String, name: String}
        CRequestGameState (0x02) { id: String }
        CRequestJoin (0x03) { id: String, name: String }
        CStateChange (0x04) { state: StateChange }
        CAnswer (0x05) { id: u8 }
        CKick (0x06) { id: String }
    }
}
