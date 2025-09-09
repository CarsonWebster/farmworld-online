use crate::messages::{ClientMessage, ServerMessage};
use crate::sim::EcsCommand;
use futures_util::{SinkExt, StreamExt};
// use serde_json;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use uuid::Uuid;

pub async fn run_websocket_server(addr: &str, tx: UnboundedSender<EcsCommand>) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("WebSocket server listening on {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let ws_stream = accept_async(stream).await.unwrap();
            println!("New WebSocket connection");

            let (mut write, mut read) = ws_stream.split();
            let player_id = Uuid::new_v4();

            // Create a channel for ECS → Networking
            let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

            // Tell ECS to spawn player
            tx_clone
                .send(EcsCommand::SpawnPlayer { player_id })
                .unwrap();
            tx_clone
                .send(EcsCommand::RegisterConnection {
                    player_id,
                    sender: out_tx,
                })
                .unwrap();

            // Task: forward ECS messages to WebSocket
            let mut write_task = tokio::spawn(async move {
                while let Some(msg) = out_rx.recv().await {
                    if let Err(e) = write.send(msg).await {
                        eprintln!("Error sending to client: {:?}", e);
                        break;
                    }
                }
            });

            let mut read_task = tokio::spawn(async move {
                while let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                                println!("Received: {:?}", client_msg);

                                match client_msg {
                                    ClientMessage::Join => {
                                        println!("Player {:?} joined", player_id);
                                    }
                                    ClientMessage::Move { dx, dy } => {
                                        tx_clone
                                            .send(EcsCommand::UpdateVelocity { player_id, dx, dy })
                                            .unwrap();
                                    }
                                }

                                // For now, just echo back a dummy state
                                let response = ServerMessage::PlayerState { players: vec![] };
                                let json = serde_json::to_string(&response).unwrap();
                                write.send(Message::Text(json.into())).await.unwrap();
                            }
                        }
                        Ok(Message::Close(_)) => {
                            println!("Client disconnected");
                            tx_clone
                                .send(EcsCommand::DespawnPlayer { player_id })
                                .unwrap();
                            break;
                        }
                        _ => {}
                    }
                }
            });
            // Wait for either task to finish
            tokio::select! {
                _ = (&mut write_task) => read_task.abort(),
                _ = (&mut read_task) => write_task.abort(),
            }
        });
    }
}
