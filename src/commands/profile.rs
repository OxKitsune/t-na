use rusqlite::{Connection, NO_PARAMS, params, Result};
use serenity::framework::standard::{
    Args, CommandResult,
    macros::command,
};
use serenity::model::prelude::*;
use serenity::prelude::*;

use log::{debug, info, trace, warn};
use crate::user::{
    TNaUser
};

#[command]
pub async fn profile(ctx: &Context, msg: &Message, args: Args) -> CommandResult {

    let user = match TNaUser::load_user(ctx, &msg.author.id).await {
        Ok(user) => {

            // Return user
            user
        },
        Err(why) => {
            warn!("Failed to load user {}", why);

            // Return user
            TNaUser::new_user(ctx, &msg.author.id).await
        }
    };

    info!("Loaded t-na user with userid: {}", user.id);

    let mut author = msg.author.clone();

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|embed| {
            embed.title(author.name);
            embed.field("Currency", user.currency, true)
        })
    }).await;

    CommandResult::Ok(())
}