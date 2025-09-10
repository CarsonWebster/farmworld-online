use farmworld_online_server::messages::{ClientMessage, ServerMessage, PlayerState};
use farmworld_online_server::sim::{EcsCommand, ServerToClientMessage};
use tokio::sync::mpsc;
use uuid::Uuid;

#[tokio::test]
async fn test_client_to_sim_channel_flow() {
    // Create channels
    let (client_to_sim_tx, mut client_to_sim_rx) = mpsc::unbounded_channel::<EcsCommand>();
    let (sim_to_client_tx, _sim_to_client_rx) = mpsc::unbounded_channel::<ServerToClientMessage>();

    // Simulate client sending Join message
    let player_id = Uuid::new_v4();
    let join_msg = ClientMessage::Join;
    let json = serde_json::to_string(&join_msg).unwrap();

    // Simulate message parsing and routing (from net.rs logic)
    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&json) {
        match client_msg {
            ClientMessage::Join => {
                // Join handled by SpawnPlayer
                let _ = client_to_sim_tx.send(EcsCommand::SpawnPlayer { player_id });
            }
            ClientMessage::Move { dx, dy } => {
                let _ = client_to_sim_tx.send(EcsCommand::UpdateVelocity { player_id, dx, dy });
            }
        }
    }

    // Verify command was received by sim
    let received_cmd = client_to_sim_rx.recv().await.unwrap();
    match received_cmd {
        EcsCommand::SpawnPlayer { player_id: pid } => {
            assert_eq!(pid, player_id);
        }
        _ => panic!("Expected SpawnPlayer command"),
    }
}

#[tokio::test]
async fn test_sim_to_client_channel_flow() {
    // Create channels
    let (_client_to_sim_tx, _client_to_sim_rx) = mpsc::unbounded_channel::<EcsCommand>();
    let (sim_to_client_tx, mut sim_to_client_rx) = mpsc::unbounded_channel::<ServerToClientMessage>();

    // Simulate sim broadcasting player state
    let player_id = Uuid::new_v4();
    let broadcast_msg = ServerMessage::PlayerJoined {
        player_id,
        x: 10.0,
        y: 20.0,
    };

    let _ = sim_to_client_tx.send(ServerToClientMessage::Broadcast {
        message: broadcast_msg,
    });

    // Verify message was received by net layer
    let received_msg = sim_to_client_rx.recv().await.unwrap();
    match received_msg {
        ServerToClientMessage::Broadcast { message } => {
            if let ServerMessage::PlayerJoined { player_id: pid, x, y } = message {
                assert_eq!(pid, player_id);
                assert_eq!(x, 10.0);
                assert_eq!(y, 20.0);
            } else {
                panic!("Expected PlayerJoined message");
            }
        }
        _ => panic!("Expected Broadcast message"),
    }
}

#[tokio::test]
async fn test_multiple_clients_channel_flow() {
    let (client_to_sim_tx, mut client_to_sim_rx) = mpsc::unbounded_channel::<EcsCommand>();
    let (sim_to_client_tx, mut sim_to_client_rx) = mpsc::unbounded_channel::<ServerToClientMessage>();

    let player_id1 = Uuid::new_v4();
    let player_id2 = Uuid::new_v4();

    // Client 1 joins
    let _ = client_to_sim_tx.send(EcsCommand::SpawnPlayer { player_id: player_id1 });

    // Client 2 joins
    let _ = client_to_sim_tx.send(EcsCommand::SpawnPlayer { player_id: player_id2 });

    // Client 1 moves
    let _ = client_to_sim_tx.send(EcsCommand::UpdateVelocity {
        player_id: player_id1,
        dx: 1.0,
        dy: 2.0,
    });

    // Verify all commands received
    let cmd1 = client_to_sim_rx.recv().await.unwrap();
    let cmd2 = client_to_sim_rx.recv().await.unwrap();
    let cmd3 = client_to_sim_rx.recv().await.unwrap();

    match (cmd1, cmd2, cmd3) {
        (
            EcsCommand::SpawnPlayer { player_id: p1 },
            EcsCommand::SpawnPlayer { player_id: p2 },
            EcsCommand::UpdateVelocity { player_id: p3, dx, dy }
        ) => {
            assert_eq!(p1, player_id1);
            assert_eq!(p2, player_id2);
            assert_eq!(p3, player_id1);
            assert_eq!(dx, 1.0);
            assert_eq!(dy, 2.0);
        }
        _ => panic!("Commands received in wrong order or format"),
    }

    // Simulate sim broadcasting to all clients
    let broadcast_msg = ServerMessage::PlayerState {
        players: vec![
            PlayerState {
                player_id: player_id1,
                x: 1.0,
                y: 2.0,
            },
            PlayerState {
                player_id: player_id2,
                x: 3.0,
                y: 4.0,
            },
        ],
    };

    let _ = sim_to_client_tx.send(ServerToClientMessage::Broadcast {
        message: broadcast_msg,
    });

    let received = sim_to_client_rx.recv().await.unwrap();
    match received {
        ServerToClientMessage::Broadcast { message } => {
            if let ServerMessage::PlayerState { players } = message {
                assert_eq!(players.len(), 2);
                assert!(players.iter().any(|p| p.player_id == player_id1 && p.x == 1.0 && p.y == 2.0));
                assert!(players.iter().any(|p| p.player_id == player_id2 && p.x == 3.0 && p.y == 4.0));
            } else {
                panic!("Expected PlayerState message");
            }
        }
        _ => panic!("Expected Broadcast message"),
    }
}

#[tokio::test]
async fn test_channel_capacity_and_error_handling() {
    let (client_to_sim_tx, mut client_to_sim_rx) = mpsc::unbounded_channel::<EcsCommand>();

    // Send many commands to test unbounded channel
    for i in 0..1000 {
        let player_id = Uuid::new_v4();
        let cmd = EcsCommand::SpawnPlayer { player_id };
        let _ = client_to_sim_tx.send(cmd);
    }

    // Verify all commands can be received
    let mut received_count = 0;
    while let Ok(_) = client_to_sim_rx.try_recv() {
        received_count += 1;
    }

    assert_eq!(received_count, 1000);
}

#[tokio::test]
async fn test_graceful_failure_malformed_message_integration() {
    let (client_to_sim_tx, mut client_to_sim_rx) = mpsc::unbounded_channel::<EcsCommand>();

    // Simulate malformed JSON from client
    let malformed_json = r#"{"action":"Move","data":{"dx":1.0}}"#; // Missing dy

    // Simulate parsing failure (from net.rs logic)
    let parse_result: Result<ClientMessage, _> = serde_json::from_str(malformed_json);
    assert!(parse_result.is_err());

    // No command should be sent
    assert!(client_to_sim_tx.is_closed() == false); // Channel still open
    assert!(matches!(client_to_sim_rx.try_recv(), Err(_))); // No message received
}

#[tokio::test]
async fn test_player_disconnect_flow() {
    let (client_to_sim_tx, mut client_to_sim_rx) = mpsc::unbounded_channel::<EcsCommand>();
    let (sim_to_client_tx, mut sim_to_client_rx) = mpsc::unbounded_channel::<ServerToClientMessage>();

    let player_id = Uuid::new_v4();

    // Player joins
    let _ = client_to_sim_tx.send(EcsCommand::SpawnPlayer { player_id });

    // Player disconnects
    let _ = client_to_sim_tx.send(EcsCommand::DespawnPlayer { player_id });

    // Verify commands received
    let join_cmd = client_to_sim_rx.recv().await.unwrap();
    let disconnect_cmd = client_to_sim_rx.recv().await.unwrap();

    match (join_cmd, disconnect_cmd) {
        (
            EcsCommand::SpawnPlayer { player_id: j_id },
            EcsCommand::DespawnPlayer { player_id: d_id }
        ) => {
            assert_eq!(j_id, player_id);
            assert_eq!(d_id, player_id);
        }
        _ => panic!("Expected join then disconnect commands"),
    }

    // Simulate sim sending disconnect notification
    let disconnect_msg = ServerMessage::PlayerLeft { player_id };
    let _ = sim_to_client_tx.send(ServerToClientMessage::Broadcast {
        message: disconnect_msg,
    });

    let received = sim_to_client_rx.recv().await.unwrap();
    match received {
        ServerToClientMessage::Broadcast { message } => {
            if let ServerMessage::PlayerLeft { player_id: pid } = message {
                assert_eq!(pid, player_id);
            } else {
                panic!("Expected PlayerLeft message");
            }
        }
        _ => panic!("Expected Broadcast message"),
    }
}