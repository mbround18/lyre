use crate::database::establish_connection;
use crate::database::models::VoiceConnection;
use crate::metrics::METRICS;
use anyhow::{Result, anyhow};
use serenity::all::{
    CommandInteraction, Context as SerenityContext, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse,
};

pub fn definition() -> CreateCommand {
    CreateCommand::new("stop").description("Stop playback and clear the queue")
}

pub async fn handle(ctx: &SerenityContext, cmd: &CommandInteraction) -> Result<()> {
    cmd.create_response(
        &ctx.http,
        CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
    )
    .await
    .ok();

    let guild_id = cmd.guild_id.ok_or_else(|| anyhow!("not in a guild"))?;
    let manager = songbird::get(ctx).await.unwrap().clone();
    let Some(call_lock) = manager.get(guild_id) else {
        cmd.edit_response(
            &ctx.http,
            EditInteractionResponse::new().content("Not connected."),
        )
        .await
        .ok();
        return Ok(());
    };
    let mut call = call_lock.lock().await;
    // Adjust metrics with current queue length if we can get it
    let qlen = call.queue().len();
    if qlen > 0 {
        METRICS.dec_queue(qlen);
    }
    // Stop current and clear queue
    call.stop();
    drop(call);
    // Also disconnect from the voice channel
    let manager_clone = manager.clone();
    if manager_clone.remove(guild_id).await.is_ok() {
        METRICS.dec_connections();

        // Update database to remove voice connection tracking
        let mut db_conn = establish_connection();
        if let Err(e) = VoiceConnection::disconnect(&mut db_conn, &guild_id.to_string()) {
            tracing::warn!(
                "Failed to update database when disconnecting from voice: {}",
                e
            );
        }
    }

    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new().content("Stopped, cleared queue, and disconnected."),
    )
    .await
    .ok();
    Ok(())
}
