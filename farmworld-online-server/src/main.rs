mod messages;
mod net;
mod sim;

use bevy::prelude::*;
use sim::*;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

fn main() {
    // Create ECS command channel
    let (tx, rx) = mpsc::unbounded_channel();

    // Run Bevy ECS in a separate thread
    std::thread::spawn(|| {
        App::new()
            .insert_resource(ClientConnections::new())
            .insert_resource(CommandQueue { rx })
            .add_plugins(MinimalPlugins) // no graphics
            .add_systems(
                Update,
                (process_commands, movement_system, broadcast_positions),
            )
            .run();
    });

    // Run WebSocket server on Tokio runtime
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        net::run_websocket_server("127.0.0.1:9001", tx).await;
    });
}
