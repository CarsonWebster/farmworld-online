use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", content = "data")]
pub enum ClientMessage {
    Join,
    Move { dx: f32, dy: f32 },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum ServerMessage {
    PlayerJoined { player_id: Uuid, x: f32, y: f32 },
    PlayerState { players: Vec<PlayerState> },
    PlayerLeft { player_id: Uuid },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub player_id: Uuid,
    pub x: f32,
    pub y: f32,
}
