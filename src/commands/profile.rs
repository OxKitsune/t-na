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
use image::{
    imageops::{
        overlay
    }
};

use crate::download_image;
use tokio::fs::File;

#[command]
pub async fn profile(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let user = match TNaUser::load_user(ctx, &msg.author.id).await {
        Ok(user) => {

            // Return user
            user
        }
        Err(why) => {
            warn!("Failed to load user {}", why);

            // Return user
            TNaUser::new_user(ctx, &msg.author.id).await
        }
    };

    info!("Loaded t-na user with userid: {}", user.id);


    let mut author = msg.author.clone();
    let mut name = author.clone().name;

    let mut avatar_url = author.clone().avatar_url().unwrap_or("https://discordapp.com/assets/322c936a8c8be1b803cd94861bdfa868.png".to_string());

    avatar_url = avatar_url.replace(".webp", ".png");
    avatar_url = avatar_url.replace("?size=1024", "?size=128");

    info!("avatar_url: {}", avatar_url);
    let avatar_tokens = avatar_url.split("/").collect::<Vec<&str>>();

    let avatar_path = avatar_tokens[5].split("?").collect::<Vec<&str>>()[0];

    info!("avatar_path: {}", avatar_path);
    download_image(&*avatar_url, avatar_path).await;


    match image::open(avatar_path) {
        Ok(avatar) => {
            let mut base = image::open("profile.png").unwrap();

            overlay(&mut base,  &avatar, 137, 153);

            base.save("output.png").unwrap();
            let f1 = File::open("output.png").await.unwrap();

            msg.channel_id.send_files(ctx, vec![(&f1, "profile.png")], |m| {
                m.content("Your profile :)")
            }).await;
        },
        Err(err) => warn!("errored: {}", err)
    };



    // msg.channel_id.send_message(ctx, |m| {
    //     m.embed(|embed| {
    //         embed.title(name);
    //         embed.thumbnail(author.clone().avatar_url().unwrap_or("https://discordapp.com/assets/322c936a8c8be1b803cd94861bdfa868.png".to_string()));
    //         embed.colour(Colour::BLITZ_BLUE);
    //         embed.field("Currency", user.currency, true)
    //     })
    // }).await;

    CommandResult::Ok(())
}
