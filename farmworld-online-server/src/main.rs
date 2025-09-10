mod messages;
mod net;
mod sim;

use bevy::prelude::*;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn main() {
    // Create two main communication channels
    // Channel 1: Client messages flow to Bevy ECS simulation
    let (client_to_sim_tx, client_to_sim_rx) = mpsc::unbounded_channel::<sim::EcsCommand>();

    // Channel 2: Bevy ECS simulation sends messages to network layer for clients
    let (sim_to_client_tx, sim_to_client_rx) =
        mpsc::unbounded_channel::<sim::ServerToClientMessage>();

    // Run Bevy ECS simulation in a separate thread
    std::thread::spawn(|| {
        App::new()
            .insert_resource(sim::CommandQueue {
                rx: client_to_sim_rx,
            })
            .insert_resource(sim::ServerToClientQueue {
                tx: sim_to_client_tx,
            })
            .insert_resource(sim::BroadcastTimer { last_broadcast: 0.0_f32 })
            .add_plugins(MinimalPlugins) // no graphics
            .add_systems(
                Update,
                (
                    sim::process_commands,
                    sim::movement_system,
                    sim::broadcast_positions,
                ),
            )
            .run();
    });

    // Run WebSocket server on Tokio runtime
    // Net layer owns: sender to sim, receiver from sim
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        net::run_websocket_server("127.0.0.1:9001", client_to_sim_tx, sim_to_client_rx).await;
    });
}
