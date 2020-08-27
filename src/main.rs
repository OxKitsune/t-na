extern crate r2d2;
extern crate r2d2_sqlite;
extern crate rusqlite;

use std::{
    collections::HashSet,
    env,
    sync::Arc,
};

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{Connection, NO_PARAMS, params, Result};
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{
        standard::macros::group,
        StandardFramework,
    },
    http::Http,
    model::{event::PresenceUpdateEvent, event::ResumedEvent, gateway::Ready, guild::*, id::*},
    prelude::*,
};
use serenity::model::channel::Message;

use commands::{
    ping::*,
    spotify::*,
};
use util::{
    colour,
    log,
};

mod commands;
mod util;

const DB_PATH: &str = "t-na.db";

struct DBPool;

impl TypeMapKey for DBPool {
    type Value = r2d2::Pool<SqliteConnectionManager>;
}


struct Handler;

#[group]
#[commands(ping, spotify)]
struct General;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, _member: Member) {
        let mut member = _member;
        let mut role_id = RoleId(637806086732120064);

        // Log info
        log::info(format!("New member joined {}: {}#{}", guild_id, member.user.name, member.user.discriminator));

        // Assign role
        member.add_role(ctx.http, role_id).await;
        log::info(format!("   - Added role {}", role_id));
    }


    async fn ready(&self, ctx: Context, ready: Ready) {
        log::info(format!("{} is connected!", ready.user.name));
    }

    async fn presence_update(&self, ctx: Context, data: PresenceUpdateEvent) {
        let presence = data.presence;
        match presence.activity {
            Some(activity) => {
                log::info(format!("Updated {} ", activity.name));

                if activity.name == "Spotify" {
                    let song_name = activity.details.expect("No song?");
                    let assets = activity.state.expect("No artists?");
                    let artists: Vec<&str> = assets.split("; ").collect();
                    let album = activity.assets.expect("No assets?").large_text.expect("No album name?");

                    log::info(format!("Playing: {} by ({}) on {}", song_name, artists.join(", "), album));
                    add_listen(song_name, artists);
                }
            }
            None => log::info(format!("none!"))
        };
    }
}

fn add_listen(song: String, artists: Vec<&str>) {

    // Create sqlite database
    let conn = Connection::open("t-na.db").expect("Failed to open connection!");

    conn.execute("INSERT INTO song (name, listen_count) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET listen_count=listen_count+1;", params![song, 1]).expect("Failed to update played count!");

    for artist in artists {
        conn.execute("INSERT INTO artist (name, listen_count) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET listen_count=listen_count+1;", params![artist, 1]).expect("Failed to update artist count!");
    }

    conn.close().expect("Failed to close connection!");
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token in the environment");

    let http = Http::new_with_token(&token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c
            .owners(owners)
            .prefix("!"))
        .group(&GENERAL_GROUP);


    log::info("Opening database connection...".to_string());

    // Create sqlite database
    let conn = Connection::open(DB_PATH).expect("Failed to open database connection!");
    let manager = SqliteConnectionManager::file(DB_PATH);
    let pool = r2d2::Pool::new(manager).unwrap();


    pool.get().unwrap().execute(
        "CREATE TABLE IF NOT EXISTS song (
                  name            TEXT PRIMARY KEY NOT NULL,
                  listen_count    INTEGER NOT NULL
                  )",
        NO_PARAMS,
    ).expect("Failed to create table?");

    pool.get().unwrap().execute(
        "CREATE TABLE IF NOT EXISTS artist (
                  name            TEXT PRIMARY KEY NOT NULL,
                  listen_count    INTEGER NOT NULL
                  )",
        NO_PARAMS,
    ).expect("Failed to create table?");

    let mut client = Client::new(&token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Err creating client");
    {
        log::info("Inserting connection into HttpContext...".to_string());
        let mut data = client.data.write().await;
        data.insert::<DBPool>(pool);
        log::info("Done!".to_string());
    }


    if let Err(why) = client.start().await {
        log::error(format!("Client error: {:?}", why));
    }
}