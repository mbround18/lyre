use futures_util::{StreamExt, TryStreamExt, stream};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_postgres::{AsyncMessage, NoTls};
use tracing::{error, info};

pub async fn start_listener(ctx: Arc<serenity::all::Context>) {
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let (client, mut connection) = tokio_postgres::connect(&db_url, NoTls)
        .await
        .expect("Failed to connect to Postgres for listener");

    // Create a channel to receive messages
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Wrap poll_message in a stream and spawn it to the background
    let stream = stream::poll_fn(move |cx| connection.poll_message(cx));
    let mut connection_stream = stream.map_err(|e| panic!("Postgres connection error: {e}"));

    tokio::spawn(async move {
        while let Some(msg_result) = connection_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    let _ = tx.send(msg);
                }
                Err(e) => {
                    error!("Listener connection stream error: {}", e);
                    break;
                }
            }
        }
        error!("Postgres listener connection closed");
    });

    // Execute LISTEN command
    client
        .batch_execute("LISTEN queue_updates;")
        .await
        .expect("Failed to execute LISTEN command");
    info!("Listening for queue updates via pg_notify...");

    // Process notifications
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let AsyncMessage::Notification(notification) = message {
                let guild_id = notification.payload();
                info!("Received queue update for guild_id: {}", guild_id);
                // Trigger playback check (implemented in voice_manager)
                crate::voice_manager::process_queue_update(&ctx, guild_id).await;
            }
        }
    });
}
