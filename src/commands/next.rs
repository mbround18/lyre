use crate::metrics::METRICS;
use anyhow::{Result, anyhow};
use serenity::all::{
    CommandInteraction, Context as SerenityContext, CreateCommand, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage,
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
    if res.is_ok() {
        METRICS.dec_queue(1);
    }

    // Check if we still have songs in queue after skipping
    let queue_len_after = queue.len();
    drop(call);

    let msg = match res {
        Ok(_) => {
            if queue_len_after == 0 {
                // No more songs, disconnect
                let _ = manager.remove(guild_id).await;

                let embed = CreateEmbed::new()
                    .title("⏭️ Queue Ended")
                    .description("Skipped to next song, but the queue is now empty. Disconnected from voice channel.")
                    .colour(0xFF6B6B); // Red

                cmd.edit_response(
                    &ctx.http,
                    serenity::all::EditInteractionResponse::new().embeds(vec![embed]),
                )
                .await
                .ok();
                return Ok(());
            } else {
                let embed = CreateEmbed::new()
                    .title("⏭️ Skipped to Next")
                    .description(format!(
                        "Now playing the next song. {} song(s) remaining in queue.",
                        queue_len_after
                    ))
                    .colour(0x00FF7F); // Spring green

                cmd.edit_response(
                    &ctx.http,
                    serenity::all::EditInteractionResponse::new().embeds(vec![embed]),
                )
                .await
                .ok();
                return Ok(());
            }
        }
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
