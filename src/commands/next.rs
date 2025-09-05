use anyhow::{Result, anyhow};
use serenity::all::{
    CommandInteraction, Context as SerenityContext, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

pub fn definition() -> CreateCommand {
    CreateCommand::new("next").description("Skip to the next queued track")
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
            serenity::all::EditInteractionResponse::new().content("Not connected."),
        )
        .await
        .ok();
        return Ok(());
    };
    let call = call_lock.lock().await;
    let queue = call.queue();
    let res = queue.skip();
    drop(call);

    let msg = match res {
        Ok(_) => "Skipped to next.".to_string(),
        Err(e) => format!("Nothing to skip: {e}"),
    };
    cmd.edit_response(
        &ctx.http,
        serenity::all::EditInteractionResponse::new().content(msg),
    )
    .await
    .ok();
    Ok(())
}
