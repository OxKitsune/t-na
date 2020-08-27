use serenity::prelude::*;
use serenity::model::prelude::*;
use serenity::framework::standard::{
    Args, CommandResult,
    macros::command,
};

#[command]
pub async fn ping(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let timestamp = msg.timestamp;
    let mut response = match msg.channel_id.say(&ctx, "pong!").await {
        Ok(response) => response,
        Err(err) => {
            println!("Errored: {}", err);
            return CommandResult::Ok(());
        }
    };

    let response_timestamp = response.timestamp;

    match response.edit(&ctx, |m| {
        m.content(format!("ping: **{}** ms", (response_timestamp.timestamp_millis() - timestamp.timestamp_millis())))
    }).await {
        Ok(res) => res,
        Err(err) => {
            println!("Errored while editing message: {}", err);
            return CommandResult::Ok(());
        }
    }

    return CommandResult::Ok(());
}
