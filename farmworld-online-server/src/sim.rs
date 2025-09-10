use crate::messages::{PlayerState, ServerMessage};
use bevy::prelude::*;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn test_movement_system_updates_position() {
        let mut app = App::new();
        app.add_systems(Update, movement_system);
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(1.0)); // Set delta to 1 second
        app.insert_resource(time);

        // Create a test entity with position and velocity
        let entity = app
            .world_mut()
            .spawn((Position { x: 0.0, y: 0.0 }, Velocity { dx: 1.0, dy: 2.0 }))
            .id();

        // Run the app to execute systems
        app.update();

        // Check position was updated (with speed multiplier)
        let pos = app.world().get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 300.0); // 1.0 * 300.0 * 1.0
        assert_eq!(pos.y, 600.0); // 2.0 * 300.0 * 1.0
    }

    #[test]
    fn test_movement_system_with_time_delta() {
        let mut app = App::new();
        app.add_systems(Update, movement_system);
        app.insert_resource(Time::<()>::default());

        // Set up time with a specific delta
        let mut time = app.world_mut().resource_mut::<Time>();
        time.advance_by(std::time::Duration::from_secs_f32(0.5));

        let entity = app
            .world_mut()
            .spawn((
                Position { x: 10.0, y: 20.0 },
                Velocity { dx: 2.0, dy: -1.0 },
            ))
            .id();

        app.update();

        let pos = app.world().get::<Position>(entity).unwrap();
        assert_eq!(pos.x, 310.0); // 10 + 2 * 300 * 0.5
        assert_eq!(pos.y, -130.0); // 20 + (-1) * 300 * 0.5
    }

    #[test]
    fn test_process_commands_spawn_player() {
        let mut app = App::new();
        app.add_systems(Update, process_commands);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (sim_tx, _sim_rx) = tokio::sync::mpsc::unbounded_channel();

        app.insert_resource(CommandQueue { rx });
        app.insert_resource(ServerToClientQueue { tx: sim_tx });

        let player_id = Uuid::new_v4();
        let spawn_cmd = EcsCommand::SpawnPlayer { player_id };

        // Send command
        let _ = tx.send(spawn_cmd);

        // Run the app to execute systems
        app.update();

        // Check entity was spawned
        let mut query = app.world_mut().query::<(&Player, &Position, &Velocity)>();
        let mut count = 0;
        for (player, pos, vel) in query.iter(app.world()) {
            assert_eq!(player.id, player_id);
            assert_eq!(pos.x, 0.0);
            assert_eq!(pos.y, 0.0);
            assert_eq!(vel.dx, 0.0);
            assert_eq!(vel.dy, 0.0);
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_process_commands_update_velocity() {
        let mut app = App::new();
        app.add_systems(Update, process_commands);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (sim_tx, _sim_rx) = tokio::sync::mpsc::unbounded_channel();

        app.insert_resource(CommandQueue { rx });
        app.insert_resource(ServerToClientQueue { tx: sim_tx });

        let player_id = Uuid::new_v4();

        // Spawn player first
        let entity = app
            .world_mut()
            .spawn((
                Player { id: player_id },
                Position { x: 0.0, y: 0.0 },
                Velocity { dx: 0.0, dy: 0.0 },
            ))
            .id();

        // Send update velocity command
        let update_cmd = EcsCommand::UpdateVelocity {
            player_id,
            dx: 3.0,
            dy: -1.5,
        };
        let _ = tx.send(update_cmd);

        app.update();

        // Check velocity was updated
        let vel = app.world().get::<Velocity>(entity).unwrap();
        assert_eq!(vel.dx, 3.0);
        assert_eq!(vel.dy, -1.5);
    }

    #[test]
    fn test_process_commands_despawn_player() {
        let mut app = App::new();
        app.add_systems(Update, process_commands);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (sim_tx, _sim_rx) = tokio::sync::mpsc::unbounded_channel();

        app.insert_resource(CommandQueue { rx });
        app.insert_resource(ServerToClientQueue { tx: sim_tx });

        let player_id = Uuid::new_v4();

        // Spawn player
        let entity = app
            .world_mut()
            .spawn((
                Player { id: player_id },
                Position { x: 0.0, y: 0.0 },
                Velocity { dx: 0.0, dy: 0.0 },
            ))
            .id();

        // Send despawn command
        let despawn_cmd = EcsCommand::DespawnPlayer { player_id };
        let _ = tx.send(despawn_cmd);

        app.update();

        // Check entity was despawned
        assert!(app.world().get::<Player>(entity).is_none());
    }

    #[test]
    fn test_broadcast_positions() {
        let mut app = App::new();
        app.add_systems(Update, broadcast_positions);

        let (sim_tx, mut sim_rx) = tokio::sync::mpsc::unbounded_channel();
        app.insert_resource(ServerToClientQueue { tx: sim_tx });
        app.insert_resource(BroadcastTimer {
            last_broadcast: 0.0,
        });

        // Insert Time with elapsed_seconds > 0.1 to trigger broadcast
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_secs_f32(0.2));
        app.insert_resource(time);

        let player_id1 = Uuid::new_v4();
        let player_id2 = Uuid::new_v4();

        // Spawn two players
        app.world_mut()
            .spawn((Player { id: player_id1 }, Position { x: 1.0, y: 2.0 }));

        app.world_mut()
            .spawn((Player { id: player_id2 }, Position { x: 3.0, y: 4.0 }));

        app.update();

        // Check broadcast message was sent
        let msg = sim_rx.try_recv().unwrap();
        match msg {
            ServerToClientMessage::Broadcast { message } => {
                if let ServerMessage::PlayerState { players } = message {
                    assert_eq!(players.len(), 2);
                    let player_states: std::collections::HashMap<Uuid, &PlayerState> =
                        players.iter().map(|p| (p.player_id, p)).collect();

                    let p1 = player_states.get(&player_id1).unwrap();
                    assert_eq!(p1.x, 1.0);
                    assert_eq!(p1.y, 2.0);

                    let p2 = player_states.get(&player_id2).unwrap();
                    assert_eq!(p2.x, 3.0);
                    assert_eq!(p2.y, 4.0);
                } else {
                    panic!("Expected PlayerState message");
                }
            }
            _ => panic!("Expected Broadcast message"),
        }
    }

    #[test]
    fn test_graceful_failure_invalid_player_update() {
        let mut app = App::new();
        app.add_systems(Update, process_commands);

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (sim_tx, _sim_rx) = tokio::sync::mpsc::unbounded_channel();

        app.insert_resource(CommandQueue { rx });
        app.insert_resource(ServerToClientQueue { tx: sim_tx });

        let invalid_player_id = Uuid::new_v4();

        // Try to update velocity for non-existent player
        let update_cmd = EcsCommand::UpdateVelocity {
            player_id: invalid_player_id,
            dx: 1.0,
            dy: 1.0,
        };
        let _ = tx.send(update_cmd);

        // Should not panic
        app.update();

        // World should remain unchanged
        let mut query = app.world_mut().query::<&Player>();
        assert_eq!(query.iter(app.world()).count(), 0);
    }
}

#[derive(Resource)]
pub struct CommandQueue {
    pub rx: UnboundedReceiver<EcsCommand>,
}

#[derive(Resource)]
pub struct ServerToClientQueue {
    pub tx: UnboundedSender<ServerToClientMessage>,
}

pub enum ServerToClientMessage {
    SendToClient {
        player_id: Uuid,
        message: ServerMessage,
    },
    Broadcast {
        message: ServerMessage,
    },
    PlayerDisconnected {
        player_id: Uuid,
    },
}

pub enum EcsCommand {
    SpawnPlayer { player_id: Uuid },
    DespawnPlayer { player_id: Uuid },
    UpdateVelocity { player_id: Uuid, dx: f32, dy: f32 },
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

#[derive(Resource)]
pub struct BroadcastTimer {
    pub last_broadcast: f32,
}

const PLAYER_SPEED: f32 = 300.0;

pub fn movement_system(mut query: Query<(&mut Position, &Velocity)>, time: Res<Time>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.dx * PLAYER_SPEED * time.delta_secs();
        pos.y += vel.dy * PLAYER_SPEED * time.delta_secs();
    }
}

pub fn process_commands(
    mut commands: Commands,
    mut queue: ResMut<CommandQueue>,
    sim_to_client: Res<ServerToClientQueue>,
    query: Query<(Entity, &Player, &Position)>,
) {
    while let Ok(cmd) = queue.rx.try_recv() {
        match cmd {
            EcsCommand::SpawnPlayer { player_id } => {
                commands.spawn((
                    Player { id: player_id },
                    Position { x: 0.0, y: 0.0 },
                    Velocity { dx: 0.0, dy: 0.0 },
                ));

                // Notify clients about new player
                let join_msg = ServerMessage::PlayerJoined {
                    player_id,
                    x: 0.0,
                    y: 0.0,
                };
                let _ = sim_to_client
                    .tx
                    .send(ServerToClientMessage::Broadcast { message: join_msg });

                // Send PlayerJoined for each existing player to the new client
                for (_, existing_player, pos) in query.iter() {
                    if existing_player.id != player_id {
                        let existing_join_msg = ServerMessage::PlayerJoined {
                            player_id: existing_player.id,
                            x: pos.x,
                            y: pos.y,
                        };
                        let _ = sim_to_client.tx.send(ServerToClientMessage::SendToClient {
                            player_id,
                            message: existing_join_msg,
                        });
                    }
                }
            }
            EcsCommand::DespawnPlayer { player_id } => {
                // Find and despawn entity by player_id
                let mut to_despawn = Vec::new();
                for (entity, player, _) in query.iter() {
                    if player.id == player_id {
                        to_despawn.push(entity);
                    }
                }
                for entity in to_despawn {
                    commands.entity(entity).despawn();
                }

                // Notify clients about player leaving
                let leave_msg = ServerMessage::PlayerLeft { player_id };
                let _ = sim_to_client
                    .tx
                    .send(ServerToClientMessage::Broadcast { message: leave_msg });
            }
            EcsCommand::UpdateVelocity { player_id, dx, dy } => {
                // Find entity by player_id and update velocity
                for (entity, player, _) in query.iter() {
                    if player.id == player_id {
                        commands.entity(entity).insert(Velocity { dx, dy });
                        break;
                    }
                }
            }
        }
    }
}

pub fn broadcast_positions(
    query: Query<(&Player, &Position)>,
    sim_to_client: Res<ServerToClientQueue>,
    time: Res<Time>,
    mut timer: ResMut<BroadcastTimer>,
) {
    if time.elapsed_secs() - timer.last_broadcast < 0.05 {
        return; // Skip if not enough time has passed
    }
    timer.last_broadcast = time.elapsed_secs();

    let players: Vec<PlayerState> = query
        .iter()
        .map(|(p, pos)| PlayerState {
            player_id: p.id,
            x: pos.x,
            y: pos.y,
        })
        .collect();

    let msg = ServerMessage::PlayerState { players };
    let _ = sim_to_client
        .tx
        .send(ServerToClientMessage::Broadcast { message: msg });
}
