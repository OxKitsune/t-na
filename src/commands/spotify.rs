use rusqlite::{Connection, NO_PARAMS, params, Result};
use serenity::framework::standard::{
    Args, CommandResult,
    macros::command,
};
use serenity::model::prelude::*;
use serenity::prelude::*;

use log::{debug, info, trace, warn};

use crate::{DBPool, SpotifyClient};

/// Struct that stores data for the listen entries
pub struct ListenEntry {
    pub name: String,
    pub listen_count: i64,
}

#[command]
pub async fn spotify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if args.len() < 1 {
        msg.channel_id.say(ctx, "Invalid arguments!").await;
        return CommandResult::Ok(());
    }

    let first = args.current().expect("No argument specified").to_lowercase();

    if first == "songs" || first == "song" {
        let song_msg = get_songs(ctx, 10).await;
        msg.channel_id.say(ctx, song_msg).await;
    } else if first == "artists" || first == "artist" {
        let artist_msg = get_artists(ctx, 10).await;
        msg.channel_id.say(ctx, artist_msg).await;
    }
    else if first == "test" {

        let mut data = ctx.data.write().await;
        let client = data.get_mut::<SpotifyClient>().expect("Expected Connection in TypeMap.");
        let mut album = client.album("spotify:album:4xCYBJ9gRaacDBXibS7hy2").await.expect("Failed to get album");

    } else {
        msg.channel_id.say(ctx, "Invalid arguments!").await;
    }

    return CommandResult::Ok(());
}

async fn get_artists(ctx: &Context, top: i32) -> String {
    let mut data = ctx.data.write().await;
    let pool = data.get_mut::<DBPool>().expect("Expected Connection in TypeMap.");

    let conn = pool.get().unwrap();
    let mut statement = conn.prepare(&format!("SELECT * FROM artist ORDER BY listen_count DESC LIMIT {};", top)).expect("Failed to create statement!");
    let song_iter = statement.query_map(params![], |row| {
        Ok(ListenEntry {
            name: row.get(0).unwrap(),
            listen_count: row.get(1).unwrap(),
        })
    }).unwrap();

    let mut song_msg = String::from("Top listened artists:\n");
    for listen_entry in song_iter {
        let entry = listen_entry.unwrap();
        song_msg.push_str(&format!("**{}**: {}\n", entry.name, entry.listen_count));
    }

    song_msg.to_string()
}


async fn get_songs(ctx: &Context, top: i32) -> String {
    let mut data = ctx.data.write().await;
    let pool = data.get_mut::<DBPool>().expect("Expected Connection in TypeMap.");

    let conn = pool.get().unwrap();
    let mut statement = conn.prepare(&format!("SELECT * FROM song ORDER BY listen_count DESC LIMIT {};", top)).expect("Failed to create statement!");
    let song_iter = statement.query_map(params![], |row| {
        Ok(ListenEntry {
            name: row.get(0).unwrap(),
            listen_count: row.get(1).unwrap(),
        })
    }).unwrap();

    let mut song_msg = String::from("Top listened songs:\n");
    for listen_entry in song_iter {
        let entry = listen_entry.unwrap();
        song_msg.push_str(&format!("**{}**: {}\n", entry.name, entry.listen_count));
    }
    song_msg.to_string()
}