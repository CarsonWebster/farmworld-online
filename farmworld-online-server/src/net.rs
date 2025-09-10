use crate::messages::ClientMessage;
use crate::sim::{EcsCommand, ServerToClientMessage};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_parse_valid_join_message() {
        let json = r#"{"action":"Join"}"#;
        let result: Result<ClientMessage, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), ClientMessage::Join));
    }

    #[test]
    fn test_parse_valid_move_message() {
        let json = r#"{"action":"Move","data":{"dx":1.5,"dy":-2.0}}"#;
        let result: Result<ClientMessage, _> = serde_json::from_str(json);
        assert!(result.is_ok());
        if let ClientMessage::Move { dx, dy } = result.unwrap() {
            assert_eq!(dx, 1.5);
            assert_eq!(dy, -2.0);
        } else {
            panic!("Expected Move message");
        }
    }

    #[test]
    fn test_parse_malformed_json() {
        let json = r#"{"action":"Move","data":{"dx":1.5,"dy":}}"#; // Missing dy value
        let result: Result<ClientMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unknown_action() {
        let json = r#"{"action":"Unknown","data":{}}"#;
        let result: Result<ClientMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_command_routing_join() {
        let (tx, mut _rx) = mpsc::unbounded_channel();
        let player_id = Uuid::new_v4();
        let json = r#"{"action":"Join"}"#;
        let client_msg: ClientMessage = serde_json::from_str(json).unwrap();

        // Simulate the routing logic from the match block
        match client_msg {
            ClientMessage::Join => {
                // Join is handled by SpawnPlayer above
            }
            ClientMessage::Move { dx, dy } => {
                let cmd = EcsCommand::UpdateVelocity { player_id, dx, dy };
                let _ = tx.send(cmd);
            }
        }

        // Verify no command was sent for Join
        assert!(matches!(_rx.try_recv(), Err(_)));
    }

    #[test]
    fn test_command_routing_move() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let player_id = Uuid::new_v4();
        let json = r#"{"action":"Move","data":{"dx":1.0,"dy":2.0}}"#;
        let client_msg: ClientMessage = serde_json::from_str(json).unwrap();

        // Simulate the routing logic
        match client_msg {
            ClientMessage::Join => {}
            ClientMessage::Move { dx, dy } => {
                let cmd = EcsCommand::UpdateVelocity { player_id, dx, dy };
                let _ = tx.send(cmd);
            }
        }

        // Verify command was sent
        let received = rx.try_recv().unwrap();
        match received {
            EcsCommand::UpdateVelocity { player_id: pid, dx, dy } => {
                assert_eq!(pid, player_id);
                assert_eq!(dx, 1.0);
                assert_eq!(dy, 2.0);
            }
            _ => panic!("Expected UpdateVelocity command"),
        }
    }

    #[test]
    fn test_graceful_failure_on_invalid_json() {
        let invalid_json = r#"{"invalid": json"#;
        let result: Result<ClientMessage, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
        // Should not panic, just return error
    }
}

pub async fn run_websocket_server(
    addr: &str,
    client_to_sim_tx: UnboundedSender<EcsCommand>,
    mut sim_to_net_rx: UnboundedReceiver<ServerToClientMessage>,
) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("WebSocket server listening on {}", addr);

    // Track all connected clients: player_id -> WebSocket sink
    let connected_clients = std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::<
        Uuid,
        futures_util::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            Message,
        >,
    >::new()));

    // Task: Receive messages from sim and route to clients
    let clients_clone = connected_clients.clone();
    tokio::spawn(async move {
        while let Some(msg) = sim_to_net_rx.recv().await {
            let mut clients = clients_clone.write().await;
            match msg {
                ServerToClientMessage::SendToClient { player_id, message } => {
                    if let Some(client_sink) = clients.get_mut(&player_id) {
                        let json = serde_json::to_string(&message).unwrap();
                        let ws_msg = Message::Text(json.into());
                        if let Err(e) = client_sink.send(ws_msg).await {
                            eprintln!("Error sending to client {}: {:?}", player_id, e);
                        }
                    }
                }
                ServerToClientMessage::Broadcast { message } => {
                    let json = serde_json::to_string(&message).unwrap();
                    let ws_msg = Message::Text(json.into());
                    for (player_id, client_sink) in clients.iter_mut() {
                        if let Err(e) = client_sink.send(ws_msg.clone()).await {
                            eprintln!("Error broadcasting to client {}: {:?}", player_id, e);
                        }
                    }
                }
                ServerToClientMessage::PlayerDisconnected { player_id } => {
                    // Client cleanup is handled when WebSocket closes
                    println!("Player {} disconnected from sim", player_id);
                }
            }
        }
    });

    // Accept new WebSocket connections
    while let Ok((stream, _)) = listener.accept().await {
        let client_to_sim_tx_clone = client_to_sim_tx.clone();
        let connected_clients_clone = connected_clients.clone();

        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.unwrap();
            println!("New WebSocket connection established");

            let (client_sink, mut client_stream) = ws_stream.split();
            let player_id = Uuid::new_v4();

            // Add client to connected clients map
            {
                let mut clients = connected_clients_clone.write().await;
                clients.insert(player_id, client_sink);
            }

            // Notify sim that player joined
            let _ = client_to_sim_tx_clone.send(EcsCommand::SpawnPlayer { player_id });

            // Handle incoming messages from this client
            while let Some(msg) = client_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            println!("Received from client {}: {:?}", player_id, client_msg);

                            match client_msg {
                                ClientMessage::Join => {
                                    // Join is handled by SpawnPlayer above
                                    println!("Player {} confirmed join", player_id);
                                }
                                ClientMessage::Move { dx, dy } => {
                                    let _ = client_to_sim_tx_clone
                                        .send(EcsCommand::UpdateVelocity { player_id, dx, dy });
                                }
                            }
                        } else {
                            eprintln!(
                                "Failed to parse message from client {}: {}",
                                player_id, text
                            );
                        }
                    }
                    Ok(Message::Close(_)) => {
                        println!("Client {} disconnected", player_id);
                        // Remove from connected clients
                        {
                            let mut clients = connected_clients_clone.write().await;
                            clients.remove(&player_id);
                        }
                        // Notify sim of disconnection
                        let _ =
                            client_to_sim_tx_clone.send(EcsCommand::DespawnPlayer { player_id });
                        break;
                    }
                    Ok(_) => {
                        // Ignore other message types (ping, pong, binary, etc.)
                    }
                    Err(e) => {
                        eprintln!("WebSocket error for client {}: {:?}", player_id, e);
                        // Remove from connected clients on error
                        {
                            let mut clients = connected_clients_clone.write().await;
                            clients.remove(&player_id);
                        }
                        let _ =
                            client_to_sim_tx_clone.send(EcsCommand::DespawnPlayer { player_id });
                        break;
                    }
                }
            }
        });
    }
}
