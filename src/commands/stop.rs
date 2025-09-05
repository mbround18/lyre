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
    // Stop current and clear queue
    call.stop();
    drop(call);
    // Also disconnect from the voice channel
    let manager_clone = manager.clone();
    let _ = manager_clone.remove(guild_id).await;

    cmd.edit_response(
        &ctx.http,
        EditInteractionResponse::new().content("Stopped, cleared queue, and disconnected."),
    )
    .await
    .ok();
    Ok(())
}
