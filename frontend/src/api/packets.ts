import {
    Vec,
    bool,
    ByteArray,
    MapType,
    PacketDefinition,
    Str,
    StructVec,
    u32,
    u8,
    VarInt
} from "wsbps-js"

// The different possible values for player data packet modes
export enum PlayerDataMode {ADD, REMOVE, SELF}

// An enum containing different states the client can request
// from the server
export enum States {DISCONNECT, START, SKIP}

// SERVER PACKETS
export const DisconnectPacket = new PacketDefinition(0x00, {reason: Str}, ['reason']);
export const ErrorPacket = new PacketDefinition(0x01, {cause: Str}, ['cause']);
export const JoinGamePacket = new PacketDefinition(0x02, {id: Str, owner: bool, title: Str}, ['id', 'owner', 'title']);
export const NameTakenResultPacket = new PacketDefinition(0x03, {result: bool}, ['result']);
export const GameStatePacket = new PacketDefinition(0x04, {state: u8}, ['state']);
export const PlayerDataPacket = new PacketDefinition(0x05, {id: Str, name: Str, mode: u8}, ['id', 'name', 'mode']);
export const TimeSyncPacket = new PacketDefinition(0x06, {total: VarInt, remaining: VarInt}, ['total', 'remaining']);
export const QuestionPacket = new PacketDefinition(0x07, {
    imageType: Str,
    image: ByteArray,
    question: Str,
    answers: Vec(Str),
}, ['image', 'question', 'answers']);
export const AnswerResultPacket = new PacketDefinition(0x08, {result: bool}, ['result']);
export const ScoresPacket = new PacketDefinition(0x09, {scores: MapType(Str, u32)}, ['scores']);

// CLIENT PACKETS
export const CreateGamePacket = new PacketDefinition(0x00, {
    title: Str,
    questions: StructVec({
        imageType: Str,
        image: ByteArray,
        question: Str,
        answers: Vec(Str),
        values: Vec(u8)
    }, ['imageType', 'image', 'question', 'answers', 'values'])
}, ['title', 'questions']);
export const CheckNameTakenPacket = new PacketDefinition(0x01, {id: Str, name: Str}, ['id', 'name']);
export const RequestGameStatePacket = new PacketDefinition(0x02, {id: Str}, ['id']);
export const RequestJoinPacket = new PacketDefinition(0x03, {id: Str, name: Str}, ['id', 'name']);
export const StateChangePacket = new PacketDefinition(0x04, {state: u8}, ['state']);
export const AnswerPacket = new PacketDefinition(0x05, {id: u8}, ['id']);
export const KickPacket = new PacketDefinition(0x06, {id: Str}, ['id']);