use rusqlite::{Connection, NO_PARAMS, params, Result};
use serenity::framework::standard::{
    Args, CommandResult,
    macros::command,
};
use serenity::{
    utils::Colour
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
    let mut name = author.clone().name;

    msg.channel_id.send_message(ctx, |m| {
        m.embed(|embed| {
            embed.title(name);
            embed.thumbnail( author.clone().avatar_url().unwrap_or("https://discordapp.com/assets/322c936a8c8be1b803cd94861bdfa868.png".to_string()));
            embed.colour(Colour::BLITZ_BLUE);
            embed.field("Currency", user.currency, true)
        })
    }).await;

    CommandResult::Ok(())
}