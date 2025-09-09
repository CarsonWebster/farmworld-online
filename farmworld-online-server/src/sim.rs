use crate::messages::{PlayerState, ServerMessage};
use bevy::prelude::*;
// use serde_json;
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

#[derive(Resource)]
pub struct ClientConnections {
    pub clients: HashMap<Uuid, UnboundedSender<Message>>,
}
impl ClientConnections {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }
}
#[derive(Resource)]
pub struct CommandQueue {
    pub rx: UnboundedReceiver<EcsCommand>,
}

pub enum EcsCommand {
    SpawnPlayer {
        player_id: Uuid,
    },
    DespawnPlayer {
        player_id: Uuid,
    },
    UpdateVelocity {
        player_id: Uuid,
        dx: f32,
        dy: f32,
    },
    RegisterConnection {
        player_id: Uuid,
        sender: UnboundedSender<Message>,
    },
}

#[derive(Component)]
pub struct Player {
    pub id: Uuid,
}

#[derive(Component)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct Velocity {
    pub dx: f32,
    pub dy: f32,
}

pub fn movement_system(mut query: Query<(&mut Position, &Velocity)>, time: Res<Time>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.dx * time.delta_secs();
        pos.y += vel.dy * time.delta_secs();
    }
}

pub fn process_commands(
    mut commands: Commands,
    mut queue: ResMut<CommandQueue>,
    mut connections: ResMut<ClientConnections>,
) {
    while let Ok(cmd) = queue.rx.try_recv() {
        match cmd {
            EcsCommand::SpawnPlayer { player_id } => {
                commands.spawn((
                    Player { id: player_id },
                    Position { x: 0.0, y: 0.0 },
                    Velocity { dx: 0.0, dy: 0.0 },
                ));
            }
            EcsCommand::DespawnPlayer { player_id } => {
                // TODO: find entity by player_id and despawn
            }
            EcsCommand::UpdateVelocity { player_id, dx, dy } => {
                // TODO: find entity by player_id and update velocity
            }
            EcsCommand::RegisterConnection { player_id, sender } => {
                connections.clients.insert(player_id, sender);
            }
        }
    }
}

pub fn broadcast_positions(
    query: Query<(&Player, &Position)>,
    connections: Res<ClientConnections>,
) {
    let players: Vec<PlayerState> = query
        .iter()
        .map(|(p, pos)| PlayerState {
            player_id: p.id,
            x: pos.x,
            y: pos.y,
        })
        .collect();

    let msg = ServerMessage::PlayerState { players };
    let json = serde_json::to_string(&msg).unwrap();
    let ws_msg = Message::Text(json.into());

    for sender in connections.clients.values() {
        let _ = sender.send(ws_msg.clone());
    }
}
